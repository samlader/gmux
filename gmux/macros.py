import ollama
from functools import lru_cache


@lru_cache(maxsize=None)
def ollama_chat(model_name, prompt):
    # TODO: Some prompt engineering here and add additional context (e.g. diff files)
    response = ollama.chat(
        model=model_name,
        messages=[
            {
                "role": "user",
                "content": prompt,
            },
        ],
    )
    return response["message"]["content"]
