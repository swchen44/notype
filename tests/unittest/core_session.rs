use notype_core::{
    CancellationToken, SessionStatus, TextFailure, TextFuture, TextProcessor, TextRequest,
    TextSelection,
};
use notype_core::{DeliveryAdapter, DeliveryFuture, DeliveryOutcome, DeliveryRequest};
use notype_core::{RecognitionMode, SessionOptions, SessionService};
use std::sync::Arc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tokio::sync::Notify;

fn options() -> SessionOptions {
    SessionOptions {
        recognition_mode: RecognitionMode::TraditionalEnglish,
        original_target: Some("test-control-1".into()),
    }
}

struct InspectOriginal {
    service: SessionService,
    id: String,
}
impl TextProcessor for InspectOriginal {
    fn polish(&self, request: TextRequest, _: CancellationToken) -> TextFuture {
        let entry = self.service.get(&self.id).unwrap();
        assert_eq!(entry.transcript, "不要刪除 42 個 API request");
        assert_eq!(entry.status, SessionStatus::Processing);
        assert_eq!(
            request,
            TextRequest {
                transcript: "不要刪除 42 個 API request".into(),
                glossary: vec!["API".into()],
                recognition_mode: Some(RecognitionMode::TraditionalEnglish)
            }
        );
        Box::pin(async { Ok("不要刪除 42 個 API request。".into()) })
    }
}

#[tokio::test]
async fn original_is_saved_before_processing_and_success_remains_pending() {
    let service = SessionService::in_memory().unwrap();
    let entry = service
        .create("不要刪除 42 個 API request", options())
        .unwrap();
    let provider = Arc::new(InspectOriginal {
        service: service.clone(),
        id: entry.id.clone(),
    });
    let result = service
        .process(&entry.id, vec!["API".into()], provider)
        .await
        .unwrap();
    assert_eq!(result.status, SessionStatus::Pending);
    assert_eq!(
        result.result.as_deref(),
        Some("不要刪除 42 個 API request。")
    );
    assert_eq!(
        service.get(&entry.id).unwrap().transcript,
        "不要刪除 42 個 API request"
    );
}

struct Reply(Result<String, TextFailure>);

#[tokio::test]
async fn missing_configuration_remains_a_persisted_recoverable_failure() {
    let service = SessionService::in_memory().unwrap();
    let entry = service.create("原稿 API 42", options()).unwrap();
    let failed = service
        .process(
            &entry.id,
            vec![],
            Arc::new(Reply(Err(TextFailure::ConfigurationRequired))),
        )
        .await
        .unwrap();
    assert_eq!(failed.status, SessionStatus::Failed);
    assert_eq!(
        service.get(&entry.id).unwrap().error.as_deref(),
        Some("configuration-required")
    );
    assert_eq!(failed.transcript, entry.transcript);
    assert_eq!(failed.result, None);
    assert_eq!(
        service.choose_original(&entry.id).unwrap().selection,
        TextSelection::Original
    );
}
impl TextProcessor for Reply {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        let response = self.0.clone();
        Box::pin(async move { response })
    }
}

#[tokio::test]
async fn timeout_keeps_original_and_only_explicit_choice_makes_it_pending() {
    let service = SessionService::in_memory().unwrap();
    let entry = service.create("不要刪除 42", options()).unwrap();
    let failed = service
        .process(
            &entry.id,
            vec![],
            Arc::new(Reply(Err(TextFailure::Timeout))),
        )
        .await
        .unwrap();
    assert_eq!(failed.status, SessionStatus::Failed);
    assert_eq!(failed.error.as_deref(), Some("llm-timeout"));
    assert_eq!(failed.transcript, "不要刪除 42");
    assert_eq!(failed.result, None);
    let input = Arc::new(TestInput {
        focused: AtomicBool::new(true),
        value: Mutex::new(String::new()),
    });
    assert_eq!(
        service
            .deliver(&entry.id, input.clone())
            .await
            .unwrap()
            .status,
        SessionStatus::Failed
    );
    assert_eq!(*input.value.lock().unwrap(), "");
    let chosen = service.choose_original(&entry.id).unwrap();
    assert_eq!(chosen.status, SessionStatus::Pending);
    assert_eq!(chosen.selection, TextSelection::Original);
    assert_eq!(chosen.transcript, "不要刪除 42");
    assert_eq!(
        service
            .deliver(&entry.id, input.clone())
            .await
            .unwrap()
            .status,
        SessionStatus::Delivered
    );
    assert_eq!(*input.value.lock().unwrap(), "不要刪除 42");
}

