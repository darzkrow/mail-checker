# 📜 Registro de Parches y Mejoras Lógicas (Changelog de Auditoría)

Este documento registra todas las correcciones, parches y mejoras aplicadas al microservicio **Email Validator API**, documentando el fallo original, su impacto, la solución implementada y los estándares RFC que respalda cada cambio.

---

## 📌 Resumen Ejecutivo de Correcciones

| ID | Componente | Tipo de Fallo | Severidad | Estándar / RFC | Estado |
|---|---|---|---|---|---|
| **PATCH-01** | `packages/api` | Contrato REST: HTTP 400 en vez de resultado de validación | 🔴 Alta | RESTful Best Practices | ✅ Resuelto |
| **PATCH-02** | `packages/validation` | Omisión de validaciones críticas de sintaxis y límites de longitud | 🔴 Alta | RFC 5321 / RFC 5322 | ✅ Resuelto |
| **PATCH-03** | `packages/validation` | Inconsistencia condicional en `format_response` | 🟡 Media | Lógica Booleana | ✅ Resuelto |
| **PATCH-04** | `packages/dns` | Null MX tratado como correo entregable | 🔴 Alta | RFC 7505 | ✅ Resuelto |
| **PATCH-05** | `packages/dns` | Ausencia de Fallback a registros A/AAAA | 🟡 Media | RFC 5321 §5.1 | ✅ Resuelto |
| **PATCH-06** | `packages/shared` | Hash criptográfico roto en `AdminPasswordHash::verify` | 🔴 Crítica | Criptografía / Auth | ✅ Resuelto |
| **PATCH-07** | `packages/api` | Endpoint `/admin/password` sin persistencia real | 🔴 Alta | Seguridad / Persistencia | ✅ Resuelto |
| **PATCH-08** | `packages/api` | Rate Limiter declarado pero no ejecutado | 🟡 Media | DoS Mitigation | ✅ Resuelto |
| **PATCH-09** | `packages/api` | Comentario de sintaxis JavaScript inválido en `admin_ui.rs` | 🟢 Baja | EcmaScript Standard | ✅ Resuelto |
| **PATCH-10** | `packages/shared`, `packages/api` | Rediseño de KPIs operativos y tracking de dominios problemáticos | 🟡 Media | Observabilidad Operativa | ✅ Resuelto |

---

## 🛠️ Detalle de Parches y Soluciones Implementadas

### 🔹 PATCH-01: Corrección del Flujo y Contrato REST en Validación
* **Ubicación:** `packages/api/src/main.rs` (`validate_email_handler`)
* **Problema:** Cuando un cliente enviaba un correo con formato inválido (por ejemplo, `test@@domain.com`), el handler retornaba inmediatamente `Err(err.into())` (HTTP 400 Bad Request) con un mensaje plano `{"error": "..."}`.
  - Esto impedía que el cliente recibiera la estructura estándar `EmailValidationResponse`.
  - Hacía imposible que `is_valid_syntax: false` fuera retornado en una respuesta 200 OK.
* **Solución:** Cuando la sintaxis es inválida, la API responde con **HTTP 200 OK** y el cuerpo estructurado:
  ```json
  {
    "email": "test@@domain.com",
    "is_valid_syntax": false,
    "has_mx_records": false,
    "is_deliverable": false,
    "reason": "Formato de correo inválido: no puede contener múltiples caracteres '@'"
  }
  ```
  El código **HTTP 400** se reserva exclusivamente para solicitudes HTTP donde el payload JSON está malformado o el body no contiene los campos requeridos.

---

### 🔹 PATCH-02: Validación Estricta de Sintaxis Conforme a RFC 5321 y RFC 5322
* **Ubicación:** `packages/validation/src/lib.rs`
* **Problema:** La validación previa solo comprobaba la presencia de `@`, un punto en el dominio y ausencia de espacios. Permitía correos como `user@.com`, `user@com.`, `user..name@domain.com`, `.user@domain.com`, caracteres de control (`\r`, `\n`) y cadenas arbitrariamente largas.
* **Solución:**
  1. **Límites de Longitud:** Máximo 254 caracteres para el correo completo (RFC 5321 §4.5.3.1.3), máximo 64 caracteres para `local_part` (§4.5.3.1.1) y máximo 253 caracteres para el dominio.
  2. **Reglas de Puntos:** No permite puntos iniciales, finales ni consecutivos (`..`) ni en la parte local ni en el dominio.
  3. **Juego de Caracteres Permitidos:** Se implementó verificación estricta para caracteres imprimibles ASCII permitidos según RFC 5322 (`atext`), rechazando caracteres de control, saltos de línea e inyecciones.
  4. **Etiquetas de Dominio (Labels):** Cada etiqueta debe medir entre 1 y 63 caracteres (RFC 1035), no puede iniciar ni terminar en guión (`-`) y el TLD debe ser alfabético y tener al menos 2 caracteres.

