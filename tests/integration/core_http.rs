use notype_core::{
    HttpTextProcessor, RecognitionMode, SessionOptions, SessionService, SessionStatus,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{mpsc, Arc},
    time::Duration,
};

fn fixture(
    status: &str,
    body: &str,
    delay: Duration,
) -> (
    String,
    mpsc::Receiver<serde_json::Value>,
    std::thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    let status = status.to_owned();
    let body = body.to_owned();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0; 2048];
        let (header_end, length) = loop {
            let size = stream.read(&mut buffer).unwrap();
            assert!(size > 0);
            bytes.extend_from_slice(&buffer[..size]);
            if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&bytes[..end]).unwrap();
                assert!(headers.starts_with("POST /polish HTTP/1.1"));
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|value| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                break (end + 4, length);
            }
        };
        while bytes.len() < header_end + length {
            let size = stream.read(&mut buffer).unwrap();
            assert!(size > 0);
            bytes.extend_from_slice(&buffer[..size]);
        }
        sender
            .send(serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap())
            .unwrap();
        std::thread::sleep(delay);
        let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
        let _ = stream.write_all(response.as_bytes());
    });
    (format!("http://{address}/polish"), receiver, server)
}

fn session(service: &SessionService) -> notype_core::SessionEntry {
    service
        .create(
            "不要刪除 API 42",
            SessionOptions {
                recognition_mode: RecognitionMode::TraditionalEnglish,
                original_target: Some("private-target".into()),
            },
        )
        .unwrap()
}

#[tokio::test]
async fn complete_session_uses_real_http_with_only_saved_original_and_manual_glossary() {
    let (endpoint, requests, server) =
        fixture("200 OK", r#"{"text":"不要刪除 API 42。"}"#, Duration::ZERO);
    let service = SessionService::in_memory().unwrap();
    let entry = session(&service);
    let processor = Arc::new(HttpTextProcessor::new(&endpoint, Duration::from_secs(2)).unwrap());
    let result = service
        .process(&entry.id, vec!["API".into()], processor)
        .await
        .unwrap();
    assert_eq!(result.status, SessionStatus::Pending);
    assert_eq!(result.transcript, "不要刪除 API 42");
    assert_eq!(result.result.as_deref(), Some("不要刪除 API 42。"));
    assert_eq!(
        requests.recv().unwrap(),
        serde_json::json!({"transcript":"不要刪除 API 42","glossary":["API"]})
    );
    server.join().unwrap();
}

#[tokio::test]
async fn invalid_http_reply_fails_closed_with_sanitized_error_and_saved_original() {
    let (endpoint, requests, server) = fixture(
        "200 OK",
        r#"{"error":"private-diagnostic"}"#,
        Duration::ZERO,
    );
    let service = SessionService::in_memory().unwrap();
    let entry = session(&service);
    let result = service
        .process(
            &entry.id,
            vec![],
            Arc::new(HttpTextProcessor::new(&endpoint, Duration::from_secs(2)).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(result.status, SessionStatus::Failed);
    assert_eq!(result.error.as_deref(), Some("llm-invalid-response"));
    assert_eq!(result.result, None);
    assert_eq!(result.transcript, entry.transcript);
    requests.recv().unwrap();
    server.join().unwrap();
}

#[tokio::test]
async fn real_http_timeout_ends_attempt_without_losing_original() {
    let (endpoint, requests, server) = fixture(
        "200 OK",
        r#"{"text":"too late"}"#,
        Duration::from_millis(250),
    );
    let service = SessionService::in_memory().unwrap();
    let entry = session(&service);
    let result = service
        .process(
            &entry.id,
            vec![],
            Arc::new(HttpTextProcessor::new(&endpoint, Duration::from_millis(100)).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(result.status, SessionStatus::Failed);
    assert_eq!(result.error.as_deref(), Some("llm-timeout"));
    assert_eq!(result.transcript, entry.transcript);
    assert_eq!(result.result, None);
    requests.recv().unwrap();
    server.join().unwrap();
}