struct Deferred {
    started: Arc<Notify>,
    release: Arc<Notify>,
}
impl TextProcessor for Deferred {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        let started = self.started.clone();
        let release = self.release.clone();
        Box::pin(async move {
            started.notify_one();
            release.notified().await;
            Ok("遲到結果".into())
        })
    }
}

#[tokio::test]
async fn cancellation_suppresses_a_provider_that_ignores_its_cancellation_token() {
    let service = SessionService::in_memory().unwrap();
    let entry = service.create("取消這份原稿", options()).unwrap();
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let worker = service.clone();
    let id = entry.id.clone();
    let processor = Arc::new(Deferred {
        started: started.clone(),
        release: release.clone(),
    });
    let task = tokio::spawn(async move { worker.process(&id, vec![], processor).await });
    started.notified().await;
    assert_eq!(
        service.cancel(&entry.id).unwrap().status,
        SessionStatus::Cancelled
    );
    release.notify_one();
    assert_eq!(
        task.await.unwrap().unwrap().status,
        SessionStatus::Cancelled
    );
    assert_eq!(service.get(&entry.id).unwrap().result, None);
}

struct TestInput {
    focused: AtomicBool,
    value: Mutex<String>,
}
impl DeliveryAdapter for TestInput {
    fn deliver(&self, request: DeliveryRequest) -> DeliveryFuture {
        assert_eq!(request.original_target, "test-control-1");
        let outcome = if self.focused.load(Ordering::SeqCst) {
            self.value.lock().unwrap().push_str(&request.text);
            DeliveryOutcome::Delivered
        } else {
            DeliveryOutcome::TargetChanged
        };
        Box::pin(async move { outcome })
    }
}

#[tokio::test]
async fn changed_target_stays_pending_and_successful_delivery_is_not_repeated() {
    let service = SessionService::in_memory().unwrap();
    let entry = service.create("原稿", options()).unwrap();
    service
        .process(&entry.id, vec![], Arc::new(Reply(Ok("整理結果".into()))))
        .await
        .unwrap();
    let input = Arc::new(TestInput {
        focused: AtomicBool::new(false),
        value: Mutex::new(String::new()),
    });
    assert_eq!(
        service
            .deliver(&entry.id, input.clone())
            .await
            .unwrap()
            .status,
        SessionStatus::Pending
    );
    assert_eq!(*input.value.lock().unwrap(), "");
    input.focused.store(true, Ordering::SeqCst);
    assert_eq!(
        service
            .deliver(&entry.id, input.clone())
            .await
            .unwrap()
            .status,
        SessionStatus::Delivered
    );
    service.deliver(&entry.id, input.clone()).await.unwrap();
    assert_eq!(*input.value.lock().unwrap(), "整理結果");
}

#[test]
fn recognized_original_is_retrievable_with_its_captured_mode_and_target() {
    let service = SessionService::in_memory().unwrap();
    let created = service
        .create("不要刪除 42 個 API request", options())
        .unwrap();
    let saved = service.get(&created.id).unwrap();
    assert_eq!(saved.transcript, "不要刪除 42 個 API request");
    assert_eq!(
        saved.recognition_mode,
        Some(RecognitionMode::TraditionalEnglish)
    );
    assert_eq!(saved.original_target.as_deref(), Some("test-control-1"));
}

