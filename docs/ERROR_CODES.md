# 📋 Códigos de Error - Email Validator API

## 🔢 **Catálogo de Códigos de Error Numéricos (`error_code`) para Mapping**

Para evitar el análisis frágil de cadenas de texto (`reason` o `error`), la API proporciona un identificador numérico estandarizado **`error_code`** en todas las respuestas JSON:

```json
{
  "email": "usuario@ejemplo.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": true,
  "error_code": 0,
  "reason": "Correo válido y con registros MX encontrados"
}
```

### **Tabla de Mapeo Directo para Clientes**

| `error_code` | Constante | HTTP Status | Categoría | Descripción / Causa | Acción recomendada del cliente |
|:---:|---|:---:|:---:|---|---|
| **`0`** | `ERR_SUCCESS` | `200 OK` | Éxito | Correo válido y con registros MX encontrados | Correo listo para envíos / registro aprobado |
| **`1`** | `ERR_UNKNOWN` | `500 Internal Server Error` | Servidor | Error de servidor no categorizado | Reintentar la petición |
| **`2`** | `ERR_INVALID_SYNTAX` | `200 OK` / `400 Bad Request` | Validación | Sintaxis incorrecta (RFC 5321/5322) | Solicitar al usuario que verifique su dirección |
| **`3`** | `ERR_NO_MX_RECORDS` | `200 OK` | Validación | Dominio sin servidores MX ni fallback SMTP | Correo no entregable (dominio inexistente o sin email) |
| **`4`** | `ERR_NULL_MX_REJECTED` | `200 OK` | Validación | Dominio con Null MX explícito (RFC 7505) | Dominio rechaza correos explícitamente |
| **`5`** | `ERR_DNS_ERROR` | `200 OK` / `502 Bad Gateway` | Red/DNS | Fallo de red o timeout en servidor DNS | Reintentar en unos segundos |
| **`6`** | `ERR_UNAUTHORIZED` | `401 Unauthorized` | Auth | API Key ausente, incorrecta o expirada | Renovar credencial en header `X-API-Key` |
| **`7`** | `ERR_RATE_LIMITED` | `429 Too Many Requests` | Límites | Límite de peticiones/segundo excedido | Pausar peticiones / backoff exponencial |
| **`8`** | `ERR_PAYLOAD_TOO_LARGE`| `413 Payload Too Large` | Validación | Cuerpo de petición mayor a 2KB | Reducir tamaño del JSON enviado |

### **Ejemplo de Implementación de Mapping en el Cliente (JavaScript / TypeScript)**

```javascript
const response = await fetch('http://localhost:3000/api/v1/validate-email', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'X-API-Key': process.env.API_KEY
  },
  body: JSON.stringify({ email: userEmail })
});

const data = await response.json();

switch (data.error_code) {
  case 0:
    // Correo verificado con éxito
    console.log("Email válido y entregable");
    break;
  case 2:
    showError("La dirección tiene errores de escritura o formato inválido.");
    break;
  case 3:
  case 4:
    showError("El dominio no posee buzón de correo electrónico activo.");
    break;
  case 5:
    showWarning("No se pudo contactar a los servidores DNS. Reintentando...");
    break;
  case 6:
    console.error("Error de autenticación con la API de validación.");
    break;
  case 7:
    console.warn("Límite de solicitudes alcanzado. Espere un momento.");
    break;
  default:
    console.error(`Error no clasificado (${data.error_code}): ${data.reason || data.error}`);
}
```

---

## 📊 **Resumen de Códigos HTTP**

| Código | Nombre | Descripción |
|--------|--------|-------------|
| **200** | OK | Validación procesada con éxito (incluye resultados válidos y con sintaxis inválida) |
| **400** | Bad Request | Formato JSON malformado o solicitud inválida |
| **401** | Unauthorized | Faltan credenciales, API Key inválida o expirada |
| **413** | Payload Too Large | Payload excede el límite de 2KB |
| **429** | Too Many Requests | Rate limit excedido (configurable vía RATE_LIMIT_QPS) |
| **502** | Bad Gateway | Error irrecuperable en resolución DNS |
| **500** | Internal Server Error | Error interno del sistema |

## 🔍 **Errores 4xx - Errores del Cliente**

