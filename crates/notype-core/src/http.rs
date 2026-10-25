use crate::observations::{
    ApiCallDescriptor, ApiCallGuard, ApiCallObserver, ApiOutcome, ApiProviderProfile, ApiReceipt,
};
use crate::{
    CancellationToken, CoreError, GenerationFuture, TextFailure, TextFuture, TextOutput,
    TextProcessor, TextRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
        self.send(request, cancellation, None)
    }
    fn generate_observed(
        &self,
        request: TextRequest,
        cancellation: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> GenerationFuture {
        let future = self.send(request, cancellation, Some(observer));
        Box::pin(async move {
            future.await.map(|text| TextOutput {
                text,
                source_language: None,
            })
        })
    }
}
impl HttpTextProcessor {
    fn send(
        &self,
        request: TextRequest,
        cancellation: CancellationToken,
        observer: Option<Arc<dyn ApiCallObserver>>,
    ) -> TextFuture {
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(TextFailure::Cancelled);
            }
            let wire = serde_json::to_vec(&RequestBody {
                transcript: request.transcript,
                glossary: request.glossary,
            })
            .map_err(|_| TextFailure::ConfigurationRequired)?;
            let descriptor = ApiCallDescriptor {
                profile: ApiProviderProfile::Development,
                wire_bytes: wire.len() as u64,
                ..Default::default()
            };
            let mut guard = observer
                .map(|observer| ApiCallGuard::begin(observer, descriptor))
                .transpose()?;
            let request = client
                .post(endpoint)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(wire);
            let operation = async {
                if let Some(guard) = &mut guard {
                    guard.mark_sent()?;
                }
                let mut response = request.send().await.map_err(failure)?;
                let success = response.status().is_success();
                let mut bytes = Vec::new();
                while let Some(chunk) = response.chunk().await.map_err(failure)? {
                    if bytes.len().saturating_add(chunk.len()) > 1_048_576 {
                        return Err(TextFailure::InvalidResponse);
                    }
                    bytes.extend_from_slice(&chunk);
                }
                let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| {
                    if success {
                        TextFailure::InvalidResponse
                    } else {
                        TextFailure::Failed
                    }
                })?;
                if let Some(guard) = &mut guard {
                    guard.response(ApiReceipt::from_chat_json(&value));
                }
                if !success {
                    return Err(TextFailure::Failed);
                }
                let response: ResponseBody =
                    serde_json::from_value(value).map_err(|_| TextFailure::InvalidResponse)?;
                if response.text.trim().is_empty() {
                    return Err(TextFailure::InvalidResponse);
                }
                Ok(response.text)
            };
            let result = tokio::select! {biased; _=cancellation.cancelled()=>Err(TextFailure::Cancelled),response=operation=>response};
            if let Some(guard) = guard {
                guard.finish(match &result {
                    Ok(_) => ApiOutcome::Success,
                    Err(TextFailure::Cancelled) => ApiOutcome::Cancelled,
                    Err(TextFailure::Timeout) => ApiOutcome::Timeout,
                    Err(TextFailure::InvalidResponse) => ApiOutcome::InvalidResponse,
                    Err(_) => ApiOutcome::Failed,
                });
            }
            result
        })
    }
}
