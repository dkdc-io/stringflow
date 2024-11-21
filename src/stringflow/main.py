import os
import json
import httpx

from pydantic import BaseModel

from stringflow.utils import load_env, str_to_tokens
from stringflow.strings import cast_system_template, choose_system_template


class StringFlow:
    def __init__(
        self,
        client: httpx.Client = None,
        provider: str = "openai",
        model: str = None,
        # TODO: proper logging
        log_level: int = 0,
        use_rich_print: bool = True,
    ):
        load_env()

        self.log = lambda x: print(x) if self.log_level > 0 else None

        self.provider = provider
        self.model = model
        self.rest = StringFlow.get_client(provider=self.provider)

        self.log_level = log_level

        if use_rich_print:
            from rich import print

        self.log(
            f"SF: provider={self.provider}, model={self.model}, rest={self.rest.base_url}"
        )

    def __call__(self, messages: list = None, **kwargs) -> str:
        self.log(f"SF: messages={messages} kwargs={kwargs}")
        if messages is None:
            return {
                "provider": self.provider,
                "model": self.model,
            }

        return self.chat(messages=messages, **kwargs)

    def chat(
        self,
        messages: str,
        model: str = None,
        max_tokens: int = 8192,
        **kwargs,
    ) -> str:
        self.log(
            f"SF.chat: messages={messages}, model={model}, max_tokens={max_tokens}, kwargs={kwargs}"
        )
        assert all(
            isinstance(message, dict) and "role" in message and "content" in message
            for message in messages
        ), "messages must be a list of dicts with role and content keys"

        model = model or self.model
        assert model, "model must be provided to chat (or a default set)"
        self.log(f"SF.chat: model={model}")

        payload = {
            "model": model,
            "messages": messages,
            **kwargs,
        }
        # TODO: hacky
        if self.provider == "ollama":
            payload["max_tokens"] = max_tokens
        elif self.provider == "openai":
            payload["max_completion_tokens"] = max_tokens
        self.log(f"SF.chat: payload={payload}")

        try:
            response = self.rest.post("/v1/chat/completions", json=payload)
            self.log(f"SF.chat: response={response}")

            r = response.json()
            self.log(f"SF.chat: r={r}")

            message = r["choices"][0]["message"]["content"]
            self.log(f"SF.chat: message={message}")
        except Exception as e:
            message = f"SF.chat: error={e}"

        return message

    def choose(
        self,
        message: str,
        system: str = None,
        model: str = None,
        options: list[str] = ["yes", "no"],
        n: int = None,
        **kwargs,
    ) -> str:
        self.log(
            f"SF.choose: message={message}, system={system}, model={model}, options={options}, n={n} kwargs={kwargs}"
        )

        model = model or self.model
        assert model, "model must be provided to choose (or a default set)"
        self.log(f"SF.choose: model={model}")

        n = n or 3 if self.provider == "openai" else 13  # TODO: excessive
        assert n > 0 and n % 2 == 1, "n must be a positive odd integer"
        self.log(f"SF.choose: n={n}")

        logit_bias = {}
        for i, _ in enumerate(options, start=1):
            logit_bias[self.tokenize(str(i), model=model)[0]] = 100.0
        self.log(f"SF.choose: logit_bias={logit_bias}")

        option_numbers = ""
        for i, option in enumerate(options, start=1):
            option_numbers += f"Option {i}: {option}\n"
        self.log(f"SF.choose: option_numbers={option_numbers}")

        system_message = system or choose_system_template.format(
            options=options, option_numbers=option_numbers
        )
        self.log(f"SF.choose: system_message={system_message}")

        messages = [
            {
                "role": "system",
                "content": system_message,
            },
            {
                "role": "user",
                "content": message,
            },
        ]
        self.log(f"SF.choose: messages={messages}")

        votes = {}
        for _n in range(n):
            self.log(f"SF.choose: iteration={_n}")
            payload = {
                "messages": messages,
                "model": model,
                "logit_bias": logit_bias,
                "max_tokens": 1,
                "temperature": 1.0,
                "n": 1,
            }
            self.log(f"SF.choose: payload={payload}")

            try:
                response = self.rest.post("/v1/chat/completions", json=payload).json()
                self.log(f"SF.choose: response={response}")

                for i, choice in enumerate(response["choices"]):
                    category = int(choice["message"]["content"])
                    count = votes.get(category, 0)
                    votes[category] = count + 1
            except Exception as e:
                self.log(f"SF.choose: error={e}")
                continue
        self.log(f"SF.choose: votes={votes}")

        max_option = max(votes, key=votes.get)
        self.log(f"SF.choose: max_option={max_option}")

        choice = options[max_option - 1]
        self.log(f"SF.choose: choice={choice}")

        return choice

    def cast(
        self,
        message: str,
        system: str = None,
        model: str = None,
        model_class: BaseModel = None,
    ) -> str:
        self.log(
            f"SF.cast: message={message}, system={system}, model={model}, model_class={model_class}"
        )
        assert model_class, "model_class must be provided to cast"

        model = model or self.model
        assert model, "model must be provided to cast"
        self.log(f"SF.cast: model={model}")

        tools = [
            {
                "type": "function",
                "function": {
                    "name": "cast",
                    "description": "cast text into a JSON format",
                    "parameters": model_class.schema(),
                },
            }
        ]
        self.log(f"SF.cast: tools={tools}")

        tool_choice = {"type": "function", "function": {"name": "cast"}}
        self.log(f"SF.cast: tool_choice={tool_choice}")

        system_message = system or cast_system_template.format(
            schema=model_class.schema()
        )
        self.log(f"SF.cast: system_message={system_message}")

        messages = [
            {
                "role": "system",
                "content": system_message,
            },
            {
                "role": "user",
                "content": message,
            },
        ]
        self.log(f"SF.cast: messages={messages}")

        payload = {
            "messages": messages,
            "model": model,
            "tools": tools,
            "tool_choice": tool_choice,
            "temperature": 1.0,
        }
        self.log(f"SF.cast: payload={payload}")

        try:
            response = self.rest.post("/v1/chat/completions", json=payload).json()
            self.log(f"SF.cast: response={response}")

            class_output = json.loads(
                response["choices"][0]["message"]["tool_calls"][0]["function"][
                    "arguments"
                ]
            )
            self.log(f"SF.cast: class_output={class_output}")

            casted = model_class(**class_output)
            self.log(f"SF.cast: casted={casted}")

            return casted
        except Exception as e:
            self.log(f"SF.cast: error={e}")
            return None

    @staticmethod
    def tokenize(message: str, model: str) -> list[str]:
        return str_to_tokens(message, model)

    @staticmethod
    def get_client(
        provider: str = None,
        hostname: str = None,
        port: int = None,
        timeout: int = 120,
        **client_kwargs,
    ) -> httpx.Client:
        if provider == "ollama":
            hostname = hostname or "localhost"
            port = port or 11434

            return httpx.Client(
                base_url=f"http://{hostname}:{port}",
                timeout=timeout,
                **client_kwargs,
            )
        elif provider == "openai":
            assert "OPENAI_API_KEY" in os.environ, "OPENAI_API_KEY must be set"

            return httpx.Client(
                base_url="https://api.openai.com",
                headers={"Authorization": f"Bearer {os.environ['OPENAI_API_KEY']}"},
                timeout=timeout,
                **client_kwargs,
            )

        raise ValueError(f"unhandled provider: {provider}")

    def list_available_lms(self, supported_only: bool = True) -> dict:
        self.log(f"list_available_lms supported_only={supported_only}")
        response = self.rest.get("/v1/models")
        self.log(f"list_available_lms response: {response.text}")
        r = response.json()
        self.log(f"list_available_lms r: {r}")
        d = sorted([x["id"] for x in r["data"]])
        self.log(f"list_available_lms d: {d}")
        if supported_only and self.provider == "openai":
            d = [
                x
                for x in d
                if x in set({"gpt-4o", "gpt-4o-mini", "gpt-4", "gpt-4-turbo"})
            ]
        self.log(f"list_available_lms d: {d}")

        return d

    def get_default_lm(self) -> str:
        lm_name = "llama3.2:3b" if self.provider == "ollama" else "gpt-4o-mini"

        return lm_name
