#!/usr/bin/env python3
"""
Script to remove comments from code files.
Supports: //, /* */, /** */, #, <!-- -->, and removes .md files.
"""

import re
import sys
import os

def remove_comments(content, file_ext):
    """
    Remove comments based on file extension.
    """
    if file_ext in ['.js', '.ts', '.jsx', '.tsx', '.java', '.c', '.cpp', '.rs', '.go', '.swift', '.css']:
        # First remove JSX comments {/* ... */}
        content = re.sub(r'\{\s*/\*.*?\*/\s*\}', '', content, flags=re.DOTALL)
        
        # Remove multi-line comments /* ... */ including Javadoc /** ... */
        content = re.sub(r'/\*.*?\*/', '', content, flags=re.DOTALL)
        
        # Remove single-line comments // ...
        # But be careful not to remove URLs like http://
        lines = content.split('\n')
        cleaned_lines = []
        for line in lines:
            # Check if // is inside a string
            in_string = False
            string_char = None
            i = 0
            comment_start = -1
            while i < len(line):
                char = line[i]
                if in_string:
                    if char == string_char and (i == 0 or line[i-1] != '\\'):
                        in_string = False
                elif char in ['"', "'", '`']:
                    in_string = True
                    string_char = char
                elif line[i:i+2] == '//':
                    comment_start = i
                    break
                i += 1
            
            if comment_start >= 0:
                line = line[:comment_start].rstrip()
            
            cleaned_lines.append(line)
        content = '\n'.join(cleaned_lines)
        
        # Remove excessive empty lines (more than 2 in a row)
        lines = content.split('\n')
        cleaned_lines = []
        empty_count = 0
        for line in lines:
            if line.strip() == '':
                empty_count += 1
                if empty_count <= 2:
                    cleaned_lines.append(line)
            else:
                empty_count = 0
                cleaned_lines.append(line)
        
        # Remove trailing empty lines
        while cleaned_lines and cleaned_lines[-1].strip() == '':
            cleaned_lines.pop()
        
        content = '\n'.join(cleaned_lines)
        
    elif file_ext in ['.py', '.sh', '.bash', '.yaml', '.yml', '.rb']:
        # Remove # comments (but not inside strings)
        lines = content.split('\n')
        cleaned_lines = []
        for line in lines:
            in_string = False
            string_char = None
            i = 0
            comment_start = -1
            while i < len(line):
                char = line[i]
                if in_string:
                    if char == string_char and (i == 0 or line[i-1] != '\\'):
                        in_string = False
                elif char in ['"', "'"]:
                    in_string = True
                    string_char = char
                elif char == '#':
                    comment_start = i
                    break
                i += 1
            
            if comment_start >= 0:
                line = line[:comment_start].rstrip()
            
            cleaned_lines.append(line)
        content = '\n'.join(cleaned_lines)
        
        # Remove excessive empty lines
        lines = content.split('\n')
        cleaned_lines = []
        empty_count = 0
        for line in lines:
            if line.strip() == '':
                empty_count += 1
                if empty_count <= 2:
                    cleaned_lines.append(line)
            else:
                empty_count = 0
                cleaned_lines.append(line)
        
        # Remove trailing empty lines
        while cleaned_lines and cleaned_lines[-1].strip() == '':
            cleaned_lines.pop()
        
        content = '\n'.join(cleaned_lines)
        
    elif file_ext in ['.html', '.xml', '.vue']:
        # Remove HTML/XML comments <!-- ... -->
        content = re.sub(r'<!--.*?-->', '', content, flags=re.DOTALL)
        
    elif file_ext in ['.sql']:
        # Remove -- comments and /* */ comments
        content = re.sub(r'/\*.*?\*/', '', content, flags=re.DOTALL)
        lines = content.split('\n')
        cleaned_lines = []
        for line in lines:
            comment_start = line.find('--')
            if comment_start >= 0:
                line = line[:comment_start].rstrip()
            cleaned_lines.append(line)
        content = '\n'.join(cleaned_lines)
    
    return content

def process_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original_content = content
        file_ext = os.path.splitext(filepath)[1].lower()
        
        if file_ext in ['.md']:
            print(f"Skipping .md file (will be deleted): {filepath}")
            return 'md'
        
        cleaned_content = remove_comments(content, file_ext)
        
        if cleaned_content != original_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(cleaned_content)
            print(f"Processed: {filepath}")
            return 'modified'
        else:
            print(f"No comments found: {filepath}")
            return 'unchanged'
        
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return 'error'

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python remove_comments.py <file_or_directory>")
        sys.exit(1)
    
    target = sys.argv[1]
    
    if os.path.isfile(target):
        process_file(target)
    elif os.path.isdir(target):
        print(f"Processing directory: {target}")
        modified = 0
        unchanged = 0
        errors = 0
        md_files = []
        
        for root, dirs, files in os.walk(target):
            # Skip node_modules, .git, target directories
            dirs[:] = [d for d in dirs if d not in ['node_modules', '.git', 'target', 'dist', '__pycache__', '.idea']]
            
            for file in files:
                filepath = os.path.join(root, file)
                file_ext = os.path.splitext(file)[1].lower()
                
                # Skip binary files and certain extensions
                if file_ext in ['.png', '.jpg', '.jpeg', '.gif', '.ico', '.svg', '.woff', '.woff2', '.ttf', '.eot', '.exe', '.dll', '.class', '.jar', '.zip', '.tar', '.gz']:
                    continue
                
                result = process_file(filepath)
                if result == 'modified':
                    modified += 1
                elif result == 'unchanged':
                    unchanged += 1
                elif result == 'md':
                    md_files.append(filepath)
                else:
                    errors += 1
        
        print(f"\n=== Summary ===")
        print(f"Modified: {modified}")
        print(f"Unchanged: {unchanged}")
        print(f"Errors: {errors}")
        print(f"Markdown files to delete: {len(md_files)}")
        
        if md_files:
            print("\nMarkdown files found:")
            for f in md_files:
                print(f"  {f}")
    else:
        print(f"Error: {target} is not a valid file or directory")
        sys.exit(1)
