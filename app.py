from preprocessing.file_utility import FileProcessor
from preprocessing.compile import compilation
import argparse
import os
    
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
    
    ####################################### Preprocessing the Linux kernel to get the driver file information #######################################
    
    class_file = FileProcessor()
    
    result = class_file.get_driver_header(linux_path, driver_name)
    if not result:
        print("Preprocessing Error: Unable to retrieve driver information.")
        exit(1)
    
    
    ####################################### Translating the Rust file #######################################
    
    
    
    
    
    