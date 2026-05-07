#!/usr/bin/env python3
# Fix catch blocks missing closing }

with open('src/App.tsx', 'rb') as f:
    content = f.read()

# Pattern: } catch (e) {\r\n  };
# This means: catch block opens, but immediately there's }; (end of arrow function)
# We need to add } before };
old = b'} catch (e) {\r\n  };'
new = b'} catch (e) {\r\n    // error ignored\r\n  }\r\n  };'

count = content.count(old)
print(f'Found {count} occurrences of catch block missing closing brace')

if count > 0:
    content = content.replace(old, new)
    with open('src/App.tsx', 'wb') as f:
        f.write(content)
    print(f'Fixed {count} occurrences!')
else:
    print('Pattern not found, trying alternative...')
    # Maybe the whitespace is different
    # Let's show what's around 'catch (e) {'
    pos = content.find(b'catch (e) {')
    if pos >= 0:
        # Show context
        start = max(0, pos - 50)
        end = min(len(content), pos + 100)
        print(f'Found catch at {pos}: ...{content[start:pos]}***{content[pos:end]}...')
    else:
        print('catch (e) { not found at all!')
