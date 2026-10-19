# 🔐 Guía de Seguridad - Email Validator API

## 📋 **Resumen de Seguridad**

Este documento detalla las medidas de seguridad implementadas en el microservicio Email Validator API y proporciona guías para su despliegue seguro en producción.

## 🛡️ **Medidas de Seguridad Implementadas**

### **1. Autenticación por API Key y Gestión de Ciclo de Vida**
- ✅ **Header requerido**: `X-API-Key` en todas las solicitudes del endpoint de validación
- ✅ **Soporte de expiración y revocación**: Claves con fecha de vencimiento configurable y desactivación inmediata
- ✅ **Persistencia atómica**: Base de datos de claves persistida en disco de forma transaccional (`.tmp` -> replace)
- ✅ **Error handling seguro**: Respuestas 401 sin fugas de información

### **2. Rate Limiting Activo**
- ✅ **Límite configurable**: Control de frecuencia por API Key basado en `RATE_LIMIT_QPS` (por defecto: 10 peticiones/segundo)
- ✅ **Ventana de control en memoria**: Algoritmo de conteo con auto-limpieza periódica para evitar fugas de memoria
- ✅ **Respuesta estándar**: HTTP 429 Too Many Requests con mensaje de reintento

### **3. Seguridad del Panel de Administración**
- ✅ **Hash SHA-256 con Sal Aleatoria (16 caracteres)**: Cada contraseña se combina con una sal alfanumérica única
- ✅ **Verificación simétrica y determinista**: Verificación inmune a discrepancias de codificación base64
- ✅ **Persistencia transaccional de credenciales**: Almacenamiento seguro en `{DATA_DIR}/admin_auth.json`
- ✅ **Política estricta de contraseñas**: Validación de longitud mínima/máxima, mayúsculas, minúsculas, números y caracteres especiales
- ✅ **Autenticación HTTP Basic con decodificación estándar**: Uso de `base64::engine::general_purpose::STANDARD`

### **4. Control de CORS y Límites de Carga**
- ✅ **Dominios restringidos**: Configurados mediante `ALLOWED_ORIGINS`
- ✅ **Límite de Payload estricto**: Máximo 2KB (`DefaultBodyLimit::max(2048)`) para prevenir ataques de saturación de memoria

### **5. Validación Rigurosa y Mitigación de Ataques DNS**
- ✅ **Cumplimiento RFC 5321 y RFC 5322**: Rechazo de inyecciones de encabezados, caracteres de control, puntos consecutivos y etiquetas malformadas
- ✅ **Protección Null MX (RFC 7505)**: Prevención de falsos positivos en dominios que rechazan correo
- ✅ **Fallback SMTP (RFC 5321 §5.1)**: Resolución hacia A/AAAA cuando no existan registros MX explícitos

### **6. Imagen Docker Segura**
- ✅ **Multi-stage build**: Entorno de compilación aislado sin herramientas en el runtime
- ✅ **Non-root user**: Contenedor ejecuta bajo el usuario sin privilegios `appuser` (UID 1000)
- ✅ **Health check integrado**: Monitoreo continuo mediante `curl` en `/health`

## 🔒 **Configuración de Seguridad por Entorno**

### **Desarrollo (Local)**
```bash
API_KEY=dev-key-12345
ALLOWED_ORIGINS=http://localhost:4000,http://localhost:5173
RATE_LIMIT_QPS=50
RUST_LOG=debug
```

### **Staging**
```bash
API_KEY=staging-key-$(openssl rand -base64 32)
ALLOWED_ORIGINS=https://staging.tudominio.com
RATE_LIMIT_QPS=20
RUST_LOG=info
```

### **Producción**
```bash
API_KEY=prod-key-$(openssl rand -base64 64)
ALLOWED_ORIGINS=https://tudominio.com,https://app.tudominio.com
RATE_LIMIT_QPS=10
RUST_LOG=warn
```

## 🚨 **Amenazas Identificadas y Mitigaciones**

### **1. Ataque de Fuerza Bruta**
**Amenaza**: Ataques DDoS o intentos masivos de validación  
**Mitigación**: Rate limiting (10 req/s por IP)  
**Detección**: Logs de rate limit hits > 100/hora

### **2. API Key Leakage**
**Amenaza**: Exposición de API Keys en logs o errores  
**Mitigación**: No logging de API Keys, errores genéricos 401  
**Detección**: Monitoreo de intentos con API Keys inválidas

### **3. Ataque de Memoria**
**Amenaza**: Payloads masivos para consumir RAM  
**Mitigación**: Límite de 2KB por request  
**Detección**: Alerta en payloads > 1KB frecuentes

### **4. DNS Amplification**
**Amenaza**: Uso del servicio para ataques DNS  
**Mitigación**: Timeout de 3s y rate limiting  
**Detección**: Múltiples consultas a dominios inexistentes

### **5. CORS Abuse**
**Amenaza**: UCSRF desde dominios no autorizados  
**Mitigación**: CORS estricto por dominio  
**Detección**: Intentos de conexión desde dominios no autorizados

## 📊 **Monitoreo de Seguridad**

### **Métricas Clave a Monitorear:**
1. **Rate limit hits**: > 100/hora = posible ataque
2. **API Key failures**: > 50/hora = posible leak
3. **Payload size average**: > 1KB = revisar clientes
4. **DNS failure rate**: > 20% = revisar resolvers
5. **Error 401 rate**: > 5% = revisar autenticación

