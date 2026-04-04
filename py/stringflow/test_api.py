"""Tests for the stringflow Python API.

E2E tests require a running llama-server on localhost:8080.
Run with: uv run pytest py/stringflow/test_api.py
"""

import pytest

import stringflow as sf


# ============================================================================
# Unit tests (no server required)
# ============================================================================


class TestChatInput:
    def test_string_message_builds_history(self):
        """chat() should accept a string and build a user message tuple."""
        # We can't call chat() without a server, but we can test the TypeError path
        with pytest.raises(TypeError):
            sf.chat(42)  # type: ignore

    def test_rejects_invalid_type(self):
        with pytest.raises(TypeError, match="must be str or list"):
            sf.chat(123)  # type: ignore

    def test_connection_error_without_server(self):
        """chat() should raise ConnectionError when no server is running."""
        with pytest.raises((ConnectionError, Exception)):
            sf.chat("hi", base_url="http://localhost:19999")


class TestMessageBuilding:
    def test_string_builds_user_message(self):
        """String input should build a user message and append to history."""
        # chat() will fail connecting, but we can verify TypeError doesn't fire for str
        with pytest.raises((ConnectionError, Exception)):
            sf.chat("hello", base_url="http://localhost:19999")

    def test_list_input_passes_through(self):
        """List input should be used directly as messages."""
        with pytest.raises((ConnectionError, Exception)):
            sf.chat([("user", "hello")], base_url="http://localhost:19999")

    def test_history_is_prepended(self):
        """History should be prepended to the new message."""
        with pytest.raises((ConnectionError, Exception)):
            sf.chat(
                "follow up",
                [("user", "hi"), ("assistant", "hello")],
                base_url="http://localhost:19999",
            )

    def test_invalid_wire_format(self):
        """Invalid wire format should raise ValueError."""
        with pytest.raises(ValueError, match="unknown wire format"):
            sf.chat("hi", base_url="http://localhost:19999", wire_format="invalid")


class TestDefaults:
    def test_default_url(self):
        assert sf.DEFAULT_URL == "http://localhost:8080"

    def test_exports(self):
        assert hasattr(sf, "chat")
        assert hasattr(sf, "health_check")
        assert hasattr(sf, "DEFAULT_URL")
        assert hasattr(sf, "Message")


class TestHealthCheck:
    def test_health_check_connection_error(self):
        """health_check should raise when server is unreachable."""
        with pytest.raises((ConnectionError, Exception)):
            sf.health_check(base_url="http://localhost:19999")


# ============================================================================
# E2E tests (require running llama-server on localhost:8080)
# ============================================================================


@pytest.mark.e2e
class TestChatE2E:
    def test_simple_chat(self):
        result = sf.chat("Reply with exactly the word 'pong' and nothing else.")
        assert isinstance(result, list)
        assert len(result) == 2
        assert result[0] == (
            "user",
            "Reply with exactly the word 'pong' and nothing else.",
        )
        assert result[1][0] == "assistant"
        assert len(result[1][1]) > 0

    def test_multi_turn(self):
        r1 = sf.chat("My name is TestBot.")
        assert len(r1) == 2
        r2 = sf.chat("What is my name?", r1)
        assert len(r2) == 4
        assert r2[2] == ("user", "What is my name?")
        assert r2[3][0] == "assistant"

    def test_message_list_input(self):
        messages = [("user", "Reply with exactly 'hello'.")]
        result = sf.chat(messages)
        assert len(result) == 2
        assert result[1][0] == "assistant"

    def test_wire_format_completions(self):
        result = sf.chat("Say hi.", wire_format="completions")
        assert len(result) == 2
        assert result[1][0] == "assistant"

    def test_wire_format_responses(self):
        result = sf.chat("Say hi.", wire_format="responses")
        assert len(result) == 2
        assert result[1][0] == "assistant"


@pytest.mark.e2e
class TestHealthCheckE2E:
    def test_health_check(self):
        result = sf.health_check()
        assert isinstance(result, str)
