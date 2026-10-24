use crate::{CancellationToken, CoreError, TextFailure, TextFuture, TextProcessor, TextRequest};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Development protocol only: POST {transcript, glossary} -> {text}.
/// Company authentication, prompts and response compatibility belong to its
/// verified desktop adapter, not this fixture transport.
#[derive(Clone)]
pub struct HttpTextProcessor {
    client: reqwest::Client,
    endpoint: reqwest::Url,
}

impl HttpTextProcessor {
    pub fn new(endpoint: &str, timeout: Duration) -> Result<Self, CoreError> {
        let endpoint = reqwest::Url::parse(endpoint).map_err(|_| CoreError::InvalidInput)?;
        let loopback = endpoint.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        });
        if timeout.is_zero()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.fragment().is_some()
            || !(endpoint.scheme() == "https" || endpoint.scheme() == "http" && loopback)
        {
            return Err(CoreError::InvalidInput);
        }
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| CoreError::InvalidInput)?;
        Ok(Self { client, endpoint })
    }
}

#[derive(Serialize)]
struct RequestBody {
    transcript: String,
    glossary: Vec<String>,
}
#[derive(Deserialize)]
struct ResponseBody {
    text: String,
}

fn failure(error: reqwest::Error) -> TextFailure {
    if error.is_timeout() {
        TextFailure::Timeout
    } else {
        TextFailure::Failed
    }
}

impl TextProcessor for HttpTextProcessor {
    fn polish(&self, request: TextRequest, cancellation: CancellationToken) -> TextFuture {
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        Box::pin(async move {
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => Err(TextFailure::Cancelled),
                response = async move {
                    let mut response = client.post(endpoint).json(&RequestBody { transcript: request.transcript, glossary: request.glossary }).send().await.map_err(failure)?;
                    if !response.status().is_success() { return Err(TextFailure::Failed); }
                    const MAX_RESPONSE_BYTES: usize = 1_048_576;
                    let mut bytes = Vec::new();
                    while let Some(chunk) = response.chunk().await.map_err(failure)? {
                        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES { return Err(TextFailure::InvalidResponse); }
                        bytes.extend_from_slice(&chunk);
                    }
                    let response: ResponseBody = serde_json::from_slice(&bytes).map_err(|_|TextFailure::InvalidResponse)?;
                    if response.text.trim().is_empty() { return Err(TextFailure::InvalidResponse); }
                    Ok(response.text)
                } => response,
            }
        })
    }
}
