# 🏗️ Arquitectura y Documentación Técnica - Email Validator API v2.0.0

> **Documentación completa con diagramas Mermaid, flujos de datos, arquitectura de componentes y especificaciones técnicas.**

---

## 📑 Índice

1. [Visión General de Arquitectura](#-visión-general-de-arquitectura)
2. [Diagrama de Componentes](#-diagrama-de-componentes)
3. [Flujo de Validación Completo](#-flujo-de-validación-completo)
4. [Transacción SMTP Detallada](#-transacción-smtp-detallada)
4. [Pipeline de Procesamiento](#-pipeline-de-procesamiento)
5. [Arquitectura de Cache y Pool](#-arquitectura-de-cache-y-pool)
6. [Modelos de Datos](#-modelos-de-datos)
7. [Manejo de Errores y Códigos](#-manejo-de-errores-y-códigos)
8. [Despliegue y Operación](#-despliegue-y-operación)
9. [Métricas y Observabilidad](#-métricas-y-observabilidad)
10. [Seguridad](#-seguridad)

---

## 🎯 Visión General de Arquitectura

El **Email Validator API v2.0.0** es un microservicio REST de alto rendimiento desarrollado en **Rust** que valida correos electrónicos mediante tres niveles de verificación:

```mermaid
graph TB
    subgraph "Cliente"
        Client[📱 Cliente HTTP<br/>cURL / Postman / SDK]
    end
    
    subgraph "API Gateway"
        Router[🔀 Axum Router<br/>/health /api/v1/*]
        Auth[🔐 Middleware Auth<br/>X-API-Key + Rate Limit]
    end
    
    subgraph "Validation Pipeline"
        Syntax[📝 Syntax Validator<br/>RFC 5321/5322]
        DNS[🌐 DNS Resolver<br/>MX + A/AAAA Fallback]
        SMTP[📧 SMTP Verifier<br/>RCPT TO Real]
    end
    
    subgraph "Infrastructure"
        Cache[(🗄️ MX Cache LRU<br/>TTL 1h)]
        Pool[(🏊 SMTP Pool<br/>Reuse Connections)]
        Metrics[📊 Metrics Collector<br/>Atomic Counters]
    end
    
    Client -->|POST /api/v1/validate-email| Router
    Router --> Auth
    Auth -->|Valid Key| Syntax
    Syntax -->|Valid Syntax| DNS
    DNS -->|MX Found| SMTP
    SMTP -->|SMTP_ENABLED| Cache
    SMTP --> Pool
    Cache --> SMTP
    Pool --> SMTP
    Auth -.->|Invalid Key| Metrics
    Syntax -.->|Invalid Syntax| Metrics
    DNS -.->|No MX| Metrics
    SMTP -.->|Result| Metrics
    
    Syntax -->|Response| Client
    DNS -->|Response| Client
    SMTP -->|Response| Client
    
    style Client fill:#e1f5fe
    style Router fill:#fff3e0
    style Auth fill:#fff3e0
    style Syntax fill:#e8f5e9
    style DNS fill:#e8f5e9
    style SMTP fill:#e8f5e9
    style Cache fill:#fce4ec
    style Pool fill:#fce4ec
    style Metrics fill:#f3e5f5
```

### Capas de Verificación

| Nivel | Descripción | RFC | Habilitado por Defecto |
|-------|-------------|-----|------------------------|
| **1. Sintaxis** | Validación RFC 5321/5322 completa | 5321, 5322 | ✅ Siempre |
| **2. DNS MX** | Resolución MX + Fallback A/AAAA + Null MX | 5321, 7505 | ✅ Siempre |
| **3. SMTP Real** | Conexión TCP + EHLO + STARTTLS + RCPT TO | 5321 | ⚙️ `SMTP_VERIFICATION_ENABLED` |

---

## 🧩 Diagrama de Componentes

### Cargo Workspace Structure

```mermaid
graph LR
    subgraph "Workspace Root"
        WS[Cargo.toml<br/>workspace.members]
    end
    
    subgraph "Crates"
        Shared[shared<br/>Models, Errors, Config]
        Validation[validation<br/>Syntax RFC 5321/5322]
        DNS[dns<br/>MX Resolution]
        SMTP[smtp<br/>SMTP Verification]
        API[api<br/>Axum HTTP Server]
    end
    
    WS --> Shared
    WS --> Validation
    WS --> DNS
    WS --> SMTP
    WS --> API
    
    Validation -.->|uses| Shared
    DNS -.->|uses| Shared
    SMTP -.->|uses| Shared
    API -.->|uses| Shared
    API -.->|uses| Validation
    API -.->|uses| DNS
    API -.->|uses| SMTP
    
    style Shared fill:#e3f2fd
    style Validation fill:#e8f5e9
    style DNS fill:#e8f5e9
    style SMTP fill:#e8f5e9
    style API fill:#fff3e0
```

### Dependency Graph

```mermaid
graph TD
    API[api<br/>email-validator] -->|depends on| Shared[shared]
    API --> Validation[validation]
    API --> DNS[dns]
    API --> SMTP[smtp]
    
    Validation --> Shared
    DNS --> Shared
    SMTP --> Shared
    SMTP -->|dev| DNS
    
    Shared -->|deps| Serde[serde/json]
    Shared -->|deps| ThisError[thiserror]
    Shared -->|deps| Chrono[chrono]
    Shared -->|deps| SHA2[sha2]
    Shared -->|deps| Base64[base64]
    Shared -->|deps| Rand[rand]
    
    Validation -->|deps| Shared
    DNS -->|deps| Shared
    DNS -->|deps| Hickory[hickory-resolver]
    SMTP -->|deps| Shared
    SMTP -->|deps| DNS
    SMTP -->|deps| RustLS[rustls/tokio-rustls]
    SMTP -->|deps| WebPKI[webpki-roots]
    
    API -->|deps| Axum[axum 0.7]
    API -->|deps| Tower[tower-http]
    API -->|deps| Tokio[tokio]
    API -->|deps| Tracing[tracing]
```

---

## 🔄 Flujo de Validación Completo

### Sequence Diagram - Request Processing

```mermaid
sequenceDiagram
    participant Client
    participant Router
    participant Auth
    participant RateLimiter
    participant SyntaxValidator
    participant DNSResolver
    participant Cache
    participant SMTPVerifier
    participant Pool
    participant Metrics
    
    Client->>Router: POST /api/v1/validate-email<br/>{"email": "user@domain.com"}<br/>X-API-Key: xxx
    Router->>Metrics: record_request()
    Router->>Auth: Validate X-API-Key
    
    alt Invalid API Key
        Auth-->>Client: 401 Unauthorized<br/>{error, error_code: 6}
        Metrics->>Metrics: record_auth_failure()
    else Valid API Key
        Auth->>RateLimiter: check(api_key)
        
        alt Rate Limited
            RateLimiter-->>Client: 429 Too Many Requests<br/>{error, error_code: 7}
            Metrics->>Metrics: record_rate_limit_hit()
        else OK
            RateLimiter->>KeyManager: increment_usage()
            RateLimiter->>SyntaxValidator: validate_email_syntax(email)
            
            alt Invalid Syntax
                SyntaxValidator-->>Client: 200 OK<br/>{is_valid_syntax: false, error_code: 2}
                Metrics->>Metrics: record_syntax_error()
                DomainTracker->>DomainTracker: record(domain, "syntax")
            else Valid Syntax
                SyntaxValidator->>DNSResolver: resolve_mx_records(domain)
                
                alt DNS Error
                    DNSResolver-->>Client: 200 OK<br/>{error_code: 5, has_mx: false}
                    Metrics->>Metrics: record_dns_error()
                else Null MX (RFC 7505)
                    DNSResolver-->>Client: 200 OK<br/>{error_code: 4, has_mx: false}
                else No MX Records
                    DNSResolver-->>Client: 200 OK<br/>{error_code: 3, has_mx: false}
                else MX Found
                    DNSResolver->>Cache: get(domain)
                    
                    alt Cache Miss
                        Cache->>DNSResolver: resolve fresh
                        DNSResolver-->>Cache: MX records
                        Cache->>Cache: store with TTL
                    else Cache Hit
                        Cache-->>SMTPVerifier: cached MX records
                    end
                    
                    alt SMTP_VERIFICATION_ENABLED = false
                        SMTPVerifier-->>Client: 200 OK<br/>{is_deliverable: true, has_mx: true}
                    else SMTP_VERIFICATION_ENABLED = true
                        SMTPVerifier->>SMTPVerifier: sort MX by priority
                        
                        loop For each MX (priority order)
                            SMTPVerifier->>Pool: get_connection(host, port)
                            
                            alt Connection from Pool
                                Pool-->>SMTPVerifier: Reused connection
                            else New Connection
                                Pool->>Pool: Create TCP/TLS connection
                                Pool-->>SMTPVerifier: New connection
                            end
                            
                            SMTPVerifier->>SMTPVerifier: SMTP Transaction
                            
                            alt Deliverable (250)
                                SMTPVerifier-->>Client: 200 OK<br/>{is_deliverable: true, smtp: Deliverable}
                                break
                            else Permanent Failure (550,551,552,553,554)
                                SMTPVerifier-->>Client: 200 OK<br/>{is_deliverable: false, error_code: 9}
                                break
                            else Temporary Failure (450,451,452)
                                SMTPVerifier->>SMTPVerifier: Retry with backoff
                                
                                alt Max Retries Exceeded
                                    SMTPVerifier-->>Client: 200 OK<br/>{error_code: 11}
                                end
                            else Connection Error
                                SMTPVerifier->>SMTPVerifier: Try next MX
                            end
                        end
                        
                        SMTPVerifier->>Pool: return_connection()
                        Pool->>Pool: Mark idle, update last_used
                    end
                    
                    Metrics->>Metrics: record_processed(has_mx, latency)
                    DomainTracker->>DomainTracker: record if needed
            end
        end
    end
```

### Decision Flow - Validation Logic

```mermaid
flowchart TD
    Start([START<br/>POST /validate-email]) --> Auth{API Key<br/>Válida?}
    Auth -->|No| Err401[401 Unauthorized<br/>error_code: 6]
    Auth -->|Sí| RateLimit{Rate Limit<br/>OK?}
    RateLimit -->|No| Err429[429 Too Many Requests<br/>error_code: 7]
    RateLimit -->|Sí| Syntax{Sintaxis<br/>Válida?}
    
    Syntax -->|No| RespSyntax[200 OK<br/>is_valid_syntax: false<br/>error_code: 2]
    Syntax -->|Sí| DNS{DNS MX<br/>Resuelto?}
    
    DNS -->|Error DNS| RespDNSErr[200 OK<br/>error_code: 5<br/>has_mx: false]
    DNS -->|Null MX| RespNullMX[200 OK<br/>error_code: 4<br/>has_mx: false]
    DNS -->|Sin MX| RespNoMX[200 OK<br/>error_code: 3<br/>has_mx: false]
    DNS -->|MX Encontrados| SMTPEnabled{SMTP<br/>Habilitado?}
    
    SMTPEnabled -->|No| RespMXOnly[200 OK<br/>is_deliverable: true<br/>has_mx: true<br/>smtp_verification: null]
    SMTPEnabled -->|Sí| SMTPVerify{Verificar<br/>RCPT TO}
    
    SMTPVerify -->|Catch-All| RespCatchAll[200 OK<br/>is_deliverable: false<br/>error_code: 10<br/>is_catch_all: true]
    SMTPVerify -->|Deliverable 250| RespDeliverable[200 OK<br/>is_deliverable: true<br/>smtp: Deliverable<br/>error_code: 0]
    SMTPVerify -->|Undeliverable 5xx| RespUndeliverable[200 OK<br/>is_deliverable: false<br/>error_code: 9<br/>smtp_detail: código 5xx]
    SMTPVerify -->|Temp Failure 4xx| Retry{Retries<br/>Disponibles?}
    Retry -->|Sí| Backoff[Backoff Exponencial<br/>5s → 10s → 20s]
    Backoff --> SMTPVerify
    Retry -->|No| RespTempFail[200 OK<br/>is_deliverable: false<br/>error_code: 11]
    SMTPVerify -->|Connection Error| NextMX{Más MX<br/>Disponibles?}
    NextMX -->|Sí| SMTPVerify
    NextMX -->|No| RespConnErr[200 OK<br/>is_deliverable: false<br/>error_code: 12]
    
    style Start fill:#e1f5fe
    style Err401 fill:#ffcdd2
    style Err429 fill:#ffcdd2
    style RespSyntax fill:#fff9c4
    style RespDNSErr fill:#fff9c4
    style RespNullMX fill:#fff9c4
    style RespNoMX fill:#fff9c4
    style RespMXOnly fill:#c8e6c9
    style RespCatchAll fill:#fff9c4
    style RespDeliverable fill:#c8e6c9
    style RespUndeliverable fill:#ffcdd2
    style RespTempFail fill:#fff9c4
    style RespConnErr fill:#fff9c4
```

---

## 📧 Transacción SMTP Detallada

### SMTP Protocol Flow

```mermaid
sequenceDiagram
    participant Client as Email Validator
    participant Server as MX Server
    
    Note over Client,Server: 1. TCP Connection
    Client->>Server: TCP SYN → SYN/ACK → ACK
    
    Note over Client,Server: 2. Banner (220)
    Server->>Client: 220 mx.example.com ESMTP Ready
    
    Note over Client,Server: 3. EHLO
    Client->>Server: EHLO email-validator.local
    Server->>Client: 250-mx.example.com Hello
    Server->>Client: 250-SIZE 35882577
    Server->>Client: 250-PIPELINING
    Server->>Client: 250-SMTPUTF8
    Server->>Client: 250 STARTTLS
    
    Note over Client,Server: 4. STARTTLS (puerto 587)
    alt Puerto 587 con STARTTLS
        Client->>Server: STARTTLS
        Server->>Client: 220 2.0.0 Ready to start TLS
        Client->>Server: TLS Handshake (ClientHello...)
        Server->>Client: ServerHello + Certificate
        Client->>Server: Certificate Verify + Finished
        Server->>Client: Finished
        Note over Client,Server: Canal encriptado establecido
    end
    
    Note over Client,Server: 5. MAIL FROM
    Client->>Server: MAIL FROM:<>
    Server->>Client: 250 2.1.0 OK
    
    Note over Client,Server: 6. RCPT TO (VERIFICACIÓN CLAVE)
    Client->>Server: RCPT TO:<user@domain.com>
    
    alt Buzón Existe (250)
        Server->>Client: 250 2.1.5 OK
        Note over Client,Server: ✅ DELIVERABLE
    else Buzón No Existe (550 5.1.1)
        Server->>Client: 550 5.1.1 User unknown
        Note over Client,Server: ❌ UNDELIVERABLE
    else Buzón Lleno (552 5.2.2)
        Server->>Client: 552 5.2.2 Mailbox full
        Note over Client,Server: ❌ MAILBOX FULL
    else Política/Spam (554 5.7.1)
        Server->>Client: 554 5.7.1 Rejected
        Note over Client,Server: ❌ REJECTED BY POLICY
    else Greylisting (450 4.7.1)
        Server->>Client: 450 4.7.1 Try again later
        Note over Client,Server: ⚠️ TEMPORARY FAILURE
    else Catch-All (250 para cualquier RCPT)
        Server->>Client: 250 2.1.5 OK
        Note over Client,Server: 🔄 CATCH-ALL DETECTADO
    end
    
    Note over Client,Server: 7. QUIT
    Client->>Server: QUIT
    Server->>Client: 221 2.0.0 Bye
    
    Note over Client,Server: 8. Connection Close
    Client->>Server: TCP FIN → FIN/ACK → ACK
```

### Port Fallback Strategy

```mermaid
flowchart TD
    Start([MX Records<br/>Resueltos]) --> Sort[Ordenar por<br/>Prioridad MX]
    Sort --> Loop{Para cada MX<br/>en orden}
    
    Loop --> Try25[Puerto 25<br/>Sin TLS]
    Try25 --> Result25{Resultado}
    Result25 -->|Éxito| Done([Retornar Resultado])
    Result25 -->|Fallo Conexión| Try587[Puerto 587<br/>STARTTLS]
    Result25 -->|Error TLS| Try587
    
    Try587 --> Result587{Resultado}
    Result587 -->|Éxito| Done
    Result587 -->|Fallo Conexión| Try465[Puerto 465<br/>SMTPS Implícito]
    Result587 -->|Sin STARTTLS| Try465
    
    Try465 --> Result465{Resultado}
    Result465 -->|Éxito| Done
    Result465 -->|Fallo| NextMX{Más MX<br/>Disponibles?}
    
    NextMX -->|Sí| Loop
    NextMX -->|No| Failed([Connection Error<br/>error_code: 12])
    
    style Start fill:#e1f5fe
    style Done fill:#c8e6c9
    style Failed fill:#ffcdd2
    style Try25 fill:#fff3e0
    style Try587 fill:#fff3e0
    style Try465 fill:#fff3e0
```

### SMTP Response Code Mapping

```mermaid
graph LR
    subgraph "Códigos 2xx - Éxito"
        Code250[250 OK<br/>→ Deliverable]
        Code251[251 User not local<br/>→ Deliverable]
        Code252[252 Cannot verify<br/>→ Deliverable]
    end
    
    subgraph "Códigos 4xx - Temporal"
        Code450[450 Mailbox unavailable<br/>→ TemporaryFailure]
        Code451[451 Local error<br/>→ TemporaryFailure]
        Code452[452 Insufficient storage<br/>→ TemporaryFailure]
    end
    
    subgraph "Códigos 5xx - Permanente"
        Code550[550 5.1.1 User unknown<br/>→ Undeliverable]
        Code551[551 User not local<br/>→ Undeliverable]
        Code552[552 5.2.2 Mailbox full<br/>→ MailboxFull]
        Code553[553 Mailbox name invalid<br/>→ Undeliverable]
        Code554[554 5.7.1 Policy rejection<br/>→ RejectedByPolicy]
    end
    
    subgraph "Casos Especiales"
        NullMX[Null MX<br/>RFC 7505<br/>→ NullMx]
        CatchAll[Catch-All<br/>250 para todo<br/>→ CatchAll]
    end
    
    style Code250 fill:#c8e6c9
    style Code251 fill:#c8e6c9
    style Code252 fill:#c8e6c9
    style Code450 fill:#fff9c4
    style Code451 fill:#fff9c4
    style Code452 fill:#fff9c4
    style Code550 fill:#ffcdd2
    style Code551 fill:#ffcdd2
    style Code552 fill:#ffcdd2
    style Code553 fill:#ffcdd2
    style Code554 fill:#ffcdd2
    style NullMX fill:#ffcdd2
    style CatchAll fill:#fff9c4
```

---

## ⚙️ Pipeline de Procesamiento

### Pipeline Architecture

```mermaid
graph LR
    subgraph "Input"
        HTTP[HTTP Request<br/>POST /validate-email]
    end
    
    subgraph "Middleware Layer"
        MW1[Body Limit<br/>2KB]
        MW2[CORS]
        MW3[Auth Middleware<br/>X-API-Key]
        MW4[Rate Limiter<br/>QPS per Key]
    end
    
    subgraph "Validation Pipeline"
        V1[Syntax Validator<br/>RFC 5321/5322]
        V2[DNS MX Resolver<br/>hickory-resolver]
        V3[SMTP Verifier<br/>RCPT TO]
    end
    
    subgraph "Cache & Pool"
        C1[MX Cache<br/>LRU + TTL]
        C2[SMTP Pool<br/>Connection Reuse]
    end
    
    subgraph "Observability"
        O1[Metrics<br/>Atomic Counters]
        O2[Domain Tracker<br/>Problematic Domains]
        O3[Log Buffer<br/>In-Memory]
    end
    
    subgraph "Output"
        OUT[JSON Response<br/>EmailValidationResponse]
    end
    
    HTTP --> MW1 --> MW2 --> MW3 --> MW4
    MW4 --> V1
    V1 -->|Invalid| OUT
    V1 -->|Valid| V2
    V2 -->|No MX / Error| OUT
    V2 -->|MX Found| C1
    C1 -->|Miss| V2
    C1 -->|Hit| V3
    V3 -->|SMTP Disabled| OUT
    V3 -->|SMTP Enabled| C2
    C2 --> V3
    V3 --> OUT
    
    MW3 -.-> O1
    MW4 -.-> O1
    V1 -.-> O1
    V2 -.-> O1
    V3 -.-> O1
    V1 -.-> O2
    V2 -.-> O2
    V3 -.-> O2
    
    style HTTP fill:#e1f5fe
    style MW1 fill:#fff3e0
    style MW2 fill:#fff3e0
    style MW3 fill:#fff3e0
    style MW4 fill:#fff3e0
    style V1 fill:#e8f5e9
    style V2 fill:#e8f5e9
    style V3 fill:#e8f5e9
    style C1 fill:#fce4ec
    style C2 fill:#fce4ec
    style O1 fill:#f3e5f5
    style O2 fill:#f3e5f5
    style O3 fill:#f3e5f5
    style OUT fill:#e1f5fe
```

### Processing Stages Detail

```mermaid
flowchart TD
    subgraph "Stage 1: Authentication & Rate Limiting"
        S1A[Extract X-API-Key<br/>from Header]
        S1B[KeyManager.validate_key<br/>Active + Not Expired]
        S1C[RateLimiter.check<br/>QPS per Key]
        S1D[KeyManager.increment_usage]
    end
    
    subgraph "Stage 2: Syntax Validation"
        S2A[validate_email_syntax<br/>RFC 5321/5322]
        S2B[Length Checks<br/>Total ≤254, Local ≤64, Domain ≤253]
        S2C[Local Part Rules<br/>No leading/trailing dots, no ..]
        S2D[Domain Rules<br/>Labels ≤63, no leading/trailing hyphens]
        S2E[TLD Validation<br/>>=2 chars, not all-numeric]
    end
    
    subgraph "Stage 3: DNS Resolution"
        S3A[hickory-resolver<br/>MX Lookup]
        S3B[Null MX Check<br/>RFC 7505: pref=0, exchange=.<br/>]
        S3C[MX Cache Lookup<br/>LRU + TTL]
        S3D[Fallback A/AAAA<br/>RFC 5321 §5.1]
        S3E[Sort by Priority<br/>Lower = Higher Priority]
    end
    
    subgraph "Stage 4: SMTP Verification"
        S4A[SMTP_VERIFICATION_ENABLED<br/>Env Var Check]
        S4B[Connection Pool<br/>Get/Reuse Connection]
        S4C[TCP Connect<br/>Port 25/587/465]
        S4D[EHLO + Capabilities<br/>Parse STARTTLS, SIZE, etc.]
        S4E[STARTTLS Upgrade<br/>Port 587]
        S4F[MAIL FROM:<>]
        S4G[RCPT TO:<email><br/>KEY VALIDATION STEP]
        S4H[Parse Response Code<br/>250/450/550/551/552/553/554]
        S4I[Catch-All Detection<br/>Random email test]
        S4J[Retry Logic<br/>Backoff on 4xx]
        S4K[Return to Pool<br/>Mark Idle]
    end
    
    S1A --> S1B --> S1C --> S1D
    S1D --> S2A --> S2B --> S2C --> S2D --> S2E
    S2E --> S3A --> S3B --> S3C --> S3D --> S3E
    S3E --> S4A
    S4A -->|Disabled| OUT1[Return MX-only result]
    S4A -->|Enabled| S4B --> S4C --> S4D --> S4E --> S4F --> S4G --> S4H --> S4I --> S4J --> S4K
    S4K --> OUT2[Return Full result]
```

---

## 🗄️ Arquitectura de Cache y Pool

### MX Cache (LRU con TTL)

```mermaid
classDiagram
    class MxCache {
        -cache: HashMap<String, MxCacheEntry>
        -ttl: Duration
        -max_entries: usize
        +new(ttl, max_entries) MxCache
        +get(domain) Option<Vec<MxRecord>>
        +set(domain, records)
        +cleanup_expired()
        +stats() MxCacheStats
    }
    
    class MxCacheEntry {
        +records: Vec<MxRecord>
        +created_at: Instant
    }
    
    class MxCacheStats {
        +total_entries: usize
        +expired_entries: usize
        +hit_rate: f64
    }
    
    MxCache --> MxCacheEntry
    MxCache --> MxCacheStats
```

### MX Cache Flow

```mermaid
flowchart TD
    Request[DNS MX Request<br/>domain.com] --> CheckCache{Cache<br/>Hit?}
    
    CheckCache -->|Hit + Not Expired| ReturnCached[Return Cached<br/>MX Records]
    CheckCache -->|Miss or Expired| QueryDNS[Query DNS<br/>hickory-resolver]
    
    QueryDNS --> Result{DNS<br/>Success?}
    Result -->|No| ReturnEmpty[Return Empty/Error]
    Result -->|Yes| StoreCache[Store in Cache<br/>with Timestamp]
    
    StoreCache --> CheckSize{Cache Size<br/>>> Max?}
    CheckSize -->|Yes| EvictLRU[Evict Oldest<br/>LRU Entry]
    CheckSize -->|No| ReturnFresh[Return Fresh<br/>MX Records]
    
    EvictLRU --> ReturnFresh
    
    style Request fill:#e1f5fe
    style ReturnCached fill:#c8e6c9
    style ReturnFresh fill:#c8e6c9
    style ReturnEmpty fill:#fff9c4
```

### SMTP Connection Pool

```mermaid
classDiagram
    class SmtpConnectionPool {
        -pools: HashMap<String, Vec<SmtpPooledConnection>>
        -max_per_host: usize
        -max_idle_time: Duration
        +new(max_per_host, max_idle_time) SmtpConnectionPool
        +get_connection(host, port) Option<SmtpPooledConnection>
        +return_connection(host, port)
        +cleanup_idle()
        +stats() SmtpPoolStats
    }
    
    class SmtpPooledConnection {
        +host: String
        +port: u16
        +created_at: Instant
        +last_used: Instant
        +in_use: bool
    }
    
    class SmtpPoolStats {
        +total_pools: usize
        +total_connections: usize
        +active_connections: usize
        +idle_connections: usize
    }
    
    SmtpConnectionPool --> SmtpPooledConnection
    SmtpConnectionPool --> SmtpPoolStats
```

### Pool Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Created: Pool Created
    
    Created --> Idle: Connection Created
    Idle --> Active: get_connection()
    Active --> Idle: return_connection()
    
    Idle --> Expired: max_idle_time exceeded
    Expired --> [*]: Cleanup
    
    Active --> Error: Connection Error
    Error --> [*]: Discard
    
    state Active {
        [*] --> Connected
        Connected --> EHLO: Send EHLO
        EHLO --> STARTTLS: If Port 587
        STARTTLS --> MAIL: MAIL FROM:<>
        MAIL --> RCPT: RCPT TO:<email>
        RCPT --> QUIT: Send QUIT
        QUIT --> Connected: Return to Pool
    }
    
    note right of Idle
        Max 10 connections per host
        Idle timeout: 5 min (configurable)
    end note
```

---

## 📊 Modelos de Datos

### Core Data Structures

```mermaid
classDiagram
    class EmailValidationResponse {
        +email: String
        +is_valid_syntax: bool
        +has_mx_records: bool
        +is_deliverable: bool
        +error_code: u32
        +reason: String
        +smtp_verification: Option<String>
        +smtp_detail: Option<String>
        +is_catch_all: Option<bool>
        +is_disposable: Option<bool>
        +is_role_based: Option<bool>
    }
    
    class SmtpVerificationResult {
        <<enumeration>>
        Deliverable
        Undeliverable
        MailboxFull
        RejectedByPolicy
        TemporaryFailure
        CatchAll
        ConnectionError
        NoMxRecords
        NullMx
    }
    
    class MxRecord {
        +host: String
        +priority: u16
        +port: u16
        +use_tls: bool
    }
    
    class SmtpConfig {
        +total_timeout: Duration
        +connect_timeout: Duration
        +command_timeout: Duration
        +max_retries: u32
        +retry_backoff_base: Duration
        +helo_domain: String
        +mail_from: String
        +primary_port: u16
        +submission_port: u16
        +smtps_port: u16
        +danger_accept_invalid_certs: bool
        +enable_catch_all_detection: bool
    }
    
    class ValidationError {
        <<enumeration>>
        InvalidEmail(String)
        InvalidApiKey
        ExpiredApiKey
        RateLimitExceeded
        PayloadTooLarge
        DnsError(String)
        InternalError(String)
    }
    
    class ApiKeyRecord {
        +id: String
        +key: String
        +name: String
        +is_active: bool
        +created_at: String
        +expires_at: Option<String>
        +total_requests: u64
    }
    
    class MetricsSnapshot {
        +total_requests: u64
        +processed_validations: u64
        +api_success_rate: f64
        +average_latency_ms: f64
        +auth_failures: u64
        +rate_limit_hits: u64
        +syntax_errors: u64
        +no_mx_count: u64
        +dns_errors: u64
        +uptime_seconds: u64
        +top_problematic_domains: Vec<ProblematicDomainEntry>
    }
    
    EmailValidationResponse --> SmtpVerificationResult
    SmtpConfig --> MxRecord
```

### Response Examples by Scenario

```mermaid
graph TD
    subgraph "Syntax Error (error_code: 2)"
        SE[{
  "email": "user@.com",
  "is_valid_syntax": false,
  "has_mx_records": false,
  "is_deliverable": false,
  "error_code": 2,
  "reason": "El dominio no puede comenzar ni terminar con un punto"
}]
    end
    
    subgraph "No MX Records (error_code: 3)"
        NMX[{
  "email": "user@nonexistent123.com",
  "is_valid_syntax": true,
  "has_mx_records": false,
  "is_deliverable": false,
  "error_code": 3,
  "reason": "El dominio no posee servidores de correo..."
}]
    end
    
    subgraph "Null MX (error_code: 4)"
        NUL[{
  "email": "user@nullmx.example.com",
  "is_valid_syntax": true,
  "has_mx_records": false,
  "is_deliverable": false,
  "error_code": 4,
  "reason": "El dominio declara explícitamente que no acepta correo..."
}]
    end
    
    subgraph "MX Only - No SMTP (error_code: 0)"
        MXO[{
  "email": "user@gmail.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": true,
  "error_code": 0,
  "reason": "Correo válido y con registros MX encontrados",
  "smtp_verification": null
}]
    end
    
    subgraph "SMTP Deliverable (error_code: 0)"
        SMD[{
  "email": "user@gmail.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": true,
  "error_code": 0,
  "reason": "Correo válido y buzón confirmado entregable...",
  "smtp_verification": "Deliverable",
  "smtp_detail": "Correo válido y buzón confirmado...",
  "is_catch_all": false,
  "is_disposable": false,
  "is_role_based": false
}]
    end
    
    subgraph "SMTP Undeliverable (error_code: 9)"
        SMU[{
  "email": "fake123@gmail.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": false,
  "error_code": 9,
  "reason": "Buzón inexistente (SMTP 550 5.1.1)",
  "smtp_verification": "Undeliverable",
  "smtp_detail": "550 5.1.1 The email account...does not exist",
  "is_catch_all": false
}]
    end
    
    subgraph "Catch-All (error_code: 10)"
        CAT[{
  "email": "random@catchall.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": false,
  "error_code": 10,
  "reason": "Dominio catch-all - acepta cualquier destinatario...",
  "smtp_verification": "CatchAll",
  "is_catch_all": true
}]
    end
    
    subgraph "Temporary Failure (error_code: 11)"
        TMP[{
  "email": "user@greylist.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": false,
  "error_code": 11,
  "reason": "Fallo temporal - greylisting o servidor ocupado...",
  "smtp_verification": "TemporaryFailure"
}]
    end
    
    subgraph "Disposable Email (error_code: 13)"
        DISP[{
  "email": "test@temp-mail.org",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": false,
  "error_code": 13,
  "reason": "Email desechable detectado (dominio temporal conocido)",
  "is_disposable": true
}]
    end
    
    subgraph "Role-Based Email (error_code: 14)"
        ROLE[{
  "email": "admin@example.com",
  "is_valid_syntax": true,
  "has_mx_records": true,
  "is_deliverable": true,
  "error_code": 0,
  "reason": "Correo válido y buzón confirmado entregable...",
  "smtp_verification": "Deliverable",
  "is_role_based": true
}]
    end
```

---

## ⚠️ Manejo de Errores y Códigos

### Error Code Registry

```mermaid
graph TD
    subgraph "Success"
        EC0[0 - ERR_SUCCESS<br/>✅ Válido y entregable]
    end
    
    subgraph "Server Errors"
        EC1[1 - ERR_UNKNOWN<br/>🔴 Error interno no clasificado]
    end
    
    subgraph "Validation Errors"
        EC2[2 - ERR_INVALID_SYNTAX<br/>🟡 Sintaxis RFC inválida]
        EC3[3 - ERR_NO_MX_RECORDS<br/>🟡 Sin servidores MX]
        EC4[4 - ERR_NULL_MX_REJECTED<br/>🟡 Null MX RFC 7505]
        EC13[13 - ERR_DISPOSABLE_EMAIL<br/>🟡 Email desechable]
        EC14[14 - WARN_ROLE_BASED_EMAIL<br/>🔵 Email role-based warning]
    end
    
    subgraph "Network/DNS Errors"
        EC5[5 - ERR_DNS_ERROR<br/>🔴 Timeout/Error DNS]
        EC12[12 - ERR_SMTP_CONNECTION_ERROR<br/>🔴 Error conexión SMTP]
    end
    
    subgraph "Auth/Rate Limit"
        EC6[6 - ERR_UNAUTHORIZED<br/>🔴 API Key inválida/expirada]
        EC7[7 - ERR_RATE_LIMITED<br/>🟡 Rate limit excedido]
        EC8[8 - ERR_PAYLOAD_TOO_LARGE<br/>🟡 Payload > 2KB]
    end
    
    subgraph "SMTP Errors"
        EC9[9 - ERR_SMTP_REJECTED<br/>🔴 Buzón rechazado 5xx]
        EC10[10 - ERR_CATCH_ALL<br/>🟡 Dominio catch-all]
        EC11[11 - ERR_SMTP_TEMPORARY_FAILURE<br/>🟡 Greylisting 4xx]
    end
    
    style EC0 fill:#c8e6c9
    style EC1 fill:#ffcdd2
    style EC2 fill:#fff9c4
    style EC3 fill:#fff9c4
    style EC4 fill:#fff9c4
    style EC5 fill:#ffcdd2
    style EC6 fill:#ffcdd2
    style EC7 fill:#fff9c4
    style EC8 fill:#fff9c4
    style EC9 fill:#ffcdd2
    style EC10 fill:#fff9c4
    style EC11 fill:#fff9c4
    style EC12 fill:#ffcdd2
    style EC13 fill:#fff9c4
    style EC14 fill:#bbdefb
```

### HTTP Status Code Mapping

| Error Code | Constante | HTTP Status | Category |
|------------|-----------|-------------|----------|
| 0 | ERR_SUCCESS | 200 OK | Success |
| 1 | ERR_UNKNOWN | 500 | Server Error |
| 2 | ERR_INVALID_SYNTAX | 200 OK / 400 | Validation |
| 3 | ERR_NO_MX_RECORDS | 200 OK | Validation |
| 4 | ERR_NULL_MX_REJECTED | 200 OK | Validation |
| 5 | ERR_DNS_ERROR | 200 OK / 502 | Network |
| 6 | ERR_UNAUTHORIZED | 401 | Auth |
| 7 | ERR_RATE_LIMITED | 429 | Rate Limit |
| 8 | ERR_PAYLOAD_TOO_LARGE | 413 | Validation |
| 9 | ERR_SMTP_REJECTED | 200 OK | SMTP |
| 10 | ERR_CATCH_ALL | 200 OK | SMTP |
| 11 | ERR_SMTP_TEMPORARY_FAILURE | 200 OK | SMTP |
| 12 | ERR_SMTP_CONNECTION_ERROR | 200 OK | Network |
| 13 | ERR_DISPOSABLE_EMAIL | 200 OK | Quality |
| 14 | WARN_ROLE_BASED_EMAIL | 200 OK | Quality |

---

## 🐳 Despliegue y Operación

### Docker Architecture

```mermaid
graph TB
    subgraph "Host Machine"
        Docker[Docker Engine]
    end
    
    subgraph "Container: email-validator-api"
        App[Email Validator Binary<br/>Rust Release Build]
        Config[.env Config<br/>Volume Mount]
        Data[/app/data<br/>Volume Mount]
    end
    
    subgraph "Network"
        Port3000[Port 3000<br/>Host ↔ Container]
        DNS[DNS Resolution<br/>Host Network/Container DNS]
    end
    
    subgraph "External Dependencies"
        MXServers[MX Servers<br/>Port 25/587/465]
        DNSServers[DNS Servers<br/>UDP 53]
    end
    
    Docker --> App
    App --> Config
    App --> Data
    App --> Port3000
    App --> DNS
    DNS --> DNSServers
    App -.->|SMTP_VERIFICATION_ENABLED| MXServers
    
    style App fill:#e1f5fe
    style Config fill:#fff3e0
    style Data fill:#fff3e0
    style MXServers fill:#fff9c4
    style DNSServers fill:#fff9c4
```

### Docker Compose Service Definition

```yaml
# docker-compose.yml
services:
  email-validator:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: email-validator-api
    ports:
      - "3000:3000"
    environment:
      - API_KEY=${API_KEY}
      - ADMIN_USER=${ADMIN_USER}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD}
      - ALLOWED_ORIGINS=${ALLOWED_ORIGINS}
      - PORT=3000
      - RATE_LIMIT_QPS=${RATE_LIMIT_QPS}
      - RUST_LOG=${RUST_LOG}
      # SMTP Configuration
      - SMTP_VERIFICATION_ENABLED=${SMTP_VERIFICATION_ENABLED:-false}
      - SMTP_CONNECT_TIMEOUT_SECS=${SMTP_CONNECT_TIMEOUT_SECS:-10}
      - SMTP_COMMAND_TIMEOUT_SECS=${SMTP_COMMAND_TIMEOUT_SECS:-10}
      - SMTP_MAX_RETRIES=${SMTP_MAX_RETRIES:-2}
    volumes:
      - ./data:/app/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s
    networks:
      - email-validator-net
    mem_limit: 256m
    cpus: "1.0"
```

### Multi-stage Dockerfile

```mermaid
flowchart TD
    subgraph "Build Stage"
        Base1[FROM rust:latest AS builder]
        Workdir1[WORKDIR /app]
        Install1[RUN apt-get update && install<br/>pkg-config libssl-dev]
        Copy1[COPY Cargo.toml packages/]
        Build1[RUN cargo build --release<br/>--bin email-validator]
    end
    
    subgraph "Runtime Stage"
        Base2[FROM debian:bookworm-slim]
        Workdir2[WORKDIR /app]
        Install2[RUN apt-get update && install<br/>ca-certificates curl<br/>useradd -m -u 1000 appuser]
        Copy2[COPY --from=builder /app/target/release/email-validator .]
        Perms2[RUN chmod +x ./email-validator<br/>chown -R appuser:appuser /app]
        User2[USER appuser]
        Env2[ENV DATA_DIR=/app/data]
        Expose2[EXPOSE 3000]
        Health2[HEALTHCHECK curl -f<br/>http://localhost:3000/health]
        Cmd2[CMD [./email-validator]]
    end
    
    Base1 --> Workdir1 --> Install1 --> Copy1 --> Build1
    Build1 --> Base2
    Base2 --> Workdir2 --> Install2 --> Copy2 --> Perms2 --> User2 --> Env2 --> Expose2 --> Health2 --> Cmd2
```

### Required Ports & Firewall Rules

```mermaid
graph LR
    subgraph "Inbound (Ingress)"
        HTTP[TCP 3000<br/>HTTP API<br/>From: Clients/Load Balancer]
        ADMIN[TCP 3000<br/>Admin Panel<br/>From: Admin IPs Only]
    end
    
    subgraph "Outbound (Egress)"
        DNS_UDP[UDP 53<br/>DNS Resolution<br/>To: DNS Servers]
        DNS_TCP[TCP 53<br/>DNS over TCP<br/>To: DNS Servers]
        SMTP_25[TCP 25<br/>SMTP Standard<br/>To: MX Servers]
        SMTP_587[TCP 587<br/>SMTP Submission/STARTTLS<br/>To: MX Servers]
        SMTP_465[TCP 465<br/>SMTPS Implicit TLS<br/>To: MX Servers]
    end
    
    style HTTP fill:#e1f5fe
    style ADMIN fill:#e1f5fe
    style DNS_UDP fill:#fff9c4
    style DNS_TCP fill:#fff9c4
    style SMTP_25 fill:#fff9c4
    style SMTP_587 fill:#fff9c4
    style SMTP_465 fill:#fff9c4
```

---

## 📈 Métricas y Observabilidad

### Metrics Architecture

```mermaid
graph TB
    subgraph "Collection"
        Atomic[AtomicMetrics<br/>Lock-free Counters]
        Domain[DomainTracker<br/>Problematic Domains]
        Logs[LogBuffer<br/>In-Memory Ring Buffer]
    end
    
    subgraph "Aggregation"
        Snapshot[MetricsSnapshot<br/>Computed on Demand]
    end
    
    subgraph "Exposure"
        AdminAPI[GET /api/v1/admin/metrics<br/>Basic Auth]
        Prometheus[Future: /metrics<br/>Prometheus Format]
        LogsAPI[GET /api/v1/admin/logs<br/>Recent Logs]
    end
    
    Atomic --> Snapshot
    Domain --> Snapshot
    Logs --> LogsAPI
    Snapshot --> AdminAPI
    Snapshot --> Prometheus
    
    style Atomic fill:#e8f5e9
    style Domain fill:#e8f5e9
    style Logs fill:#e8f5e9
    style Snapshot fill:#fff3e0
    style AdminAPI fill:#e1f5fe
    style Prometheus fill:#e1f5fe
    style LogsAPI fill:#e1f5fe
```

### Key Metrics

```mermaid
graph LR
    subgraph "Request Metrics"
        M1[total_requests<br/>Total HTTP requests received]
        M2[processed_validations<br/>Validations completed]
        M3[api_success_rate<br/>% requests without server errors]
        M4[average_latency_ms<br/>Mean processing time]
    end
    
    subgraph "Error Metrics"
        M5[auth_failures<br/>401 Unauthorized]
        M6[rate_limit_hits<br/>429 Too Many Requests]
        M7[syntax_errors<br/>Invalid email syntax]
        M8[no_mx_count<br/>Domains without MX]
        M9[dns_errors<br/>DNS resolution failures]
    end
    
    subgraph "SMTP Metrics (v2.0)"
        M10[smtp_verifications_total<br/>Total SMTP attempts]
        M11[smtp_deliverable<br/>RCPT TO 250]
        M12[smtp_undeliverable<br/>RCPT TO 5xx]
        M13[smtp_temporary_failure<br/>RCPT TO 4xx]
        M14[smtp_connection_errors<br/>Connection/Timeout]
        M15[smtp_catch_all<br/>Catch-all domains]
        M16[mx_cache_hits/misses<br/>Cache performance]
        M17[smtp_pool_size<br/>Active connections]
    end
    
    subgraph "Domain Intelligence"
        M18[top_problematic_domains<br/>Recurring bad domains]
        M19[disposable_detected<br/>Temp email domains]
        M20[role_based_detected<br/>admin@, info@, etc.]
    end
```

### Admin Dashboard Endpoints

```mermaid
graph TD
    Admin[Admin Panel<br/>http://host:3000/admin] --> Login[Basic Auth<br/>Username + Password]
    
    Login --> Dashboard[Dashboard<br/>Real-time Metrics]
    Login --> Metrics[GET /api/v1/admin/metrics<br/>JSON MetricsSnapshot]
    Login --> Logs[GET /api/v1/admin/logs<br/>Recent Log Entries]
    Login --> Keys[GET/POST/DELETE<br/>/api/v1/admin/keys<br/>API Key Management]
    Login --> Password[POST /api/v1/admin/password<br/>Change Admin Password]
    
    Dashboard --> Charts[Latency Charts<br/>Error Rate<br/>Throughput]
    Dashboard --> Tables[Top Domains<br/>Error Breakdown]
```

---

## 🔒 Seguridad

### Security Architecture

```mermaid
graph TB
    subgraph "Transport Security"
        TLS[TLS 1.2/1.3<br/>HTTPS via Reverse Proxy]
        CORS[CORS Policy<br/>ALLOWED_ORIGINS]
    end
    
    subgraph "Authentication"
        APIKey[API Key Auth<br/>X-API-Key Header]
        KeyRotation[Key Rotation<br/>Expiration Support]
        AdminAuth[Admin Basic Auth<br/>SHA-256 + Salt]
    end
    
    subgraph "Rate Limiting"
        PerKey[Per-Key QPS<br/>In-Memory Sliding Window]
        PerIP[Per-IP QPS<br/>X-Forwarded-For Support]
    end
    
    subgraph "Input Validation"
        BodyLimit[Body Limit 2KB<br/>Axum DefaultBodyLimit]
        SyntaxCheck[RFC 5321/5322<br/>Strict Validation]
        PayloadParse[JSON Parse<br/>Serde with Limits]
    end
    
    subgraph "Secrets Management"
        EnvVars[Environment Variables<br/>.env File]
        HashStorage[Password Hash<br/>SHA-256 + Random Salt]
        FilePersist[Atomic File Write<br/>data/admin_auth.json]
    end
    
    subgraph "Network Security"
        NonRoot[Non-Root User<br/>UID 1000 in Container]
        ReadOnly[Read-Only FS<br/>Except /app/data]
        CapDrop[Dropped Capabilities<br/>Minimal Privileges]
    end
    
    style TLS fill:#c8e6c9
    style APIKey fill:#c8e6c9
    style AdminAuth fill:#c8e6c9
    style BodyLimit fill:#fff9c4
    style EnvVars fill:#fff9c4
    style NonRoot fill:#c8e6c9
```

### Security Headers & Best Practices

```mermaid
checklist
    title Security Checklist
    section Transport
    [x] HTTPS enforced via reverse proxy (nginx/Traefik)
    [x] HSTS headers configured
    [x] Secure cookies (if sessions used)
    section Authentication
    [x] API Key required for all endpoints
    [x] Key expiration support
    [x] Admin panel Basic Auth with salted SHA-256
    [x] Timing-safe password comparison
    section Authorization
    [x] Per-key rate limiting
    [x] Per-IP rate limiting (X-Forwarded-For)
    [x] Admin endpoints require auth
    section Input Validation
    [x] 2KB body limit
    [x] Strict JSON parsing
    [x] RFC 5321/5322 email syntax
    section Secrets
    [x] No secrets in code/git
    [x] .env file for configuration
    [x] Passwords hashed with salt
    [x] Atomic file writes for persistence
    section Container
    [x] Non-root user (UID 1000)
    [x] Read-only root filesystem
    [x] Dropped capabilities
    [x] Health checks configured
    section Network
    [x] CORS restricted to ALLOWED_ORIGINS
    [x] Outbound only to DNS (53) and SMTP (25/587/465)
    [x] No inbound except 3000
```

---

## 📚 Referencias y Estándares

| RFC/Estándar | Descripción | Uso en el Proyecto |
|--------------|-------------|-------------------|
| **RFC 5321** | Simple Mail Transfer Protocol | Sintaxis, límites, fallback A/AAAA, transacción SMTP |
| **RFC 5322** | Internet Message Format | Validación sintaxis email completa |
| **RFC 7505** | Null MX Record | Detección dominio que no acepta correo |
| **RFC 1035** | Domain Names - Implementation | Validación labels dominio (≤63 chars) |
| **RFC 3207** | SMTP STARTTLS | Upgrade TLS en puerto 587 |
| **RFC 8314** | Submission/Access over TLS | Puerto 465 SMTPS implícito |
| **OWASP API Security** | Top 10 API Risks | Rate limiting, auth, input validation |

---

## 📝 Version History

| Versión | Fecha | Cambios Principales |
|---------|-------|---------------------|
| **v2.0.0** | 2026-11-19 | Verificación SMTP real (RCPT TO), Cache MX, Pool conexiones, detección catch-all/disposable/role-based, 6 nuevos error codes |
| **v1.x** | 2026-10 | Validación sintaxis RFC 5321/5322, resolución MX asíncrona, Null MX, fallback A/AAAA, panel admin, API keys con expiración |

---

*Documentación generada automáticamente para Email Validator API v2.0.0*  
*Última actualización: 19 de noviembre de 2026*