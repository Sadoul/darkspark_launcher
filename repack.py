"""
Repacks the patched class back into the JAR, then repacks the JAR into the final ZIP.
"""

import zipfile
import os
import shutil

lib_dir = r'C:\Users\smopo\Downloads\explyt-work\bootstrap\lib'
jar_name = 'Agent Plugins.ij-tool-chat.jar'
jar_path = os.path.join(lib_dir, jar_name)

extracted_dir = r'C:\tmp\explyt-toolchat'
patched_class_rel = 'com/explyt/plugin/toolchat/tools/terminal/ExecuteTerminalCommandTool.class'

# Step 1: Repack the JAR with the patched class
# We need to update the JAR in-place.
# Strategy: create a new JAR, copy all entries, replace the patched class.

new_jar_path = jar_path + '.new'
patched_class_path = os.path.join(extracted_dir, patched_class_rel.replace('/', os.sep))

print(f"Repacking JAR: {jar_name}")
with zipfile.ZipFile(jar_path, 'r') as orig_jar:
    with zipfile.ZipFile(new_jar_path, 'w', compression=zipfile.ZIP_DEFLATED) as new_jar:
        for item in orig_jar.infolist():
            if item.filename == patched_class_rel:
                # Use patched version
                with open(patched_class_path, 'rb') as pf:
                    new_jar.writestr(item, pf.read())
                print(f"  -> Replaced: {item.filename}")
            else:
                new_jar.writestr(item, orig_jar.read(item.filename))

# Replace original JAR with new one
os.replace(new_jar_path, jar_path)
print(f"JAR repacked: {jar_path}")

# Step 2: Repack the main ZIP with the updated JAR
orig_zip = r'C:\Users\smopo\Downloads\explyt-obfuscated-5.8.0-IJ-261-mp.zip'
new_zip = r'C:\Users\smopo\Downloads\explyt-patched-5.8.0-IJ-261-mp.zip'
work_dir = r'C:\Users\smopo\Downloads\explyt-work'

# The JAR inside the ZIP has path: bootstrap/lib/Agent Plugins.ij-tool-chat.jar
zip_entry_name = 'bootstrap/lib/Agent Plugins.ij-tool-chat.jar'

print(f"\nRepacking main ZIP...")
with zipfile.ZipFile(orig_zip, 'r') as orig:
    with zipfile.ZipFile(new_zip, 'w', compression=zipfile.ZIP_DEFLATED, allowZip64=True) as new:
        for item in orig.infolist():
            if item.filename == zip_entry_name:
                with open(jar_path, 'rb') as jf:
                    new.writestr(item, jf.read())
                print(f"  -> Replaced: {item.filename}")
            else:
                new.writestr(item, orig.read(item.filename))

print(f"\nDone! Patched plugin ZIP: {new_zip}")
print("Install this ZIP in IntelliJ via Settings -> Plugins -> Install Plugin from Disk")
