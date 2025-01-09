import os
import csv
import shutil
import re
import os 

class File:
    def __init__(self):
        self.file_info = []
    
    def find_file_path(self, path, file_type):
        file_paths = []
        for root, dirs, files in os.walk(path):
            for file in files:
                if file.endswith(file_type):
                    file_paths.append(os.path.join(root, file))
        return file_paths
    
    def remove_comments(self, file):
        # Check if it is a valid file path or raw content
        if os.path.isfile(file):
            with open(file, 'r') as f:
                lines = f.read()
        else:
            lines = file 
    
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
        # Remove ``` style comment (single-line) result producted by LLM
        code = re.sub(pattern4llm,"",code)
    
        return code
    
    def extract_file_name(self, log_message):
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
    
    def log_file(self, path2folder, csv_name):
        if os.path.isdir(path2folder):
            print(f"Path Checked: {path2folder}\n")
            # Find the lines of code and remove the comments from each file in the target directory
            file_type = ".c"
            driver_name = ""
            for root, dirs, files in os.walk(path2folder):
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
                        self.file_info.append({
                                                'Driver Name': driver_name,   
                                                'Path': path2file, 
                                                'File': file,
                                                'LOC': line_of_code
                                                }
                                              )
                        
            # Log the information into a CSV file
            if self.file_info:          
                # Sort the lines of code of each file in ascending order
                sorted_lod= sorted(self.file_info, key=lambda x: x['LOC'])
                # Get the fieldnames of the dictionary
                field_names = sorted_lod[0].keys()
                # Log information into a CSV file
                with open(f"{csv_name}", 'w', newline='') as csvfile:
                    writer = csv.DictWriter(csvfile, fieldnames = field_names )
                    writer.writeheader()
                    writer.writerows(sorted_lod)
                print("Field names:", field_names)
                    
        else:
            print("The list of dictionaries is empty.")
            
    def count_driver_loc(self, path2csv,driver_name):
        # Count the total lines of code for each driver
        driver_loc = {}
        with open(path2csv, 'r') as info:
            csvreader = csv.DictReader(info)
            # count LOC for each driver name
            total_loc = 0
            for row in csvreader:
                driver_name = row["Driver Name"]
                loc = int(row["LOC"])
                driver_loc[driver_name] = driver_loc.get(driver_name, 0) + loc

        # Print the LOC summary for each driver
        for driver, total_loc in driver_loc.items():
            print(f"Driver Name: {driver}, Total LOC: {total_loc}")
            
        # Open the CSV file for writing
        with open(f"{driver_name}.csv", 'w') as info:
            writer = csv.writer(info)
            # Write the header row
            writer.writerow(["Driver Name", "Total LOC"])
            # Write each driver and its LOC to the file
            for driver, loc in driver_loc.items():
                writer.writerow([driver, loc])
    
        print("Driver LOC data successfully written to driver_loc")
    
    def get_c_headers(self, file):
        if isinstance(file, str):
            if os.path.isfile(file):  # Check if it's a valid file path
                with open(file, 'r') as f:
                    lines = f.read()
            else:  # Assume it's raw file content as a string
                lines = file
        # Pattern for finding headers
        pattern4h = re.compile(r'#include.*')
        # Find headers in the code
        headers = re.findall(pattern4h, lines)
        return headers

    def get_driver_info(self, path2csv, driver_name):
        self.count_driver_loc(path2csv, driver_name)
        headers = []
        with open(path2csv, 'r') as info:
            csvreader = csv.DictReader(info)
            for row in csvreader:
                if row['Driver Name'] == f'{driver_name}':
                    os.chdir(os.path.dirname(path2csv))
                    os.makedirs(os.getcwd()+ "/" + driver_name + "/" + "d_" + row['File'])
                    print("Path Created: ", os.getcwd()+ "/" + driver_name + "/" + "d_" + row['File'])
                    shutil.copy(row['Path'], os.getcwd()+ "/" + driver_name + "/" + "d_" + row['File'])
                    for header in self.get_c_headers(row['Path']):
                        if header not in headers:
                            headers.append(header)
        print("Headers: ", headers)
        with open(f"{os.path.dirname(path2csv)}/{driver_name}/{driver_name}_headers.txt", 'w') as f:
            for header in headers:
                f.write(header + "\n")


# file = File()

# path2csv = "/home/wsh/test/linux.csv"
# driver_name = "connector"
# file.get_driver_info(path2csv, driver_name)

# path2folder = "/home/wsh/linux/drivers"
# csv_name = "linux.csv"
# file.log_file(path2folder, csv_name)
