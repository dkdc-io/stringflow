# StringFlow

[![GitHub Release](https://img.shields.io/github/v/release/dkdc-io/stringflow?color=blue)](https://github.com/dkdc-io/stringflow/releases)
[![crates.io](https://img.shields.io/crates/v/stringflow?color=blue)](https://crates.io/crates/stringflow)
[![PyPI](https://img.shields.io/pypi/v/stringflow?color=blue)](https://pypi.org/project/stringflow/)
[![CI](https://img.shields.io/github/actions/workflow/status/dkdc-io/stringflow/ci.yml?branch=main&label=CI)](https://github.com/dkdc-io/stringflow/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-8A2BE2.svg)](https://github.com/dkdc-io/stringflow/blob/main/LICENSE)

Flow strings through language models.

## Install

```bash
cargo add stringflow
```

```bash
uv add stringflow
```

## Usage

### Rust

```rust
use stringflow::{AuthConfig, ChatMessage, ProviderConfig, WireFormat, chat_async};

let config = ProviderConfig {
    name: "local".to_string(),
    base_url: "http://localhost:8080".to_string(),
    wire_format: WireFormat::Messages,
    auth: AuthConfig::None,
    model: None,
    max_tokens: None,
};

let messages = vec![
    ChatMessage { role: "user".to_string(), content: "Hello!".to_string() },
];

let response = chat_async(&config, &messages).await?;
```

### Python

```python
import stringflow

response = stringflow.chat(
    base_url="http://localhost:8080",
    messages=[("user", "Hello!")],
)
```

> **Note:** Streaming (`chat_stream()`) and async (`chat_async()`) APIs are currently Rust-only. Python provides synchronous `chat()` and `health_check()` only.
