//! OpenAI Responses wire format (`/v1/responses`)

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ChatMessage, DEFAULT_MAX_TOKENS, DEFAULT_MODEL, Error, ProviderConfig, StreamEvent};

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize)]
struct ResponsesRequest {
    model: String,
    input: Vec<ChatMessage>,
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ResponsesOutput {
    #[serde(default)]
    content: Vec<ResponsesContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ResponsesContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponsesResponse {
    output: Vec<ResponsesOutput>,
}

// ============================================================================
// Build / parse
// ============================================================================

pub(crate) fn build_request(
    messages: &[ChatMessage],
    config: &ProviderConfig,
) -> Result<Value, Error> {
    serde_json::to_value(ResponsesRequest {
        model: config
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        input: messages.to_vec(),
        max_output_tokens: config.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
    })
    .map_err(|e| Error::RequestFailed(e.to_string()))
}

pub(crate) fn parse_response(bytes: &[u8]) -> Result<String, Error> {
    let resp: ResponsesResponse =
        serde_json::from_slice(bytes).map_err(|e| Error::RequestFailed(e.to_string()))?;
    resp.output
        .into_iter()
        .flat_map(|o| o.content)
        .find(|b| b.content_type == "output_text" && b.text.is_some())
        .and_then(|b| b.text)
        .ok_or(Error::EmptyResponse)
}

pub(crate) fn parse_stream_chunk(data: &str) -> Option<StreamEvent> {
    let v: Value = serde_json::from_str(data).ok()?;
    let event_type = v.get("type")?.as_str()?;
    if event_type == "response.output_text.delta" {
        let text = v.get("delta")?.as_str()?;
        if text.is_empty() {
            None
        } else {
            Some(StreamEvent::Delta(text.to_string()))
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
        let arr = val["input"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["role"], "user");
        assert!(val["model"].as_str().is_some());
        assert!(val["max_output_tokens"].as_u64().is_some());
        assert!(val.get("messages").is_none());
    }

    #[test]
    fn request_custom_model() {
        let msgs = crate::test_messages();
        let mut config = test_config();
        config.model = Some("custom-model".to_string());
        config.max_tokens = Some(2048);
        let val = build_request(&msgs, &config).unwrap();
        assert_eq!(val["model"], "custom-model");
        assert_eq!(val["max_output_tokens"], 2048);
    }

    #[test]
    fn parse_response_ok() {
        let json = serde_json::json!({
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [
                    { "type": "output_text", "text": "Hello from responses!" }
                ]
            }]
        });
        let bytes = serde_json::to_vec(&json).unwrap();
        let text = parse_response(&bytes).unwrap();
        assert_eq!(text, "Hello from responses!");
    }

    #[test]
    fn parse_response_no_output_text() {
        let json = serde_json::json!({
            "output": [{
                "type": "message",
                "content": [
                    { "type": "refusal", "refusal": "nope" }
                ]
            }]
        });
        let bytes = serde_json::to_vec(&json).unwrap();
        assert!(matches!(parse_response(&bytes), Err(Error::EmptyResponse)));
    }

    #[test]
    fn stream_chunk_ok() {
        let data = r#"{"type":"response.output_text.delta","delta":"world"}"#;
        let event = parse_stream_chunk(data).unwrap();
        assert!(matches!(event, StreamEvent::Delta(ref s) if s == "world"));
    }

    #[test]
    fn stream_chunk_wrong_type() {
        let data = r#"{"type":"response.created","response":{}}"#;
        assert!(parse_stream_chunk(data).is_none());
    }
}
