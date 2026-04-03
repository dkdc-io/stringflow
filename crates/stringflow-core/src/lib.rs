//! Flow strings through language models.
//!
//! Provider-agnostic LLM client supporting multiple wire formats:
//! - OpenAI Chat Completions (`/v1/chat/completions`)
//! - OpenAI Responses (`/v1/responses`)
//! - Anthropic Messages (`/v1/messages`)
//!
//! A provider is just a base URL + wire format + auth config.
//!
//! # Module structure
//!
//! - `wire_formats/` — request/response types and parsing per format
//! - `providers/` — provider configurations (llama, openai, anthropic, etc.)
//! - `client` — HTTP client (chat, streaming, health checks)

use serde::{Deserialize, Serialize};

mod client;
pub mod providers;
mod wire_formats;

// ============================================================================
// Core types
// ============================================================================

/// Default model name sent in request bodies (Messages + Responses formats)
const DEFAULT_MODEL: &str = "gemma-4-26b-a4b-it";
/// Default max tokens for response generation
const DEFAULT_MAX_TOKENS: u32 = 4096;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("service unavailable: {0}")]
    Unavailable(String),

    #[error("request failed: {0}")]
    RequestFailed(String),

    #[error("empty response")]
    EmptyResponse,
}

/// Wire format for LLM API requests
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum WireFormat {
    /// OpenAI Chat Completions (`/v1/chat/completions`)
    Completions,
    /// OpenAI Responses (`/v1/responses`)
    Responses,
    /// Anthropic Messages (`/v1/messages`)
    #[default]
    Messages,
}

/// A chat message (role + content), shared across all wire formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// A streaming event from a chat response
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    /// A text delta (partial content)
    Delta(String),
    /// Stream finished
    Done,
}

// ============================================================================
// Re-exports
// ============================================================================

pub use client::HealthResponse;
pub use client::{chat, chat_async, chat_stream, health_check, health_check_blocking};
pub use providers::{AuthConfig, ProviderConfig};

// ============================================================================
// Test helpers
// ============================================================================

#[cfg(test)]
fn test_messages() -> Vec<ChatMessage> {
    vec![
        ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        },
        ChatMessage {
            role: "assistant".to_string(),
            content: "Hi there!".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "How are you?".to_string(),
        },
    ]
}
