//! End-to-end tests against a running llama-server.
//!
//! These tests require llama-server to be running on localhost:8080.
//! Run with: `cargo test -p stringflow -- --ignored`

use futures::StreamExt;
use stringflow::{
    AuthConfig, ChatMessage, ProviderConfig, StreamEvent, WireFormat, chat_async, chat_stream,
    health_check,
};

fn simple_prompt() -> Vec<ChatMessage> {
    vec![ChatMessage {
        role: "user".to_string(),
        content: "Reply with exactly the word 'pong' and nothing else.".to_string(),
    }]
}

fn config_with_format(format: WireFormat) -> ProviderConfig {
    ProviderConfig {
        name: "llama".to_string(),
        base_url: "http://localhost:8080".to_string(),
        wire_format: format,
        auth: AuthConfig::None,
        model: None,
        max_tokens: None,
    }
}

// ============================================================================
// Health check
// ============================================================================

#[tokio::test]
#[ignore]
async fn health_check_ok() {
    let resp = health_check("http://localhost:8080").await.unwrap();
    assert_eq!(resp.status, "ok");
}

// ============================================================================
// Non-streaming: all 3 wire formats
// ============================================================================

#[tokio::test]
#[ignore]
async fn chat_completions_format() {
    let config = config_with_format(WireFormat::Completions);
    let result = chat_async(&config, &simple_prompt()).await.unwrap();
    assert!(!result.is_empty(), "expected non-empty response");
}

#[tokio::test]
#[ignore]
async fn chat_responses_format() {
    let config = config_with_format(WireFormat::Responses);
    let result = chat_async(&config, &simple_prompt()).await.unwrap();
    assert!(!result.is_empty(), "expected non-empty response");
}

#[tokio::test]
#[ignore]
async fn chat_messages_format() {
    let config = config_with_format(WireFormat::Messages);
    let result = chat_async(&config, &simple_prompt()).await.unwrap();
    assert!(!result.is_empty(), "expected non-empty response");
}

// ============================================================================
// Streaming
// ============================================================================

#[tokio::test]
#[ignore]
async fn stream_completions_format() {
    let config = config_with_format(WireFormat::Completions);
    let mut stream = chat_stream(&config, &simple_prompt()).await.unwrap();

    let mut got_delta = false;
    let mut got_done = false;
    while let Some(event) = stream.next().await {
        match event.unwrap() {
            StreamEvent::Delta(text) => {
                assert!(!text.is_empty());
                got_delta = true;
            }
            StreamEvent::Done => {
                got_done = true;
                break;
            }
        }
    }
    assert!(got_delta, "expected at least one Delta event");
    assert!(got_done, "expected Done event");
}

#[tokio::test]
#[ignore]
async fn stream_messages_format() {
    let config = config_with_format(WireFormat::Messages);
    let mut stream = chat_stream(&config, &simple_prompt()).await.unwrap();

    let mut got_delta = false;
    while let Some(event) = stream.next().await {
        match event.unwrap() {
            StreamEvent::Delta(text) => {
                assert!(!text.is_empty());
                got_delta = true;
            }
            StreamEvent::Done => break,
        }
    }
    assert!(got_delta, "expected at least one Delta event");
}

// ============================================================================
// Multi-turn conversation
// ============================================================================

#[tokio::test]
#[ignore]
async fn multi_turn_conversation() {
    let config = config_with_format(WireFormat::Messages);
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "My name is TestBot.".to_string(),
        },
        ChatMessage {
            role: "assistant".to_string(),
            content: "Hello TestBot!".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "What is my name?".to_string(),
        },
    ];
    let result = chat_async(&config, &messages).await.unwrap();
    assert!(!result.is_empty(), "expected non-empty response");
}
