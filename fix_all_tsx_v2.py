#!/usr/bin/env python3
import os
import re

def fix_file(filepath):
    # Read as binary to preserve line endings
    with open(filepath, 'rb') as f:
        content = f.read()
    
    original = content
    fixes = 0
    
    # Fix 1: catch \r\n or catch \n followed by };
    # Pattern: catch \r\n    };
    # or: catch \n    };
    # We need to add (e) { and closing }
    
    # Find all positions of 'catch \r\n' or 'catch \n'
    # We'll do a simple find and replace
    
    # Pattern 1: catch \r\n    }; (with possible spaces before };)
    pattern1 = rb'catch \r\n\s*\};'
    replacement1 = b'catch (e) {\r\n        // error ignored\r\n    }\r\n    };'
    
    count1 = len(re.findall(pattern1, content))
    if count1 > 0:
        content = re.sub(pattern1, replacement1, content)
        fixes += count1
        print(f'  Fixed {count1} catch \\r\\n }}; patterns')
    
    # Pattern 2: catch \n    }; (Unix line endings)
    pattern2 = rb'catch \n\s*\};'
    replacement2 = b'catch (e) {\n        // error ignored\n    }\n    };'
    
    count2 = len(re.findall(pattern2, content))
    if count2 > 0:
        content = re.sub(pattern2, replacement2, content)
        fixes += count2
        print(f'  Fixed {count2} catch \\n }}; patterns')
    
    # Fix 2: catch (e) { at end of line but missing closing }
    # Pattern: catch (e) {\r\n    };
    pattern3 = rb'catch \(e\) \{\r\n\s*\};'
    replacement3 = b'catch (e) {\r\n        // error ignored\r\n    }\r\n    };'
    
    count3 = len(re.findall(pattern3, content))
    if count3 > 0:
        content = re.sub(pattern3, replacement3, content)
        fixes += count3
        print(f'  Fixed {count3} catch (e) { missing }')
    
    # Pattern 4: catch (e) {\n    }; (Unix)
    pattern4 = rb'catch \(e\) \{\n\s*\};'
    replacement4 = b'catch (e) {\n        // error ignored\n    }\n    };'
    
    count4 = len(re.findall(pattern4, content))
    if count4 > 0:
        content = re.sub(pattern4, replacement4, content)
        fixes += count4
        print(f'  Fixed {count4} catch (e) { missing } (Unix)')
    
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
        fixes = fix_file(filepath)
        if fixes > 0:
            print(f'{filename}: {fixes} fixes')
            files_fixed += 1
            total_fixes += fixes

print(f'\nDone! Fixed {files_fixed} files, {total_fixes} total fixes')
