class CommentRemover:
    
    def __init__(self):
        self.is_block = False
        self.single_line_comment = ["//", "```"]
        self.comment_type_pairs = [["/*", "*", "*/"]]

    def set_is_block(self, is_block):
        self.is_block = is_block

    def remove_comment(self, file):
        clean_lines = []
        with open(file, 'r') as f:
            lines = f.readlines()
                    
        for line in lines:
            clean_line = line
            
            # Skip empty line
            if not line.strip():
                continue
            
            content = line.split(" ")
            
            # Check single-line comments 
            for slc in self.single_line_comment:
                if slc in content:
                    # Remove everything after the single-line comment
                    clean_line = ' '.join(content[:content.index(slc)]) + "\n"
                    break  # Exit loop as we've found a comment

            # Check for inline block comments (code + block comment on the same line)
            for ctp in self.comment_type_pairs:
                if ctp[0] in content and ctp[-1] in content: 
                    # Combine code before the block comment and after the closing tag
                    prefix = content[:content.index(ctp[0])]
                    suffix = content[-len(content.index(ctp[-1])) + 1:]                    
                    clean_line = ' '.join(prefix + suffix) + "\n"
                    break

            # Check for block comments that span multiple lines
            # if self.is_block:
            #     # Check for the end of the block comment
            #     if any(ctp[-1] in content for ctp in self.comment_type_pairs):
            #         self.set_is_block(False)

            # else:
            #     # Check for the start of a block comment
            #     if any(ctp[0] in content for ctp in self.comment_type_pairs):
            #         self.set_is_block(True)
                    
            #         # If the block comment starts and ends on the same line, close it
            #     elif any(ctp[-1] in content for ctp in self.comment_type_pairs):
            #         self.set_is_block(True)

            # Append the cleaned line if it's not empty and we're not inside a block comment
            if clean_line.strip() and self.is_block == False:
                clean_lines.append(clean_line)
                
        return ''.join(clean_lines)