import subprocess
import os
import json
from file_utility import FileProcessor
from gpt import prompt2gpt
import shutil
import csv

class compilation:
    COMPILATION_ERROR = False
    
    def remove_file(self, rust_path, c_path, compilation_errors):
        if compilation_errors == False:
            os.remove(c_path)
            os.rename(c_path, rust_path)
            print(f"Replaced {c_path} with {rust_path}")
        else:
            os.remove(rust_path)
            os.rename(rust_path, c_path)
            print(f"Replaced {rust_path} with {c_path}")
    
    def replace_file(self, kernel_driver_path, rust_file_path):
        class_file = FileProcessor()
        if not os.path.exists(kernel_driver_path):
            raise FileNotFoundError(f"The kernel driver path {kernel_driver_path} does not exist.")
        if not os.path.exists(rust_file_path):
            raise FileNotFoundError(f"The Rust file path {rust_file_path} does not exist.")

        # List all file paths in the target directory
        kernel_files = class_file.list_files(kernel_driver_path, ".c")
        # print("Files in the kernel driver directory:", kernel_files)
        # Replace the .c file with the .rs file
        for c_file_path in kernel_files:
            # Retrieve the base name of the C file
            c_file_name = os.path.basename(c_file_path)
            c_base_name = os.path.splitext(c_file_name)[0]
            # Retrieve the base name of the Rust file
            rs_file_name = os.path.basename(rust_file_path)
            rs_base_name = os.path.splitext(rs_file_name)[0]
            if c_base_name == rs_base_name:
                self.remove_file(rust_file_path, c_file_path, self.compilation_errors)
                break
        

    def compile_linux(self, linux_path):
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
                print("Compilation failed:")
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
    
    def fix_compilation_errors(self, compilation_error, linux_path, rust_folder, target_driver_folder, driver_name, max_attempts):
        class_file = FileProcessor()
        c_file_paths = class_file.list_files(rust_folder, ".c")
        kernel_rust_file_paths = class_file.list_files(target_driver_folder, ".rs")
        file_name = class_file.extract_file_name(compilation_error.get("stdout"))
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
            
    def get_obj_files(self, linux_path, driver_name, file_type, output_csv):
        kernel_driver_path = os.path.join(linux_path, driver_name)
        class_file  = FileProcessor()
        result = class_file.log_file(kernel_driver_path, file_type, output_csv)
        result_ = class_file.log_file(linux_path, ".c", "summary.csv")
        if result and result_== True:
            file_info = []
            print(f"Object files logged successfully to {output_csv}.")
            
            # Replace the Line of code with the actual value
            with open("summary.csv", 'r') as summary_info:
                summary_reader = csv.DictReader(summary_info)
            with open(output_csv, 'r') as obj_info:
                obj_reader = csv.DictReader(obj_info)
            for row in obj_reader:
                obj_basename = os.path.basename(row[class_file.PATH_KEY])
                obj_driver_name = driver_name
                for row in summary_reader:
                    summary_basename = os.path.basename(row[class_file.PATH_KEY])
                    summary_driver_name = row[class_file.DRIVER_NAME_KEY]
                    if obj_basename == summary_basename and obj_driver_name == summary_driver_name:
                        loc = int(row[class_file.LOC_KEY])
                        driver_name = row[class_file.DRIVER_NAME_KEY]
                        file_path = row[class_file.PATH_KEY]
                        file_name = row[class_file.FILE_KEY]
                        file_info.append({
                            "Driver_Name": driver_name,
                            "Path": file_path,
                            "File_Name": file_name,
                            "Line_of_Code": loc
                        })
                        # print(f"Driver Name: {driver_name}, Path: {file_path}, File Name: {file_name} Total LOC: {loc}")
            # Write the file info to the CSV file
            result = class_file.write_lod(file_info, output_csv)
            if not result:
                return False
            
            print(f"Object file info logged successfully to {output_csv}.")
            return True
        
        else:
            print("Error: Failed to log object files.")
            return False
                            
if __name__ == "__main__":
    driver_name = "rtc"
    linux_path = "/home/wsh/linux"
    kernel_driver_path = os.path.join(linux_path, driver_name)
    print(f"Kernel driver path: {kernel_driver_path}")
    
    # Log compatible kernel C files into a CSV file"
    file_type = ".o"
    output_csv = "obj_files.csv"
    class_file  = FileProcessor()
    class_compilation = compilation()
    result = class_compilation.get_obj_files(linux_path, driver_name, file_type, output_csv)
    
    if result is True:
        # Replace the kernel driver path and Rust file path with actual values
    #     rust_file_path = "/home/wsh/test/rtc"
    #     linux_path = "/home/wsh/linux"
    #     rust_files = class_file.list_files(rust_file_path, ".rs")
        
    
    # print(class_compilation.COMPILATION_ERROR)
    
    # for file in rust_files:
    #     class_compilation.COMPILATION_ERROR = False
    #     class_compilation.replace_file(kernel_driver_path, file, class_compilation.COMPILATION_ERROR)
    #     # Compile the Linux kernel
    #     compile_result = class_compilation.compile_linux(linux_path)
    #     # Write the results to a JSON file
    #     output_json_path = os.path.splitext(file)[0] + ".json"
    #     try:
    #         with open(output_json_path, 'w') as json_file:
    #             json.dump(compile_result, json_file, indent=4)
    #             print(f"Compilation errors written to {output_json_path}")
    #     except Exception as e:
    #         print(f"Failed to write JSON file: {e}")
    
    # print(class_compilation.COMPILATION_ERROR)
    
    # class_compilation = compilation()
    # linux_path = "/home/wsh/linux"
    # print(compile_result := class_compilation.compile_linux(linux_path))