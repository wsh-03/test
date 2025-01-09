import re

# The input log as a multiline string
log_message = """[Your entire log content here]"""  # Replace this with your actual log content.

# Regular expression to capture error messages
error_pattern = re.compile(r"^error:.*", re.MULTILINE)

# Find all matches
error_messages = error_pattern.findall(log_message)

# Print or process the extracted error messages
print("Extracted Error Messages:")
for error in error_messages:
    print(error)
