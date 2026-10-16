#!/usr/bin/env python3
import sys

path = r"C:\Users\Gfran\Desktop\Email-Validador\packages\api\src\admin_ui.rs"
with open(path, 'r', encoding='utf8') as f:
    content = f.read()

# First change: master key hiding in loadKeys function
old_section1 = """        keys.forEach(k => {
          const tr = document.createElement('tr');
          const isMaster = k.id === 'master-key';

          // Expiration status"""
          
new_section1 = """        keys.forEach(k => {
          # Ocultar la clave maestra del .env - no mostrar su valor por seguridad
          if (k.id === 'master-key') {
            const tr = document.createElement('tr');
            tr.innerHTML = '''
              <td colspan="6" style="text-align:center; color:var(--text-muted); font-style:italic;">
                <strong>Clave Maestra</strong><br>
                <span style="font-size:0.7rem; color:var(--text-muted);">Configurada en .env - valor no mostrado por seguridad</span>
              </td>
            ''';
            tbody.appendChild(tr);
            return; // Skip further processing for master key
          }

          const tr = document.createElement('tr');
          const isMaster = k.id === 'master-key';

          # Expiration status"""

if old_section1 in content:
    content = content.replace(old_section1, new_section1)
    print("First replacement done")
else:
    print("First old section NOT found")
    # Debug: show surrounding lines
    lines = content.split('\n')
    for i, line in enumerate(lines[730:750], 731):
        print(f"{i}: {repr(line)}")

# Second change: master key in loadKeysForTester
old_section2 = """      keys.forEach(k => {
        const opt = document.createElement('option');
        opt.value = k.key;
        opt.text = `${k.name} (${k.key.substring(0, 10)}...)`;
        select.appendChild(opt);"""

new_section2 = """      keys.forEach(k => {
        # Skip master key from tester selector
        if (k.id === 'master-key') return;
        const opt = document.createElement('option');
        opt.value = k.key;
        opt.text = `${k.name} (${k.key.substring(0, 10)}...)`;
        select.appendChild(opt);"""

if old_section2 in content:
    content = content.replace(old_section2, new_section2)
    print("Second replacement done")
else:
    print("Second old section NOT found")

with open(path, 'w', encoding='utf8') as f:
    f.write(content)

print("File updated successfully")