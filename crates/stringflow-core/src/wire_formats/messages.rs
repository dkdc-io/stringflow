//! Anthropic Messages wire format (`/v1/messages`)

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ChatMessage, DEFAULT_MAX_TOKENS, DEFAULT_MODEL, Error, ProviderConfig, StreamEvent};

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
}

/// A content block in the Anthropic response.
/// `"type": "thinking"` blocks lack `text`, so it's optional.
#[derive(Debug, Deserialize)]
struct MessagesContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<MessagesContentBlock>,
}

// ============================================================================
// Build / parse
// ============================================================================

pub(crate) fn build_request(
    messages: &[ChatMessage],
    config: &ProviderConfig,
) -> Result<Value, Error> {
    serde_json::to_value(MessagesRequest {
        model: config
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        messages: messages.to_vec(),
        max_tokens: config.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
    })
    .map_err(|e| Error::RequestFailed(e.to_string()))
}

pub(crate) fn parse_response(bytes: &[u8]) -> Result<String, Error> {
    let resp: MessagesResponse =
        serde_json::from_slice(bytes).map_err(|e| Error::RequestFailed(e.to_string()))?;
    resp.content
        .into_iter()
        .find(|b| b.content_type == "text" && b.text.is_some())
        .and_then(|b| b.text)
        .ok_or(Error::EmptyResponse)
}

pub(crate) fn parse_stream_chunk(data: &str) -> Option<StreamEvent> {
    let v: Value = serde_json::from_str(data).ok()?;
    let event_type = v.get("type")?.as_str()?;
    if event_type == "content_block_delta" {
        let delta = v.get("delta")?;
        let delta_type = delta.get("type")?.as_str()?;
        if delta_type == "text_delta" {
            let text = delta.get("text")?.as_str()?;
            if text.is_empty() {
                None
            } else {
                Some(StreamEvent::Delta(text.to_string()))
            }
        } else {
            None
        }
    } else {
        None
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
        let val = build_request(&msgs, &config).unwrap();
        let arr = val["messages"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert!(val["model"].as_str().is_some());
        assert!(val["max_tokens"].as_u64().is_some());
        assert!(val.get("max_output_tokens").is_none());
    }

    #[test]
    fn request_custom_model() {
        let msgs = crate::test_messages();
        let mut config = test_config();
        config.model = Some("claude-opus".to_string());
        config.max_tokens = Some(8192);
        let val = build_request(&msgs, &config).unwrap();
        assert_eq!(val["model"], "claude-opus");
        assert_eq!(val["max_tokens"], 8192);
    }

    #[test]
    fn parse_response_ok() {
        let json = serde_json::json!({
            "content": [
                { "type": "text", "text": "Hello from messages!" }
            ]
        });
        let bytes = serde_json::to_vec(&json).unwrap();
        let text = parse_response(&bytes).unwrap();
        assert_eq!(text, "Hello from messages!");
    }

    #[test]
    fn parse_response_skips_thinking_blocks() {
        let json = serde_json::json!({
            "content": [
                { "type": "thinking", "thinking": "hmm..." },
                { "type": "text", "text": "The answer is 42." }
            ]
        });
        let bytes = serde_json::to_vec(&json).unwrap();
        let text = parse_response(&bytes).unwrap();
        assert_eq!(text, "The answer is 42.");
    }

    #[test]
    fn parse_response_empty() {
        let json = serde_json::json!({ "content": [] });
        let bytes = serde_json::to_vec(&json).unwrap();
        assert!(matches!(parse_response(&bytes), Err(Error::EmptyResponse)));
    }

    #[test]
    fn stream_chunk_ok() {
        let data = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"foo"}}"#;
        let event = parse_stream_chunk(data).unwrap();
        assert!(matches!(event, StreamEvent::Delta(ref s) if s == "foo"));
    }

    #[test]
    fn stream_chunk_thinking_delta_skipped() {
        let data =
            r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"hmm"}}"#;
        assert!(parse_stream_chunk(data).is_none());
    }

    #[test]
    fn stream_chunk_message_start_skipped() {
        let data = r#"{"type":"message_start","message":{"id":"x","role":"assistant"}}"#;
        assert!(parse_stream_chunk(data).is_none());
    }
}
