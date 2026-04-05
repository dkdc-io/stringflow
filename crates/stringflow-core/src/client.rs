//! HTTP client for AI providers.
//!
//! Async and blocking chat, streaming, and health checks.

use std::pin::Pin;

use futures_core::Stream;
use serde::Deserialize;

use crate::providers::AuthConfig;
use crate::wire_formats;
use crate::{Error, ProviderConfig, StreamEvent, WireFormat};

// ============================================================================
// Auth helpers
// ============================================================================

fn apply_auth(builder: reqwest::RequestBuilder, auth: &AuthConfig) -> reqwest::RequestBuilder {
    match auth {
        AuthConfig::None => builder,
        AuthConfig::Bearer(token) => builder.bearer_auth(token),
        AuthConfig::ApiKey { header, value } => builder.header(header.as_str(), value.as_str()),
    }
}

fn apply_auth_blocking(
    builder: reqwest::blocking::RequestBuilder,
    auth: &AuthConfig,
) -> reqwest::blocking::RequestBuilder {
    match auth {
        AuthConfig::None => builder,
        AuthConfig::Bearer(token) => builder.bearer_auth(token),
        AuthConfig::ApiKey { header, value } => builder.header(header.as_str(), value.as_str()),
    }
}

// ============================================================================
// SSE parsing
// ============================================================================

/// Parse SSE data lines from a buffer. Returns (events, remaining_buffer).
fn parse_sse_buffer(buffer: &str, format: WireFormat) -> (Vec<StreamEvent>, String) {
    let mut events = Vec::new();

    // Split on double-newline (SSE event boundaries)
    let mut parts = buffer.split("\n\n").peekable();

    while let Some(chunk) = parts.next() {
        if chunk.is_empty() {
            continue;
        }

        // Last chunk is incomplete if buffer didn't end with \n\n
        if parts.peek().is_none() && !buffer.ends_with("\n\n") {
            return (events, chunk.to_owned());
        }

        for line in chunk.lines() {
            let line = line.trim();
            if let Some(data) = line.strip_prefix("data: ") {
                let data = data.trim();
                if data == "[DONE]" {
                    events.push(StreamEvent::Done);
                } else if let Some(event) = wire_formats::parse_stream_chunk(data, format) {
                    events.push(event);
                }
            }
        }
    }

    (events, String::new())
}

// ============================================================================
// Chat
// ============================================================================

/// Max retries for transient server errors
const MAX_RETRIES: u32 = 10;
/// Base delay between retries (doubles each attempt)
const RETRY_BASE_MS: u64 = 500;
/// Per-request timeout
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
/// Read timeout for streaming requests
const STREAM_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
/// Max SSE buffer size (10 MB) to prevent unbounded growth
const MAX_SSE_BUFFER_BYTES: usize = 10 * 1024 * 1024;

/// Whether a status code is retryable (transient server errors).
fn is_retryable(status: reqwest::StatusCode) -> bool {
    matches!(
        status,
        reqwest::StatusCode::TOO_MANY_REQUESTS
            | reqwest::StatusCode::BAD_GATEWAY
            | reqwest::StatusCode::SERVICE_UNAVAILABLE
            | reqwest::StatusCode::GATEWAY_TIMEOUT
    )
}

/// Compute retry delay with pseudo-jitter to avoid thundering herd.
/// Returns `base_delay + (base_delay * (attempt % 3)) / 4`.
fn retry_delay(attempt: u32) -> std::time::Duration {
    let base = RETRY_BASE_MS.saturating_mul(2u64.pow(attempt - 1));
    let jitter = base.saturating_mul((attempt % 3) as u64) / 4;
    std::time::Duration::from_millis(base.saturating_add(jitter))
}

/// Send an async chat request. Retries on transient errors with exponential backoff + jitter.
pub async fn chat_async(
    config: &ProviderConfig,
    messages: &[crate::ChatMessage],
) -> Result<String, Error> {
    let url = wire_formats::endpoint(&config.base_url, config.wire_format);
    let body = wire_formats::build_request(messages, config)?;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| Error::Unavailable(e.to_string()))?;
    let mut last_err = Error::Unavailable("no attempts made".to_string());

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(retry_delay(attempt)).await;
        }

        let resp = apply_auth(client.post(&url), &config.auth)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Unavailable(e.to_string()))?;

        if is_retryable(resp.status()) {
            last_err = Error::RequestFailed(format!(
                "server error ({}), retrying...",
                resp.status().as_u16()
            ));
            continue;
        }

        let bytes = resp
            .error_for_status()
            .map_err(|e| Error::RequestFailed(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| Error::RequestFailed(e.to_string()))?;

        return wire_formats::parse_response(&bytes, config.wire_format);
    }

    Err(last_err)
}

/// Send a blocking chat request. Retries on transient errors with exponential backoff + jitter.
pub fn chat(config: &ProviderConfig, messages: &[crate::ChatMessage]) -> Result<String, Error> {
    let url = wire_formats::endpoint(&config.base_url, config.wire_format);
    let body = wire_formats::build_request(messages, config)?;

    let client = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| Error::Unavailable(e.to_string()))?;
    let mut last_err = Error::Unavailable("no attempts made".to_string());

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            std::thread::sleep(retry_delay(attempt));
        }

        let resp = apply_auth_blocking(client.post(&url), &config.auth)
            .json(&body)
            .send()
            .map_err(|e| Error::Unavailable(e.to_string()))?;

        if is_retryable(resp.status()) {
            last_err = Error::RequestFailed(format!(
                "server error ({}), retrying...",
                resp.status().as_u16()
            ));
            continue;
        }

        let bytes = resp
            .error_for_status()
            .map_err(|e| Error::RequestFailed(e.to_string()))?
            .bytes()
            .map_err(|e| Error::RequestFailed(e.to_string()))?;

        return wire_formats::parse_response(&bytes, config.wire_format);
    }

    Err(last_err)
}