### **400 Bad Request**

> **Nota:** La validación de sintaxis de un correo devuelve **200 OK** con `is_valid_syntax: false` y el motivo en el campo `reason`. El código **400 Bad Request** solo se genera si el cliente envía un JSON inválido, campos de tipo incompatible o un payload vacío.

**Causas:**
- Formato JSON malformado o no parseable
- Payload HTTP vacío o sin el campo requerido `"email"`

**Ejemplo de respuesta:**
```json
{
  "error": "Failed to parse the request body as JSON"
}
```

### **401 Unauthorized**

**Causas:**
- Header `X-API-Key` ausente
- API Key incorrecta o no configurada

**Ejemplo de respuesta:**
```json
{
  "error": "API Key inválida",
  "status": 401
}
```

**Sub-errores:**
1. `Falta el header X-API-Key` - Header ausente
2. `API Key inválida` - API Key incorrecta

### **413 Payload Too Large**

**Causa:**
- Payload JSON excede el límite de 2048 bytes (2KB)

**Ejemplo de respuesta:**
```json
{
  "error": "Payload demasiado grande. Máximo 2KB permitidos.",
  "status": 413
}
```

### **429 Too Many Requests**

**Causa:**
- Límite de solicitudes por segundo excedido
- Configuración por defecto: 10 solicitudes/segundo por IP

**Ejemplo de respuesta:**
```json
{
  "error": "Rate limit excedido. Intente más tarde.",
  "status": 429
}
```

## 🔍 **Errores 5xx - Errores del Servidor**

### **502 Bad Gateway**

**Causas:**
- Error en la resolución DNS
- Timeout de 3 segundos en consulta MX
- Problemas de conectividad con servidores DNS

**Ejemplo de respuesta:**
```json
{
  "error": "Error en la resolución DNS: Timeout (3s) al resolver registros MX para 'dominio.com'",
  "status": 502
}
```

**Sub-errores:**
1. `Error creando resolver: <detalle>`
2. `Timeout (3s) al resolver registros MX para '<dominio>'`

### **500 Internal Server Error**

**Causas:**
- Error interno inesperado del sistema
- Excepciones no manejadas
- Problemas de memoria o recursos

**Ejemplo de respuesta:**
```json
{
  "error": "Error interno del servidor: Error inesperado en el sistema",
  "status": 500
}
```

## 🛠️ **Flujo de Validación y Errores**

### **Fase 1: Autenticación (Middleware)**
```
Cliente → Servidor
      ↓
[Header X-API-Key]
      ↓
      ├── Ausente → 401 "Falta el header X-API-Key"
      ├── Inválida → 401 "API Key inválida"
      └── Válida → Continuar
```

### **Fase 2: Validación de Payload**
```
Payload JSON
      ↓
      ├── > 2KB → 413 "Payload demasiado grande"
      ├── Campo 'email' ausente → 400 "Campo email requerido"
      └── Formato JSON inválido → 400 "JSON malformado"
```

### **Fase 3: Validación de Sintaxis**
```
Campo email
      ↓
      ├── Vacío → 400 "El correo no puede estar vacío"
      ├── Sin @ → 400 "El correo debe tener exactamente un '@'"
      ├── Múltiples @ → 400 "El correo debe tener exactamente un '@'"
      └── Sintaxis inválida → 400 "<mensaje específico>"
```

### **Fase 4: Resolución DNS MX**
```
Dominio extraído
      ↓
Resolver → Consulta MX
      ↓
      ├── Timeout (3s) → 502 "Timeout al resolver..."
      ├── Error DNS → 502 "Error en la resolución DNS"
      └── Sin registros MX → Respuesta con has_mx_records: false
```

## 📈 **Diagrama de Flujo de Errores**

