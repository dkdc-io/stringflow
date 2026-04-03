//! Wire format implementations for LLM provider APIs.

pub(crate) mod completions;
pub(crate) mod messages;
pub(crate) mod responses;

use serde_json::Value;

use crate::{ChatMessage, Error, ProviderConfig, StreamEvent, WireFormat};

// ============================================================================
// Dispatch
// ============================================================================

pub(crate) fn endpoint(base_url: &str, format: WireFormat) -> String {
    let path = match format {
        WireFormat::Completions => "/v1/chat/completions",
        WireFormat::Responses => "/v1/responses",
        WireFormat::Messages => "/v1/messages",
    };
    format!("{}{}", base_url, path)
}

pub(crate) fn build_request(messages: &[ChatMessage], config: &ProviderConfig) -> Value {
    match config.wire_format {
        WireFormat::Completions => completions::build_request(messages, config),
        WireFormat::Responses => responses::build_request(messages, config),
        WireFormat::Messages => messages::build_request(messages, config),
    }
}

pub(crate) fn parse_response(bytes: &[u8], format: WireFormat) -> Result<String, Error> {
    match format {
        WireFormat::Completions => completions::parse_response(bytes),
        WireFormat::Responses => responses::parse_response(bytes),
        WireFormat::Messages => messages::parse_response(bytes),
    }
}

pub(crate) fn parse_stream_chunk(data: &str, format: WireFormat) -> Option<StreamEvent> {
    match format {
        WireFormat::Completions => completions::parse_stream_chunk(data),
        WireFormat::Responses => responses::parse_stream_chunk(data),
        WireFormat::Messages => messages::parse_stream_chunk(data),
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
    fn endpoint_paths() {
        let base = "http://localhost:8080";
        assert_eq!(
            endpoint(base, WireFormat::Completions),
            "http://localhost:8080/v1/chat/completions"
        );
        assert_eq!(
            endpoint(base, WireFormat::Responses),
            "http://localhost:8080/v1/responses"
        );
        assert_eq!(
            endpoint(base, WireFormat::Messages),
            "http://localhost:8080/v1/messages"
        );
    }

    #[test]
    fn build_request_dispatches_correctly() {
        let msgs = crate::test_messages();

        let mut config = test_config();
        config.wire_format = WireFormat::Completions;
        let completions = build_request(&msgs, &config);
        assert!(completions.get("messages").is_some());
        assert!(completions.get("model").is_none());

        config.wire_format = WireFormat::Responses;
        let responses = build_request(&msgs, &config);
        assert!(responses.get("input").is_some());
        assert!(responses.get("model").is_some());

        config.wire_format = WireFormat::Messages;
        let messages = build_request(&msgs, &config);
        assert!(messages.get("messages").is_some());
        assert!(messages.get("max_tokens").is_some());
    }

    #[test]
    fn parse_response_dispatches_correctly() {
        let comp_json = serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content": "a" } }]
        });
        assert_eq!(
            parse_response(
                &serde_json::to_vec(&comp_json).unwrap(),
                WireFormat::Completions
            )
            .unwrap(),
            "a"
        );

        let resp_json = serde_json::json!({
            "output": [{ "content": [{ "type": "output_text", "text": "b" }] }]
        });
        assert_eq!(
            parse_response(
                &serde_json::to_vec(&resp_json).unwrap(),
                WireFormat::Responses
            )
            .unwrap(),
            "b"
        );

        let msg_json = serde_json::json!({
            "content": [{ "type": "text", "text": "c" }]
        });
        assert_eq!(
            parse_response(
                &serde_json::to_vec(&msg_json).unwrap(),
                WireFormat::Messages
            )
            .unwrap(),
            "c"
        );
    }

    #[test]
    fn wire_format_default_is_messages() {
        assert!(matches!(WireFormat::default(), WireFormat::Messages));
    }
}
