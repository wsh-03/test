
import argparse
import os
import json
from file_utility import FileProcessor
from translate import CodeTranslator


def log_compilation_json(compilation_result):
    # Write the error messages to a JSON file
    file_name = FileProcessor().extract_base_name(compilation_result.get("stdout"))
    output_json_path = file_name + ".json"
    with open(output_json_path, 'w') as json:
        json.dump(compilation_result, json, indent=4)
        print(f"Compilation errors written to {output_json_path}")
        
    return True

    
if __name__ == "__main__":
    
    parser = argparse.ArgumentParser(description="translator for Linux kernel driver")
    parser.add_argument(
        "linux_path",
        help="Path to the Linux kernel source code directory"
    )
    parser.add_argument(
        "driver_name",
        help="Name of the driver to be translated"
    )
    parser.add_argument(
        "language",
        help="Choose source language to be translated to Rust",
        choices=["C"]
    )
    
    args  = parser.parse_args()
    
    linux_path = args._get_args().linux_path
    driver_name = args._get_args().driver_name
    language = args._get_args().language
    
    result = FileProcessor().get_driver_header(linux_path, driver_name)
    if not result:
        print("Preprocessing Error: Unable to retrieve driver information.")
        exit(1)
    
    # Translate C code to Rust code
    path2folder = os.getcwd() + "test" + f"/{driver_name}/"
    CodeTranslator().translate(path2folder)
    
    path2driver = os.path.join(linux_path, "drivers", driver_name)
    # Compile the kernel to get object file, and replace the C code with the translated Rust code, 
    # where path2driver is the path to the driver folder in linux
    # and path2folder is the path to the folder containing the translated Rust code.
    FileProcessor().compile_linux(path2driver, FileProcessor().kernel_compiles)
    FileProcessor().replace_file(path2driver, path2folder, FileProcessor().compilation_error)
    
    # Clean the driver and prepare it to compile Rust code
    print(f"kernel status : {FileProcessor().kernel_compiles}")
    FileProcessor().compile_linux(path2driver, FileProcessor().kernel_compiles)
    print(f"kernel status : {FileProcessor().kernel_compiles}")
    
    # Get compilation results by compiling the driver
    print(f"Compilation error: {FileProcessor().compilation_error}")
    compile_result = FileProcessor().compile_linux(path2driver, FileProcessor().kernel_compiles)
    print(f"kernel status : {FileProcessor().kernel_compiles}")
    print(f"Compilation error: {FileProcessor().compilation_error}")
    
    # Log the compilation results to a JSON file
    log_compilation_json(compile_result)