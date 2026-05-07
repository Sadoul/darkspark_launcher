#!/usr/bin/env python3
import re
import os

def fix_catch_in_file(filepath):
    with open(filepath, 'rb') as f:
        content = f.read()
    
    original = content
    
    # Fix 1: catch at end of line without (e) {
    # Pattern: } catch \r\n    (before }, or };)
    # Replace with: } catch (e) {\r\n        // error ignored\r\n    }\r\n
    pattern1 = rb'\} catch \r\n'
    replacement1 = b'} catch (e) {\r\n        // error ignored\r\n    }\r\n'
    
    # Count occurrences
    count1 = content.count(pattern1)
    if count1 > 0:
        # We need to add missing braces
        content = content.replace(pattern1, replacement1)
    
    # Fix 2: catch (e) { at end of line, missing closing }
    # Pattern: } catch (e) {\r\n    };
    pattern2 = rb'catch \(e\) \{\r\n    \};'
    replacement2 = b'catch (e) {\r\n        // error ignored\r\n    }\r\n    };'
    
    count2 = content.count(pattern2)
    if count2 > 0:
        content = content.replace(pattern2, replacement2)
    
    # Fix 3: General fix for try { ... } catch \r\n    };
    # Find: } catch \r\n    };
    pattern3 = rb'try \{[^}]*\} catch \r\n    \};'
    # Replace with proper catch block
    def repl(match):
        return match.group(0).replace(b'catch \r\n    };', b'catch (e) {\r\n        // error ignored\r\n    }\r\n    };')
    
    content = re.sub(pattern3, repl, content, flags=re.DOTALL)
    
    if content != original:
        with open(filepath, 'wb') as f:
            f.write(content)
        return True, count1 + count2
    return False, 0

# Process all .tsx files in src/components/
components_dir = 'src/components'
fixed_files = 0
total_fixes = 0

for filename in os.listdir(components_dir):
    if filename.endswith('.tsx'):
        filepath = os.path.join(components_dir, filename)
        fixed, count = fix_catch_in_file(filepath)
        if fixed:
            print(f'Fixed {filename}: {count} fixes')
            fixed_files += 1
            total_fixes += count

print(f'\nTotal: Fixed {fixed_files} files, {total_fixes} fixes')
