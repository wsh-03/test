import subprocess
import os
import json
from file_utility import FileProcessor

class compilation:
    class_file = FileProcessor()
    compilation_errors = False
    
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
        kernel_files = class_file.find_file_path(kernel_driver_path, ".c")
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
        compile_command = f"make -C {linux_path} LLVM=1 ARCH=x86_64"
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
        
        
if __name__ == "__main__":
    # Replace the kernel driver path and Rust file path with actual values
    kernel_driver_path = "/home/e62562sw/linux_kernel/linux/drivers/rtc"
    rust_file_path = "/home/e62562sw/test/rtc"
    linux_path = "/home/e62562sw/linux_kernel/linux"
        
    class_compilation  = compilation()
    rust_files = class_compilation.class_file.list_files(rust_file_path, ".rs")
    for file in rust_files:
        class_compilation.compilation_errors = False
        class_compilation.replace_file(kernel_driver_path, file, class_compilation.compilation_errors)
        # Compile the Linux kernel
        compile_result = class_compilation.compile_linux(linux_path)
        # Write the results to a JSON file
        output_json_path = os.path.splitext(file)[0] + ".json"
        try:
            with open(output_json_path, 'w') as json_file:
                json.dump(compile_result, json_file, indent=4)
                print(f"Compilation errors written to {output_json_path}")
        except Exception as e:
            print(f"Failed to write JSON file: {e}")

        if compile_result.get("status") == "success":
            print("Compilation succeeded.")
        else:
            class_compilation.compilation_errors = True
            class_compilation.replace_file(kernel_driver_path, file, class_compilation.compilation_errors)
            print("Compilation failed:")
            print(compile_result.get("stderr"))
