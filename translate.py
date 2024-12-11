import os
from gpt import prompt2gpt
from pathlib import Path
from fix_compilation_error import compile_error
from files import File


def translate(path2folder):
    
    # with open("/home/e62562sw/test-case-example/hello_world/helloworld.c", 'r') as f:
    #     c_example = f.read()

    # with open("/home/e62562sw/test-case-example/hello_world/helloworld.rs", 'r') as f:
    #     rs_example = f.read()
    
    # c_example = ""
    # rs_example = ""
    
    # example = f'''
    #     Linux kernel module in C language: {c_example}\n 
    #     Linux kernel module in Rust language: {rs_example}
    #     '''
    file_type = ".c"
    file_name = ""
    driver_name = "rtc"
    
    if (os.path.isdir(path2folder)):
        print(f"\nPath checked successfully\n")
        for dir, subdirs, files in os.walk(path2folder):      
            # print(files)     
            for file in files:
                outputDir = path2folder + f"/d_{file}"
                # print(os.path.join(dir, file))
                if file.endswith(file_type):
                    with open(os.path.join(dir, file), 'r') as f:
                        file_content = f.read()

                    # Get response from GPT
                    # propmt = f"A simple Hello World example: {example}\n Provided file: {file_content}\n Provided C file is related to a Linux kernel driver located in the PCI directory. Please translate it from C to Rust, including only the Rust code without any comments."
                        
                    propmt = f"The provided {file} file is related to a Rust for Linux kernel driver located in the {driver_name} directory. Please translate it from C to Rust, including only the Rust code without any comments. \n\n{file_content}"
                    print("translating file: ", file)
                    response = prompt2gpt(propmt)
                        
                    # Remove comments
                    comment = File()
                    clean_code = comment.remove_comments(response)
                        
                    print(clean_code)
                        
                    os.chdir(outputDir)
                    # Create a file contianing the code from LLM in correct base name (Rust file)
                    RustBase = file.split(".")[0] + ".rs" 
                    with open(RustBase, "w") as f:
                        f.write(clean_code)
                    
                else:
                    print(f"Error: {file_type} type file not found in {path2folder}")

        return "Translation Successfully Completed"
    else:
        return f"{path2folder} ERROR: Path not found"

path2folder = "/home/e62562sw/test/rtc"
print(translate(path2folder))

