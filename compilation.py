import subprocess
import os
import json
from file_utility import FileProcessor
import csv

class compilation:
    COMPILATION_ERROR = False
    
            
            


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