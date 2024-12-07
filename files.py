import os
import csv
import shutil

from comments import remove_comments

class File:
    def __init__(self):
        self.fileInfo = []
        self.fileType = ".c"
                
                
    def logFile(self, path2folder):
        if os.path.isdir(path2folder):
            print(f"Path Checked: {path2folder}\n")
            # Find the lines of code and remove the comments from each file in the target directory
            for dir, _, files in os.walk(path2folder):
                for file in files:
                    # Check if the file is of the specified type
                    if os.path.splitext(file)[1] == self.file_type:
                        # Get the path to the file
                        path2file = os.path.join(dir, file)
                        # Remove comments in the file
                        processedFile = remove_comments(path2file)

                        # print("clean code: \n", processedFile)
                        
                        # Separate the code into individual lines
                        splitByLine = processedFile.split("\n")
                        # Append the file information to the list of Dictionaries
                        self.fileInfo.append({'Path': path2file, 
                                                   'File Name': file,
                                                   'LOC': len(splitByLine)})
                        
                        # with open(path2file, 'r') as f:
                        #     print("Original LOC: ", len(f.readlines()))
                        #     print("LOC After Processed", len(splitByLine))
                        
        # Log the information into a CSV file
        if self.fileInfo:          # Check if the list is not empty
            # Sort the lines of code of each file in ascending order
            sortedLod= sorted(self.fileInfo, key=lambda x: x['LOC'])
            # Get the fieldnames of the dictionary
            fieldnames = sortedLod[0].keys()
            # Log information into a CSV file
            with open('info.csv', 'w', newline='') as csvfile:
                writer = csv.DictWriter(csvfile, fieldnames = fieldnames )
                writer.writeheader()
                writer.writerows(sortedLod)
            print("Fieldnames:", fieldnames)
        else:
            print("The list of dictionaries is empty.")
    
    def createDir(self, path2csv, driverName, number):
        # Check files that has less than 200 lines of code listed in the csv file
        with open(path2csv, 'r') as pathInfo:
            csvreader = csv.DictReader(pathInfo)
            for row in csvreader:
                if int(row['LOC']) < number:
                    os.chdir(os.path.dirname(path2csv))
                    os.makedirs(os.getcwd()+ "/" + driverName + "/" + "d_" + row['File Name'])
                    print("Path Created: ", os.getcwd()+ "/" + driverName + "/" + "d_" + row['File Name'])
                    shutil.copy(row['Path'], os.getcwd()+ "/" + driverName + "/" + "d_" + row['File Name'])

file = File()

path2csv = "/home/e62562sw/test/info.csv"
number = 200
driverName = "rtc"
file.createDir(path2csv, driverName, number)

# path2folder = "/home/e62562sw/linux_kernel/linux/drivers/rtc"
# file_clean.clean_file(path2folder)