---

### 🔹 PATCH-03: Corrección Lógica en `format_response`
* **Ubicación:** `packages/validation/src/lib.rs`
* **Problema:** `if has_mx { ... }` evaluaba `has_mx` antes de `is_valid`. Si una llamada contenía `is_valid = false` y `has_mx = true`, reportaba que el correo era válido.
* **Solución:** La comprobación evalúa primero `if !is_valid`:
  ```rust
  let reason = if !is_valid {
      "Formato de correo inválido".to_string()
  } else if is_deliverable {
      "Correo válido y con registros MX encontrados".to_string()
  } else {
      "Correo con sintaxis válida pero sin registros MX o servidor de correo inalcanzable".to_string()
  };
  ```

---

### 🔹 PATCH-04: Soporte para Null MX (RFC 7505)
* **Ubicación:** `packages/dns/src/lib.rs`
* **Problema:** Si un dominio configuraba un "Null MX" (`0 .` - preferencia 0 hacia el dominio raíz `.`), el validador interpretaba la existencia de este registro como `has_mx = true` e `is_deliverable = true`.
* **Solución:** Según RFC 7505, un registro MX con prioridad 0 apuntando a `.` o cadena vacía es una declaración explícita de que **el dominio no acepta correo electrónico**. La función detecta esta condición y retorna `Ok(false)`.

---

### 🔹 PATCH-05: Fallback SMTP a Registros A / AAAA (RFC 5321 §5.1)
* **Ubicación:** `packages/dns/src/lib.rs`
* **Problema:** Dominios sin registros MX explícitos pero con servidores de correo operando en su registro A/AAAA base eran catalogados erróneamente como no entregables.
* **Solución:** Si la consulta MX no retorna registros, se realiza una consulta de resolución de IP (`lookup_ip`). Si el host tiene registros A/AAAA válidos, se clasifica como alcanzable de acuerdo con el estándar SMTP.

---

### 🔹 PATCH-06: Corrección del Bug Criptográfico en `AdminPasswordHash::verify`
* **Ubicación:** `packages/shared/src/lib.rs`
* **Problema:** En `AdminPasswordHash::verify`, se utilizaba `format!("{salt_bytes:?}{password}")` seguido de múltiples escrituras al hasher con tipos incompatibles. La firma resultante nunca coincidía con la generada por `new()`, provocando que el login de administrador siempre fallara si se utilizaba hash.
* **Solución:** Decodificación limpia de la sal desde base64 a UTF-8 y cálculo simétrico exacto:
  ```rust
  let salt_str = std::str::from_utf8(&salt_bytes)?;
  hasher.update(format!("{salt_str}{password}").as_bytes());
  base64::encode(hasher.finalize()) == self.hash
  ```

---

### 🔹 PATCH-07: Persistencia Atómica del Cambio de Contraseña de Administrador
* **Ubicación:** `packages/shared/src/lib.rs` y `packages/api/src/main.rs`
* **Problema:** El endpoint `POST /api/v1/admin/password` validaba la nueva contraseña pero no guardaba el hash resultante en ningún medio de almacenamiento persistente ni actualizaba el estado en memoria.
* **Solución:**
  1. Se implementó la estructura `AdminAuthStore` y el método `save_admin_auth` con escritura atómica (`.tmp` -> rename) en `{DATA_DIR}/admin_auth.json`.
  2. `AppState` incorpora `admin_auth: Arc<RwLock<AdminPasswordHash>>`, permitiendo que el cambio de contraseña tome efecto de inmediato en memoria y persista tras reinicios del servicio.

---

### 🔹 PATCH-08: Activación del Limitador de Tasa (Rate Limiter)
* **Ubicación:** `packages/api/src/main.rs`
* **Problema:** La variable `RATE_LIMIT_QPS` se leía del entorno pero ningún componente en el pipeline HTTP aplicaba control de frecuencia.
* **Solución:** Se implementó `RateLimiter` con control de ventana de 1 segundo por API Key. Si una clave excede el límite de consultas por segundo configurado en `.env`, el servidor responde con **HTTP 429 Too Many Requests**.

---

### 🔹 PATCH-09: Corrección de Sintaxis en la UI de Administración (Líneas 735, 746 y 843)
* **Ubicación:** `packages/api/src/admin_ui.rs`
* **Problema:** El bloque `<script>` contenía comentarios de estilo Python/Shell (`# ...`) en varias funciones de JavaScript:
  - Línea 735: `# Ocultar la clave maestra del .env - no mostrar su valor por seguridad`
  - Línea 746: `# Expiration status`
  - Línea 843: `# Skip master key from tester selector`
  Esto provocaba en el navegador el error en consola: `admin:733 Uncaught SyntaxError: Invalid or unexpected token` al intentar ejecutar el login o renderizar la tabla de claves.
