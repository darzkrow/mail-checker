# 🚀 Email Validator API

Microservicio REST API de alto rendimiento y seguridad desarrollado en **Rust** para validar correos electrónicos mediante:
- **Sintaxis RFC 5322** - Validación estricta de formato
- **Resolución DNS MX asíncrona real** - Con soporte Null MX (RFC 7505) y fallback SMTP A/AAAA (RFC 5321)
- **Verificación SMTP real (v2.0)** - Conexión real a servidores MX con `RCPT TO` para confirmar entregabilidad

---

## ⚡ Inicio Rápido (3 Pasos)

### 1. Configuración de entorno
Crea el archivo `.env` a partir del ejemplo:
```bash
# Windows (PowerShell)
Copy-Item .env.example .env

# Linux / Mac
cp .env.example .env
```
*(Edita `.env` para configurar tu `API_KEY` o usa la clave de desarrollo predefinida).*

### 2. Iniciar el servicio con Docker
```bash
# Iniciar contenedor en segundo plano
docker compose up -d
```
> También puedes usar los asistentes automáticos ejecutando `run.bat` (Windows) o `./run.sh` (Linux/Mac).

### 3. Probar el servicio
```powershell
# Verificar estado
Invoke-RestMethod -Uri "http://localhost:3001/health"

# Validar un correo
$headers = @{
    "Content-Type" = "application/json"
    "X-API-Key" = "TU_API_KEY_AQUI"
}
$body = '{"email":"test@gmail.com"}'
Invoke-RestMethod -Uri "http://localhost:3001/api/v1/validate-email" -Method Post -Headers $headers -Body $body | ConvertTo-Json
```

---

## 🏗️ Arquitectura Modular (Cargo Workspace)

El proyecto está organizado en paquetes desacoplados e independientes dentro del directorio `packages/`:

| Módulo | Ruta | Responsabilidad |
|---|---|---|
| **`shared`** | [`packages/shared`](packages/shared) | Modelos compartidos, configuración y tipos de error centralizados. |
| **`validation`** | [`packages/validation`](packages/validation) | Lógica pura de validación de sintaxis (RFC 5322 simplificada). |
| **`dns`** | [`packages/dns`](packages/dns) | Resolución asíncrona de registros MX usando `hickory-resolver`. |
| **`smtp`** | [`packages/smtp`](packages/smtp) | Verificación real de entregabilidad SMTP mediante `RCPT TO`, cache MX, pool conexiones, detección catch-all/disposable/role-based. |
| **`api`** | [`packages/api`](packages/api) | Servidor web HTTP (`Axum 0.7`), enrutamiento, CORS, límite de 2KB y seguridad. |

---

## 🔍 Verificación SMTP Real (v2.0)

A partir de la versión 2.0, la API incluye **verificación real de entregabilidad** conectándose a los servidores MX del dominio:

```
CONNECT → EHLO → STARTTLS (opcional) → MAIL FROM:<> → RCPT TO:<email> → QUIT
```

### Niveles de verificación:

| Nivel | Descripción | Configuración |
|---|---|---|
| **Solo MX** | Verifica existencia de registros MX | `SMTP_VERIFICATION_ENABLED=false` (default) |
| **SMTP Real** | Conecta al servidor y ejecuta `RCPT TO` | `SMTP_VERIFICATION_ENABLED=true` |

### Resultado SMTP en respuesta:

```json
{
  "smtp_verification": "Deliverable",
  "smtp_detail": "Correo válido y buzón confirmado entregable (SMTP RCPT TO 250)",
  "is_catch_all": false,
  "is_disposable": false,
  "is_role_based": false
}
```

