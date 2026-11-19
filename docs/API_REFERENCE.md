# 📚 Referencia de API - Email Validator API

## 📋 **Información General**

- **Base URL**: `http://localhost:3000` (o tu dominio de producción)
- **Versión de API**: `v1`
- **Formato de respuesta**: JSON
- **Autenticación**: API Key en header `X-API-Key`
- **Rate limiting**: 10 solicitudes/segundo por IP (configurable)
- **Limite de payload**: 2KB máximo

## 🔐 **Autenticación**

### **API Key Authentication**
Todas las solicitudes requieren el header `X-API-Key` con una clave válida.

**Header requerido:**
```http
X-API-Key: your-super-secret-api-key-change-me-in-production
```

**Configuración de API Key:**
```bash
# Archivo .env
API_KEY=tu-clave-secreta-aqui
```

**Recomendaciones de seguridad:**
- Generar claves largas y aleatorias (mínimo 32 caracteres)
- Rotar claves periódicamente en producción
- Usar diferentes claves por entorno (dev, staging, prod)
- Nunca commitear claves en el código fuente

## 📡 **Endpoints**

### **POST /api/v1/validate-email**

Valida una dirección de correo electrónico mediante:
1. Validación de sintaxis según RFC 5322 simplificada
2. Resolución de registros MX del dominio
3. Verificación de entregabilidad

---

**URL:** `/api/v1/validate-email`

**Método:** `POST`

**Headers:**
```http
Content-Type: application/json
X-API-Key: <tu-api-key>
```

**Body:**
```json
{
  "email": "usuario@dominio.com"
}
```

**Respuesta exitosa - Correo válido y entregable (200 OK):**
```json
{
  "email": "usuario@dominio.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": true,
  "error_code": 0,
  "reason": "Correo válido y con registros MX encontrados"
}
```

**Respuesta con sintaxis inválida (200 OK):**
> *En una API REST de validación, un correo con error sintáctico es un resultado evaluado exitosamente por el servicio y retorna `error_code: 2` para mapping directo.*
```json
{
  "email": "usuario.sin-arroba.com",
  "is_valid_syntax": false,
  "has_mx_records": false,
  "is_deliverable": false,
  "error_code": 2,
  "reason": "Formato de correo inválido: falta el carácter separador '@'"
}
```

**Respuesta con sintaxis válida pero dominio sin correo o Null MX (200 OK):**
```json
{
  "email": "usuario@dominio-sin-mx.com",
  "is_valid_syntax": true,
  "has_mx_records": false,
  "is_deliverable": false,
  "error_code": 3,
  "reason": "El dominio no posee servidores de correo (registros MX) ni host resoluble"
}
```

**Respuesta de error de solicitud del cliente (400 Bad Request):**
> *Ocurre si el payload JSON está malformado o el cuerpo está vacío.*
```json
{
  "error": "Failed to parse the request body as JSON",
  "error_code": 2
}
```

**Respuesta de autenticación rechazada (401 Unauthorized):**
```json
{
  "error": "Falta el header X-API-Key o es inválido",
  "error_code": 6
}
```

---

### **Parámetros del Request**

| Campo | Tipo | Requerido | Descripción | Ejemplo |
|---|---|---|---|---|
| `email` | string | ✅ | Dirección de correo a validar | `"usuario@gmail.com"` |

### **Campos de Respuesta**

| Campo | Tipo | Descripción |
|---|---|---|
| `email` | string | Dirección de correo validada |
| `is_valid_syntax` | boolean | `true` si la sintaxis cumple con RFC 5321 / RFC 5322 |
| `has_mx_records` | boolean | `true` si el dominio tiene servidores de correo activos |
| `is_deliverable` | boolean | `true` si cumple sintaxis y servidores MX operativos **y verificación SMTP exitosa** |
| `error_code` | integer | Código numérico entero para **mapping directo** de estados y errores |
| `reason` | string | Explicación detallada del resultado |
| `smtp_verification` | string (opcional) | Resultado de verificación SMTP: `Deliverable`, `Undeliverable`, `CatchAll`, `TemporaryFailure`, `ConnectionError`, `NoMxRecords`, `NullMx` |
| `smtp_detail` | string (opcional) | Detalle adicional del resultado SMTP (código y mensaje del servidor) |
| `is_catch_all` | boolean (opcional) | `true` si el dominio es catch-all (acepta cualquier destinatario) |
| `is_disposable` | boolean (opcional) | `true` si el email pertenece a un dominio desechable conocido |
| `is_role_based` | boolean (opcional) | `true` si el email es role-based (admin@, info@, support@, etc.) |

