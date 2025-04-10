from file_utility import FileProcessor
from model import prompt2gpt
import os
import json

class error_fix:
    def fix_compilation_errors(self, compilation_error, linux_path, rust_folder, target_driver_folder, driver_name, max_attempts):
        
        class_file = FileProcessor()
        
        c_file_paths = class_file.list_files(rust_folder, ".c")
        kernel_rust_file_paths = class_file.list_files(target_driver_folder, ".rs")
        file_name = class_file.extract_base_name(compilation_error.get("stdout"))
        
        c_code = ""
        rust_code = ""
        target_file_path = ""
        result = {}
    
        # Find the C file and Rust file that match the provided file name
        for c_file_path in c_file_paths:
            c_base_name = os.path.splitext(os.path.basename(c_file_path))[0]
            if c_base_name == file_name:
                c_code = class_file.remove_comments(c_file_path)
                break
        for rust_file_path in kernel_rust_file_paths:
            rust_base_name = os.path.splitext(os.path.basename(rust_file_path))[0]
            if rust_base_name == file_name:
                rust_code = class_file.remove_comments(rust_file_path)
                target_file_path = rust_file_path
                break
    
        # Prompt to correct the Rust code based on the provided compilation error
    
        prompt = (
                    f"Your task is to correct the Rust code with given compilation errors, always apply your corrections to the code and provide the corrected Rust code without comments. Rust code: ```{rust_code}```; Compilation error: ```{compilation_error.get('stderr')}``` "
                )
        # Attempt to fix the error using the provided prompt
        for attempt in range(1, max_attempts + 1):
            response = prompt2gpt(prompt, True)
            # Remove comments from the response
            clean_code = class_file.remove_comments(response)
            # Write the corrected Rust code to the file
            print(f"Writing the corrected Rust code to {target_file_path}")
            with open(target_file_path, 'w') as f:
                f.write(clean_code)
            # Compile the Linux kernel
            print(compilation_error.get("stderr"))
            if compilation_error.get("stderr") not in result.get("stderr"):
                compilation_result = self.compile_linux(linux_path)
                compilation_result["attempts"] = attempt
            result.update(compilation_result)
            # Write the results to a JSON file
            with open(f"{driver_name}.json", 'w') as json_file:
                json.dump(result, json_file, indent=4)
                print(f"Compilation info written to {driver_name}.json")

            if result.get("status") == "success":
                print(f"Compilation succeeded after {attempt} attempts.")
                return result