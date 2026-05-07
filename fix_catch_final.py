#!/usr/bin/env python3
import os
import re

def fix_component_file(filepath):
    # Read as binary to preserve line endings
    with open(filepath, 'rb') as f:
        content = f.read()
    
    original = content
    fixes = 0
    
    # Pattern 1: catch \r\n    };
    # This is: catch at end of line, then }, 700); or };
    # We need to replace with: catch (e) {\r\n        // error ignored\r\n    }\r\n    };
    
    # Find: catch \r\n    followed by optional whitespace then };
    pattern1 = rb'catch \r\n\s*\};'
    replacement1 = b'catch (e) {\r\n        // error ignored\r\n    }\r\n    };'
    
    count1 = len(re.findall(pattern1, content))
    if count1 > 0:
        content = re.sub(pattern1, replacement1, content)
        fixes += count1
        print(f'  Fixed {count1} catch }} patterns')
    
    # Pattern 2: catch \r\n      };
    # Same but with 6 spaces
    pattern2 = rb'catch \r\n\s*\};'
    replacement2 = b'catch (e) {\r\n          // error ignored\r\n        }\r\n          };'
    
    count2 = len(re.findall(pattern2, content))
    if count2 > 0:
        content = re.sub(pattern2, replacement2, content)
        fixes += count2
        print(f'  Fixed {count2} catch }} patterns (6 spaces)')
    
    # Pattern 3: General catch without (e) { at end of line
    # Find: } catch (e) {\r\n    };
    # This means catch block opens but no closing } before };
    pattern3 = rb'catch \(e\) \{\r\n\s*\};'
    replacement3 = rb'catch (e) \{\r\n        // error ignored\r\n    \}\r\n    \};'
    
    count3 = len(re.findall(pattern3, content))
    if count3 > 0:
        content = re.sub(pattern3, replacement3, content)
        fixes += count3
        print(f'  Fixed {count3} catch blocks missing closing }')
    
    if content != original:
        with open(filepath, 'wb') as f:
            f.write(content)
        return fixes
    return 0

# Process all .tsx files in src/components/
components_dir = 'src/components'
total_fixes = 0
files_fixed = 0

print('Starting fix of all .tsx component files...')

for filename in os.listdir(components_dir):
    if filename.endswith('.tsx'):
        filepath = os.path.join(components_dir, filename)
        fixes = fix_component_file(filepath)
        if fixes > 0:
            print(f'{filename}: {fixes} fixes')
            files_fixed += 1
            total_fixes += fixes

print(f'\nDone! Fixed {files_fixed} files, {total_fixes} total fixes')