---

### 🔢 **Catálogo de Códigos de Error (`error_code`) para Mapping Directo**

| `error_code` | Constante | HTTP Status | Causa | Acción recomendada del cliente |
|:---:|---|:---:|---|---|
| **`0`** | `ERR_SUCCESS` | `200 OK` | Correo válido y con registros MX encontrados | Correo listo para envíos / registro aprobado |
| **`1`** | `ERR_UNKNOWN` | `500 Internal Server Error` | Error de servidor no categorizado | Reintentar la petición |
| **`2`** | `ERR_INVALID_SYNTAX` | `200 OK` / `400 Bad Request` | Sintaxis incorrecta (RFC 5321/5322) | Solicitar al usuario que verifique su dirección |
| **`3`** | `ERR_NO_MX_RECORDS` | `200 OK` | Dominio sin servidores MX ni fallback SMTP | Correo no entregable (dominio inexistente o sin email) |
| **`4`** | `ERR_NULL_MX_REJECTED` | `200 OK` | Dominio con Null MX explícito (RFC 7505) | Dominio rechaza correos explícitamente |
| **`5`** | `ERR_DNS_ERROR` | `200 OK` / `502 Bad Gateway` | Fallo de red o timeout en servidor DNS | Reintentar en unos segundos |
| **`6`** | `ERR_UNAUTHORIZED` | `401 Unauthorized` | API Key ausente, incorrecta o expirada | Renovar credencial en header `X-API-Key` |
| **`7`** | `ERR_RATE_LIMITED` | `429 Too Many Requests` | Límite de peticiones/segundo excedido | Pausar peticiones / backoff exponencial |
| **`8`** | `ERR_PAYLOAD_TOO_LARGE`| `413 Payload Too Large` | Cuerpo de petición mayor a 2KB | Reducir tamaño del JSON enviado |
| **`9`** | `ERR_SMTP_REJECTED` | `200 OK` | Buzón rechazado por servidor SMTP (inexistente, lleno, política) | Correo no entregable - buzón inválido |
| **`10`** | `ERR_CATCH_ALL` | `200 OK` | Dominio catch-all (acepta cualquier destinatario) | No verificable individualmente - precaución |
| **`11`** | `ERR_SMTP_TEMPORARY_FAILURE` | `200 OK` | Fallo temporal SMTP (greylisting, servidor ocupado) | Reintentar automáticamente |
| **`12`** | `ERR_SMTP_CONNECTION_ERROR` | `200 OK` | Error de conexión/timeout al servidor SMTP | Reintentar / verificar conectividad |
| **`13`** | `ERR_DISPOSABLE_EMAIL` | `200 OK` | Email desechable (dominio temporal conocido) | No recomendado para registros |
| **`14`** | `WARN_ROLE_BASED_EMAIL` | `200 OK` | Email basado en rol (admin@, info@, support@, etc.) | Decisión de negocio - no error técnico |

## 🛠️ **Ejemplos de Uso**

### **Ejemplo 1: Validación exitosa**

**Request:**
```bash
curl -X POST http://localhost:3000/api/v1/validate-email \
  -H "Content-Type: application/json" \
  -H "X-API-Key: tu-api-key-aqui" \
  -d '{"email":"usuario@gmail.com"}'
```

**Response:**
```json
{
  "email": "usuario@gmail.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": true,
  "reason": "Correo válido y con registros MX encontrados"
}
```

### **Ejemplo 2: Error de sintaxis**

**Request:**
```bash
curl -X POST http://localhost:3000/api/v1/validate-email \
  -H "Content-Type: application/json" \
  -H "X-API-Key: tu-api-key-aqui" \
  -d '{"email":"usuario.sin.arroba.com"}'
```

**Response:**
```json
{
  "error": "Correo electrónico inválido: El correo debe tener exactamente un '@'",
  "status": 400
}
```

