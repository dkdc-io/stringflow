# stringflow

Flow strings through language models.

## Commands

```bash
bin/build          # Build all (Rust + Python)
bin/build-rs       # Build Rust crate
bin/build-py       # Build Python bindings (maturin develop)
bin/check          # Run all checks (format, lint, test)
bin/check-rs       # Rust checks (fmt, clippy, test)
bin/check-py       # Python checks (ruff, ty)
bin/test           # Run all tests
bin/format         # Format all code
bin/bump-version   # Bump version (--patch, --minor (default), --major)
```

## Architecture

```
crates/stringflow-core/       # Core library (stringflow on crates.io)
  src/lib.rs                   # Core types (Error, WireFormat, ChatMessage, StreamEvent)
  src/client.rs                # HTTP client (chat, streaming, health checks)
  src/providers/mod.rs         # ProviderConfig, AuthConfig types
  src/wire_formats/mod.rs      # Wire format dispatch
  src/wire_formats/completions.rs  # OpenAI Chat Completions
  src/wire_formats/responses.rs    # OpenAI Responses
  src/wire_formats/messages.rs     # Anthropic Messages
crates/stringflow-py/         # PyO3 bindings (cdylib)
py/stringflow/                # Python wrapper + type stubs
tests/e2e.rs                  # E2E tests (require running llama-server)
```

Library only — no binaries. Supports async + blocking + streaming.

## Wire formats

- OpenAI Chat Completions (`/v1/chat/completions`)
- OpenAI Responses (`/v1/responses`)
- Anthropic Messages (`/v1/messages`)

## Adding a new wire format

1. Create `src/wire_formats/<name>.rs` with `build_request`, `parse_response`, `parse_stream_chunk`
2. Add variant to `WireFormat` enum in `src/lib.rs`
3. Add dispatch arms in `src/wire_formats/mod.rs`
