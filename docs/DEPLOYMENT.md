# 🐳 Guía de Despliegue y Pruebas - Email Validator API

Esta guía contiene los pasos necesarios para desplegar, gestionar y probar el servicio en cualquier entorno.

---

## 📋 Prerrequisitos

- **Docker Desktop** (en Windows/Mac) o **Docker Engine** con Docker Compose (en Linux).

---

## 🚀 Despliegue con Docker

### Opción 1: Scripts Automáticos (Recomendado)
- **Windows:** Ejecuta el archivo [`run.bat`](../run.bat) haciendo doble clic o desde la terminal.
- **Linux / macOS:**
  ```bash
  chmod +x ./run.sh
  ./run.sh
  ```

---

### Opción 2: Docker Compose (Comandos Clave)

1. **Reconstruir la imagen:**
   ```bash
   docker compose build
   ```

2. **Levantar el contenedor en segundo plano:**
   ```bash
   docker compose up -d
   ```

3. **Ver logs en tiempo real:**
   ```bash
   docker compose logs -f
   ```

4. **Verificar estado de salud:**
   ```bash
   docker ps
   # El estado debe indicar: Up X seconds (healthy)
   ```

5. **Detener el servicio:**
   ```bash
   docker compose down
   ```

---

## 🧪 Pruebas de Funcionamiento

### 1. Healthcheck (Sin autenticación)
Permite a balanceadores o a Docker verificar si el servicio está operativo.
```bash
# cURL
curl http://localhost:3000/health

# PowerShell
Invoke-RestMethod -Uri "http://localhost:3000/health"
```
**Respuesta (200 OK):**
```text
✅ Email Validator API is running
```

---

### 2. Validación de Correo con Entrega Exitosa
```powershell
$headers = @{
    "Content-Type" = "application/json"
    "X-API-Key" = "8T4sEA3ifZSMYpXwkUVogKamc2Ht0Nz6"
}
$body = '{"email":"test@gmail.com"}'
Invoke-RestMethod -Uri "http://localhost:3000/api/v1/validate-email" -Method Post -Headers $headers -Body $body | ConvertTo-Json
```
**Respuesta (200 OK):**
```json
{
  "email": "test@gmail.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": true,
  "reason": "Correo válido y con registros MX encontrados"
}
```

---

### 3. Validación de Correo con Dominio sin Registros MX
```powershell
$headers = @{
    "Content-Type" = "application/json"
    "X-API-Key" = "8T4sEA3ifZSMYpXwkUVogKamc2Ht0Nz6"
}
$body = '{"email":"admin@glinyer.com"}'
Invoke-RestMethod -Uri "http://localhost:3000/api/v1/validate-email" -Method Post -Headers $headers -Body $body | ConvertTo-Json
```
**Respuesta (200 OK):**
```json
{
  "email": "admin@glinyer.com",
  "is_valid_syntax": true,
  "has_mx_records": false,
  "is_deliverable": false,
  "reason": "Correo con sintaxis válida pero sin registros MX"
}
```

---

### 4. Prueba de Sintaxis Inválida
```powershell
$headers = @{
    "Content-Type" = "application/json"
    "X-API-Key" = "8T4sEA3ifZSMYpXwkUVogKamc2Ht0Nz6"
}
$body = '{"email":"usuario-invalido"}'
Invoke-RestMethod -Uri "http://localhost:3000/api/v1/validate-email" -Method Post -Headers $headers -Body $body
```
**Respuesta (400 Bad Request):**
```json
{
  "error": "Formato de correo inválido: debe contener exactamente un @"
}
```

---

### 5. Prueba de Autenticación Fallida (Sin Header)
```powershell
$headers = @{ "Content-Type" = "application/json" }
$body = '{"email":"test@gmail.com"}'
Invoke-RestMethod -Uri "http://localhost:3000/api/v1/validate-email" -Method Post -Headers $headers -Body $body
```
**Respuesta (401 Unauthorized):**
```json
{
  "error": "Falta el header X-API-Key o es inválido"
}
```

---

## 📮 Colección Postman

En esta misma carpeta se incluye la colección lista para usar:
- Importa [`Email-Validator-API.postman_collection.json`](./Email-Validator-API.postman_collection.json) en Postman.
- Configura las variables:
  - `base_url`: `http://localhost:3000`
  - `api_key`: Tu valor de `API_KEY` definido en `.env`.