# openai conda environment
# python 3.6.9
from openai import OpenAI
import os

# question_history = []
# translation_history = []


def prompt2gpt(prompt, fix_error):
    if fix_error == False:
        model_type = "o3-mini" 
        key =  os.environ.get('OPENAI_API_KEY')
        if not key:
            raise ValueError("OPENAI_API_KEY is not set.")
    
        # Set the API key for OpenAI
        client = OpenAI(api_key = key)
        # Call the OpenAI API
        chat_completion  = client.chat.completions.create(
            messages = [
                
                    {
                        "role": "system", 
                        "content": "You are a C to Rust system programming translator for the Linux kernel. You will be provided with C code. Your task is to translate it into equivalent and safer Rust code."
                    },
                    {
                        "role": "user", 
                        "content": prompt
                    }
                        ],
            model = model_type,
            temperature=1
        )
        # Get the reply content
        reply_content = chat_completion.choices[0].message.content
        return reply_content
    else:
        model_type = "o3-mini" 
        key =  os.environ.get('OPENAI_API_KEY')
        if not key:
            raise ValueError("OPENAI_API_KEY is not set.")
        
        # Set the API key for OpenAI
        client = OpenAI(api_key = key)
        # Call the OpenAI API
        chat_completion  = client.chat.completions.create(
            messages = [
                    {
                        "role": "system", 
                        "content": "You are a Rust systems programmer for the Linux kernel. You will be provided with Rust code and its compilation errors. Your task is to correct the Rust code based on the provided compiler feedback."
                    },
                
                    {
                        "role": "user", 
                         "content": prompt
                    }
                        ],
            temperature=1,
            model = model_type)
        
        # Get the reply content
        reply_content = chat_completion.choices[0].message.content
        
        # Track the history
        # question_history.append({"role": "user", "content": prompt})
        # translation_history.append({"role": "system", "content": reply_content})
        
        return reply_content