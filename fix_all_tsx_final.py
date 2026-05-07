#!/usr/bin/env python3
import os
import re

def fix_tsx_file(filepath):
    with open(filepath, 'rb') as f:
        content = f.read()
    
    original = content
    fixes = 0
    
    # Fix 1: catch without parameter at end of line
    # Pattern: catch \r\n (before }; or , 700); etc.)
    # Replace with: catch (e) {\r\n        // error ignored\r\n    }\r\n
    
    # Find all catch \r\n patterns
    # We need to add (e) { and closing }
    pattern1 = rb'catch \r\n'
    replacement1 = rb'catch (e) {\r\n        // error ignored\r\n    }\r\n'
    
    # We need to be smart - only fix if not followed by (e)
    # Let's find positions of 'catch \r\n'
    pos = 0
    while True:
        pos = content.find(pattern1, pos)
        if pos == -1:
            break
        
        # Check if this catch already has (e) nearby
        # Look at next 50 bytes
        next_part = content[pos+len(pattern1):pos+len(pattern1)+50]
        
        # If next part contains (e) {, then skip
        if b'(e)' in next_part[:20]:
            pos += len(pattern1)
            continue
            
        # Need to fix
        # Replace catch \r\n with catch (e) {\r\n        content = content[:pos] + b'catch (e) {\r\n        // error ignored\r\n    }\r\n' + content[pos+len(pattern1):]
        fixes += 1
        
    # Fix 2: catch (e) { at end of line without closing }
    # This is hard to detect, skip for now
    
    if content != original:
        with open(filepath, 'wb') as f:
            f.write(content)
        return fixes
    return 0

# Process all .tsx files in src/components/
components_dir = 'src/components'
total_fixes = 0
files_fixed = 0

for filename in os.listdir(components_dir):
    if filename.endswith('.tsx'):
        filepath = os.path.join(components_dir, filename)
        fixes = fix_tsx_file(filepath)
        if fixes > 0:
            print(f'{filename}: {fixes} fixes')
            files_fixed += 1
            total_fixes += fixes

print(f'\nTotal: {files_fixed} files fixed, {total_fixes} fixes')
