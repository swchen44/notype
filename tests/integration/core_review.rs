use notype_core::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct Translator {
    service: SessionService,
    id: String,
    back_started: Arc<tokio::sync::Notify>,
    hold: bool,
}
impl TextProcessor for Translator {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Ok("primary".into()) })
    }
    fn generate(&self, _: TextRequest, _: CancellationToken) -> GenerationFuture {
        Box::pin(async {
            Ok(TextOutput {
                text: "primary".into(),
                source_language: Some("zh-TW".into()),
            })
        })
    }
    fn back_translate(
        &self,
        request: BackTranslationRequest,
        cancel: CancellationToken,
    ) -> TextFuture {
        assert_eq!(
            self.service.get(&self.id).unwrap().result.as_deref(),
            Some("primary")
        );
        assert_eq!(request.text, "primary");
        assert_eq!(request.target_language, "zh-TW");
        let signal = self.back_started.clone();
        let hold = self.hold;
        Box::pin(async move {
            signal.notify_one();
            if hold {
                cancel.cancelled().await;
                Err(TextFailure::Cancelled)
            } else {
                Ok("back".into())
            }
        })
    }
}
struct Delivery(Arc<AtomicUsize>);
impl DeliveryAdapter for Delivery {
    fn deliver(&self, _: DeliveryRequest) -> DeliveryFuture {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { DeliveryOutcome::TargetChanged })
    }
}
fn translated(service: &SessionService) -> SessionEntry {
    service
        .create(
            "original",
            SessionOptions {
                scenario: Scenario::Translation,
                translation_target: Some("en-US".into()),
                original_target: Some("captured-target".into()),
                ..Default::default()
            },
        )
        .unwrap()
}

#[tokio::test]
async fn review_confirmation_binds_displayed_pair_revision_and_still_uses_target_guard() {
    let service = SessionService::in_memory().unwrap();
    let original = translated(&service);
    let processor = Arc::new(Translator {
        service: service.clone(),
        id: original.id.clone(),
        back_started: Arc::new(tokio::sync::Notify::new()),
        hold: false,
    });
    let pair = service
        .process_with_review(&original.id, vec![], processor.clone(), true)
        .await
        .unwrap();
    assert_eq!(pair.back_translation.as_deref(), Some("back"));
    assert!(pair.review_required);
    assert!(!pair.review_confirmed);
    let count = Arc::new(AtomicUsize::new(0));
    let delivery = Arc::new(Delivery(count.clone()));
    assert_eq!(
        service
            .deliver(&pair.id, delivery.clone())
            .await
            .unwrap()
            .status,
        SessionStatus::Pending
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
    service.confirm_review(&pair.id, pair.revision).unwrap();
    let newer = service
        .process_with_review(&pair.id, vec![], processor, true)
        .await
        .unwrap();
    assert!(service.confirm_review(&pair.id, pair.revision).is_err());
    assert!(!newer.review_confirmed);
    service.confirm_review(&pair.id, newer.revision).unwrap();
    let guarded = service.deliver(&pair.id, delivery).await.unwrap();
    assert_eq!(guarded.error.as_deref(), Some("target-changed"));
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cancellation_during_back_step_preserves_committed_primary_across_restart_and_never_resurrects_deleted_row(
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let service = SessionService::open(&path).unwrap();
    let original = translated(&service);
    let signal = Arc::new(tokio::sync::Notify::new());
    let worker = service.clone();
    let id = original.id.clone();
    let processor = Arc::new(Translator {
        service: service.clone(),
        id: id.clone(),
        back_started: signal.clone(),
        hold: true,
    });
    let task = tokio::spawn(async move {
        worker
            .process_with_review(&id, vec![], processor, true)
            .await
    });
    signal.notified().await;
    let during = service.get(&original.id).unwrap();
    assert_eq!(during.result.as_deref(), Some("primary"));
    assert_eq!(during.status, SessionStatus::Processing);
    task.abort();
    let _ = task.await;
    drop(service);
    let reopened = SessionService::open(&path).unwrap();
    let saved = reopened.get(&original.id).unwrap();
    assert_eq!(saved.result.as_deref(), Some("primary"));
    assert_eq!(saved.status, SessionStatus::Failed);
    assert!(!saved.review_confirmed);
    let signal = Arc::new(tokio::sync::Notify::new());
    let worker = reopened.clone();
    let id = original.id.clone();
    let processor = Arc::new(Translator {
        service: reopened.clone(),
        id: id.clone(),
        back_started: signal.clone(),
        hold: true,
    });
    let task = tokio::spawn(async move {
        worker
            .process_with_review(&id, vec![], processor, true)
            .await
    });
    signal.notified().await;
    reopened.delete(&original.id).unwrap();
    assert_eq!(task.await.unwrap().unwrap_err(), CoreError::NotFound);
    assert!(reopened.search("").unwrap().is_empty());
}

struct MissingSource;
impl TextProcessor for MissingSource {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Ok("primary without known source".into()) })
    }
}
#[tokio::test]
async fn failed_back_step_keeps_primary_but_requires_retry_or_explicit_original_and_revision_confirmation(
) {
    let service = SessionService::in_memory().unwrap();
    let original = translated(&service);
    let failed = service
        .process_with_review(&original.id, vec![], Arc::new(MissingSource), true)
        .await
        .unwrap();
    assert_eq!(
        failed.result.as_deref(),
        Some("primary without known source")
    );
    assert_eq!(failed.error.as_deref(), Some("back-translation-failed"));
    assert_eq!(failed.status, SessionStatus::Pending);
    assert!(service.confirm_review(&failed.id, failed.revision).is_err());
    let raw = service.choose_original(&failed.id).unwrap();
    assert!(service.confirm_review(&raw.id, failed.revision).is_err());
    service.confirm_review(&raw.id, raw.revision).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    service
        .deliver(&raw.id, Arc::new(Delivery(count.clone())))
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn source_context_rejects_full_urls_before_saving_any_history() {
    let service = SessionService::in_memory().unwrap();
    assert_eq!(
        service
            .create(
                "raw",
                SessionOptions {
                    source_context: SourceContext {
                        application: Some("Browser".into()),
                        website_domain: Some("https://example.test/private?input=secret".into())
                    },
                    ..Default::default()
                }
            )
            .unwrap_err(),
        CoreError::InvalidInput
    );
    assert!(service.search("").unwrap().is_empty());
}
