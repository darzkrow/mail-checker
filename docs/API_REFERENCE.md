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

**Respuesta exitosa (200):**
```json
{
  "email": "usuario@dominio.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": true,
  "reason": "Correo válido y con registros MX encontrados"
}
```

**Respuesta con error de sintaxis (400):**
```json
{
  "email": "usuario.sin-arroba.com",
  "is_valid_syntax": false,
  "has_mx_records": false,
  "is_deliverable": false,
  "reason": "Formato de correo inválido"
}
```

**Respuesta sin registros MX (200):**
```json
{
  "email": "usuario@dominio-inexistente.com",
  "is_valid_syntax": true,
  "has_mx_records": false,
  "is_deliverable": false,
  "reason": "El dominio no tiene registros MX, el correo no es entregable"
}
```

---

### **Parámetros del Request**

| Campo | Tipo | Requerido | Descripción | Ejemplo |
|-------|------|-----------|-------------|---------|
| `email` | string | ✅ | Dirección de correo a validar | `"usuario@gmail.com"` |

### **Campos de Respuesta**

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `email` | string | Dirección de correo validada |
| `is_valid_syntax` | boolean | `true` si la sintaxis es válida |
| `has_mx_records` | boolean | `true` si el dominio tiene registros MX |
| `is_deliverable` | boolean | `true` si ambas validaciones pasan |
| `reason` | string | Explicación detallada del resultado |

---

### **Valores posibles de `reason`:**

| Valor | Significado |
|-------|-------------|
| `"Correo válido y con registros MX encontrados"` | Validación exitosa |
| `"Formato de correo inválido"` | Error de sintaxis |
| `"El dominio no tiene registros MX, el correo no es entregable"` | Dominio sin MX |
| `"Error en la resolución DNS: <detalle>"` | Error DNS específico |

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