/// Send an async streaming chat request. Returns a stream of events.
pub async fn chat_stream(
    config: &ProviderConfig,
    messages: &[crate::ChatMessage],
) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>, Error> {
    let url = wire_formats::endpoint(&config.base_url, config.wire_format);
    let mut body = wire_formats::build_request(messages, config)?;
    body.as_object_mut()
        .ok_or_else(|| Error::RequestFailed("request body is not a JSON object".to_string()))?
        .insert("stream".into(), true.into());

    let client = reqwest::Client::builder()
        .read_timeout(STREAM_READ_TIMEOUT)
        .build()
        .map_err(|e| Error::Unavailable(e.to_string()))?;

    let resp = apply_auth(client.post(&url), &config.auth)
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Unavailable(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(Error::RequestFailed(format!("HTTP {}", resp.status())));
    }

    let format = config.wire_format;
    let byte_stream = resp.bytes_stream();

    use futures_util::StreamExt;
    let event_stream = futures_util::stream::unfold(
        (byte_stream, String::new()),
        move |(mut byte_stream, mut buffer)| async move {
            type Items = Vec<Result<StreamEvent, Error>>;

            loop {
                match byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        if buffer.len() > MAX_SSE_BUFFER_BYTES {
                            let items: Items = vec![Err(Error::RequestFailed(
                                "SSE buffer exceeded 10 MB limit".to_string(),
                            ))];
                            let stream = futures_util::stream::iter(items);
                            return Some((stream, (byte_stream, String::new())));
                        }
                        let (events, remaining) = parse_sse_buffer(&buffer, format);
                        buffer = remaining;
                        if !events.is_empty() {
                            let is_done = events.iter().any(|e| matches!(e, StreamEvent::Done));
                            let items: Items = events.into_iter().map(Ok).collect();
                            let stream = futures_util::stream::iter(items);
                            if is_done {
                                return Some((stream, (byte_stream, String::new())));
                            }
                            return Some((stream, (byte_stream, buffer)));
                        }
                    }
                    Some(Err(e)) => {
                        let items: Items = vec![Err(Error::RequestFailed(e.to_string()))];
                        let stream = futures_util::stream::iter(items);
                        return Some((stream, (byte_stream, String::new())));
                    }
                    None => {
                        if !buffer.is_empty() {
                            let (events, _) = parse_sse_buffer(&buffer, format);
                            if !events.is_empty() {
                                let items: Items = events.into_iter().map(Ok).collect();
                                let stream = futures_util::stream::iter(items);
                                return Some((stream, (byte_stream, String::new())));
                            }
                        }
                        return None;
                    }
                }
            }
        },
    )
    .flatten();

    Ok(Box::pin(event_stream))
}

// ============================================================================
// Health check
// ============================================================================

/// Health check response from /health
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Send an async health check
pub async fn health_check(base_url: &str) -> Result<HealthResponse, Error> {
    let url = format!("{}/health", base_url);
    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| Error::Unavailable(e.to_string()))?
        .error_for_status()
        .map_err(|e| Error::RequestFailed(e.to_string()))?
        .json()
        .await
        .map_err(|e| Error::RequestFailed(e.to_string()))?;
    Ok(resp)
}

/// Send a blocking health check
pub fn health_check_blocking(base_url: &str) -> Result<HealthResponse, Error> {
    let url = format!("{}/health", base_url);
    let resp = reqwest::blocking::get(&url)
        .map_err(|e| Error::Unavailable(e.to_string()))?
        .error_for_status()
        .map_err(|e| Error::RequestFailed(e.to_string()))?
        .json()
        .map_err(|e| Error::RequestFailed(e.to_string()))?;
    Ok(resp)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_buffer_single_event() {
        let buffer = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n";
        let (events, remaining) = parse_sse_buffer(buffer, WireFormat::Completions);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::Delta(s) if s == "hi"));
        assert!(remaining.is_empty());
    }

    #[test]
    fn parse_sse_buffer_done_signal() {
        let buffer = "data: [DONE]\n\n";
        let (events, remaining) = parse_sse_buffer(buffer, WireFormat::Completions);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::Done));
        assert!(remaining.is_empty());
    }

    #[test]
    fn parse_sse_buffer_multiple_events() {
        let buffer = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"b\"}}]}\n\n";
        let (events, _) = parse_sse_buffer(buffer, WireFormat::Completions);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn parse_sse_buffer_incomplete_chunk() {
        let buffer = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\ndata: partial";
        let (events, remaining) = parse_sse_buffer(buffer, WireFormat::Completions);
        assert_eq!(events.len(), 1);
        assert_eq!(remaining, "data: partial");
    }

    #[test]
    fn parse_sse_buffer_with_event_prefix() {
        let buffer = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n";
        let (events, _) = parse_sse_buffer(buffer, WireFormat::Messages);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::Delta(s) if s == "hi"));
    }
}
