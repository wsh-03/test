import os
import csv
import shutil
import re
import os 

class File:
    def __init__(self):
        self.file_info = []
                
    def find_file_path(self, path2folder, file_type):
        file_path = []
        for root, dirs, files in os.walk(path2folder):
            for file in files:
                # Check if the file is of the specified type
                if os.path.splitext(file)[1] == file_type:
                    # Get the path to the file
                    path2file = os.path.join(root, file)
                    file_path.append(path2file)
        return file_path
    

    def remove_comments(self, file):
        # Check if `file` is a valid file path or raw content
        if os.path.isfile(file):
            with open(file, 'r') as f:
                lines = f.read()
        else:
            lines = file  # Assume it's raw file content as a string
    
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
    
    def get_headers(self, file):
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
    
    def log_file(self, path2folder):
        if os.path.isdir(path2folder):
            print(f"Path Checked: {path2folder}\n")
            # Find the lines of code and remove the comments from each file in the target directory
            file_type = ".c"
            for root, dirs, files in os.walk(path2folder):
                for file in files:
                    # Check if the file is of the specified type
                    if os.path.splitext(file)[1] == file_type:
                        # Get the path to the file
                        path2file = os.path.join(dir, file)
                        # Remove comments in the file
                        processed_file = self.remove_comments(path2file)

                        # print("clean code: \n", processedFile)
                        
                        # Separate the code into individual lines
                        split_by_line = processed_file.split("\n")
                        # Append the file information to the list of Dictionaries
                        self.file_info.append({'Path': path2file, 
                                                   'File': file,
                                                   'LOC': len(split_by_line)})
                        
                        # with open(path2file, 'r') as f:
                        #     print("Original LOC: ", len(f.readlines()))
                        #     print("LOC After Processed", len(splitByLine))
                        
        # Log the information into a CSV file
        if self.file_info:          # Check if the list is not empty
            # Sort the lines of code of each file in ascending order
            sorted_lod= sorted(self.file_info, key=lambda x: x['LOC'])
            # Get the fieldnames of the dictionary
            fieldnames = sorted_lod[0].keys()
            # Log information into a CSV file
            with open('info.csv', 'w', newline='') as csvfile:
                writer = csv.DictWriter(csvfile, fieldnames = fieldnames )
                writer.writeheader()
                writer.writerows(sorted_lod)
            print("Fieldnames:", fieldnames)
        else:
            print("The list of dictionaries is empty.")
    
    def create_dir(self, path2csv, driver_name, number):
        # Check files that has less than 200 lines of code listed in the csv file
        with open(path2csv, 'r') as pathInfo:
            csvreader = csv.DictReader(pathInfo)
            for row in csvreader:
                if int(row['LOC']) < number:
                    os.chdir(os.path.dirname(path2csv))
                    os.makedirs(os.getcwd()+ "/" + driver_name + "/" + "d_" + row['File'])
                    print("Path Created: ", os.getcwd()+ "/" + driver_name + "/" + "d_" + row['File'])
                    shutil.copy(row['Path'], os.getcwd()+ "/" + driver_name + "/" + "d_" + row['File'])

# file = File()

# path2csv = "/home/e62562sw/test/info.csv"
# number = 200
# driver_name = "rtc"
# file.createDir(path2csv, driver_name, number)

# path2folder = "/home/e62562sw/linux_kernel/linux/drivers/rtc"
# file_clean.clean_file(path2folder)