### **Ejemplo 3: Dominio sin MX**

**Request:**
```bash
curl -X POST http://localhost:3000/api/v1/validate-email \
  -H "Content-Type: application/json" \
  -H "X-API-Key: tu-api-key-aqui" \
  -d '{"email":"usuario@dominio-inexistente-12345.com"}'
```

**Response:**
```json
{
  "email": "usuario@dominio-inexistente-12345.com",
  "is_valid_syntax": true,
  "has_mx_records": false,
  "is_deliverable": false,
  "reason": "El dominio no tiene registros MX, el correo no es entregable"
}
```

## 🔧 **Configuración del Servicio**

### **Variables de Entorno**

| Variable | Descripción | Por defecto | Ejemplo |
|----------|-------------|-------------|---------|
| `API_KEY` | Clave de autenticación | **(requerido)** | `super-secret-key-12345` |
| `ALLOWED_ORIGINS` | Dominios CORS permitidos | `http://localhost:3000,http://localhost:5173` | `https://tudominio.com` |
| `PORT` | Puerto del servidor | `3000` | `8080` |
| `RATE_LIMIT_QPS` | Solicitudes por segundo | `10` | `50` |
| `RUST_LOG` | Nivel de logging | `info` | `debug` |

### **Archivo .env de ejemplo:**
```bash
# API Key (¡CAMBIAR EN PRODUCCIÓN!)
API_KEY=my-super-secret-api-key-12345

# Dominios permitidos para CORS
ALLOWED_ORIGINS=https://tudominio.com,https://app.tudominio.com

# Puerto del servidor
PORT=3000

# Rate limiting
RATE_LIMIT_QPS=10

# Logging
RUST_LOG=info
```

## 🚀 **Despliegue**

### **Con Docker Compose:**
```bash
# 1. Configurar variables de entorno
cp .env.example .env
# EDITAR .env con tus valores

# 2. Construir y ejecutar
docker-compose up --build -d

# 3. Verificar
curl http://localhost:3000/api/v1/validate-email \
  -H "X-API-Key: $(grep API_KEY .env | cut -d'=' -f2)" \
  -H "Content-Type: application/json" \
  -d '{"email":"test@gmail.com"}'
```

### **Con Kubernetes:**
```yaml
apiVersion: v1
kind: Service
metadata:
  name: email-validator
spec:
  selector:
    app: email-validator
  ports:
    - port: 3000
      targetPort: 3000
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: email-validator
spec:
  replicas: 3
  selector:
    matchLabels:
      app: email-validator
  template:
    metadata:
      labels:
        app: email-validator
    spec:
      containers:
      - name: email-validator
        image: email-validator:latest
        ports:
        - containerPort: 3000
        env:
        - name: API_KEY
          valueFrom:
            secretKeyRef:
              name: email-validator-secrets
              key: api-key
        - name: ALLOWED_ORIGINS
          value: "https://tudominio.com"
```

## 📊 **Monitoreo y Métricas**

### **Health Check (implícito)**
El endpoint principal sirve como health check:
- 200: Servicio funcionando
- 401: Servicio funcionando pero requiere autenticación
- Otros códigos: Servicio con problemas

### **Métricas recomendadas:**
1. **Tiempo de respuesta**: Promedio y percentiles (p95, p99)
2. **Rate limiting**: Solicitudes bloqueadas por rate limit
3. **Error rate**: Porcentaje de solicitudes con error
4. **DNS resolution time**: Tiempo de consulta MX
5. **Throughput**: Solicitudes por minuto/hora

### **Logs estructurados:**
```json
{
  "timestamp": "2026-10-15T12:00:00Z",
  "level": "INFO",
  "message": "Validación exitosa",
  "email": "usuario@gmail.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "response_time_ms": 245,
  "client_ip": "192.168.1.1"
}
```

## 🔒 **Consideraciones de Seguridad**

### **1. API Key Management**
- Usar variables de entorno, no hardcode
- Rotar claves periódicamente
- Diferentes claves por entorno
- Registrar uso de claves para auditoría

### **2. Rate Limiting**
- Previene ataques de fuerza bruta
- Protege recursos del servidor
- Configurable por entorno

### **3. CORS Configuration**
- Limitar a dominios específicos
- No usar `*` en producción
- Configurar métodos y headers permitidos

