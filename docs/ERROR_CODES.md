# 📋 Códigos de Error - Email Validator API

## 📊 **Resumen de Códigos HTTP**

| Código | Nombre | Descripción |
|--------|--------|-------------|
| **200** | OK | Validación exitosa |
| **400** | Bad Request | Error de sintaxis o validación |
| **401** | Unauthorized | Faltan credenciales o API Key inválida |
| **413** | Payload Too Large | Payload excede el límite de 2KB |
| **429** | Too Many Requests | Rate limit excedido |
| **502** | Bad Gateway | Error en resolución DNS |
| **500** | Internal Server Error | Error interno del sistema |

## 🔍 **Errores 4xx - Errores del Cliente**

### **400 Bad Request**

**Causas:**
- Formato JSON inválido
- Campo `email` ausente o vacío
- Errores de validación de sintaxis

**Ejemplo de respuesta:**
```json
{
  "error": "Correo electrónico inválido: El correo debe tener exactamente un '@'",
  "status": 400
}
```

**Sub-errores específicos:**
1. `El correo no puede estar vacío`
2. `El correo debe tener exactamente un '@'`
3. `La parte local del correo no puede estar vacía`
4. `La parte local no puede empezar ni terminar con '.'`
5. `La parte local no puede tener '.' consecutivos`
6. `La parte local no puede contener espacios`
7. `El dominio no puede estar vacío`
8. `El dominio debe tener al menos un '.'`
9. `El dominio no puede empezar ni terminar con '.' o '-'`
10. `El dominio no puede tener '-' consecutivos`
11. `Caracter no ASCII imprimible: <carácter>`

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
