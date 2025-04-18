# openai conda environment
# python 3.6.9
import OpenAI
import os
from file_utility import FileProcessor

def generate_prompt(path2file, error):
    
    bindgen_import = "use kernel::bindings::*;"
    task = f"""
            You are a Rust system programming expert.
            Your task is to translate the Original C code to Rust code. You will be provided with information about the target machine and a C file from the Linux kernel source code.
            If compilation errors occur, you will be asked to fix them and correct the Rust code accordingly with given error message, otherwise provide the translated Rust code.
            Your translation must follow these rules: 
            - You must translate the C code into idiomatic Rust code, while preserving the original functionality.
            - Your translation must use Rust equivalent API for the C code, including but not limited to data types, functions, and structures.
            - You must ensure that the translated Rust code is safe and follows Rust's ownership and borrowing rules.
            - You must use the foreign function interfaces from "{bindgen_import}" only if there is no Rust equivalent API.
            - You must mimic the original functionality, if no FFIs and Rust equivalent API available.
            - You must not use any unsafe code in your translation unless absolutely necessary.
            - Your translation must be free of comments, you must not include any additional information or explanations in your response.
            - Do not include any additional formatting
        """
        
    file_type = ".c"
    driver_name = "rtc"
    
    file_result = FileProcessor().get_file_info(path2file, file_type)
    if file_result is not None:
        file_content, file_name = file_result
        if file_content is not None and file_name is not None:
            print(f"Filename: {file_name}")
            print(f"Content:\n{file_content}")
    else:
        raise Exception("Error encountered during reading file content or retrieving filename.")  
      
    clean_code = FileProcessor().remove_comments(file_content)
    
    
    message = f"""
            The Linux kernel runs on a x86 machine, where the target {file_name} file belongs to {driver_name} in the Linux kernel.
            Target C file: "{clean_code}"
            Error message: "{error}"
            """
    return task, message

def gpt_translate(path2file, error):
    if error is None:
        task, message = generate_prompt(path2file, error)
        prompt = f"{task}\n{message}"
        
        model_type = "o3-mini" 
        
        key =  os.environ.get('OPENAI_API_KEY')
        if not key:
            raise ValueError("OPENAI_API_KEY is not set.")
    
        client = OpenAI(api_key = key)
        response = client.responses.create(
            model = model_type,
            input = [
                {
                    "role": "user", 
                    "content": prompt
                }
            ]
        )
        # Get the reply content
        reply_content = response.output_text
        return reply_content
    
    elif error is not None:
        task, message = generate_prompt(path2file, error)
        model_type = "gpt-4o" 
        key =  os.environ.get('OPENAI_API_KEY')
        if not key:
            raise ValueError("OPENAI_API_KEY is not set.")
        
        # Set the API key for OpenAI
        client = OpenAI(api_key = key)
        # Call the OpenAI API
        response  = client.responses.create(
            model = model_type,
            messages = [
                    {
                        "role": "system developer", 
                        "content": f"{task}"
                    },
                
                    {
                        "role": "user",
                         "content": f"{message}"
                    }
                        ])
        
        # Get the reply content
        reply_content = response.output_text    
        return reply_content
    
