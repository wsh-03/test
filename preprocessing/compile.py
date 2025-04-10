import subprocess
import os
import json
from preprocessing.file_utility import FileProcessor
import csv

class compilation:
    COMPILATION_ERROR = False
    
    def remove_file(self, rust_path, c_path, compilation_errors):
        if os.path.isfile(c_path) and os.path.isfile(rust_path):
            if compilation_errors == False:
                # Remove the C file and replace the Rust with the C file 
                os.remove(c_path)
                os.rename(c_path, rust_path)
                print(f"Replaced {c_path} with {rust_path}")
                return True
            elif compilation_errors == True:
                # Remove the Rust file and replace the C with Rust file 
                os.remove(rust_path)
                os.rename(rust_path, c_path)
                print(f"Replaced {rust_path} with {c_path}")
                return True
        else:
            raise FileNotFoundError(f"remove_file_Error: {c_path} or {rust_path} file not found")
        return False
    
    
    def replace_file(self, target_driver_dir, rs_dir, compilation_errors):
        class_file = FileProcessor()
        
        if len(target_driver_dir) or len(rs_dir) == 0:
            raise ValueError("The driver path or rust file path is empty.")
        elif not os.path.exists(target_driver_dir):
            raise FileNotFoundError(f"The kernel driver path '{target_driver_dir}' does not exist.")
        elif not os.path.exists(rs_dir):
            raise FileNotFoundError(f"The Rust file path '{rs_dir}' does not exist.")
        
        # List all file paths in the target directory
        rs_files = class_file.list_files(rs_dir, ".rs")
        c_files = class_file.list_files(target_driver_dir, ".c")
        # print("Files in the kernel driver directory:", kernel_files)
            
        # Replace the .c file with the .rs file
        for rs_file_path in rs_files:
            # Extract the base name from Rust file
            rs_file_name = os.path.basename(rs_file_path)
            rs_base_name = os.path.splitext(rs_file_name)[0]
                
            for c_file_path in c_files:
                # Extract the base name from C file 
                c_file_name = os.path.basename(c_file_path)
                c_base_name = os.path.splitext(c_file_name)[0]
                    
                if c_base_name == rs_base_name:
                    # Replace the source C file with the target Rust file based on the compilation error
                    self.remove_file(c_file_path, rs_file_path, compilation_errors)
        return True            
                
        
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
            
            
    def get_obj_files(self, path2driver, driver_name, output_csv):
        class_file = FileProcessor()

        # Log object files and summary
        result_c = class_file.log_file(path2driver, ".c", "summary.csv")
        result_o = class_file.log_file(path2driver, ".o", output_csv)

        if not (result_c and result_o):
            print("get_obj_files_ERROR: Missing CSV files.")
            return False
        
        file_info = []

        with open("summary.csv", 'r') as summary_info:
            summary_data = list(csv.DictReader(summary_info))
            # print(summary_data)
            
        # Change the object path with actual C file path
        with open(output_csv, 'r') as obj_info:
            obj_reader = csv.DictReader(obj_info)
            for obj_row in obj_reader:
                obj_basename = os.path.basename(obj_row[class_file.PATH_KEY])[:-2]
                obj_driver_name = obj_row[class_file.DRIVER_NAME_KEY]
                
                for summary_row in summary_data:
                    summary_basename = os.path.basename(summary_row[class_file.PATH_KEY])[:-2]
                    summary_driver_name = summary_row[class_file.DRIVER_NAME_KEY]
                    
                    if obj_basename == summary_basename and obj_driver_name == driver_name and summary_driver_name == driver_name:
                        loc = int(summary_row[class_file.LOC_KEY])
                        file_path = summary_row[class_file.PATH_KEY]
                        file_name = summary_row[class_file.FILE_KEY]

                        file_info.append({
                            "Driver_Name": summary_driver_name,
                            "Path": file_path,
                            "File_Name": file_name,
                            "Line_of_Code": loc
                        })

        # print("File info:", file_info)

        # Write updated file info to the output CSV
        result = class_file.write_lod(file_info, output_csv)
        if not result:
            return False
        else:
            print(f"File info written to {output_csv}")
            return True


                            
if __name__ == "__main__":
    driver_name = "rtc"
    path2driver = "/home/wsh/linux/drivers"
    
    # Log compatible kernel C files into a CSV file"
    output_csv = "obj_files.csv"
    class_file  = FileProcessor()
    class_compilation = compilation()
    result = class_compilation.get_obj_files(path2driver, driver_name, output_csv)
    
    if result is True:
        # Replace the kernel driver path and Rust file path with actual values
        rust_file_path = "/home/wsh/test/rtc"
        linux_path = "/home/wsh/linux"
        rust_files = class_file.list_files(rust_file_path, ".rs")
        
    
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