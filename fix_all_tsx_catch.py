#!/usr/bin/env python3
import re
import os

def fix_file(filepath):
    with open(filepath, 'rb') as f:
        content = f.read()
    
    original = content
    fixes = 0
    
    # Fix 1: catch without parameter at end of line
    # Pattern: } catch \r\n    (before }; or , 700); etc.)
    pattern1 = rb'catch \r\n'
    replacement1 = b'} catch (e) {\r\n        // error ignored\r\n    }\r\n'
    
    # We need to be smart - check what follows catch
    # If it's };, then we need to add }
    # If it's }, 700); then we need to add } before }, 700);
    
    # Let's find all positions of 'catch \r\n'
    pos = 0
    while True:
        pos = content.find(b'catch \r\n', pos)
        if pos == -1:
            break
        
        # Check what comes after catch \r\n
        next_part = content[pos+len(b'catch \r\n'):pos+len(b'catch \r\n')+50]
        
        # If next non-whitespace is };
        if b'};' in next_part[:20]:
            # Add closing brace
            content = content[:pos] + b'} catch (e) {\r\n        // error ignored\r\n    }\r\n' + content[pos+len(b'catch \r\n'):]
            fixes += 1
        # If next is }, 700); or similar
        elif b'}, 700);' in next_part:
            # Insert } before }, 700);
            idx = content.find(b'}, 700);', pos)
            if idx > pos:
                content = content[:idx] + b'\r\n    }\r\n' + content[idx:]
                # Now fix the catch line
                content = content[:pos] + b'} catch (e) {\r\n        // error ignored' + content[pos+len(b'catch \r\n'):]
                fixes += 1
        else:
            # General fix
            content = content[:pos] + b'} catch (e) {\r\n        // error ignored\r\n    }\r\n' + content[pos+len(b'catch \r\n'):]
            fixes += 1
        
        pos += 1  # Move forward
    
    # Fix 2: catch (e) { at end of line without closing }
    # Pattern: } catch (e) {\r\n    (before }, or };)
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
        fixes = fix_file(filepath)
        if fixes > 0:
            print(f'{filename}: {fixes} fixes')
            files_fixed += 1
            total_fixes += fixes

print(f'\nTotal: {files_fixed} files fixed, {total_fixes} fixes')