### **4. Payload Limits**
- Limitar tamaño de request a 2KB
- Previene ataques de memoria
- Protege contra payloads maliciosos

### **5. Timeouts**
- DNS timeout de 3 segundos
- Previene bloqueo de hilos
- Respuestas predecibles

## 🧪 **Testing**

### **Pruebas con cURL:**
```bash
# Test básico
curl -X POST http://localhost:3000/api/v1/validate-email \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{"email":"test@gmail.com"}'

# Test de rate limiting
for i in {1..20}; do
  curl -s -o /dev/null -w "%{http_code}\n" \
    -X POST http://localhost:3000/api/v1/validate-email \
    -H "Content-Type: application/json" \
    -H "X-API-Key: $API_KEY" \
    -d '{"email":"test@gmail.com"}'
done
```

### **Pruebas de carga:**
```bash
# Usar ab (Apache Bench)
ab -n 1000 -c 10 \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -p test-payload.json \
  http://localhost:3000/api/v1/validate-email
```

## 📝 **Best Practices para Clientes**

### **1. Manejo de errores:**
```javascript
async function validateEmail(email) {
  try {
    const response = await fetch('/api/v1/validate-email', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-API-Key': process.env.API_KEY,
      },
      body: JSON.stringify({ email }),
    });
    
    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Error ${response.status}: ${error.error}`);
    }
    
    return await response.json();
  } catch (error) {
    // Manejo de errores específicos
    if (error.message.includes('429')) {
      // Rate limit excedido - reintentar después
      return { error: 'Rate limit excedido, intente más tarde' };
    }
    throw error;
  }
}
```

### **2. Caching de respuestas:**
- Cachear respuestas exitosas por 5-10 minutos
- No cachear respuestas de error 4xx/5xx
- Invalidar cache cuando cambian los registros MX

### **3. Backoff exponencial:**
- Implementar reintentos con backoff exponencial
- No reintentar en errores 4xx (excepto 429)
- Límite de reintentos: 3-5 veces

## 🛡️ **Endpoints Administrativos y Métricas Operativas**

Todos los endpoints administrativos requieren autenticación mediante cabecera `Authorization: Basic <base64(usuario:contraseña)>`.

### **GET /api/v1/admin/metrics**

Retorna los indicadores clave de rendimiento (KPIs) de salud y tráfico de la API, junto con el ranking de dominios problemáticos más recurrentes detectados en las solicitudes recibidas.

**Headers:**
```http
Authorization: Basic QWRtaW5pc3RyYXRvcjphZG1pbjEyMzQ1
```

**Respuesta Exitosa (200 OK):**
```json
{
  "total_requests": 1420,
  "processed_validations": 1395,
  "api_success_rate": 100.0,
  "average_latency_ms": 148.5,
  "auth_failures": 25,
  "rate_limit_hits": 0,
  "syntax_errors": 180,
  "no_mx_count": 95,
  "dns_errors": 2,
  "uptime_seconds": 86400,
  "top_problematic_domains": [
    {
      "domain": "gmial.com",
      "occurrences": 42,
      "reason": "Sin registros MX configurados o Null MX (RFC 7505)",
      "error_category": "sin_mx"
    },
    {
      "domain": "dominio..invalido.com",
      "occurrences": 18,
      "reason": "El dominio no puede contener puntos consecutivos (..)",
      "error_category": "sintaxis"
    }
  ]
}
```

---

## 🔄 **Versioning de API**

### **Current: v1**
- Endpoint: `/api/v1/validate-email`
- Formato de respuesta estable
- No breaking changes planeadas

### **Future: v2 (planeado)**
- Campos adicionales en respuesta
- Soporte para validación adicional (SPF, DKIM)
- Batch validation (múltiples correos en una solicitud)

## 📞 **Soporte y Contacto**

- **Documentación**: `docs/` folder
- **Issues**: Reportar bugs en GitHub Issues
- **Soporte técnico**: tech-support@tudominio.com
- **Actualizaciones de seguridad**: security@tudominio.com

---

**Última actualización:** 15 de octubre de 2026  
**Versión de API:** v1  
**Mantenido por:** DevSecOps Team  
**Estado:** ✅ Production Ready