### Códigos de error SMTP nuevos:
- `9` - `ERR_SMTP_REJECTED`: Buzón rechazado (inexistente, lleno, política)
- `10` - `ERR_CATCH_ALL`: Dominio catch-all (no verificable individualmente)
- `11` - `ERR_SMTP_TEMPORARY_FAILURE`: Greylisting / servidor ocupado (reintentar)
- `12` - `ERR_SMTP_CONNECTION_ERROR`: Error de conexión/timeout
- `13` - `ERR_DISPOSABLE_EMAIL`: Email desechable conocido
- `14` - `WARN_ROLE_BASED_EMAIL`: Email role-based (admin@, info@, etc.)

---
 
## 📚 Documentación del Proyecto
 
Toda la documentación detallada se encuentra estandarizada en la carpeta [`docs/`](docs/):
 
- 📖 **[Referencia de API](docs/API_REFERENCE.md)**: Detalle completo de endpoints, cabeceras, requests y responses JSON.
- 🚦 **[Códigos de Error](docs/ERROR_CODES.md)**: Catálogo de errores (400, 401, 413, 429, 502, 500) y cómo interpretarlos.
- 📜 **[Registro de Parches y Mejoras Lógicas](docs/CHANGELOG_PATCHES.md)**: Bitácora detallada de correcciones de auditoría y cumplimiento de RFCs.
- 🐳 **[Guía de Despliegue y Pruebas](docs/DEPLOYMENT.md)**: Construcción, comandos Docker Compose, pruebas y entornos.
- 🛡️ **[Seguridad y Hardening](docs/SECURITY.md)**: Políticas de autenticación, límites de payload, rate limiting y CORS.
- 🪟 **[Instalación en Windows](docs/SETUP_WINDOWS.md)**: Guía paso a paso para entornos Windows con Docker Desktop.
- 🧪 **Colección Postman**: [`docs/Email-Validator-API.postman_collection.json`](docs/Email-Validator-API.postman_collection.json) para importar y probar de inmediato.
 
---
 
## 🔒 Variables de Entorno
 
| Variable | Descripción | Valor por defecto |
|---|---|---|
| `API_KEY` | Clave secreta requerida en el header `X-API-Key` | *(requerido en producción)* |
| `ALLOWED_ORIGINS` | Dominios autorizados para CORS (separados por coma) | `http://localhost:4000,http://localhost:5173` |
| `PORT` | Puerto de escucha del servidor | `3001` |
| `RATE_LIMIT_QPS` | Solicitudes por segundo permitidas | `10` |
| `RUST_LOG` | Nivel de logs (`debug`, `info`, `warn`, `error`) | `info` |
| `SMTP_VERIFICATION_ENABLED` | Activar verificación SMTP real (RCPT TO) | `false` |
| `SMTP_CONNECT_TIMEOUT_SECS` | Timeout conexión TCP a servidor MX | `10` |
| `SMTP_COMMAND_TIMEOUT_SECS` | Timeout comandos SMTP individuales | `10` |
| `SMTP_TOTAL_TIMEOUT_SECS` | Timeout total verificación SMTP | `30` |
| `SMTP_MAX_RETRIES` | Reintentos en fallos temporales (4xx) | `2` |
| `SMTP_RETRY_BACKOFF_SECS` | Backoff base entre reintentos | `5` |
| `SMTP_PRIMARY_PORT` | Puerto SMTP principal | `25` |
| `SMTP_SUBMISSION_PORT` | Puerto Submission (STARTTLS) | `587` |
| `SMTP_SMTPS_PORT` | Puerto SMTPS implícito | `465` |
| `SMTP_HELO_DOMAIN` | Dominio para EHLO | `email-validator.local` |
| `SMTP_MAIL_FROM` | Remitente MAIL FROM (vacío = <>) | `""` |
| `SMTP_DANGER_ACCEPT_INVALID_CERTS` | Aceptar certs TLS inválidos (solo testing) | `false` |
| `SMTP_ENABLE_CATCH_ALL_DETECTION` | Habilitar detección catch-all | `true` |

---

## 📄 Licencia

Este proyecto está bajo la licencia [MIT](LICENSE).
