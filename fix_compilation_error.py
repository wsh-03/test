import subprocess
import os
# import json

# # define the path to the rust file
# path2rust = "e:\work\test\test.rs"
# # define the file name and the output executable file name
# output_executable = "e:\work\test\test.exe"

# # run the rust compiler to compile the rust file
# result = subprocess.run(["rustc", path2rust, '-o', output_executable], 
#                         capture_output=True, text=True)

# def write_script():
#     script_content = """
#     rustc /home/wsh-v22/test/work/test/test.rs --error-format json 2>&1 | tee error.json
#     """
#     return script_content

def compile_error(path2linux):
    # path2script = "/home/wsh-v22/test/work/test/bash_script.sh"
    # script = write_script()
    # with open(path2script, "w") as f:
    #     f.write(script)
    # os.chmod(path2script, 0o777)
    if not os.path.exists(path2linux):
        return f"Error: {path2linux} does not exist"
    else:
        os.chdir(path2linux)
        
    try:
        result = subprocess.run(['make', '-C', path2linux, 'LLVM=1','ARCH=x86_64'], capture_output=True, text=True)
    except Exception as e:
        return f"Error occured when run compilation:{e}"
    return result


# path2file = "/home/wsh-v22/test/work/exp/pci_bus/pci_bus.rs"
# if compile_error(path2file).stderr == "":
#     print("No Compilaton Error")
# else:
#     print(compile_error(path2file).stderr)
