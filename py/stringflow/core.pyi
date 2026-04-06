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
    """Low-level: send a chat request. Returns the response text.

    Raises:
        ConnectionError: ``[connection] ...`` when the server is unreachable.
        RuntimeError: ``[request] ...`` when the API returns an error, or
            ``[empty_response] ...`` when the model returns no content.
        ValueError: When *wire_format* is not a recognised format.
    """
    ...

def health_check(base_url: str) -> str:
    """Low-level: send a health check. Returns the status string.

    Raises:
        ConnectionError: ``[connection] ...`` when the server is unreachable.
        RuntimeError: ``[request] ...`` when the server returns an error, or
            ``[empty_response] ...`` when the response has no content.
    """
    ...
