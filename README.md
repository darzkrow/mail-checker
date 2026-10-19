# 🚀 Email Validator API

Microservicio REST API de alto rendimiento y seguridad desarrollado en **Rust** para validar correos electrónicos mediante **sintaxis RFC 5322** y **resolución DNS MX asíncrona real**.

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
| **`api`** | [`packages/api`](packages/api) | Servidor web HTTP (`Axum 0.7`), enrutamiento, CORS, límite de 2KB y seguridad. |

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

---

## 📄 Licencia

Este proyecto está bajo la licencia [MIT](LICENSE).
