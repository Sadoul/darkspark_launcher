"""
Patches ExecuteTerminalCommandTool.class so that checkDangerLevel()
always returns DangerLevel$Safe.INSTANCE (disables the dangerous-command warning).

Strategy: parse the class file binary, find the checkDangerLevel(ExecuteTerminalCommandArgs)
method's Code attribute, and replace its bytecode with:
  getstatic  <DangerLevel$Safe.INSTANCE>   (opcode b2, 2-byte index)
  checkcast  <DangerLevel>                 (opcode c0, 2-byte index)
  areturn                                  (opcode b0)
  nop * N   (pad to original length)

We keep the same Code attribute size so the class file stays valid.
"""

import struct
import sys

CLASS_FILE = r'C:\tmp\explyt-toolchat\com\explyt\plugin\toolchat\tools\terminal\ExecuteTerminalCommandTool.class'

with open(CLASS_FILE, 'rb') as f:
    data = bytearray(f.read())

# ---- minimal class-file parser ----

pos = 0

def read_u1():
    global pos
    v = data[pos]; pos += 1; return v

def read_u2():
    global pos
    v = struct.unpack_from('>H', data, pos)[0]; pos += 2; return v

def read_u4():
    global pos
    v = struct.unpack_from('>I', data, pos)[0]; pos += 4; return v

def skip(n):
    global pos
    pos += n

magic = read_u4()
assert magic == 0xCAFEBABE, "Not a class file"
minor = read_u2()
major = read_u2()

# --- constant pool ---
cp_count = read_u2()
cp = [None] * cp_count   # 1-based

i = 1
while i < cp_count:
    tag = read_u1()
    if tag == 1:   # Utf8
        length = read_u2()
        value = data[pos:pos+length].decode('utf-8', errors='replace')
        skip(length)
        cp[i] = ('Utf8', value)
    elif tag == 3 or tag == 4:  # Integer / Float
        skip(4)
        cp[i] = (tag,)
    elif tag == 5 or tag == 6:  # Long / Double
        skip(8)
        cp[i] = (tag,)
        i += 1   # takes two slots
    elif tag == 7:  # Class
        idx = read_u2()
        cp[i] = ('Class', idx)
    elif tag == 8:  # String
        idx = read_u2()
        cp[i] = ('String', idx)
    elif tag in (9, 10, 11):  # Fieldref, Methodref, InterfaceMethodref
        c = read_u2(); nt = read_u2()
        cp[i] = ('Ref', tag, c, nt)
    elif tag == 12: # NameAndType
        n = read_u2(); t = read_u2()
        cp[i] = ('NameAndType', n, t)
    elif tag == 15: # MethodHandle
        skip(3)
        cp[i] = (tag,)
    elif tag == 16: # MethodType
        skip(2)
        cp[i] = (tag,)
    elif tag == 17 or tag == 18: # Dynamic / InvokeDynamic
        skip(4)
        cp[i] = (tag,)
    elif tag == 19 or tag == 20: # Module / Package
        skip(2)
        cp[i] = (tag,)
    else:
        raise ValueError(f'Unknown cp tag {tag} at cp index {i}, file offset {pos-1}')
    i += 1

# Find constant pool index for "Code" utf8
def find_utf8(s):
    for idx, e in enumerate(cp):
        if e and e[0] == 'Utf8' and e[1] == s:
            return idx
    return None

code_utf8 = find_utf8('Code')
check_danger_utf8 = find_utf8('checkDangerLevel')

print(f"cp index for 'Code': {code_utf8}")
print(f"cp index for 'checkDangerLevel': {check_danger_utf8}")

# --- access flags, this, super ---
skip(2); skip(2); skip(2)

# --- interfaces ---
iface_count = read_u2()
skip(iface_count * 2)

# --- fields ---
field_count = read_u2()
for _ in range(field_count):
    skip(2); skip(2); skip(2)
    attr_count = read_u2()
    for _ in range(attr_count):
        skip(2)
        alen = read_u4()
        skip(alen)

# --- methods ---
method_count = read_u2()

# We need to find checkDangerLevel(ExecuteTerminalCommandArgs) - NOT the bridge method.
# Bridge method has descriptor: (Ljava/lang/Object;)Lcom/explyt/plugin/toolchat/tools/DangerLevel;
# Real method has descriptor: (Lcom/explyt/plugin/toolchat/tools/terminal/ExecuteTerminalCommandArgs;)Lcom/explyt/plugin/toolchat/tools/DangerLevel;

target_name = 'checkDangerLevel'
# We want the one with ExecuteTerminalCommandArgs in descriptor, not Object
target_desc_substring = 'ExecuteTerminalCommandArgs'

