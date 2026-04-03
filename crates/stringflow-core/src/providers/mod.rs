//! Provider configurations.
//!
//! A provider = base_url + wire_format + auth_config.
//! Each provider module exports a constructor function.

use crate::WireFormat;

// ============================================================================
// Core types
// ============================================================================

/// Authentication configuration for a provider
#[derive(Debug, Clone)]
pub enum AuthConfig {
    /// No authentication (e.g. local llama-server)
    None,
    /// Bearer token (e.g. OpenAI, Google AI Studio)
    Bearer(String),
    /// Custom header (e.g. Anthropic x-api-key)
    ApiKey { header: String, value: String },
}

/// Configuration for an AI provider
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub wire_format: WireFormat,
    pub auth: AuthConfig,
    /// Override default model name if set
    pub model: Option<String>,
    /// Override default max tokens if set
    pub max_tokens: Option<u32>,
}

/// Create a test provider config (Messages format, no auth, localhost:8080)
#[cfg(test)]
pub(crate) fn test_config() -> ProviderConfig {
    ProviderConfig {
        name: "test".to_string(),
        base_url: "http://localhost:8080".to_string(),
        wire_format: WireFormat::Messages,
        auth: AuthConfig::None,
        model: None,
        max_tokens: None,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_config_variants() {
        let none = AuthConfig::None;
        assert!(matches!(none, AuthConfig::None));

        let bearer = AuthConfig::Bearer("sk-test".to_string());
        assert!(matches!(bearer, AuthConfig::Bearer(ref s) if s == "sk-test"));

        let api_key = AuthConfig::ApiKey {
            header: "x-api-key".to_string(),
            value: "key-123".to_string(),
        };
        assert!(
            matches!(api_key, AuthConfig::ApiKey { ref header, ref value } if header == "x-api-key" && value == "key-123")
        );
    }
}
