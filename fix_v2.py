#!/usr/bin/env python3
# Simple fix - just add (e) { after catch if missing.

with open('src/App.tsx', 'r', encoding='utf-8') as f:
    lines = f.readlines()

new_lines = []
i = 0
while i < len(lines):
    line = lines[i]
    
    # Check if line ends with 'catch ' (with possible whitespace)
    stripped = line.rstrip()
    if stripped.endswith('catch'):
        # Add '(e) {' to the end
        line = stripped + ' (e) {\r\n'
        
        # Check if next line is blank (just whitespace + \r\n)
        if i + 1 < len(lines) and lines[i+1].strip() == '':
            i += 1  # Skip blank line
    
    new_lines.append(line)
    i += 1

with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.writelines(new_lines)

print("Done - added (e) { after catch")
