import os
import csv
import shutil
import re
import os 

class FileProcessor:
    DRIVER_NAME_KEY = "Driver Name"
    LOC_KEY = "Line of Code"
    PATH_KEY = "Path"
    FILE_KEY = "File"
    home_dir = os.environ.get("HOME")
    helper_file_path = f"{home_dir}/linux/rust/bindings/bindings_helper.h"
    
    # Retrieve the file content
    def get_file_info(self, path2file, file_type):
        if not os.path.isfile(path2file):
            print(f"Error: {path2file} is not a valid file path.")
            return None, None
        try:
            os.path.splitext(path2file)[1] == file_type
            file_name = os.path.basename(path2file)
            with open(path2file, 'r') as f:
                file_content = f.read()
                return file_content, file_name
        except Exception as e:
            print(f"Error: {e}")
            return None, None
        
        return None, None
    
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
    
    # Remove comments from a file
    def remove_comments(self, file):
        # Check if `file` is a valid file path or raw content
        if os.path.isfile(file):
            with open(file, 'r') as f:
                lines = f.read()
        elif file != "":
            lines = file  # Assume it's raw file content as a string
        else:
            return None
        
        code = lines
    
        # Patterns for /* */ muilti line style comment
        pattern4m = re.compile(r'/\*.*?\*/', flags=re.DOTALL)
        # Pattern for ``` style single line comment
        pattern4llm = re.compile(r'```.*')
        # Patther for // style single line comment
        patthern4s = re.compile(r'//.*')
    
        # Remove all /* */ style comments (multi-line): C style comment
        code = re.sub(pattern4m, "", code)
        # Remove all // style comments (single-line): C style comment
        code = re.sub(patthern4s, "", code)
        # Remove ``` style comment (single-line) result generated by LLM
        code = re.sub(pattern4llm,"",code)
    
        return code
    
    def get_base_name(self, log_message):
        # Define the regex pattern to match the RUSTC line
        pattern = r"RUSTC\s+(.*\.o)"
        match = re.search(pattern, log_message)
        if match:
            # Extract the full file path
            file_path = match.group(1)
            # Get the base name of the file without extension
            file_name = file_path.split("/")[-1].replace(".o", "")
            return file_name
        else:
            return None
    
    # log the information of all driver files in the Linux directory
    def log_file(self, path2folder):
        file_info = []
        if os.path.isdir(path2folder):
            print(f"Path Checked: {path2folder}\n")
            # Find the lines of code and remove the comments from each file in the target directory
            file_type = ".c"
            driver_name = ""
            for root, _, files in os.walk(path2folder):
                driver_name = os.path.relpath(root, path2folder).split(os.sep)[0]
                for file in files:
                    # Extract specified file type
                    if os.path.splitext(file)[1] == file_type:
                        # Get the path to the file
                        path2file = os.path.join(root, file)
                        # Remove comments in the file
                        processed_file = self.remove_comments(path2file)
                        
                        # print("clean code: \n", processedFile)
                        
                        # Get the lines of code in the file
                        split_by_line = processed_file.split("\n")  # Split the code into lines
                        line_of_code = len(split_by_line)
                        # Append the file information to the list of Dictionaries
                        file_info.append({f'{self.DRIVER_NAME_KEY}': driver_name, 
                                          f'{self.PATH_KEY}': path2file, 
                                          f'{self.FILE_KEY}': file, 
                                          f'{self.LOC_KEY}': line_of_code
                                         })
    
            # If the list of dictionaries is not empty, sort the lines of code and log the information into a CSV file
            if file_info:          
                # Sort the lines of code of each file in ascending order
                sorted_lod= sorted(file_info, key=lambda x: x[f'{self.LOC_KEY}'])
                # Get the fieldnames of the dictionary
                field_names = sorted_lod[0].keys()
                # Log information into a CSV file
                with open("driver_file.csv", 'w', newline='') as csvfile:
                    writer = csv.DictWriter(csvfile, fieldnames = field_names )
                    writer.writeheader()
                    writer.writerows(sorted_lod)
                print("Field names:", field_names)
                print("Driver file information successfully written to 'driver_file.csv'")            
        else:
            print("The list of dictionaries is empty.")
            return None


    # Count the lines of code for each driver and log the information in a CSV file
    def count_driver_loc(self, path2csv):
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
                    file_info[driver_name] = file_info.get(driver_name, 0) + loc

            # Print LOC
            for driver, loc in file_info.items():
                print(f"Driver Name: {driver}, Total LOC: {loc}")
            
            # Write summary to a CSV file
            field_names = [self.DRIVER_NAME_KEY, self.LOC_KEY]                
            data = [{f"{self.DRIVER_NAME_KEY}": driver, f"{self.LOC_KEY}": loc} for driver, loc in file_info.items()]
            sorted_data = sorted(data, key=lambda x: x[self.LOC_KEY])
            with open("driver_loc_summary.csv", 'w', newline='') as summary_file:
                writer = csv.DictWriter(summary_file, fieldnames=field_names)
                writer.writeheader()
                writer.writerows(sorted_data)

            print("Driver LOC data successfully written to 'driver_loc_summary.csv'")
        except Exception as e:
            print(f"Error: {e}")

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
                print(f"Header {header} is not present in the header helper")
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
        print("Header helper file updated")
    
    # Get the driver information and copy the files to the new directory
    def get_driver_info(self, path2csv, path2folder, driver_name):
        if not os.path.isdir(path2folder):
            return None
        
        headers = []
        header_output = f"{self.home_dir}/test/{driver_name}/{driver_name}_headers.h"
        if not os.path.isfile(path2csv):
            self.log_file(path2folder)
            self.count_driver_loc(path2csv)
        with open(path2csv, 'r') as info:
            file_info = csv.DictReader(info)
            for row in file_info:
                if row[self.DRIVER_NAME_KEY] == f'{driver_name}':
                    output_dir = os.path.join(os.path.dirname(path2csv), driver_name, f"d_{row[self.FILE_KEY]}")
                    # Create the directory and Copy the file to the new directory according to the file location in csv
                    os.makedirs(output_dir, exist_ok=True)
                    print("Path Created: ", output_dir)
                    shutil.copy(row[f'{self.PATH_KEY}'], output_dir)
                    # Collect only unique headers in the C files
                    for header in self.get_headers(row[f'{self.PATH_KEY}']):
                        if header not in headers:
                            headers.append(header)
        print("Headers: ", headers)
        # Write all unique headers to a C file in the directory
        with open(header_output, 'w') as f:
            for header in headers:
                f.write(header + "\n")
        # Update the header helper file
        self.update_header_helper(header_output)
        
        
file = FileProcessor()


path2csv = "/Users/harrywang/test/driver_summary.csv"
driver_name = "connector"
file.get_driver_info(path2csv, driver_name)

# path2folder = "/home/wsh/linux/drivers"
# csv_name = "linux.csv"
# file.log_file(path2folder, csv_name)

