#!/usr/bin/env python3
import sys

path = r'C:\Users\Gfran\Desktop\Email-Validador\packages\shared\src\lib.rs'
with open(path, 'r', encoding='utf8') as f:
    content = f.read()

# Replace the specific section - fix the match to produce consistent types
old = """            match (stored_hash, stored_salt) {
                (Some(hash), Some(salt)) => {
                    // Usar hash y salt existentes
                    let password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin12345".to_string());
                    // Crear AdminPasswordHash desde los valores almacenados
                    AdminPasswordHash {
                        salt,
                        hash,
                    }
                }
                _ => {
                    // Generar nuevos hash con la contraseña actual del .env
                    let raw_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin12345".to_string());
                    let hash = AdminPasswordHash::new(&raw_password);
                    (raw_password, hash)
                }
            }"""

new = """            match (stored_hash, stored_salt) {
                (Some(hash), Some(salt)) => {
                    // Usar hash y salt existentes
                    let password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin12345".to_string());
                    // Crear AdminPasswordHash desde los valores almacenados
                    (password, AdminPasswordHash {
                        salt,
                        hash,
                    })
                }
                _ => {
                    // Generar nuevos hash con la contraseña actual del .env
                    let raw_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin12345".to_string());
                    let hash = AdminPasswordHash::new(&raw_password);
                    (raw_password, hash)
                }
            }"""

if old in content:
    content = content.replace(old, new)
    with open(path, 'w', encoding='utf8') as f:
        f.write(content)
    print("Reemplazo exitoso")
else:
    print("No se encontró el patrón antiguo")
    lines = content.split('\n')
    for i in range(219, 236):
        print(f"{i+1}: {lines[i]}")