import os
from gpt import prompt2gpt
from pathlib import Path
from check_e import compile_error
from comments import remove_comments


def translate(path2folder, ouput_dir):
    
    with open("/home/e62562sw/test-case-example/hello_world/helloworld.c", 'r') as f:
        c_example = f.read()

    with open("/home/e62562sw/test-case-example/hello_world/helloworld.rs", 'r') as f:
        rs_example = f.read()
    
    c_example = ""
    rs_example = ""
    
    example = f'''
        Linux kernel module in C language: {c_example}\n 
        Linux kernel module in Rust language: {rs_example}
        '''
    file_type = ".c"
    
    if (os.path.isdir(path2folder) and os.path.isdir(ouput_dir)):
        print(f"\nPath checked successfully\n")
        for dir, subdirs, files in os.walk(path2folder):
            if files != []:
                for subdir in subdirs:
                    full_path = os.path.join(dir, subdir)
                    sub_path = os.path.relpath(full_path, path2folder)
                    target_path = os.path.join(ouput_dir, sub_path)
                    print(target_path)
                    os.makedirs(target_path, mode=0o777, exist_ok=True)
                    if(os.path.isdir(target_path)):
                        print(f"Path Created: {target_path}")
                    else:
                        print(f"Error Occurred when creating {target_path}")
                for file in files:
                    # print(file)
                    if file.endswith(file_type):
                        path2file = os.path.join(dir, file)
                        with open(os.path.join(dir, file), 'r') as f:
                            file_content = f.read()
                            
                        # Get response from GPT
                        propmt = f"A simple Hello World example: {example}\n Provided file: {file_content}\n Provided C file is related to a Linux kernel driver located in the PCI directory. Please translate it from C to Rust, including only the Rust code without any comments."
                        response = prompt2gpt(propmt)
                        
                        # Remove comments
                        clean_code = remove_comments(response)
                        
                        print(response)
                        sub_path = os.path.relpath(path2file, path2folder)
                        c_path2file = os.path.join(ouput_dir, sub_path)
                        try:
                            with open(c_path2file, "w") as f:
                                f.write(clean_code)
                        except FileNotFoundError:
                            raise (f"Error: file not found in'{c_path2file}'")
                        file_path = Path(c_path2file)
                        print(f"change '{c_path2file}' to '{file_path}'")
                        new_file_path = file_path.with_suffix(".rs")
                        print(f"new file path: {new_file_path}")
                        rename_path = file_path.rename(new_file_path)
                        
                        
                    else:
                        pass
            else: 
                raise Exception(f"No file found in {path2folder}")
        return "Translation Successfully Completed"
    else:
        return f"{path2folder} or {ouput_dir} incorrect"

path2folder = "/home/e62562sw/test/exp/c_file"
ouput_path = "/home/e62562sw/test/exp/rs_file"
print (translate(path2folder,ouput_path))