#[tokio::test]
async fn history_search_matches_both_texts_literally_and_deletion_stops_active_work() {
    let service = SessionService::in_memory().unwrap();
    let entry = service.create("原稿 100% API_42", options()).unwrap();
    service
        .process(
            &entry.id,
            vec![],
            Arc::new(Reply(Ok("整理結果 launch".into()))),
        )
        .await
        .unwrap();
    service.create("unrelated", options()).unwrap();
    assert_eq!(service.search("100%").unwrap()[0].id, entry.id);
    assert_eq!(
        service.search("launch").unwrap()[0].transcript,
        "原稿 100% API_42"
    );
    assert!(service.search("nonexistent").unwrap().is_empty());
    assert_eq!(service.search("").unwrap().len(), 2);
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let worker = service.clone();
    let id = entry.id.clone();
    let processor = Arc::new(Deferred {
        started: started.clone(),
        release,
    });
    let task = tokio::spawn(async move { worker.process(&id, vec![], processor).await });
    started.notified().await;
    service.delete(&entry.id).unwrap();
    assert!(matches!(
        task.await.unwrap(),
        Err(notype_core::CoreError::NotFound)
    ));
    assert!(service.search("API").unwrap().is_empty());
    assert!(matches!(
        service.get(&entry.id),
        Err(notype_core::CoreError::NotFound)
    ));
}

#[tokio::test]
async fn aborting_processing_releases_the_session_for_retry_from_its_saved_original() {
    let service = SessionService::in_memory().unwrap();
    let entry = service
        .create("不要刪除 42 個 API request", options())
        .unwrap();
    let started = Arc::new(Notify::new());
    let worker = service.clone();
    let id = entry.id.clone();
    let processor = Arc::new(Deferred {
        started: started.clone(),
        release: Arc::new(Notify::new()),
    });
    let task = tokio::spawn(async move { worker.process(&id, vec![], processor).await });
    started.notified().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let interrupted = service.get(&entry.id).unwrap();
    assert_eq!(interrupted.status, SessionStatus::Failed);
    assert_eq!(interrupted.error.as_deref(), Some("interrupted"));
    let retried = service
        .process(
            &entry.id,
            vec!["API".into()],
            Arc::new(InspectOriginal {
                service: service.clone(),
                id: entry.id.clone(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(retried.status, SessionStatus::Pending);
    assert_eq!(retried.id, entry.id);
    assert_eq!(retried.created_at, entry.created_at);
    assert_eq!(retried.recognition_mode, entry.recognition_mode);
}

struct DeferredDelivery {
    started: Arc<Notify>,
    value: Arc<Mutex<String>>,
}
impl DeliveryAdapter for DeferredDelivery {
    fn deliver(&self, request: DeliveryRequest) -> DeliveryFuture {
        let started = self.started.clone();
        let value = self.value.clone();
        Box::pin(async move {
            value.lock().unwrap().push_str(&request.text);
            started.notify_one();
            std::future::pending().await
        })
    }
}

#[tokio::test]
async fn aborted_delivery_is_uncertain_and_cannot_resend_even_when_text_may_have_been_inserted() {
    let service = SessionService::in_memory().unwrap();
    let entry = service.create("原稿不能重送", options()).unwrap();
    service.choose_original(&entry.id).unwrap();
    let started = Arc::new(Notify::new());
    let value = Arc::new(Mutex::new(String::new()));
    let adapter = Arc::new(DeferredDelivery {
        started: started.clone(),
        value: value.clone(),
    });
    let worker = service.clone();
    let id = entry.id.clone();
    let task = tokio::spawn(async move { worker.deliver(&id, adapter).await });
    started.notified().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(
        service.get(&entry.id).unwrap().status,
        SessionStatus::DeliveryUnknown
    );
    let unexpected_input = Arc::new(TestInput {
        focused: AtomicBool::new(true),
        value: Mutex::new(String::new()),
    });
    service
        .deliver(&entry.id, unexpected_input.clone())
        .await
        .unwrap();
    assert_eq!(*unexpected_input.value.lock().unwrap(), "");
    assert_eq!(*value.lock().unwrap(), "原稿不能重送");
    assert!(matches!(
        service
            .process(&entry.id, vec![], Arc::new(Reply(Ok("again".into()))))
            .await,
        Err(notype_core::CoreError::InvalidTransition)
    ));
}