code_attr_pos = None   # byte offset in data[] of the Code attribute's content start (after attr_name+len)
code_length_pos = None # byte offset of the code_length field (4 bytes before the actual code)

for m in range(method_count):
    m_start = pos
    access = read_u2()
    name_idx = read_u2()
    desc_idx = read_u2()
    attr_count = read_u2()

    m_name = cp[name_idx][1] if cp[name_idx] else ''
    m_desc = cp[desc_idx][1] if cp[desc_idx] else ''

    for a in range(attr_count):
        attr_name_idx = read_u2()
        attr_len = read_u4()
        attr_start = pos
        attr_name = cp[attr_name_idx][1] if cp[attr_name_idx] else ''

        if (m_name == target_name and
                target_desc_substring in m_desc and
                attr_name == 'Code'):
            print(f"Found method: {m_name}{m_desc}")
            print(f"  Code attribute at file offset {attr_start}, length={attr_len}")
            # Code attribute layout: max_stack(2), max_locals(2), code_length(4), code(N), ...
            max_stack = struct.unpack_from('>H', data, attr_start)[0]
            max_locals = struct.unpack_from('>H', data, attr_start+2)[0]
            code_len = struct.unpack_from('>I', data, attr_start+4)[0]
            code_start = attr_start + 8
            print(f"  max_stack={max_stack}, max_locals={max_locals}, code_length={code_len}")
            print(f"  Code bytes start at file offset {code_start}")
            code_attr_pos = attr_start
            code_length_pos = attr_start + 4
            actual_code_start = code_start
            actual_code_len = code_len

        skip(attr_len)

if code_attr_pos is None:
    print("ERROR: could not find checkDangerLevel Code attribute!")
    sys.exit(1)

# ---- Now find the constant pool indices used in the method ----
# From javap output:
#   getstatic #42  // Field DangerLevel$Safe.INSTANCE
#   checkcast #18  // class DangerLevel (base class)
#
# These CP indices are 1-based in the class file.
# Let's find them by looking at the CP entries.

def find_fieldref_for(class_name_substr, field_name):
    for idx, e in enumerate(cp):
        if e and e[0] == 'Ref' and e[1] == 9:  # Fieldref
            class_idx = e[2]
            nt_idx = e[3]
            class_entry = cp[class_idx]
            nt_entry = cp[nt_idx]
            if class_entry and class_entry[0] == 'Class':
                cname = cp[class_entry[1]][1] if cp[class_entry[1]] else ''
                fname = cp[nt_entry[1]][1] if nt_entry and nt_entry[0] == 'NameAndType' else ''
                if class_name_substr in cname and fname == field_name:
                    return idx, cname
    return None, None

def find_class_ref(class_name_substr):
    for idx, e in enumerate(cp):
        if e and e[0] == 'Class':
            cname = cp[e[1]][1] if cp[e[1]] else ''
            if class_name_substr in cname and 'Dangerous' not in cname and 'Safe' not in cname:
                # We want the base DangerLevel class
                return idx, cname
    return None, None

safe_instance_idx, safe_class = find_fieldref_for('DangerLevel$Safe', 'INSTANCE')
danger_level_class_idx, danger_class = find_class_ref('DangerLevel')

print(f"\nFieldref for DangerLevel$Safe.INSTANCE: cp[{safe_instance_idx}] = {safe_class}")
print(f"Class ref for DangerLevel: cp[{danger_level_class_idx}] = {danger_class}")

if safe_instance_idx is None or danger_level_class_idx is None:
    print("ERROR: could not find required constant pool entries!")
    sys.exit(1)

# Build new bytecode:
# getstatic <safe_instance_idx>   -> b2 HH HH
# checkcast <danger_level_class_idx> -> c0 HH HH
# areturn                          -> b0
# nop * (code_len - 7)
new_code = bytearray()
new_code += bytes([0xb2]) + struct.pack('>H', safe_instance_idx)   # getstatic
new_code += bytes([0xc0]) + struct.pack('>H', danger_level_class_idx)  # checkcast
new_code += bytes([0xb0])   # areturn
# Pad with nops to keep same length
while len(new_code) < actual_code_len:
    new_code += b'\x00'

print(f"\nNew bytecode ({len(new_code)} bytes): {new_code[:10].hex()}...")

# Patch data
data[actual_code_start:actual_code_start + actual_code_len] = new_code

# Also update max_stack to 1 (we only need 1 for getstatic result)
struct.pack_into('>H', data, code_attr_pos, 1)

# Write patched class
out_path = CLASS_FILE
with open(out_path, 'wb') as f:
    f.write(data)

print(f"\nPatched class written to: {out_path}")
print("checkDangerLevel will now always return DangerLevel$Safe (no more warning dialog)")
