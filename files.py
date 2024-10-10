import os
import csv
from comments import remove_comments

class File:
    def __init__(self):
        self.file_info = []
        self.file_type = ".c"
    
    def log_info(self, lod):
        # Check if the list is not empty
        if lod: 
            fieldnames = lod[0].keys()
            # Log information into a CSV file
            with open('info.csv', 'w', newline='') as csvfile:
                writer = csv.DictWriter(csvfile, fieldnames = fieldnames )
                writer.writeheader()
                writer.writerows(lod)
            print("Fieldnames:", fieldnames)
        else:
            print("The list of dictionaries is empty.")
  
        
    def clean_file(self, path2folder):
        if os.path.isdir(path2folder):
            print(f"Path Checked: {path2folder}\n")
            # Find the lines of code and remove the comments from each file in the target directory
            for dir, _, files in os.walk(path2folder):
                for file in files:
                    if os.path.splitext(file)[1] == self.file_type:
                        
                        path2file = os.path.join(dir, file)
                        # Remove comments in the file
                        processed_file = remove_comments(path2file)

                        # print("clean code: \n", processed_file)
                        # Separate the code into individual lines
                        split_by_line = processed_file.split("\n")
                        self.file_info.append({'Path': path2file, 
                                                   'File Name': file,
                                                   'LOC': len(split_by_line)})
                        
                        # f = open(path2file, "r")
                        # print("Original LOC: ", len(f.readlines()))
                        # print("LOC After Processed", len(split_by_line))
                        
        # Sort the LOC of each file in ascending order
        sorted_data = sorted(self.file_info, key=lambda x: x['LOC'])
        self.log_info(sorted_data)


# file_clean = File()
# path2folder = "/home/e62562sw/linux_kernel/v6.11.1/drivers"
# file_clean.clean_file(path2folder)