```
┌─────────────────┐
│  Cliente envía  │
│    solicitud    │
└────────┬────────┘
         ↓
┌─────────────────┐
│  Middleware:    │
│    API Key      │
└────────┬────────┘
   401 ┌─┴─┐ 200
      ↓    ↓
┌─────────────┐  ┌─────────────────┐
│  Faltan     │  │  Rate Limiting  │
│credenciales │  └────────┬────────┘
└─────────────┘     429 ┌─┴─┐ 200
                        ↓    ↓
                ┌─────────────┐  ┌─────────────────┐
                │Rate limit   │  │  Body Size      │
                │excedido     │  │   Limiting      │
                └─────────────┘  └────────┬────────┘
                                    413 ┌─┴─┐ 200
                                       ↓    ↓
                               ┌─────────────┐  ┌─────────────────┐
                               │Payload      │  │  Validación     │
                               │demasiado    │  │   Sintaxis     │
                               │grande       │  └────────┬────────┘
                               └─────────────┘     400 ┌─┴─┐ 200
                                                      ↓    ↓
                                              ┌─────────────┐  ┌─────────────────┐
                                              │Sintaxis     │  │  DNS MX Query   │
                                              │inválida     │  └────────┬────────┘
                                              └─────────────┘     502 ┌─┴─┐ 200
                                                                     ↓    ↓
                                                             ┌─────────────┐  ┌─────────────────┐
                                                             │Error DNS    │  │  Respuesta      │
                                                             │o Timeout    │  │  exitosa (200)  │
                                                             └─────────────┘  └─────────────────┘
```

## 🔧 **Configuración de Rate Limiting**

### **Variables de Entorno**
```bash
RATE_LIMIT_QPS=10  # Solicitudes por segundo por IP
```

### **Comportamiento por defecto:**
- **10 solicitudes/segundo por IP**
- Basado en header `X-Forwarded-For` cuando está detrás de proxy
- Fallback a IP del cliente cuando no hay proxy

### **Recomendaciones por Entorno:**

#### **Desarrollo:**
```bash
RATE_LIMIT_QPS=50  # Más permisivo para testing
```

#### **Producción:**
```bash
RATE_LIMIT_QPS=10  # Estricto para prevenir abuso
```

#### **Alto tráfico:**
```bash
RATE_LIMIT_QPS=100  # Escalar según capacidad del servidor
```

## 🚨 **Solución de Problemas**

### **Error 401 - "Falta el header X-API-Key"**
**Solución:**
```bash
curl -X POST http://localhost:3000/api/v1/validate-email \
  -H "X-API-Key: $(cat .env | grep API_KEY | cut -d'=' -f2)" \
  -H "Content-Type: application/json" \
  -d '{"email":"test@gmail.com"}'
```

### **Error 429 - "Rate limit excedido"**
**Solución:**
1. Aumentar `RATE_LIMIT_QPS` en `.env`
2. Implementar backoff exponencial en el cliente
3. Usar caching de respuestas

### **Error 502 - "Error en la resolución DNS"**
**Solución:**
1. Verificar conectividad a Internet
2. Probar con un dominio conocido (gmail.com)
3. Aumentar timeout DNS (modificar código)

### **Error 413 - "Payload demasiado grande"**
**Solución:**
1. Reducir tamaño del payload
2. Verificar que solo se envíe el campo `email`

## 📋 **Logs de Error por Nivel**

### **Nivel INFO:**
```
INFO  email_validator > Validación exitosa para usuario@gmail.com
INFO  email_validator > Rate limit excedido para IP: 192.168.1.1
```

### **Nivel WARN:**
```
WARN  email_validator > Error resolviendo MX para dominio-inexistente.com: request timed out
WARN  email_validator > API Key inválida recibida de IP: 192.168.1.2
```

### **Nivel ERROR:**
```
ERROR email_validator > Error interno: Error inesperado al procesar solicitud
```

## 🔍 **Monitoreo de Errores**

### **Métricas recomendadas:**
1. **Error rate**: Porcentaje de solicitudes con error
2. **Rate limit hits**: Número de solicitudes bloqueadas por rate limiting
3. **DNS failure rate**: Porcentaje de fallos en resolución DNS
4. **Average response time**: Tiempo promedio de respuesta
5. **Error distribution**: Distribución por tipo de error

### **Alertas recomendadas:**
- Error rate > 5% por 5 minutos
- Rate limit hits > 100 por hora
- DNS failure rate > 20%
- Response time > 3 segundos

---

**Última actualización:** 15 de octubre de 2026  
**Responsable:** DevSecOps Team  
**Documentación mantenida en:** `/docs/ERROR_CODES.md`
