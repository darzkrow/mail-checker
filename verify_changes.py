#!/usr/bin/env python3
import os
import sys

def check_file(path, labels):
    """Check if labels exist in file and return results."""
    results = {}
    try:
        with open(path, 'r', encoding='utf8') as f:
            content = f.read()
        for label in labels:
            results[label] = label in content
    except FileNotFoundError:
        for label in labels:
            results[label] = False
    return results

print("=== Verificación Final de la Implementación ===\n")

all_ok = True

# Check 1: Root Cargo.toml
path = os.path.join(os.getcwd(), "Cargo.toml")
results = check_file(path, ["base64", "sha2", "rand"])
print("1. Root Cargo.toml:")
for label, found in results.items():
    status = "✓" if found else "✗"
    print(f"   {status} {label}")
    if not found:
        all_ok = False

# Check 2: Shared Cargo.toml
path = os.path.join(os.getcwd(), "packages", "shared", "Cargo.toml")
results = check_file(path, ["sha2.workspace", "base64.workspace", "rand.workspace"])
print("\n2. packages/shared/Cargo.toml:")
for label, found in results.items():
    status = "✓" if found else "✗"
    print(f"   {status} {label}")
    if not found:
        all_ok = False

# Check 3: API Cargo.toml
path = os.path.join(os.getcwd(), "packages", "api", "Cargo.toml")
results = check_file(path, ["base64.workspace", "sha2.workspace"])
print("\n3. packages/api/Cargo.toml:")
for label, found in results.items():
    status = "✓" if found else "✗"
    print(f"   {status} {label}")
    if not found:
        all_ok = False

# Check 4: Shared src has correct imports
path = os.path.join(os.getcwd(), "packages", "shared", "src", "lib.rs")
results = check_file(path, ["use sha2", "use base64", "use rand"])
print("\n4. packages/shared/src/lib.rs:")
for label, found in results.items():
    status = "✓" if found else "✗"
    print(f"   {status} {label}")
    if not found:
        all_ok = False

# Check 5: API main.rs has key features
path = os.path.join(os.getcwd(), "packages", "api", "src", "main.rs")
results = check_file(path, [
    "change_password_handler",
    "verify_admin_password",
    "ChangePasswordRequest",
    "[0, 0, 0, 0]"
])
print("\n5. packages/api/src/main.rs:")
for label, found in results.items():
    status = "✓" if found else "✗"
    print(f"   {status} {label}")
    if not found:
        all_ok = False

# Check 6: Admin UI changes
path = os.path.join(os.getcwd(), "packages", "api", "src", "admin_ui.rs")
results = check_file(path, [
    "Configurada en .env",
    "if (k.id === 'master-key') return;"
])
print("\n6. packages/api/src/admin_ui.rs:")
for label, found in results.items():
    status = "✓" if found else "✗"
    print(f"   {status} {label}")
    if not found:
        all_ok = False

# Check 7: .env.example
path = os.path.join(os.getcwd(), ".env.example")
results = check_file(path, ["ADMIN_PASSWORD_SALT", "ADMIN_PASSWORD_HASH"])
print("\n7. .env.example:")
for label, found in results.items():
    status = "✓" if found else "✗"
    print(f"   {status} {label}")
    if not found:
        all_ok = False

print("\n" + "=" * 50)
if all_ok:
    print("✅ TODOS LOS CAMBIOS ESTÁN EN SU LUGAR")
else:
    print("⚠️ Algunos elementos faltan - revisar arriba")
print("=" * 50)