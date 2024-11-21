# functions
def load_env() -> None:
    import os

    from dotenv import load_dotenv

    load_dotenv()
    load_dotenv(os.path.join(os.getcwd(), ".env"))
    load_dotenv(os.path.join(os.path.expanduser("~"), ".env"))
    load_dotenv(os.path.join(os.path.expanduser("~"), ".dkdc", ".env"))


# TODO: this is kinda hacky right now but works for openai + ollama llama models
def str_to_tokens(text: str, model: str = "llama3.2:3b") -> list[int]:
    """
    tokenize text
    """
    import tiktoken

    try:
        enc = tiktoken.encoding_for_model(model)
    except Exception as _e:
        enc = _get_llama_tokenizer()

    return enc.encode(text)


def _get_llama_tokenizer():
    """
    tokenize text
    """
    import os
    import tiktoken

    from tiktoken.load import load_tiktoken_bpe

    # https://levelup.gitconnected.com/building-llama-3-from-scratch-with-python-e0cf4dbbc306
    tokenizer_path = os.path.join(os.getcwd(), "tokenizers", "llama3.model")
    tokenizer_model = load_tiktoken_bpe(tokenizer_path)
    special_tokens = [
        "<|begin_of_text|>",  # Marks the beginning of a text sequence.
        "<|end_of_text|>",  # Marks the end of a text sequence.
        "<|reserved_special_token_0|>",  # Reserved for future use.
        "<|reserved_special_token_1|>",  # Reserved for future use.
        "<|reserved_special_token_2|>",  # Reserved for future use.
        "<|reserved_special_token_3|>",  # Reserved for future use.
        "<|start_header_id|>",  # Indicates the start of a header ID.
        "<|end_header_id|>",  # Indicates the end of a header ID.
        "<|reserved_special_token_4|>",  # Reserved for future use.
        "<|eot_id|>",  # Marks the end of a turn (in a conversational context).
    ] + [
        f"<|reserved_special_token_{i}|>" for i in range(5, 256 - 5)
    ]  # A large set of tokens reserved for future use.
    #  patterns based on which text will be break into tokens
    tokenize_breaker = r"(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}{1,3}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+"
    # Initialize tokenizer with specified parameters
    tokenizer = tiktoken.Encoding(
        # make sure to set path to tokenizer.model file
        # name="tokenizer.model",
        name=tokenizer_path,
        # Define tokenization pattern string
        pat_str=tokenize_breaker,
        # Assign BPE mergeable ranks from tokenizer_model of LLaMA-3
        mergeable_ranks=tokenizer_model,
        # Set special tokens with indices
        special_tokens={
            token: len(tokenizer_model) + i for i, token in enumerate(special_tokens)
        },
    )

    return tokenizer