* **Solución:** Se sustituyeron todos los numerales (`#`) por la sintaxis estándar de comentarios en JavaScript (`//`), restaurando la ejecución sin errores de los scripts del panel administrativo.

---

### 🔹 PATCH-10: Rediseño de KPIs Operativos y Diagnóstico de Dominios Problemáticos
* **Ubicación:** `packages/shared/src/lib.rs`, `packages/api/src/main.rs`, `packages/api/src/admin_ui.rs`
* **Problema:** El sistema anterior medía la "tasa de entregabilidad" de los correos subidos por los clientes (`deliverability_rate`). Ese indicador penalizaba la métrica si el cliente enviaba una lista de mala calidad o correos falsos, confundiendo la calidad de los datos del cliente con la salud de la API. Además, el operador no disponía de visibilidad sobre los dominios mal escritos que los usuarios enviaban repetidamente (ej: typos como `gmial.com`, `hotmial.com` o dominios sin servidor MX).
* **Solución:**
  1. **Rediseño de KPIs de Salud de la API:**
     - **Peticiones Totales:** Volumen real de tráfico HTTP recibido.
     - **Tasa de Éxito Operativo de la API (`api_success_rate`):** Porcentaje de llamadas que la API procesó y respondió con éxito (sin errores 500 de servidor).
     - **Peticiones Procesadas Exitosamente:** Solicitudes que recibieron un diagnóstico estructurado.
     - **Métricas de Control:** Desglose de errores de sintaxis del cliente, dominios sin MX, fallos de DNS externos, bloqueos por Rate Limit (429) y rechazos por autenticación (401).
  2. **Componente `DomainTracker`:**
     - Rastreador thread-safe en memoria que contabiliza dominios problemáticos recurrentes, su frecuencia de envío, categoría (`sintaxis`, `sin_mx`, `dns_fallo`) y el motivo exacto del rechazo.
  3. **Visualización en el Dashboard:**
     - Panel administrativo rediseñado con tarjetas KPI operativas y la nueva tabla **"🔍 Dominios Recurrentes Problemáticos (Top Errores de Clientes)"** con ranking, etiquetas de categoría y frecuencia.

---

### 🔹 PATCH-11: Estandarización de Código de Error Numérico (`error_code`) para Mapping de Clientes
* **Ubicación:** `packages/shared/src/lib.rs`, `packages/dns/src/lib.rs`, `packages/validation/src/lib.rs`, `packages/api/src/main.rs`, `docs/API_REFERENCE.md`, `docs/ERROR_CODES.md`
* **Problema:** Los clientes de la API tenían que realizar parsing de cadenas de texto en el campo `reason` o `error` para determinar la causa del fallo (ej. si era un error de sintaxis, falta de servidor MX, o un bloqueo de autenticación). Esto provocaba integraciones frágiles y acopladas al lenguaje del mensaje. Además, existía riesgo de inconsistencia si se mantenían nombres redundantes o bilingües.
* **Solución:**
  1. **Campo `error_code` Único y Consistente:** Se unificó bajo un único campo entero estandarizado `error_code` en todas las respuestas JSON (tanto en respuestas `200 OK` de validación como en errores HTTP `401`, `429`, `413`, `500`), eliminando cualquier campo duplicado.
  2. **Catálogo de Errores Numéricos (`shared::error_codes`):**
     - `0` (`ERR_SUCCESS`): Correo válido y entregable.
     - `1` (`ERR_UNKNOWN`): Error de servidor o desconocido.
     - `2` (`ERR_INVALID_SYNTAX`): Sintaxis de correo inválida (RFC 5321/5322).
     - `3` (`ERR_NO_MX_RECORDS`): Dominio sin servidores de correo ni host accesible.
     - `4` (`ERR_NULL_MX_REJECTED`): Dominio con Null MX explícito (RFC 7505).
     - `5` (`ERR_DNS_ERROR`): Fallo o timeout al consultar DNS externo.
     - `6` (`ERR_UNAUTHORIZED`): API Key faltante, inválida o expirada.
     - `7` (`ERR_RATE_LIMITED`): Límite de peticiones por segundo excedido.
     - `8` (`ERR_PAYLOAD_TOO_LARGE`): Carga útil superior a 2KB.
  3. **Documentación:** Se incorporó la matriz de códigos y ejemplos prácticos de mapping (ej. `switch (data.error_code)`) en [`docs/API_REFERENCE.md`](file:///c:/Users/Gfran/Desktop/Email-Validador/docs/API_REFERENCE.md) y [`docs/ERROR_CODES.md`](file:///c:/Users/Gfran/Desktop/Email-Validador/docs/ERROR_CODES.md).
