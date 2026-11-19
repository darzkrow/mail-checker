# 🔧 Guía de Troubleshooting - Email Validator API v2.0.0

> **Diagnóstico rápido, solución de problemas comunes, y procedimientos de recuperación para operación en producción.**

---

## 📑 Índice

1. [Diagnóstico Rápido (Quick Start)](#-diagnóstico-rápido-quick-start)
2. [Árbol de Decisión de Problemas](#-árbol-de-decisión-de-problemas)
3. [Problemas por Categoría](#-problemas-por-categoría)
4. [Análisis de Logs](#-análisis-de-logs)
5. [Métricas de Diagnóstico](#-métricas-de-diagnóstico)
6. [Herramientas de Debug](#-herramientas-de-debug)
7. [Procedimientos de Recuperación](#-procedimientos-de-recuperación)
8. [Contactos de Escalación](#-contactos-de-escalación)

---

## 🚀 Diagnóstico Rápido (Quick Start)

### Verificación en 30 Segundos

```bash
# 1. ¿El contenedor está corriendo?
docker compose ps
# ✅ Up (healthy)

# 2. ¿Health check responde?
curl -sf http://localhost:3000/health
# ✅ Email Validator API is running

# 3. ¿API valida correctamente?
curl -s -X POST http://localhost:3000/api/v1/validate-email \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{"email":"test@gmail.com"}' | jq .is_deliverable
# ✅ true

# 4. ¿Métricas accesibles?
curl -u admin:password http://localhost:3000/api/v1/admin/metrics | jq .total_requests
# ✅ número > 0
```

### Si algo falla → Ir a [Árbol de Decisión](#-árbol-de-decisión-de-problemas)

---

## 🌳 Árbol de Decisión de Problemas

```mermaid
flowchart TD
    Start([Problema Detectado]) --> Health{Health Check<br/>/health OK?}
    
    Health -->|No| ContainerStatus{Container<br/>Status?}
    ContainerStatus -->|Restarting| OOMCheck[OOM Kill?\ndocker stats --no-stream]
    ContainerStatus -->|Exited| ExitCode[Exit Code?\ndocker inspect]
    ContainerStatus -->|Created| PortBusy[Puerto 3000 ocupado?\nss -tlnp \| grep 3000]
    
    Health -->|Sí| APICheck{API Response<br/>Correcto?}
    
    APICheck -->|401 Unauthorized| AuthCheck[API Key válida?\nHeader X-API-Key]
    APICheck -->|429 Rate Limited| RateCheck[RATE_LIMIT_QPS\nDemasiado bajo?]
    APICheck -->|413 Payload Large| SizeCheck[Tamaño JSON\n> 2KB?]
    APICheck -->|500/502| ServerErr[Logs Error\n5xx]
    APICheck -->|200 pero wrong| LogicErr[Resultado incorrecto\nis_deliverable false?]
    
    ServerErr --> DNSErr{Error DNS\n502?}
    DNSErr -->|Sí| DNSFix[Firewall UDP/TCP 53\nResolvers válidos]
    DNSErr -->|No| SMTPErr{Error SMTP\nConnection?}
    
    SMTPErr -->|Sí| SMTPFix[Puertos 25/587/465\nEgress abiertos?]
    SMTPErr -->|No| PerfErr{Latencia alta\n> 2s?}
    
    PerfErr -->|Sí| PerfFix[Cache Hit Rate?\nPool Size? Timeouts?]
    PerfErr -->|No| Unknown[Debug Mode\nRUST_LOG=debug]
    
    LogicErr --> SMTPLogic{SMTP habilitado?}
    SMTPLogic -->|Sí| SMTPResult[Verificar RCPT TO\n250 vs 550]
    SMTPLogic -->|No| MXOnly[Solo MX - expected]
    
    style Start fill:#e1f5fe
    style DNSFix fill:#c8e6c9
    style SMTPFix fill:#c8e6c9
    style PerfFix fill:#c8e6c9
    style Unknown fill:#ffcdd2
```

---

## 📂 Problemas por Categoría

### 1. 🐳 Contenedor y Despliegue

| Síntoma | Diagnóstico | Solución |
|---------|-------------|----------|
| **Container `Restarting`** | `docker compose logs` muestra `OOM Killed` | Aumentar `mem_limit: 512m` en docker-compose.yml |
| **Container `Exited (137)`** | OOM Kill (SIGKILL) | Aumentar memoria, reducir `RUST_LOG` |
| **Container `Exited (1)`** | Error inicio app | `docker compose logs email-validator` ver error |
| **Puerto 3000 `Address in use`** | `ss -tlnp | grep 3000` muestra otro proceso | Cambiar `PORT` en `.env` o matar proceso |
| **Health check `unhealthy`** | App no responde en `/health` | Verificar logs, app no inició correctamente |
| **Imagen no se pull** | `Error pulling image` | Verificar `image:` en docker-compose, credenciales registry |

**Comandos de diagnóstico:**
```bash
# Ver estado detallado
docker compose ps -a

# Ver logs completos
docker compose logs email-validator

# Inspeccionar contenedor
docker inspect email-validator-api

# Stats en tiempo real
docker stats email-validator-api --no-stream

# Entrar al contenedor (debug)
docker compose exec email-validator sh
```

### 2. 🔐 Autenticación y Autorización

| Error | Causa | Verificación | Solución |
|-------|-------|--------------|----------|
| **401 Unauthorized** | API Key faltante | Header `X-API-Key` presente? | Agregar header |
| **401 Unauthorized** | API Key inválida | `API_KEY` en `.env` coincide? | Corregir `.env` o header |
| **401 Unauthorized** | API Key expirada | `expires_at` en `keys.json` | Renovar o crear nueva key |
| **401 Unauthorized** | API Key inactiva | `is_active: false` en `keys.json` | Activar via admin panel |
| **403 Forbidden (Admin)** | IP no permitida | `ALLOWED_ORIGINS` o IP allowlist | Configurar proxy/nginx |

**Verificación rápida:**
```bash
# Ver API keys registradas
curl -u admin:password http://localhost:3000/api/v1/admin/keys | jq .

# Test auth directo
curl -H "X-API-Key: $API_KEY" http://localhost:3000/health

# Verificar key en storage
cat data/keys.json | jq .
```

### 3. 📊 Rate Limiting

| Síntoma | Causa | Métrica | Solución |
|---------|-------|---------|----------|
| **429 inmediato** | `RATE_LIMIT_QPS` muy bajo | `rate_limit_hits` alto | Aumentar `RATE_LIMIT_QPS` |
| **429 en ráfagas** | Burst > límite | Revisar patrón cliente | Implementar backoff exponencial |
| **429 por IP compartida** | `X-Forwarded-For` mal configurado | Verificar proxy headers | Configurar `proxy_set_header X-Forwarded-For` |

**Configuración recomendada:**
```bash
# Desarrollo
RATE_LIMIT_QPS=50

# Producción API pública
RATE_LIMIT_QPS=100

# Producción API privada
RATE_LIMIT_QPS=1000

# Cliente debe implementar backoff:
# 429 → wait 1s → retry → 429 → wait 2s → retry → 429 → wait 4s...
```

### 4. 📝 Validación y Lógica

| Comportamiento Incorrecto | Causa | Verificación |
|---------------------------|-------|--------------|
| **Email válido → `is_deliverable: false`** | SMTP deshabilitado | `SMTP_VERIFICATION_ENABLED=false` (default) |
| **Email inválido → `is_deliverable: true`** | Bug en lógica | Revisar `format_response` |
| **Catch-all no detectado** | Detección deshabilitada | `SMTP_ENABLE_CATCH_ALL_DETECTION=true` |
| **Disposable no detectado** | Dominio no en lista | Agregar a `DisposableEmailDetector` |
| **Role-based no detectado** | Prefijo no en lista | Agregar a `RoleBasedEmailDetector` |

**Test casos edge:**
```bash
# Casos de prueba
curl -X POST -H "X-API-Key: $KEY" -H "Content-Type: application/json" \
  -d '{"email":"test@temp-mail.org"}' http://localhost:3000/api/v1/validate-email | jq .is_disposable
# true

curl -X POST ... -d '{"email":"admin@example.com"}' | jq .is_role_based
# true

curl -X POST ... -d '{"email":"random@catchall.com"}' | jq .is_catch_all
# true (si catch-all detection enabled)
```

### 5. 🌐 DNS y Resolución MX

| Error | Causa | Diagnóstico | Solución |
|-------|-------|-------------|----------|
| **`error_code: 5` (DNS_ERROR)** | Timeout DNS | `dig @8.8.8.8 MX gmail.com` | Verificar firewall UDP/TCP 53 |
| **`error_code: 5` (DNS_ERROR)** | Servidor DNS caído | `dig @1.1.1.1 MX gmail.com` | Cambiar DNS resolvers |
| **`error_code: 3` (NO_MX_RECORDS)** | Dominio sin MX | `dig MX dominio.com` | Dominio inexistente o sin email |
| **`error_code: 4` (NULL_MX_REJECTED)** | Null MX RFC 7505 | `dig MX dominio.com` muestra `.` | Dominio rechaza email explícitamente |
| **Resolución intermitente** | DNS cache/load | `dig +trace MX gmail.com` | Aumentar timeout, múltiples resolvers |

**Comandos de diagnóstico DNS:**
```bash
# Desde contenedor/host
dig MX gmail.com +short
dig A gmail.com +short
dig AAAA gmail.com +short

# Trace completo
dig +trace MX gmail.com

# Test con DNS específico
dig @8.8.8.8 MX gmail.com
dig @1.1.1.1 MX gmail.com

# Verificar conectividad
nc -zv 8.8.8.8 53
nc -zv 1.1.1.1 53
```

### 6. 📧 SMTP y Verificación Real

| Error | Causa | Diagnóstico | Solución |
|-------|-------|-------------|----------|
| **`error_code: 12` (CONNECTION_ERROR)** | Firewall bloquea puertos | `nc -zv mx.gmail.com 25` | Abrir 25/587/465 en firewall |
| **`error_code: 12` (CONNECTION_ERROR)** | Timeout conexión | `timeout 10 nc -zv mx.gmail.com 587` | Aumentar `SMTP_CONNECT_TIMEOUT_SECS` |
| **`error_code: 9` (SMTP_REJECTED)** | Buzón inexistente/lleno | Logs muestran 550/552 | Comportamiento esperado |
| **`error_code: 11` (TEMPORARY_FAILURE)** | Greylisting | Logs muestran 450 | Aumentar `SMTP_MAX_RETRIES` |
| **`error_code: 10` (CATCH_ALL)** | Dominio catch-all | Logs: RCPT TO random → 250 | `is_catch_all: true` en respuesta |
| **TLS Handshake failed** | Certificado inválido | `openssl s_client -connect mx:587 -starttls smtp` | `SMTP_DANGER_ACCEPT_INVALID_CERTS=true` (solo test) |

**Test conectividad SMTP:**
```bash
# Puerto 25 (estándar)
timeout 10 nc -zv mx.gmail.com 25

# Puerto 587 (STARTTLS)
openssl s_client -connect mx.gmail.com:587 -starttls smtp -quiet < /dev/null

# Puerto 465 (SMTPS implícito)
openssl s_client -connect mx.gmail.com:465 -quiet < /dev/null

# Verificar MX records
dig MX gmail.com +short

# Test RCPT TO manual
# (requiere telnet/netcat interactivo)
```

### 7. ⚡ Performance y Latencia

| Síntoma | Métrica Afectada | Causa Probable | Solución |
|---------|------------------|----------------|----------|
| **Latencia P99 > 2s** | `average_latency_ms` alta | DNS lento | Verificar DNS resolvers, aumentar timeouts |
| **Latencia P99 > 5s** | `average_latency_ms` muy alta | SMTP timeout | Aumentar `SMTP_TOTAL_TIMEOUT_SECS`, reducir `SMTP_MAX_RETRIES` |
| **Cache miss rate > 50%** | `mx_cache_misses` alto | TTL muy bajo / dominios únicos | Aumentar `MX_CACHE_TTL_SECS` (default 3600) |
| **Pool saturado** | `smtp_pool_size` = max | Muchas requests concurrentes | Aumentar pool size o reducir concurrencia |
| **Memoria crece** | `docker stats` memory ↑ | Memory leak | Reiniciar periódicamente, revisar `RUST_LOG` |

**Optimización recomendada:**
```bash
# Timeouts balanceados para producción
SMTP_CONNECT_TIMEOUT_SECS=10
SMTP_COMMAND_TIMEOUT_SECS=10
SMTP_TOTAL_TIMEOUT_SECS=30
SMTP_MAX_RETRIES=2
SMTP_RETRY_BACKOFF_SECS=5

# Cache MX
MX_CACHE_TTL_SECS=3600        # 1 hora
MX_CACHE_MAX_ENTRIES=10000

# Pool SMTP
SMTP_POOL_MAX_PER_HOST=10
SMTP_POOL_MAX_IDLE_SECS=300
```

---

## 📋 Análisis de Logs

### Estructura de Logs

```json
// Log estructurado (RUST_LOG=info)
{
  "timestamp": "2026-11-19T15:30:45.123Z",
  "level": "INFO",
  "target": "email_validator",
  "message": "Validación completada para gmail.com: MX=true, SMTP=Deliverable (código=0, 145ms)",
  "fields": {
    "domain": "gmail.com",
    "has_mx": true,
    "smtp_result": "Deliverable",
    "error_code": 0,
    "latency_ms": 145
  }
}
```

### Niveles de Log

| Nivel | Uso | Filtrado |
|-------|-----|----------|
| **ERROR** | Fallos críticos, exceptions | `grep ERROR` |
| **WARN** | Degradación, reintentos, rate limits | `grep WARN` |
| **INFO** | Operaciones normales, validaciones completadas | `grep INFO` |
| **DEBUG** | Detalle completo, troubleshooting | `RUST_LOG=debug` |
| **TRACE** | Verbose máximo, solo desarrollo | `RUST_LOG=trace` |

### Patrones de Búsqueda Comunes

```bash
# Errores de validación
docker compose logs email-validator 2>&1 | grep -i "error_code.*[2-9]"

# Rate limiting
docker compose logs email-validator 2>&1 | grep -i "rate.limit"

# Errores DNS
docker compose logs email-validator 2>&1 | grep -i "dns.*error\|timeout.*mx"

# Errores SMTP
docker compose logs email-validator 2>&1 | grep -i "smtp.*error\|connection.*refused\|timeout.*smtp"

# Rate limiting hits
docker compose logs email-validator 2>&1 | grep -i "rate.limit.exceeded"

# Autenticación fallida
docker compose logs email-validator 2>&1 | grep -i "auth.*fail\|invalid.*key"

# Latencia alta
docker compose logs email-validator 2>&1 | grep -E "latency_ms.*[5-9][0-9]{2,}|[1-9][0-9]{3,}"

# Dominios problemáticos
docker compose logs email-validator 2>&1 | grep -i "problematic_domain"
```

### Exportar Logs para Análisis

```bash
# Últimas 24h
docker compose logs --since=24h email-validator > logs_$(date +%Y%m%d).txt

# Solo errores
docker compose logs --since=24h email-validator 2>&1 | grep -i error > errors_$(date +%Y%m%d).txt

# Comprimir para enviar
tar czf logs_$(date +%Y%m%d_%H%M).tar.gz logs_*.txt

# Análisis con jq (si logs JSON)
docker compose logs --since=1h email-validator 2>&1 | grep "^{" | jq -s 'group_by(.level) | map({level: .[0].level, count: length})'
```

---

## 📈 Métricas de Diagnóstico

### Endpoint de Métricas Admin

```bash
# Métricas completas
curl -u admin:password http://localhost:3000/api/v1/admin/metrics | jq .

# Respuesta esperada:
{
  "total_requests": 15420,
  "processed_validations": 14890,
  "api_success_rate": 99.2,
  "average_latency_ms": 156.7,
  "auth_failures": 45,
  "rate_limit_hits": 12,
  "syntax_errors": 320,
  "no_mx_count": 89,
  "dns_errors": 3,
  "uptime_seconds": 3600,
  "top_problematic_domains": [
    {"domain": "temp-mail.org", "occurrences": 45, "reason": "Dominio desechable conocido", "error_category": "calidad"},
    {"domain": "fake123@gmail.com", "occurrences": 23, "reason": "Buzón inexistente (SMTP 550 5.1.1)", "error_category": "smtp_rejected"}
  ]
}
```

### Interpretación de Métricas Clave

```mermaid
graph TB
    subgraph "Salud General"
        M1[total_requests<br/>Tráfico total]
        M2[api_success_rate > 99%<br/>✅ Saludable]
        M3[average_latency_ms < 500ms<br/>✅ Buena performance]
    end
    
    subgraph "Errores Cliente"
        E1[auth_failures > 100/hora<br/>⚠️ Posible ataque/bug cliente]
        E2[rate_limit_hits > 50/hora<br/>⚠️ Clientes mal configurados]
        E3[syntax_errors > 10% requests<br/>⚠️ Cliente envía datos malos]
    end
    
    subgraph "Errores Sistema"
        S1[no_mx_count > 5% requests<br/>📊 Normal (dominios inexistentes)]
        S2[dns_errors > 1% requests<br/>🔴 Problema DNS/Red]
        S3[smtp_connection_errors > 5%<br/>🔴 Problema Red/Firewall SMTP]
    end
    
    subgraph "Inteligencia Dominios"
        D1[top_problematic_domains<br/>Identificar patrones abuse]
        D2[disposable_detected rate<br/>📊 Calidad lista]
        D3[role_based_detected rate<br/>📊 Decisión negocio]
    end
```

### Queries de Alerting (Prometheus/Grafana)

```promql
# Alerta: API Down
up{job="email-validator"} == 0

# Alerta: Error Rate > 5%
rate(email_validator_auth_failures[5m]) 
+ rate(email_validator_rate_limit_hits[5m]) 
+ rate(email_validator_dns_errors[5m]) 
+ rate(email_validator_syntax_errors[5m]) 
> 0.05 * rate(email_validator_total_requests[5m])

# Alerta: Latencia P99 > 5s
histogram_quantile(0.99, rate(email_validator_latency_bucket[5m])) > 5000

# Alerta: DNS Failure Rate > 10%
rate(email_validator_dns_errors[5m]) / rate(email_validator_total_requests[5m]) > 0.10

# Alerta: SMTP Connection Errors > 20%
rate(email_validator_smtp_connection_errors[5m]) / rate(email_validator_smtp_verifications_total[5m]) > 0.20

# Alerta: Cache Hit Rate < 30%
rate(email_validator_mx_cache_hits[5m]) / (rate(email_validator_mx_cache_hits[5m]) + rate(email_validator_mx_cache_misses[5m])) < 0.30
```

---

## 🛠️ Herramientas de Debug

### Scripts de Diagnóstico Incluidos

```bash
# 1. Health check completo
./scripts/health-check.sh

# 2. Test SMTP conectividad
./scripts/test-smtp.sh gmail.com

# 3. Test DNS resolución
./scripts/test-dns.sh gmail.com

# 4. Benchmark performance
./scripts/benchmark.sh 100

# 5. Verificar configuración
./scripts/verify-config.sh
```

### Script: health-check.sh

```bash
#!/bin/bash
# health-check.sh - Diagnóstico completo

set -euo pipefail

API_KEY="${API_KEY:-}"
ADMIN_USER="${ADMIN_USER:-admin}"
ADMIN_PASS="${ADMIN_PASSWORD:-admin12345}"
BASE_URL="http://localhost:3000"

echo "🔍 Email Validator Health Check"
echo "================================"

# 1. Container status
echo -n "📦 Container: "
docker compose ps --format "table {{.Name}}\t{{.Status}}" | grep email-validator || echo "❌ NO ENCONTRADO"

# 2. Health endpoint
echo -n "🏥 Health endpoint: "
if curl -sf "$BASE_URL/health" >/dev/null; then
    echo "✅ OK"
else
    echo "❌ FAIL"
fi

# 3. API validation (si hay API_KEY)
if [ -n "$API_KEY" ]; then
    echo -n "📧 API validation: "
    RESULT=$(curl -s -X POST "$BASE_URL/api/v1/validate-email" \
      -H "Content-Type: application/json" \
      -H "X-API-Key: $API_KEY" \
      -d '{"email":"test@gmail.com"}' 2>/dev/null)
    
    if echo "$RESULT" | jq -e '.is_deliverable == true' >/dev/null 2>&1; then
        echo "✅ OK (deliverable)"
    elif echo "$RESULT" | jq -e '.is_valid_syntax == true' >/dev/null 2>&1; then
        echo "⚠️  Solo MX (SMTP disabled)"
    else
        echo "❌ FAIL: $(echo "$RESULT" | jq -r '.reason // .error')"
    fi
fi

# 4. Admin metrics (si credenciales)
echo -n "📊 Admin metrics: "
METRICS=$(curl -s -u "$ADMIN_USER:$ADMIN_PASS" "$BASE_URL/api/v1/admin/metrics" 2>/dev/null)
if echo "$METRICS" | jq -e '.total_requests' >/dev/null 2>&1; then
    TOTAL=$(echo "$METRICS" | jq -r '.total_requests')
    SUCCESS=$(echo "$METRICS" | jq -r '.api_success_rate')
    LATENCY=$(echo "$METRICS" | jq -r '.average_latency_ms')
    echo "✅ OK (reqs: $TOTAL, success: $SUCCESS%, latency: ${LATENCY}ms)"
else
    echo "❌ FAIL (auth?)"
fi

# 5. DNS check
echo -n "🌐 DNS resolution: "
if dig +short MX gmail.com @8.8.8.8 | grep -q "."; then
    echo "✅ OK"
else
    echo "❌ FAIL"
fi

# 6. SMTP ports (si enabled)
echo -n "📧 SMTP ports: "
for PORT in 25 587 465; do
    if timeout 5 nc -zv mx.gmail.com $PORT 2>/dev/null; then
        echo -n " $PORT✅"
    else
        echo -n " $PORT❌"
    fi
done
echo ""

echo ""
echo "================================"
echo "✅ Health check completado"
```

### Herramientas Externas Recomendadas

| Herramienta | Propósito | Instalación |
|-------------|-----------|-------------|
| **jq** | Procesar JSON responses | `apt/yum/brew install jq` |
| **httpie** | HTTP client amigable | `pip install httpie` |
| **hey** | Load testing | `go install github.com/rakyll/hey@latest` |
| **hyperfine** | Benchmarking | `cargo install hyperfine` |
| **netcat (nc)** | Test puertos TCP | `apt/yum/brew install netcat` |
| **dig** | DNS queries | `apt/yum/brew install bind-utils/dnsutils` |
| **openssl** | TLS/SMTP testing | Pre-instalado |

### Benchmarking

```bash
# Test de carga simple (100 requests, 10 concurrentes)
hey -n 100 -c 10 -H "X-API-Key: $API_KEY" -H "Content-Type: application/json" \
  -d '{"email":"test@gmail.com"}' \
  -m POST http://localhost:3000/api/v1/validate-email

# Benchmark con hyperfine (latencia)
hyperfine --warmup 3 --runs 50 \
  'curl -s -X POST -H "X-API-Key: '$API_KEY'" -H "Content-Type: application/json" -d '"'"'{"email":"test@gmail.com"}"'"' http://localhost:3000/api/v1/validate-email'

# Resultado esperado:
# Benchmark 1: curl ...
#   Time (mean ± σ):     145.2 ms ±  23.4 ms
#   Range (min … max):   112.3 ms … 245.7 ms
```

---

## 🔄 Procedimientos de Recuperación

### 1. Reinicio Limpio (Graceful Restart)

```bash
#!/bin/bash
# graceful-restart.sh

echo "🔄 Reinicio graceful..."

# 1. Drain connections (stop accepting new)
docker compose stop --timeout 30 email-validator

# 2. Verificar que no hay requests en proceso
sleep 5

# 3. Restart
docker compose start email-validator

# 4. Wait for healthy
for i in {1..30}; do
    if curl -sf http://localhost:3000/health >/dev/null; then
        echo "✅ Servicio saludable"
        exit 0
    fi
    sleep 2
done

echo "❌ Timeout esperando healthy"
exit 1
```

### 2. Rollback de Versión

```bash
#!/bin/bash
# rollback.sh <backup_tag>

BACKUP_TAG="${1:-}"
if [ -z "$BACKUP_TAG" ]; then
    echo "Tags disponibles:"
    docker images --format "{{.Repository}}:{{.Tag}}" | grep email-validator-backup
    exit 1
fi

echo "🔙 Rollback a $BACKUP_TAG"

# 1. Tag actual como fallback
docker tag darzkrow/email-validator:current darzkrow/email-validator:pre-rollback-$(date +%s)

# 2. Restaurar backup
docker tag "email-validator-backup:$BACKUP_TAG" darzkrow/email-validator:current

# 3. Reiniciar
docker compose up -d email-validator

# Verificar
for i in {1..30}; do
    if curl -sf http://localhost:3000/health >/dev/null; then
        echo "✅ Rollback completado"
        exit 0
    fi
    sleep 2
done

echo "❌ Rollback falló"
exit 1
```

### 3. Recuperación de Datos Corruptos

```bash
#!/bin/bash
# recover-data.sh

echo "⚠️  Recuperación de datos - Último recurso"

# 1. Backup estado actual
docker commit email-validator-api "email-validator-corrupt-$(date +%s)"

# 2. Detener
docker compose stop email-validator

# 3. Restaurar desde backup conocido bueno
# (requiere backup previo - ver DEPLOYMENT.md)
docker run --rm \
  -v email-validator_data:/data \
  -v /backups:/backup \
  alpine tar xzf /backup/email-validator_20261119_020000.tar.gz -C /data

# 4. Iniciar
docker compose start email-validator

echo "✅ Datos restaurados desde backup"
```

### 4. Limpieza de Emergencia (Disk Full)

```bash
#!/bin/bash
# emergency-cleanup.sh

echo "🧹 Limpieza de emergencia - Espacio en disco"

# 1. Logs Docker
docker system prune -f --volumes

# 2. Logs aplicación (mantener últimas 24h)
find /var/lib/docker/containers -name "*.log" -mtime +1 -delete

# 3. Backup antiguos (>30 días)
find /backups -name "email-validator_*.tar.gz" -mtime +30 -delete

# 4. Imágenes Docker no usadas
docker image prune -a -f --filter "until=24h"

# 5. Verificar espacio
df -h /
```

---

## 📞 Contactos de Escalación

### Matriz de Escalación

| Severidad | Tiempo Respuesta | Canal | Responsable | Ejemplos |
|-----------|------------------|-------|-------------|----------|
| **P1 - Crítico** | 15 min | 📞 Tel + Slack #oncall | On-Call Primary | Servicio caído, data loss, security breach |
| **P2 - Alto** | 1 hora | 💬 Slack #oncall + Email | On-Call Secondary | Degradación severa, error rate >5% |
| **P3 - Medio** | 4 horas | 📧 Email + Ticket | Team Lead | Bug no crítico, performance degradada |
| **P4 - Bajo** | 1 día hábil | 🎫 Ticket | Backlog Owner | Documentación, feature request |

### Contactos

```text
┌─────────────────────────────────────────────────────────────┐
│                    ESCALATION CONTACTS                       │
├─────────────────────────────────────────────────────────────┤
│  🔴 Primary On-Call:     +1-XXX-XXX-XXXX  (rotación semanal) │
│  🟠 Secondary On-Call:   +1-XXX-XXX-XXXX  (backup)           │
│  🟡 Engineering Lead:    lead@empresa.com                    │
│  🔵 Security Team:       security@empresa.com               │
│  🟢 Infra/DevOps Team:   infra@empresa.com                  │
│  🟣 Product Owner:       product@empresa.com                │
├─────────────────────────────────────────────────────────────┤
│  Slack Channels:                                         │
│  #oncall          - Alertas P1/P2 (pagerduty/opsgenie)    │
│  #email-validator - Canal equipo, debugging               │
│  #incidents       - Coordinación incidentes P1/P2         │
│  #deployments     - Notificaciones deployments            │
└─────────────────────────────────────────────────────────────┘
```

### Runbooks de Incidente

| Incidente | Runbook | Owner | SLA Resolución |
|-----------|---------|-------|----------------|
| Servicio caído | `runbooks/service-down.md` | On-Call | P1: 30 min |
| Latencia alta (P99>5s) | `runbooks/high-latency.md` | On-Call | P2: 2 horas |
| Error rate spike (>5%) | `runbooks/error-spike.md` | On-Call | P2: 2 horas |
| SMTP failures masivos | `runbooks/smtp-failures.md` | Infra | P2: 4 horas |
| DNS resolution fails | `runbooks/dns-failures.md` | Infra | P2: 2 horas |
| Rate limit abuse | `runbooks/rate-limit-abuse.md` | Security | P2: 4 horas |
| API Key comprometida | `runbooks/api-key-compromise.md` | Security | P1: 1 hora |
| Certificado TLS expirado | `runbooks/tls-cert-expiry.md` | Infra | P2: 4 horas |
| Disk full | `runbooks/disk-full.md` | Infra | P1: 30 min |
| Memory leak / OOM | `runbooks/oom-kills.md` | Infra | P2: 2 horas |

### Plantilla de Incidente

```markdown
# INCIDENT REPORT - [INC-YYYYMMDD-XXX]

## 📋 Resumen
- **Severidad**: P1/P2/P3/P4
- **Estado**: Investigating / Identified / Monitoring / Resolved
- **Inicio**: YYYY-MM-DD HH:MM UTC
- **Impacto**: % usuarios afectados, funcionalidad degradada

## 🔍 Descripción
[Qué está fallando, síntomas observados, métricas anómalas]

## 🕵️ Investigación
- [ ] Health check status
- [ ] Logs recientes (`docker compose logs`)
- [ ] Métricas admin (`/admin/metrics`)
- [ ] Conectividad DNS/SMTP
- [ ] Cambios recientes (deploy, config, infra)

## 🎯 Causa Raíz
[Análisis 5 Whys]

## 🛠️ Resolución
[Pasos tomados para resolver]

## ✅ Verificación
- [ ] Health check OK
- [ ] API validation OK
- [ ] Métricas normalizadas
- [ ] Alertas resueltas

## 📚 Lecciones Aprendidas
[Qué mejorar: monitoring, runbooks, arquitectura]

## 📋 Post-Mortem
[Programar dentro de 48h para P1/P2]
```

---

## 📚 Referencias Rápidas

### Comandos Esenciales

```bash
# === CONTENEDOR ===
docker compose ps                    # Estado
docker compose logs -f email-validator  # Logs tiempo real
docker compose logs --tail=100 email-validator  # Últimas 100 líneas
docker compose restart email-validator        # Reinicio rápido
docker compose stop email-validator           # Parar
docker compose up -d email-validator          # Iniciar

# === DIAGNÓSTICO ===
curl -sf http://localhost:3000/health         # Health check
curl -u admin:pass http://localhost:3000/api/v1/admin/metrics | jq .  # Métricas
curl -X POST -H "X-API-Key: $KEY" -d '{"email":"test@gmail.com"}' http://localhost:3000/api/v1/validate-email | jq .  # Test API

# === DNS ===
dig MX gmail.com +short
dig @8.8.8.8 MX gmail.com
dig +trace MX gmail.com

# === SMTP ===
nc -zv mx.gmail.com 25
openssl s_client -connect mx.gmail.com:587 -starttls smtp -quiet
openssl s_client -connect mx.gmail.com:465 -quiet

# === LOGS ===
docker compose logs --since=1h email-validator | grep ERROR
docker compose logs --since=24h email-validator > logs_$(date +%Y%m%d).txt

# === BACKUP ===
docker run --rm -v email-validator_data:/data -v $(pwd):/backup alpine tar czf /backup/backup_$(date +%Y%m%d).tar.gz -C /data .

# === LIMPIEZA ===
docker system prune -f
docker image prune -a -f --filter "until=24h"
```

### Variables de Entorno Críticas

```bash
# Verificar configuración actual
docker compose exec email-validator env | grep -E "API_KEY|SMTP_|RATE_LIMIT|RUST_LOG"

# Deben estar configuradas:
# API_KEY=sk-prod-xxx...
# SMTP_VERIFICATION_ENABLED=true|false
# RATE_LIMIT_QPS=100
# RUST_LOG=warn
```

---

*Guía de Troubleshooting v2.0.0 - Última actualización: 19 Noviembre 2026*  
*Mantenido por: Platform Engineering Team*