import ollama
from functools import lru_cache


@lru_cache(maxsize=None)
def ollama_chat(model_name: str, prompt: str) -> str:
    """
    TODO: Some prompt engineering here and add additional context (e.g. diff files)
    """

    assistant_prompt = """
        Your response to the user will be used as a snippet in a pull request,
        it will be inserted into surrounding text.
        You MUST write for the user as if you are the author of this PR.
        You MUST be clear and concise.
        Do NOT start your response with adverbs like "sure" or "certainly".
        Do NOT finish your response with a summarising sentence.
    """

    messages = [
        {
            "role": "assistant",
            "content": assistant_prompt,
        },
        {
            "role": "user",
            "content": prompt,
        },
    ]

    result = ""

    try:
        response = ollama.chat(
            model=model_name,
            messages=messages,
        )
        result = response["message"]["content"]

    except Exception as e:
        print(f"An error occurred with ollama: {e}")

    return result
