import os
import csv
import shutil

from comments import remove_comments

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
                        processed_file = remove_comments(path2file)

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
