import os
import csv
import shutil
import re
import os
import subprocess
class FileProcessor:
    
    DRIVER_NAME_KEY = "Driver_Name"
    LOC_KEY = "Line_of_Code"
    PATH_KEY = "Path"
    FILE_KEY = "File_Name"
    home_dir = os.environ.get("HOME")
    helper_file_path = f"{home_dir}/linux/rust/bindings/bindings_helper.h"
    
    # kernel already compiled and prepared for translation so set it to True
    kernel_compiles = False
    compilation_error = False
    
    # List the full path of a file in the target directory
    def list_files(self, path2folder, file_type):
        if not os.path.isdir(path2folder):
            return None
        file_path = []
        for root, _, files in os.walk(path2folder):
            for file_name in files:
                # Check if the file name match the specific type
                if os.path.splitext(file_name)[1] == file_type:
                    # Get the path to the file
                    path2file = os.path.join(root, file_name)
                    file_path.append(path2file)
        return file_path
    
    
    def extract_base_name(self, compilation_error):
        pattern = r"RUSTC\s+(.*\.o)"
        match = re.search(pattern, compilation_error)
        if match:
            # Extract the full file path
            file_path = match.group(1)
            # Get the base name of the file without extension
            file_name = file_path.split("/")[-1].replace(".o", "")
            return file_name
        else:
            return None
        
    
    def write_lod(self, list_of_dic, output_file):
        if not list_of_dic:
            print("write_lod_Error: The list of dictionaries is empty, cannot log into the csv file.")
            return False
        
        # Sort the lines of code of each file in ascending order
        sorted_lod= sorted(list_of_dic, key=lambda x: x[f'{self.LOC_KEY}'])
        # Get the fieldnames of the dictionary
        field_names = sorted_lod[0].keys()
        # Log information into a CSV file
        with open(f"{output_file}", 'w', newline='') as csvfile:
            writer = csv.DictWriter(csvfile, fieldnames = field_names )
            writer.writeheader()
            writer.writerows(sorted_lod)
        return True
    
    
    # log the information of all driver files in the Linux directory
    def log_file(self, path2driver, file_type, output_file):
        
        if not os.path.isdir(path2driver):
            print(f"log_file_ERROR: {path2driver} not found")
            return False
        
        # Find the lines of code and remove the comments from each file in the target directory
        file_info = []
        for root, dirs, files in os.walk(path2driver):
            for file in files:
                # Extract specified file type
                if os.path.splitext(file)[1] == file_type:
                    # Get the path to the file
                    path2file = os.path.join(root, file)
                    # print(f"Path to file: {path2file}")
                    
                    # Remove comments in the file
                    if file_type != ".o":
                        processed_file = self.remove_comments(path2file)
                        # print("clean code: \n", processedFile)
                        split_by_line = processed_file.split("\n")  # Split the code into lines
                        line_of_code = len(split_by_line)
                    else:
                        line_of_code = 0
                    
                    # Extract the driver name based on the path
                    relative_path = os.path.relpath(path2file, path2driver)
                    split_path = relative_path.split(os.sep)
                    if len(split_path) > 1:
                        driver_name = split_path[0]
                    
                    # print(f"Driver Name: {driver_name}")
                    # print(f"Path to File: {path2file}")
                    # print(f"Lines of Code: {line_of_code}")
                    
                    
                    # Append the file information to the list of Dictionaries
                    file_info.append({f'{self.DRIVER_NAME_KEY}': driver_name, 
                                        f'{self.PATH_KEY}': path2file, 
                                        f'{self.FILE_KEY}': file, 
                                        f'{self.LOC_KEY}': line_of_code
                                        })
            
        result = self.write_lod(file_info, output_file)
        if not result:
            return False
        return True
    

    # Summary the lines of code of each file in a driver and log the information into a CSV file
    def count_driver_loc(self, path2csv, output_file):
        if not os.path.isfile(path2csv):
            return None
        # Store LOC in a single dictionary
        file_info = {}
        try:
            # Read CSV file
            with open(path2csv, 'r') as info:
                csvreader = csv.DictReader(info)
                for row in csvreader:
                    loc = int(row[self.LOC_KEY])
                    driver_name = row[self.DRIVER_NAME_KEY]
                    file_info[driver_name] = file_info.get(driver_name, 0) + loc
                    print(f"Driver Name: {driver_name}, Path: {row[self.PATH_KEY]}, Total LOC: {loc}")

            # Print LOC
            # for driver, loc in file_info.items():
            #     print(f"Driver Name: {driver}, Total LOC: {loc}")
            
            # Write summary to a CSV file
            field_names = [self.DRIVER_NAME_KEY, self.LOC_KEY]                
            data = [{f"{self.DRIVER_NAME_KEY}": driver, f"{self.LOC_KEY}": loc} for driver, loc in file_info.items()]
            sorted_data = sorted(data, key=lambda x: x[self.LOC_KEY])
            with open(f"{output_file}", 'w', newline='') as summary_file:
                writer = csv.DictWriter(summary_file, fieldnames=field_names)
                writer.writeheader()
                writer.writerows(sorted_data)

        except Exception as e:
            print(f"count_driver_loc_Error: {e}")


    # Get the headers from a C file
    def get_headers(self, file):
        # Check if file is a valid string file path or header file data
        if isinstance(file, str):
            if os.path.isfile(file): 
                with open(file, 'r') as f:
                    lines = f.read()
            else: 
                # Assume it's raw file content as a string
                lines = file
        else:
            return None
        # Pattern for C header
        pattern4h = re.compile(r'#include.*')
        # Find all headers
        headers = re.findall(pattern4h, lines)
        return headers
    
    
    # Update the header helper file with the unique headers from the C files
    def update_header_helper(self, file_path):
        if not os.path.isfile(file_path):
            return None
        
        # Compare the headers in the binding helper file with the headers in the C files
        unique_headers = []
        for header in self.get_headers(file_path):
            if header not in self.get_headers(self.helper_file_path):
                unique_headers.append(header)
        # Write the unique headers to the header helper file
        with open(self.helper_file_path, 'r') as f:
            helper_content = f.readlines()
        for header in unique_headers:
            for line in range(1, len(helper_content)):
                if helper_content[line] == "\n":
                    helper_content.insert(line, header)
                    break
        with open(self.helper_file_path, 'w') as f:
            f.writelines(helper_content)            
    
    
    def get_obj_files(self, path2driver, driver_name, output_csv):
        # Get object files and summary
        if not os.path.isfile("summary.csv"):
            summary_result = self.log_file(path2driver, ".c", "summary.csv")
            print("get_obj_files_ERROR: summary.csv not found. Regenerating...")
        
        result_o = self.log_file(path2driver, ".o", output_csv)

        if not (result_o):
            print("get_obj_files_ERROR: Missing CSV files.")
            return False
        
        elif summary_result is False:
            print("get_obj_files_ERROR: Unable to retrieve driver information.")
            return False
        
        elif result_o is False:
            print("get_obj_files_ERROR: Unable to retrieve object files. Recompilig the kernel.")
            
        file_info = []
        
        with open("summary.csv", 'r') as summary_info:
            summary_data = list(csv.DictReader(summary_info))
            # print(summary_data)
            
        # Change the object path with actual C file path
        with open(output_csv, 'r') as obj_info:
            obj_reader = csv.DictReader(obj_info)
            for obj_row in obj_reader:
                obj_basename = os.path.basename(obj_row[self.PATH_KEY])[:-2]
                obj_driver_name = obj_row[self.DRIVER_NAME_KEY]
                
                for summary_row in summary_data:
                    summary_basename = os.path.basename(summary_row[self.PATH_KEY])[:-2]
                    summary_driver_name = summary_row[self.DRIVER_NAME_KEY]
                    
                    if obj_basename == summary_basename and obj_driver_name == driver_name and summary_driver_name == driver_name:
                        loc = int(summary_row[self.LOC_KEY])
                        file_path = summary_row[self.PATH_KEY]
                        file_name = summary_row[self.FILE_KEY]

                        file_info.append({
                            "Driver_Name": summary_driver_name,
                            "Path": file_path,
                            "File_Name": file_name,
                            "Line_of_Code": loc
                        })

        # print("File info:", file_info)

        # Write updated file info to the output CSV
        result = self.write_lod(file_info, output_csv)
        if not result:
            return False
        else:
            print(f"File info written to {output_csv}")
            return True
    
    
    # Get the target driver header files and copy them to the new directory according to the file location in the csv file
    def get_driver_header(self, path2linux, driver_name):
        
        if not os.path.isdir(path2linux):
            print(f"ERROR: {path2linux} not found")
            return False
        
        
        headers = []
        obj_csv = f"object_{driver_name}.csv"
        header_output = f"{self.home_dir}/test/{driver_name}/{driver_name}_headers.h"
        output_csv = "driver_loc_summary.csv"
        
        if not os.path.isfile("summary.csv"):
            self.log_file(path2linux, ".c", "summary.csv")
            self.count_driver_loc("summary.csv", output_csv)
            
        path2driver = os.path.join(path2linux, driver_name)
        self.get_obj_files(path2driver, driver_name, obj_csv)
        
        with open(obj_csv, 'r') as obj_info:
            obj_reader = csv.DictReader(obj_info)
            for obj_row in obj_reader:
                
                obj_driver_name = obj_row[self.DRIVER_NAME_KEY]
                if obj_driver_name == driver_name:
                    
                    output_dir = os.path.join(os.getcwd(), driver_name, f"d_{obj_row[self.FILE_KEY]}")
                    
                    # Create the directory and Copy the file to the new directory according to the file location in csv
                    os.makedirs(output_dir, exist_ok=True)
                    shutil.copy(obj_row[f'{self.PATH_KEY}'], output_dir)
                    
                    # Collect only unique headers in the C files
                    for header in self.get_headers(obj_row[f'{self.PATH_KEY}']):
                        if header not in headers:
                            headers.append(header)
                            
        print("Headers: ", headers)
        
        # Write all unique headers to a C file in the directory
        with open(header_output, 'w') as f:
            for header in headers:
                f.write(header + "\n")
        # Update it to the header helper file
        self.update_header_helper(header_output)
        return True
    
        
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
    
    
    def compile_linux(self, path2driver, kernel_compiles):
        remove_command = f"make -C {path2driver} LLVM=1 clean"
        compile_command = f"make -C {path2driver} LLVM=1"
        
        if kernel_compiles == True:
            command = remove_command
        else:
            command = compile_command
        
        try:
            result = subprocess.run(
                command,
                shell=True,
                text=True,
                capture_output=True
                )
        except subprocess.CalledProcessError as e:
            if command == remove_command:
                self.kernel_compiles = True
            else:
                self.kernel_compiles = False
                self.compilation_error = True
                
            # True_failure means failed to clean kernel 
            # False_failure means failed to compile kernel 
            return {
                "status": f"{kernel_compiles}_failure",
                "stdout": e.stdout,
                "stderr": e.stderr 
                }
        else:
            if command == remove_command:
                self.kernel_compiles = False
            else:
                self.kernel_compiles = True
                self.compilation_error = False
                
            # True_success means kernel cleaned successfully
            # False_success means kernel compiled successfully
            return {
                "status": f"{kernel_compiles}_success",
                "stdout": result.stdout,
                "stderr": result.stderr
                }



if __name__ == '__main__':
    file = FileProcessor()

    # path2csv = "/home/wsh/test/driver_summary.csv"
    path2folder = "/home/wsh/linux/drivers"
    # driver_name = "rtc"
    # file.get_driver_header(path2csv, path2folder, driver_name)
    
    file.log_file(path2folder, ".o", "summary_o.csv")