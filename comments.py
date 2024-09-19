import re
import os 

def remove_comments(file):
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