### **Alertas de Seguridad:**
```yaml
alerts:
  - name: high-rate-limit-hits
    condition: rate_limit_hits > 100 per hour
    severity: warning
    
  - name: api-key-brute-force
    condition: api_key_failures > 50 per hour
    severity: critical
    
  - name: large-payload-attack
    condition: avg_payload_size > 1500 bytes
    severity: warning
    
  - name: dns-amplification
    condition: dns_queries_per_ip > 100 per minute
    severity: critical
```

## 🔧 **Hardening de Docker**

### **1. Security Context:**
```dockerfile
# Usar usuario no-root
USER nonroot:nonroot

# Limitar capabilities
# docker run --cap-drop=ALL --cap-add=NET_BIND_SERVICE
```

### **2. Resource Limits:**
```yaml
# docker-compose.yml
deploy:
  resources:
    limits:
      cpus: '1'
      memory: 256M
    reservations:
      cpus: '0.5'
      memory: 128M
```

### **3. Network Security:**
```yaml
# Solo puerto necesario expuesto
ports:
  - "3001:3001"

# Network segmentation
networks:
  email-validator-net:
    driver: bridge
    internal: true  # Para producción
```

### **4. Health Checks:**
```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:3001/api/v1/validate-email"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 10s
```

## 📝 **Política de Rotación de Claves**

### **Frecuencia de Rotación:**
- **API Keys de desarrollo**: Cada 90 días
- **API Keys de staging**: Cada 30 días  
- **API Keys de producción**: Cada 15 días
- **Claves comprometidas**: Inmediatamente

### **Proceso de Rotación:**
1. Generar nueva clave: `openssl rand -base64 64`
2. Actualizar variable de entorno
3. Notificar a todos los clientes
4. Mantener clave anterior por 24h
5. Eliminar clave antigua después de 24h

### **Registro de Rotación:**
```bash
# Ejemplo de registro
2026-10-15 14:30:00 - Rotación API Key producción
  - Antigua: prod-key-abc123... (último uso: 2026-10-15 14:25:00)
  - Nueva: prod-key-def456... (primero uso: 2026-10-15 14:31:00)
  - Clientes notificados: 3/3
```

## 🧪 **Penetration Testing Checklist**

### **Pruebas de Autenticación:**
- [ ] Solicitud sin API Key → 401
- [ ] API Key inválida → 401
- [ ] API Key correcta → 200
- [ ] Rate limit funciona → 429 después de 10 req/s
- [ ] Rate limit por IP → IPs diferentes tienen límites separados

### **Pruebas de Input Validation:**
- [ ] Payload > 2KB → 413
- [ ] JSON malformado → 400
- [ ] Email sin @ → 400
- [ ] Email con múltiples @ → 400
- [ ] Email con espacios → 400
- [ ] Email vacío → 400
- [ ] Email con caracteres Unicode → 400

### **Pruebas de CORS:**
- [ ] Dominio no autorizado → CORS error
- [ ] Dominio autorizado → Acceso permitido
- [ ] Método no permitido (GET) → CORS error
- [ ] Header no permitido → CORS error

### **Pruebas de Rendimiento/Seguridad:**
- [ ] 1000 solicitudes concurrentes → Rate limiting activo
- [ ] Payload de 5MB → Rechazado temprano (413)
- [ ] Dominios inexistentes → Timeout DNS (502)
- [ ] Service health → Health check funciona

## 🚨 **Respuesta a Incidentes**

### **1. API Key Comprometida:**
**Acciones inmediatas:**
1. Rotar clave inmediatamente
2. Revocar clave comprometida
3. Auditar logs para uso no autorizado
4. Notificar a clientes afectados
5. Incrementar logging de autenticación

### **2. Ataque DDoS Detectado:**
**Acciones inmediatas:**
1. Reducir RATE_LIMIT_QPS temporalmente
2. Habilitar WAF/CDN si disponible
3. Monitorear métricas de rate limiting
4. Notificar al equipo de infraestructura
5. Considerar bloqueo de IPs maliciosas

### **3. Fuga de Información:**
**Acciones inmediatas:**
1. Identificar fuente de la fuga (logs, errores)
2. Corregir la configuración/ código
3. Rotar claves si fueron expuestas
4. Auditar accesos recientes
5. Revisar políticas de logging

## 📚 **Referencias y Estándares**

### **Estándares Seguidos:**
- OWASP API Security Top 10
- NIST Cybersecurity Framework
- RFC 5322 (Email Syntax)
- Docker Security Best Practices

### **Herramientas Recomendadas:**
- **SAST**: `cargo-audit`, `clippy`
- **DAST**: OWASP ZAP, Burp Suite
- **Container Scanning**: Trivy, Clair
- **Secrets Scanning**: GitGuardian, TruffleHog

### **Documentación Relacionada:**
- `docs/ERROR_CODES.md` - Códigos de error
- `docs/API_REFERENCE.md` - Referencia de API
- `README.md` - Guía de despliegue

## 📞 **Contacto de Seguridad**

**Reportar vulnerabilidades:** security@tudominio.com  
**Responsable de seguridad:** security-team@tudominio.com  
**Respuesta SLA:** 24 horas para vulnerabilidades críticas

**Proceso de reporte:**
1. Email a security@ con detalles
2. No divulgar públicamente hasta resolución
3. Esperar confirmación de recepción
4. Seguir proceso coordinado de disclosure

---

**Última revisión:** 15 de octubre de 2026  
**Revisado por:** DevSecOps Team  
**Próxima revisión:** 15 de enero de 2027  
**Estado:** ✅ Aprobado para producción
