from __future__ import annotations

from stringflow.core import chat as _chat_raw
from stringflow.core import health_check as _health_check_raw

DEFAULT_URL = "http://localhost:8080"

Message = tuple[str, str]


def chat(
    message: str | list[Message],
    history: list[Message] | None = None,
    *,
    base_url: str = DEFAULT_URL,
    wire_format: str = "messages",
    model: str | None = None,
    max_tokens: int | None = None,
) -> list[Message]:
    """Chat with an LLM. Returns conversation history you can pass back in.

    >>> import stringflow as sf
    >>> r = sf.chat("hi")
    >>> print(r[-1][1])  # assistant's response
    >>> r = sf.chat("follow up", r)  # multi-turn
    """
    if isinstance(message, str):
        messages = list(history or [])
        messages.append(("user", message))
    elif isinstance(message, list):
        messages = message
    else:
        raise TypeError(
            f"message must be str or list of (role, content) tuples, got {type(message).__name__}"
        )

    try:
        response = _chat_raw(base_url, messages, wire_format, model, max_tokens)
    except ConnectionError as e:
        raise ConnectionError(
            f"cannot reach LLM server at {base_url} — is dkdc-ai running?\n"
            f"  start: dkdc-ai start\n"
            f"  install: cargo install --path crates/dkdc-ai-cli"
        ) from e

    messages.append(("assistant", response))
    return messages


def health_check(base_url: str = DEFAULT_URL) -> str:
    """Check if the LLM server is healthy. Returns status string."""
    return _health_check_raw(base_url)


__all__ = [
    "chat",
    "health_check",
    "DEFAULT_URL",
    "Message",
]
