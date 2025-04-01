import os
from pathlib import Path
from file_utility import FileProcessor
import gpt_translation

def translate(path2folder):
    file_class = FileProcessor()
    file_type = ".c"
    
    if (os.path.isdir(path2folder)):
        print("\nPath checked successfully\n")
        for dir, subdirs, files in os.walk(path2folder):      
            for file in files:
                output_dir = path2folder + f"/gpt_{file}/"
                print(os.path.join(dir, file))
                if file.endswith(file_type):
                    path2file = os.path.join(dir, file)
                    
                    file_result = file_class.get_file_info(path2file, file_type)
                    if file_result is not None:
                        file_content, file_name = file_result
                        if file_content is not None and file_name is not None:
                            print(f"Filename: {file_name}")
                            print(f"Content:\n{file_content}")
                        else:
                            raise Exception ("Error encountered during reading file content or retrieving filename.")                     

                translation_result = gpt_translation.gpt_translate(file_content, None)
                print(translation_result)
                
                # Remove comments
                clean_code = file_class.remove_comments(translation_result)                        
                print(clean_code)

                # write translated code into correct Rust format
                rust_file = os.path.splitext(file)[0] + ".rs"
                with open(output_dir + rust_file, "w") as f:
                        f.write(clean_code)
        print ("Translation Successfully Completed")
        return True
    else:
        print(f"ERROR: {path2folder} not found")
        return False

path2folder = "/home/wsh/test/connector"
print(translate(path2folder))


