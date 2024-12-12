# openai conda environment
# python 3.6.9
from openai import OpenAI
import os

# Ensure OPENAI_API_KEY and other variables are defined
model_type = "o1-preview" 
question_history = []
response_history = []

def prompt2gpt(prompt):
    key =  os.environ.get('OPENAI_API_KEY')
    if not key:
        raise ValueError("OPENAI_API_KEY is not set.")
    
    # Set the API key for OpenAI
    client = OpenAI(api_key = key)
    # Call the OpenAI API
    chat_completion  = client.chat.completions.create(
        messages = [
            
                {
                    "role": "user", 
                     "content": "You are a C to Rust programming language translator. You will be given driver code and you must generate equivalent Rust code. " + prompt
                }
                    ],
        model = model_type,

    )
    OPENAI_API_KEY
    # Get the reply content
    reply_content = chat_completion.choices[0].message.content
    
    # Track the history
    question_history.append({"role": "user", "content": prompt})
    response_history.append({"role": "system", "content": reply_content})
    
    return reply_content
