#!/usr/bin/env python3
# Simple fix for catch statements

with open('src/App.tsx', 'r', encoding='utf-8') as f:
    lines = f.readlines()

output = []
i = 0
while i < len(lines):
    line = lines[i]
    
    # Check if line has '} catch ' at the end (missing parameter)
    if line.rstrip().endswith('} catch'):
        # Add '(e) {' to the end of this line
        line = line.rstrip() + ' (e) {\r\n'
        
        # Check if next line is just whitespace (blank line)
        if i + 1 < len(lines) and lines[i+1].strip() == '':
            # Skip the blank line
            i += 1
            
    output.append(line)
    i += 1

with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.writelines(output)

print("Fixed catch statements - simple version")
