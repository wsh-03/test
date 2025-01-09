import subprocess
import os
import json
from gpt import prompt2gpt
from files import File
import shutil

def compile_linux(linux_path):
    # Change directory to the Linux kernel source directory
    if not os.path.exists(linux_path):
        error_message = f"Error: {linux_path} does not exist"
        print(error_message)
        return {
            "status": "failure",
            "stdout": None,
            "stderr": None,
            "error": error_message
        }
    
    # Run the make command to compile the Linux kernel
    compile_command = f"make -C {linux_path} LLVM=1"
    try:
        result = subprocess.run(
            compile_command,
            shell=True,
            text=True,
            capture_output=True
        )
        if result.returncode == 0:
            print("Compilation succeeded.")
            return {
                "status": "success",
                "stdout": result.stdout.strip(),
                "stderr": None
            }
        else:
            print("Compilation failed.")
            return {
                "status": "failure",
                "stdout": result.stdout.strip(),
                "stderr": result.stderr.strip()
            }
    except Exception as e:
        error_message = f"Unexpected error: {e}"
        print(error_message)
        return {
            "status": "failure",
            "error": str(e)
        }
        
def fix_compilation_errors(compilation_error, linux_path, rust_folder, target_driver_folder, driver_name, max_attempts):
    c_file_paths = File().find_file_path(rust_folder, ".c")
    kernel_rust_file_paths = File().find_file_path(target_driver_folder, ".rs")
    file_name = File().extract_file_name(compilation_error.get("stdout"))
    c_code = ""
    rust_code = ""
    target_file_path = ""
    result = {}
    
    # Find the C file and Rust file that match the provided file name
    for c_file_path in c_file_paths:
        c_base_name = os.path.splitext(os.path.basename(c_file_path))[0]
        if c_base_name == file_name:
            c_code = File().remove_comments(c_file_path)
            break
    for rust_file_path in kernel_rust_file_paths:
        rust_base_name = os.path.splitext(os.path.basename(rust_file_path))[0]
        if rust_base_name == file_name:
            rust_code = File().remove_comments(rust_file_path)
            target_file_path = rust_file_path
            break
    
    # Prompt to correct the Rust code based on the provided compilation error
    
    prompt = (
                f"Your task is to correct the Rust code with given compilation errors, always apply your corrections to the code and provide the corrected Rust code without any comments. Rust code: ```{rust_code}```; Compilation error: ```{compilation_error.get('stderr')}``` "
            )
    # Attempt to fix the error using the provided prompt
    for attempt in range(1, max_attempts + 1):
        response = prompt2gpt(prompt, True)
        # Remove comments from the response
        clean_code = File().remove_comments(response)
        # Write the corrected Rust code to the file
        print(f"Writing the corrected Rust code to {target_file_path}")
        with open(target_file_path, 'w') as f:
            f.write(clean_code)
        # Compile the Linux kernel
        print(compilation_error.get("stderr"))
        if compilation_error.get("stderr") not in result.get("stderr"):
            compilation_result = compile_linux(linux_path)
            compilation_result["attempts"] = attempt
        result.update(compilation_result)
        # Write the results to a JSON file
        with open(f"{driver_name}.json", 'w') as json_file:
            json.dump(result, json_file, indent=4)
            print(f"Compilation info written to {driver_name}.json")

        if result.get("status") == "success":
            print(f"Compilation succeeded after {attempt} attempts.")
            return result
        
def replace_file(kernel_driver_foler, rust_file_folder):
    
    if not os.path.exists(kernel_driver_foler):
        raise FileNotFoundError(f"The kernel driver path {kernel_driver_path} does not exist.")
    if not os.path.exists(rust_file_folder):
        raise FileNotFoundError(f"The Rust file path {rust_file_folder} does not exist.")
    
    # Find all Rust file paths in the target directory
    rust_file_paths = File().find_file_path(rust_file_folder, ".rs")
    # Find all C file paths in the kernel driver directory
    kernel_file_paths = File().find_file_path(kernel_driver_path, ".c")
    # Replace the.rs file with the.c file
    for c_file_path in kernel_file_paths:
        # Retrieve the base name of the C file
        c_base_name = os.path.splitext(os.path.basename(c_file_path))[0]
        # Retrieve the base name of the Rust file
        for rust_file_path in rust_file_paths:
            if os.path.basename(rust_file_path) == c_base_name:
                print(f"Replacing {c_file_path} with {rust_file_path}")
                shutil.move(c_file_path, rust_file_path)
                break

if __name__ == "__main__":
    rust_file_folder = "/home/wsh/test/connector"
    linux_path = "/home/wsh/linux"
    driver_name = "connector"
    kernel_driver_path = f"/home/wsh/linux/drivers/{driver_name}"
    
    # Replace the.rs file with the.c file
    # replace_file(kernel_driver_path, rust_file_folder)
    # Compile the Linux kernel
    compile_result = compile_linux(linux_path)
    if compile_result.get("status") == "failure":
        # Attempt to fix the compilation errors
        fix_compilation_errors(compile_result, linux_path, rust_file_folder, kernel_driver_path, driver_name, 10)
    else:
        print("Compilation succeeded.")

