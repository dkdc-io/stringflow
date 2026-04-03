def chat(
    base_url: str,
    messages: list[tuple[str, str]],
    wire_format: str = "messages",
    model: str | None = None,
    max_tokens: int | None = None,
    auth_bearer: str | None = None,
    auth_header: str | None = None,
    auth_value: str | None = None,
) -> str:
    """Low-level: send a chat request. Returns the response text."""
    ...

def health_check(base_url: str) -> str:
    """Low-level: send a health check. Returns the status string."""
    ...
