import ollama


def ollama_chat(model_name, prompt):
    # TODO: Some prompt engineering here and add additional context
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
