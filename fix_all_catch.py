#!/usr/bin/env python3
# Fix ALL catch blocks that are missing closing }

with open('src/App.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

# Find pattern: catch (e) { at end of line, followed by another catch or };
# We need to add } before the next catch or };

import re

# Fix 1: catch (e) { followed by another catch
# Pattern: } catch (e) {\r\n      } catch (e) {
old1 = r'catch \(e\) \{\r\n      \} catch \(e\) \{'
new1 = r'catch (e) {\r\n      }\r\n    } catch (e) {'
content = re.sub(old1, new1, content)

# Fix 2: catch (e) { followed by }; (end of arrow function)
# Pattern: } catch (e) {\r\n  };
old2 = r'catch \(e\) \{\r\n  \};'
new2 = r'catch (e) {\r\n    // error ignored\r\n  }\r\n};'
content = re.sub(old2, new2, content)

# Write back
with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed all catch blocks!")
