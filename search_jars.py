import zipfile
import os

lib_dir = r'C:\Users\smopo\Downloads\explyt-work\bootstrap\lib'
search_terms = [b'dangerous', b'may change the system', b'looks dangerous']

for fname in os.listdir(lib_dir):
    if not fname.endswith('.jar'):
        continue
    fpath = os.path.join(lib_dir, fname)
    try:
        with zipfile.ZipFile(fpath, 'r') as zf:
            for entry in zf.namelist():
                try:
                    data = zf.read(entry)
                    for term in search_terms:
                        if term in data:
                            print(f'FOUND "{term.decode()}" in {fname} -> {entry}')
                            break
                except Exception:
                    pass
    except Exception as e:
        print(f'Error opening {fname}: {e}')
