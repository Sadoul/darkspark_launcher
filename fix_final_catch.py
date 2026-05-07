#!/usr/bin/env python3
import os
import re

def fix_file(filepath):
    # Read as binary to preserve line endings
    with open(filepath, 'rb') as f:
        content = f.read()
    
    original = content
    fixes = 0
    
    # The bad pattern: } catch \r\n    } catch (e) {
    # We need to remove the first line
    
    # Pattern 1: } catch \r\n    } catch (e) {
    bad_pattern1 = b'} catch \r\n    } catch \(e\) \{'
    # Replace with just: } catch (e) {
    good_replacement1 = b'} catch (e) {'
    
    if re.search(bad_pattern1, content):
        # We need to remove the line break and the first '} catch'
        # Actually, the pattern is: \r\n    } catch (e) {
        # Which means: line ends with '} catch \r\n', next line starts with '    } catch (e) {'
        # So we need to replace '} catch \r\n    } catch (e) {' with '    } catch (e) {'
        
        # Let's find the position of '} catch \r\n    } catch (e) {'
        # Actually, it's easier: remove the line '      } catch \r\n' (or similar)
        
        # Find lines that are just '} catch \r\n'
        lines = content.split(b'\r\n')
        new_lines = []
        i = 0
        while i < len(lines):
            line = lines[i]
            # Check if this line is just '} catch ' (possibly with spaces) and next line starts with '} catch (e) {'
            decoded_line = line.decode('utf-8', errors='ignore').strip()
            if decoded_line == '} catch' or decoded_line.endswith('} catch'):
                # Check if next line has '} catch (e) {'
                if i + 1 < len(lines):
                    next_line = lines[i+1].decode('utf-8', errors='ignore')
                    if '} catch (e)' in next_line:
                        # Skip this line (the bad one)
                        i += 1
                        fixes += 1
                        continue
            new_lines.append(line)
            i += 1
        
        if fixes > 0:
            content = b'\r\n'.join(new_lines)
    
    # Pattern 2: General fix for catch without (e) at end of line
    # Find: } catch \r\n (catch at end of line, no parameter)
    # Replace with: } catch (e) {\r\n
    old2 = b'} catch \r\n'
    # But we need to add (e) {
    # Actually, let's find '} catch \r\n' and replace with '} catch (e) {\r\n'
    if content.count(old2) > 0:
        # This is tricky. Let's just ensure all 'catch' have '(e)'
        # Find 'catch \r\n' and replace with 'catch (e) {\r\n'
        content = content.replace(old2, b'} catch (e) {\r\n')
        fixes += content.count(b'} catch (e) {\r\n')  # This is wrong logic
    
    # Better approach: let's just fix the specific corruption we saw
    # The file has: } catch \r\n    } catch (e) {
    # We want: } catch (e) {  
    # So: find '} catch \r\n    } catch (e) {' and replace with '} catch (e) {'
    
    # Let's do a simple string replacement
    bad = b'} catch \r\n    } catch \(e\) \{'
    good = b'} catch (e) {'
    
    count = content.count(bad)
    if count > 0:
        content = content.replace(bad, good)
        fixes += count
    
    if content != original:
        with open(filepath, 'wb') as f:
            f.write(content)
        return fixes
    return 0

# Process all .tsx files
files_to_fix = [
    'src/components/GamePanel.tsx',
    'src/components/AdminPanel.tsx',
    'src/components/CustomModpackPanel.tsx',
    'src/components/SettingsPanel.tsx',
    'src/components/Sidebar.tsx',
    'src/App.tsx'
]

total_fixes = 0
fixed_files = 0

for fpath in files_to_fix:
    if os.path.exists(fpath):
        fixes = fix_file(fpath)
        if fixes > 0:
            print(f'{fpath}: {fixes} fixes')
            fixed_files += 1
            total_fixes += fixes
        else:
            print(f'{fpath}: no fixes needed')
    else:
        print(f'{fpath}: not found!')

print(f'\nDone! Fixed {fixed_files} files, {total_fixes} total fixes')
