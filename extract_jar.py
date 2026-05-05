import zipfile
import os

lib_dir = r'C:\Users\smopo\Downloads\explyt-work\bootstrap\lib'
jar_name = 'Agent Plugins.ij-tool-chat.jar'
out_dir = r'C:\tmp\explyt-toolchat'

os.makedirs(out_dir, exist_ok=True)

jar_path = os.path.join(lib_dir, jar_name)
with zipfile.ZipFile(jar_path, 'r') as zf:
    zf.extractall(out_dir)
    print(f'Extracted {len(zf.namelist())} files to {out_dir}')

props_path = os.path.join(out_dir, 'messages', 'ToolChatTexts.properties')
if os.path.exists(props_path):
    with open(props_path, 'r', encoding='utf-8') as f:
        content = f.read()
    print('\n--- ToolChatTexts.properties ---')
    for line in content.splitlines():
        if 'danger' in line.lower() or 'change' in line.lower() or 'command' in line.lower():
            print(line)
