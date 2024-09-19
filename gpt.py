# openai conda environment
# python 3.6.9
import os
from openai import OpenAI

OPENAI_API_KEY = os.environ.get("OPENAI_API_KEY")
model_type= "gpt-4o-mini"

response_history = []
question_history = []

def prompt2gpt(file, question):
    while OPENAI_API_KEY != "":
        client = OpenAI(api_key=OPENAI_API_KEY)
        response = client.chat.completions.create(
            model=model_type,
            messages=[
                {"role": "system", "content": file},
                {"role": "user", "content": question}
            ],
            # max_tokens=500,
            # temperature=
        )
        reply_content = response.choices[0].message.content
        # print(reply_content, end="")
        question_history.append({"role": "user", "content": question})
        response_history.append({"role": "system", "content": reply_content})
        return reply_content


def gpt_repair(previous_code, previous_repair, question):
    while OPENAI_API_KEY is not None:
        client = OpenAI(api_key=OPENAI_API_KEY)
        stream = client.chat.completions.create(
            model=model_type,
            messages=[
                {"role": "system", "content": "translate the C program into Rust" + previous_code},
                {"role": "system", "content": ""},
                {"role": "user", "content": question}
            ],
            # max_tokens=500,
            stream = True,
        )
    
            

                
