//! OpenAI Chat Completions wire format (`/v1/chat/completions`)

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ChatMessage, Error, ProviderConfig, StreamEvent};

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize)]
struct CompletionsRequest {
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct CompletionsChoice {
    message: CompletionsChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct CompletionsChoiceMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct CompletionsResponse {
    choices: Vec<CompletionsChoice>,
}

// ============================================================================
// Build / parse
// ============================================================================

pub(crate) fn build_request(messages: &[ChatMessage], _config: &ProviderConfig) -> Value {
    serde_json::to_value(CompletionsRequest {
        messages: messages.to_vec(),
    })
    .expect("serialize completions request")
}

pub(crate) fn parse_response(bytes: &[u8]) -> Result<String, Error> {
    let resp: CompletionsResponse =
        serde_json::from_slice(bytes).map_err(|e| Error::RequestFailed(e.to_string()))?;
    resp.choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or(Error::EmptyResponse)
}

pub(crate) fn parse_stream_chunk(data: &str) -> Option<StreamEvent> {
    let v: Value = serde_json::from_str(data).ok()?;
    let delta = v.get("choices")?.get(0)?.get("delta")?;
    let content = delta.get("content")?.as_str()?;
    if content.is_empty() {
        None
    } else {
        Some(StreamEvent::Delta(content.to_string()))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::test_config;

    #[test]
    fn request_shape() {
        let msgs = crate::test_messages();
        let config = test_config();
        let val = build_request(&msgs, &config);
        let arr = val["messages"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["role"], "user");
        assert_eq!(arr[0]["content"], "Hello");
        assert_eq!(arr[2]["role"], "user");
        assert!(val.get("model").is_none());
    }

    #[test]
    fn parse_response_ok() {
        let json = serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": "I'm fine!" }
            }]
        });
        let bytes = serde_json::to_vec(&json).unwrap();
        let text = parse_response(&bytes).unwrap();
        assert_eq!(text, "I'm fine!");
    }

    #[test]
    fn parse_response_empty_choices() {
        let json = serde_json::json!({ "choices": [] });
        let bytes = serde_json::to_vec(&json).unwrap();
        assert!(matches!(parse_response(&bytes), Err(Error::EmptyResponse)));
    }

    #[test]
    fn stream_chunk_ok() {
        let data = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
        let event = parse_stream_chunk(data).unwrap();
        assert!(matches!(event, StreamEvent::Delta(ref s) if s == "hello"));
    }

    #[test]
    fn stream_chunk_empty_content() {
        let data = r#"{"choices":[{"delta":{"content":""}}]}"#;
        assert!(parse_stream_chunk(data).is_none());
    }

    #[test]
    fn stream_chunk_null_content_skipped() {
        let data = r#"{"choices":[{"delta":{"role":"assistant","content":null}}]}"#;
        assert!(parse_stream_chunk(data).is_none());
    }

    #[test]
    fn stream_chunk_reasoning_content_skipped() {
        let data = r#"{"choices":[{"delta":{"reasoning_content":"thinking..."}}]}"#;
        assert!(parse_stream_chunk(data).is_none());
    }
}
