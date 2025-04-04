import os
from file_utility import FileProcessor
import gpt_translation
class CodeTranslator:
    FILE_TYPE = ".c"
    
    def translate(self, path2folder):
        file_class = FileProcessor()
        # Check if the provided path is a valid directory
        if (os.path.isdir(path2folder)):
            print("\nPath checked successfully\n")
            for dir, subdirs, files in os.walk(path2folder):      
                for file in files:
                    output_dir = path2folder + f"/d_{file}/"
                    print(os.path.join(dir, file))
                    if file.endswith(self.FILE_TYPE):
                        path2file = os.path.join(dir, file)
                        file_result = file_class.get_file_info(path2file, self.FILE_TYPE)
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
                        
                        # Write translated code into correct Rust format
                        rust_bn = os.path.splitext(file)[0] + ".rs"
                        with open(output_dir + rust_bn, "w") as f:
                            f.write(clean_code)
            print ("Translation Successfully Completed")
            return True
        else:
            print(f"ERROR: {path2folder} not found")
            return False


if __name__ == "__main__":
    translator = CodeTranslator()
    path2folder = "/home/wsh/test/connector"
    print(translator.translate(path2folder))
