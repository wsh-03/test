import os
from gpt import prompt2gpt
from pathlib import Path
from files import File


def translate(path2folder):
    file_type = ".c"
    driver_name = "rtc"
    
    if (os.path.isdir(path2folder)):
        print("\nPath checked successfully\n")
        for dir, subdirs, files in os.walk(path2folder):      
            for file in files:
                output_dir = path2folder + f"/d_{file}"
                # print(os.path.join(dir, file))
                if file.endswith(file_type):
                    with open(os.path.join(dir, file), 'r') as f:
                        file_content = f.read()
                    prompt = (
                                f"The provided {file} code is located under the {driver_name} directory of the Linux kernel. Your task is to translate the given C code into equivalent Rust code and use the corresponding FFIs generated by Bindgen, such as ```use kernel::bindings::*```, always apply your corrections to the code and provide the translated Rust code without any comments. C code: ```{file_content}``` "
                            )
                    print(f"translating {file}")
                    response = prompt2gpt(prompt,False)
                    print(response)
                    # Remove comments
                    clean_code = File().remove_comments(response)                        
                    print(clean_code)
                        
                    os.chdir(output_dir)
                    # Create a file contianing the code from LLM in correct base name (Rust file)
                    rust_base = file.split(".")[0] + ".rs" 
                    with open(rust_base, "w") as f:
                        f.write(clean_code)
                    
                else:
                    print(f"Error: {file} in the {path2folder} is not the target {file_type} file")
        
        return "Translation Successfully Completed"
    else:
        return f"ERROR: {path2folder} not found"

path2folder = "/home/wsh/test/connector"
print(translate(path2folder))


