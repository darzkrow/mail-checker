# 🚀 Email Validator API v2.0.0

[![Rust](https://img.shields.io/badge/rust-1.78+-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-2.0.0-blue.svg)](https://github.com/darzkrow/MX-Checker/releases/tag/v2.0.0)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Tests](https://img.shields.io/badge/tests-24%20passing-brightgreen.svg)]()

Microservicio REST API de **alto rendimiento** y **seguridad** desarrollado en **Rust** para validar correos electrónicos mediante:

- ✅ **Sintaxis RFC 5321/5322** — Validación estricta de formato y longitudes
- ✅ **Resolución DNS MX asíncrona** — Soporte Null MX (RFC 7505) + fallback A/AAAA (RFC 5321 §5.1)
- ✅ **Verificación SMTP real (v2.0)** — Conexión real a servidores MX con `RCPT TO` para confirmar entregabilidad
- ✅ **Cache MX inteligente** — LRU con TTL configurable + Pool de conexiones SMTP reutilizables
- ✅ **Detección avanzada** — Catch-all, dominios desechables, emails role-based

---

## ⚡ Inicio Rápido (3 Pasos)

### 1. Configuración
```bash
# Clonar y configurar
git clone https://github.com/darzkrow/MX-Checker.git
cd MX-Checker
cp .env.example .env
# Edita .env y configura tu API_KEY
```

### 2. Desplegar con Docker
```bash
docker compose up -d --build
```

### 3. Probar
```bash
# Health check
curl http://localhost:3001/health

# Validar email (reemplaza TU_API_KEY)
curl -X POST http://localhost:3001/api/v1/validate-email \
  -H "Content-Type: application/json" \
  -H "X-API-Key: TU_API_KEY" \
  -d '{"email":"usuario@gmail.com"}' | jq .
```

> **Alternativa Windows**: `run.bat` | **Linux/Mac**: `./run.sh`

---

## 🏗️ Arquitectura Modular (Cargo Workspace)

| Módulo | Ruta | Responsabilidad |
|--------|------|-----------------|
| **`shared`** | [`packages/shared`](packages/shared) | Modelos, errores, config, métricas atómicas, DomainTracker |
| **`validation`** | [`packages/validation`](packages/validation) | Sintaxis RFC 5321/5322 pura (sin I/O) |
| **`dns`** | [`packages/dns`](packages/dns) | Resolución MX asíncrona con `hickory-resolver` |
| **`smtp`** | [`packages/smtp`](packages/smtp) | **Verificación SMTP real** (`RCPT TO`), Cache MX LRU, Pool conexiones, detección Catch-all/Disposable/Role-based |
| **`api`** | [`packages/api`](packages/api) | Servidor HTTP (Axum 0.7), Auth, Rate Limit, Admin Panel, Métricas |

---

## 🔍 Verificación SMTP Real (v2.0)

Conecta realmente a los servidores MX y ejecuta la transacción SMTP completa:

```
CONNECT → EHLO → STARTTLS (opcional) → MAIL FROM:<> → RCPT TO:<email> → QUIT
```

### Niveles de Verificación

| Nivel | Descripción | Configuración | Latencia típica |
|-------|-------------|---------------|-----------------|
| **Solo MX** | Solo verifica existencia de registros MX | `SMTP_VERIFICATION_ENABLED=false` (default) | ~50-150ms |
| **SMTP Real** | Conecta al servidor y ejecuta `RCPT TO` | `SMTP_VERIFICATION_ENABLED=true` | ~200-800ms |

### Fallback de Puertos Automático
```
Puerto 25 (plano) → Puerto 587 (STARTTLS) → Puerto 465 (SMTPS implícito)
```

### Códigos de Resultado SMTP

| Código | Enum | Significado | Entregable |
|--------|------|-------------|------------|
| `250` | `Deliverable` | Buzón confirmado | ✅ **Sí** |
| `550/551/552/553/554` | `Undeliverable`/`MailboxFull`/`RejectedByPolicy` | Rechazo permanente | ❌ **No** |
| `450/451/452` | `TemporaryFailure` | Greylisting / servidor ocupado | ⚠️ Reintentar |
| — | `CatchAll` | Dominio acepta cualquier destinatario | ⚠️ No verificable |
| — | `ConnectionError` | Timeout / error de red | ⚠️ Reintentar |

### Respuesta JSON con SMTP Habilitado

```json
{
  "email": "usuario@gmail.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": true,
  "error_code": 0,
  "reason": "Correo válido y buzón confirmado entregable (SMTP RCPT TO 250)",
  "smtp_verification": "Deliverable",
  "smtp_detail": "Correo válido y buzón confirmado entregable (SMTP RCPT TO 250)",
  "is_catch_all": false,
  "is_disposable": false,
  "is_role_based": false
}
```

### Nuevos Códigos de Error (v2.0)

| Code | Constante | Categoría | Descripción |
|------|-----------|-----------|-------------|
| `9` | `ERR_SMTP_REJECTED` | SMTP | Buzón rechazado (inexistente, lleno, política) |
| `10` | `ERR_CATCH_ALL` | SMTP | Dominio catch-all (no verificable individualmente) |
| `11` | `ERR_SMTP_TEMPORARY_FAILURE` | SMTP | Greylisting / servidor ocupado (reintentar) |
| `12` | `ERR_SMTP_CONNECTION_ERROR` | Red | Error de conexión/timeout al servidor SMTP |
| `13` | `ERR_DISPOSABLE_EMAIL` | Calidad | Email desechable (dominio temporal conocido) |
| `14` | `WARN_ROLE_BASED_EMAIL` | Calidad | Email role-based (admin@, info@, support@, etc.) |

---

## ⚡ Performance & Optimizaciones

| Feature | Descripción | Configuración |
|---------|-------------|---------------|
| **Cache MX LRU** | Cache en memoria con TTL + evicción LRU | `MX_CACHE_TTL_SECS=3600`, `MX_CACHE_MAX_ENTRIES=10000` |
| **Pool Conexiones SMTP** | Reutilización de conexiones por host MX | `SMTP_POOL_MAX_PER_HOST=10`, `SMTP_POOL_MAX_IDLE_SECS=300` |
| **Reintentos Inteligentes** | Backoff exponencial en códigos 4xx | `SMTP_MAX_RETRIES=2`, `SMTP_RETRY_BACKOFF_SECS=5` |

---

## 🛡️ Detección Avanzada de Calidad

| Detector | Descripción | Ejemplos |
|----------|-------------|----------|
| **Catch-all** | Test con email aleatorio → detecta dominios que aceptan todo | `random@catchall.com` → 250 OK |
| **Disposable** | 12+ dominios temporales conocidos | `temp-mail.org`, `guerrillamail.com`, `10minutemail.com` |
| **Role-based** | 25+ prefijos de roles funcionales | `admin@`, `info@`, `support@`, `noreply@`, `sales@` |

**Campos en respuesta:**
```json
{
  "is_catch_all": true,
  "is_disposable": true,
  "is_role_based": true
}
```

---

## 📚 Documentación Completa

| Documento | Descripción |
|-----------|-------------|
| 📖 **[API_REFERENCE.md](docs/API_REFERENCE.md)** | Endpoints, schemas, códigos error (0-14), ejemplos curl |
| 🚦 **[ERROR_CODES.md](docs/ERROR_CODES.md)** | Mapeo directo 0-14, ejemplo JS/TypeScript |
| 📜 **[CHANGELOG_PATCHES.md](docs/CHANGELOG_PATCHES.md)** | 10 parches de auditoría + release v2.0.0 |
| 🐳 **[DEPLOYMENT.md](docs/DEPLOYMENT.md)** | Docker, Kubernetes, nginx, firewall, backup, monitoring |
| 🔧 **[TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)** | Árbol de decisión, logs, métricas, runbooks, scripts |
| 🛡️ **[SECURITY_HARDENING.md](docs/SECURITY_HARDENING.md)** | STRIDE, hardening contenedor/red, secretos, GDPR/SOC2 |
| 🏗️ **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** | 15+ diagramas Mermaid (componentes, secuencias, flujos) |
| 📋 **[DOCUMENTATION_INDEX.md](docs/DOCUMENTATION_INDEX.md)** | Índice maestro por tema/audiencia |
| 🪟 **[SETUP_WINDOWS.md](docs/SETUP_WINDOWS.md)** | Guía Windows con Docker Desktop |
| 🧪 **[Postman Collection](docs/Email-Validator-API.postman_collection.json)** | 20+ tests listos para importar |

---

## 🔧 Variables de Entorno (`.env`)

### Core
| Variable | Descripción | Default | Requerido |
|----------|-------------|---------|-----------|
| `API_KEY` | Clave maestra para `X-API-Key` | — | ✅ Prod |
| `ALLOWED_ORIGINS` | CORS origins (coma-separated) | `http://localhost:3001,http://localhost:5173` | ❌ |
| `PORT` | Puerto HTTP | `3001` | ❌ |
| `RATE_LIMIT_QPS` | Requests/seg por IP | `10` | ❌ |
| `RUST_LOG` | Nivel logging | `info` | ❌ |

### SMTP (v2.0)
| Variable | Descripción | Default |
|----------|-------------|---------|
| `SMTP_VERIFICATION_ENABLED` | Activar verificación RCPT TO | `false` |
| `SMTP_CONNECT_TIMEOUT_SECS` | Timeout conexión TCP | `10` |
| `SMTP_COMMAND_TIMEOUT_SECS` | Timeout comandos SMTP | `10` |
| `SMTP_TOTAL_TIMEOUT_SECS` | Timeout total verificación | `30` |
| `SMTP_MAX_RETRIES` | Reintentos en 4xx | `2` |
| `SMTP_RETRY_BACKOFF_SECS` | Backoff base (exponencial) | `5` |
| `SMTP_PRIMARY_PORT` | Puerto SMTP principal | `25` |
| `SMTP_SUBMISSION_PORT` | Puerto Submission (STARTTLS) | `587` |
| `SMTP_SMTPS_PORT` | Puerto SMTPS implícito | `465` |
| `SMTP_HELO_DOMAIN` | Dominio para EHLO | `email-validator.local` |
| `SMTP_MAIL_FROM` | Remitente MAIL FROM | `""` (null sender `<>`) |
| `SMTP_DANGER_ACCEPT_INVALID_CERTS` | Aceptar certs TLS inválidos (solo test) | `false` |
| `SMTP_ENABLE_CATCH_ALL_DETECTION` | Detectar dominios catch-all | `true` |

---

## 🚀 Despliegue Rápido

### Docker Compose (Producción)
```bash
# 1. Configurar
cp .env.example .env
# Edita .env con tu API_KEY segura

# 2. Construir y levantar
docker compose up -d --build

# 3. Verificar
curl http://localhost:3001/health
```

### Kubernetes
```bash
kubectl apply -f k8s/
# Incluye: ConfigMap, Secret, Deployment (3 replicas), Service, Ingress, PVC
```

### Reverse Proxy (nginx)
```nginx
upstream email_validator {
    server 127.0.0.1:3001 max_fails=3 fail_timeout=30s;
    keepalive 32;
}
server {
    listen 443 ssl http2;
    server_name api.tuempresa.com;
    location / { proxy_pass http://email_validator; }
}
```

---

## 🔐 Seguridad

- **Autenticación**: API Key obligatoria (`X-API-Key`)
- **Rate Limiting**: 10 req/s por IP (configurable)
- **Body Limit**: 2KB máximo
- **Container Hardening**: Non-root (UID 1000), read-only FS, capabilities dropped
- **TLS**: Termination en reverse proxy (nginx/Traefik) con HSTS
- **Admin Panel**: IP allowlist + Basic Auth
- **Secrets**: Variables de entorno, sin secretos en código/git

---

## 📊 Monitoreo

```bash
# Métricas admin (Basic Auth)
curl -u admin:password http://localhost:3001/api/v1/admin/metrics | jq .

# Logs recientes
curl -u admin:password http://localhost:3001/api/v1/admin/logs | jq .
```

**Métricas clave:**
- `total_requests`, `api_success_rate`, `average_latency_ms`
- `auth_failures`, `rate_limit_hits`, `syntax_errors`
- `dns_errors`, `smtp_verifications_total`, `mx_cache_hits/misses`
- `top_problematic_domains` (dominios problemáticos recurrentes)

---

## 🧪 Testing

```bash
# Tests unitarios + integración (24 tests)
cargo test --workspace

# Solo tests SMTP
cargo test -p email-validator-smtp

# Benchmark
cargo install hyperfine
hyperfine --warmup 3 'curl -s -X POST -H "X-API-Key: $API_KEY" -d '"'"'{"email":"test@gmail.com"}"'"' http://localhost:3001/api/v1/validate-email'
```

---

## 📄 Licencia

Este proyecto está bajo la licencia [MIT](LICENSE).

---

## 🤝 Contribuir

1. Fork del repositorio
2. Crea rama: `git checkout -b feature/mejora`
3. Commit: `git commit -m "feat: descripción"`
4. Push: `git push origin feature/mejora`
5. Abre Pull Request

---

## 📞 Soporte

- **Issues**: [GitHub Issues](https://github.com/darzkrow/MX-Checker/issues)
- **Discussions**: [GitHub Discussions](https://github.com/darzkrow/MX-Checker/discussions)
- **Security**: `security@tuempresa.com` (para vulnerabilidades)

---

**Email Validator API v2.0.0** — Validación real de entregabilidad con Rust 🦀