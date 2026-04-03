# stringflow

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
