//! Email Validator API - Servidor REST con Autenticación de Administrador, Panel Admin y API Keys con Expiración

mod admin_ui;
mod key_manager;
mod log_buffer;

use admin_ui::ADMIN_HTML;
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse, Json, Redirect, Response},
    routing::{delete, get, post},
    Router,
};
use dns::{resolve_mx_records, DnsResolutionResult};
use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use key_manager::{KeyManager, KeyValidationResult};
use log_buffer::{LogBuffer, LogLayer};
use serde::Deserialize;
use serde_json::json;
use shared::{
    error_codes, AdminPasswordHash, ApiKeyRecord, AppConfig, AtomicMetrics, DomainTracker,
    EmailValidationResponse, MetricsSnapshot, PasswordPolicy, ValidationError,
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{atomic::Ordering, Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use validation::{format_response, format_response_custom, validate_email_syntax};

// ==================== RATE LIMITER EN MEMORIA ====================

#[derive(Debug)]
pub struct RateLimiter {
    qps: u32,
    buckets: Mutex<HashMap<String, (u64, u32)>>,
}

impl RateLimiter {
    pub fn new(qps: u32) -> Self {
        Self {
            qps,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Verificar si la clave excede el límite de solicitudes por segundo (QPS)
    pub fn check(&self, key: &str) -> bool {
        if self.qps == 0 {
            return true;
        }
        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Ok(mut map) = self.buckets.lock() {
            let entry = map.entry(key.to_string()).or_insert((now_sec, 0));
            if entry.0 == now_sec {
                if entry.1 >= self.qps {
                    return false;
                }
                entry.1 += 1;
            } else {
                entry.0 = now_sec;
                entry.1 = 1;
            }

            // Mantenimiento periódico de memoria si el mapa crece
            if map.len() > 5000 {
                map.retain(|_, (sec, _)| now_sec.saturating_sub(*sec) < 60);
            }
            true
        } else {
            true
        }
    }
}

// ==================== ERRORES ====================

#[derive(Debug)]
pub enum ApiError {
    Validation(ValidationError),
}

impl From<ValidationError> for ApiError {
    fn from(err: ValidationError) -> Self {
        ApiError::Validation(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, err_code, error_msg) = match self {
            ApiError::Validation(ValidationError::InvalidEmail(msg)) => (
                StatusCode::BAD_REQUEST,
                error_codes::ERR_INVALID_SYNTAX,
                msg,
            ),
            ApiError::Validation(ValidationError::InvalidApiKey) => (
                StatusCode::UNAUTHORIZED,
                error_codes::ERR_UNAUTHORIZED,
                "Falta el header X-API-Key o es inválido".to_string(),
            ),
            ApiError::Validation(ValidationError::ExpiredApiKey) => (
                StatusCode::UNAUTHORIZED,
                error_codes::ERR_UNAUTHORIZED,
                "La API Key proporcionada ha expirado. Contacte al administrador.".to_string(),
            ),
            ApiError::Validation(ValidationError::RateLimitExceeded) => (
                StatusCode::TOO_MANY_REQUESTS,
                error_codes::ERR_RATE_LIMITED,
                "Rate limit excedido. Intente más tarde.".to_string(),
            ),
            ApiError::Validation(ValidationError::PayloadTooLarge) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                error_codes::ERR_PAYLOAD_TOO_LARGE,
                "Payload demasiado grande. Máximo 2KB permitidos.".to_string(),
            ),
            ApiError::Validation(ValidationError::DnsError(msg)) => (
                StatusCode::BAD_GATEWAY,
                error_codes::ERR_DNS_ERROR,
                msg,
            ),
            ApiError::Validation(ValidationError::InternalError(msg)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_codes::ERR_UNKNOWN,
                msg,
            ),
        };

        (
            status,
            Json(json!({
                "error": error_msg,
                "error_code": err_code
            })),
        )
            .into_response()
    }
}

// ==================== MODELOS ====================

#[derive(Deserialize)]
struct EmailRequest {
    email: String,
}

#[derive(Deserialize)]
struct CreateKeyRequest {
    name: String,
    expires_in_days: Option<u32>,
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    new_password: String,
}

// ==================== ESTADO GLOBAL ====================

#[derive(Clone)]
struct AppState {
    config: Arc<AppConfig>,
    admin_auth: Arc<RwLock<AdminPasswordHash>>,
    resolver: TokioAsyncResolver,
    key_manager: Arc<KeyManager>,
    rate_limiter: Arc<RateLimiter>,
    metrics: Arc<AtomicMetrics>,
    domain_tracker: Arc<DomainTracker>,
    log_buffer: LogBuffer,
    start_time: Instant,
}

// ==================== VERIFICACIÓN DE CREDENCIALES ADMIN ====================

async fn check_admin_auth(headers: &HeaderMap, state: &AppState) -> bool {
    use base64::Engine;
    if let Some(auth_header) = headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        if let Some(base64_part) = auth_header.strip_prefix("Basic ") {
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(base64_part.trim()) {
                if let Ok(cred_str) = String::from_utf8(decoded) {
                    let parts: Vec<&str> = cred_str.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let user = parts[0];
                        let pass = parts[1];
                        let hash = state.admin_auth.read().await;
                        return user == state.config.admin_user && hash.verify(pass);
                    }
                }
            }
        }
    }
    false
}

// ==================== HANDLERS API PÚBLICA ====================

async fn validate_email_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<EmailRequest>,
) -> Result<(StatusCode, Json<EmailValidationResponse>), ApiError> {
    let timer = Instant::now();

    // Contabilizar toda llamada recibida en la API
    state.metrics.record_request();

    // 1. Validar autenticación de API Key
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            state.metrics.record_auth_failure();
            ValidationError::InvalidApiKey
        })?;

    match state.key_manager.validate_key(api_key).await {
        KeyValidationResult::Valid => {}
        KeyValidationResult::Expired => {
            state.metrics.record_auth_failure();
            return Err(ValidationError::ExpiredApiKey.into());
        }
        KeyValidationResult::Invalid => {
            state.metrics.record_auth_failure();
            return Err(ValidationError::InvalidApiKey.into());
        }
    }

    // 2. Control de flujo / Rate Limiting por clave
    if !state.rate_limiter.check(api_key) {
        state.metrics.record_rate_limit_hit();
        return Err(ValidationError::RateLimitExceeded.into());
    }

    // Registrar uso para la clave
    state.key_manager.increment_usage(api_key).await;

    let email = payload.email.trim().to_string();

    // 3. Validar sintaxis (RFC 5321 y RFC 5322)
    if let Err(err) = validate_email_syntax(&email) {
        state.metrics.record_syntax_error();
        let reason = match err {
            ValidationError::InvalidEmail(msg) => msg,
            _ => "Formato de correo inválido".to_string(),
        };

        // Rastrear dominio mal escrito o inválido
        let domain_guess = email.split('@').nth(1).unwrap_or("dominio_malformado");
        state.domain_tracker.record(domain_guess, &reason, "sintaxis");

        let response = EmailValidationResponse {
            email,
            is_valid_syntax: false,
            has_mx_records: false,
            is_deliverable: false,
            error_code: error_codes::ERR_INVALID_SYNTAX,
            reason,
        };
        return Ok((StatusCode::OK, Json(response)));
    }

    // 4. Resolución DNS MX asíncrona (con detección de Null MX y fallback SMTP A/AAAA)
    let domain = email.split('@').nth(1).unwrap_or("");
    let (has_mx, error_code, custom_reason) = match resolve_mx_records(&state.resolver, domain).await {
        Ok(DnsResolutionResult::ValidMx) => {
            (true, error_codes::ERR_SUCCESS, None)
        }
        Ok(DnsResolutionResult::NullMx) => {
            state.domain_tracker.record(domain, "Dominio con Null MX explícito según RFC 7505 (no acepta correos)", "sin_mx");
            (false, error_codes::ERR_NULL_MX_REJECTED, Some("El dominio declara explícitamente que no acepta correo electrónico (Null MX según RFC 7505)".to_string()))
        }
        Ok(DnsResolutionResult::NoRecords) => {
            state.domain_tracker.record(domain, "Sin servidores de correo ni host resoluble", "sin_mx");
            (false, error_codes::ERR_NO_MX_RECORDS, Some("El dominio no posee servidores de correo (registros MX) ni host resoluble".to_string()))
        }
        Err(ValidationError::DnsError(msg)) => {
            state.metrics.record_dns_error();
            state.domain_tracker.record(domain, &format!("Fallo DNS: {}", msg), "dns_fallo");
            (false, error_codes::ERR_DNS_ERROR, Some(format!("Sintaxis válida, pero no fue posible contactar con los servidores DNS: {}", msg)))
        }
        Err(err) => {
            state.metrics.record_internal_error();
            return Err(err.into());
        }
    };

    let elapsed = timer.elapsed().as_millis() as u64;
    state.metrics.record_processed(has_mx, elapsed);

    info!("Validación completada para {}: MX={} (código={}, {}ms)", domain, has_mx, error_code, elapsed);

    // 5. Formatear y construir respuesta coherente con código numérico para mapeo
    let response = if let Some(reason) = custom_reason {
        format_response_custom(email, true, false, error_code, reason)
    } else {
        format_response(email, true, has_mx, error_code)
    };

    Ok((StatusCode::OK, Json(response)))
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "✅ Email Validator API is running")
}

/// La raíz no tiene contenido propio: llevamos directo al panel administrativo.
async fn root_redirect_handler() -> Redirect {
    Redirect::to("/admin")
}

// ==================== HANDLERS PANEL ADMINISTRATIVO ====================

async fn admin_dashboard_handler() -> Html<&'static str> {
    Html(ADMIN_HTML)
}

async fn admin_login_check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    if check_admin_auth(&headers, &state).await {
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn get_metrics_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MetricsSnapshot>, StatusCode> {
    if !check_admin_auth(&headers, &state).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let total = state.metrics.total_requests.load(Ordering::Relaxed);
    let processed = state.metrics.processed_validations.load(Ordering::Relaxed);
    let auth_fail = state.metrics.auth_failures.load(Ordering::Relaxed);
    let rate_hits = state.metrics.rate_limit_hits.load(Ordering::Relaxed);
    let syntax_err = state.metrics.syntax_errors.load(Ordering::Relaxed);
    let no_mx = state.metrics.no_mx_count.load(Ordering::Relaxed);
    let dns_err = state.metrics.dns_errors.load(Ordering::Relaxed);
    let internal_err = state.metrics.internal_errors.load(Ordering::Relaxed);
    let total_lat = state.metrics.total_latency_ms.load(Ordering::Relaxed);

    // Tasa de éxito operativo: % de llamadas recibidas que la API logró atender y procesar sin fallos internos
    let api_success_rate = if total > 0 {
        let server_failures = internal_err;
        let correctly_handled = total.saturating_sub(server_failures);
        (correctly_handled as f64 / total as f64) * 100.0
    } else {
        100.0
    };

    let average_latency_ms = if processed > 0 {
        total_lat as f64 / processed as f64
    } else {
        0.0
    };

    let uptime_seconds = state.start_time.elapsed().as_secs();
    let top_problematic_domains = state.domain_tracker.get_top(20);

    Ok(Json(MetricsSnapshot {
        total_requests: total,
        processed_validations: processed,
        api_success_rate,
        average_latency_ms,
        auth_failures: auth_fail,
        rate_limit_hits: rate_hits,
        syntax_errors: syntax_err,
        no_mx_count: no_mx,
        dns_errors: dns_err,
        uptime_seconds,
        top_problematic_domains,
    }))
}

async fn list_keys_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiKeyRecord>>, StatusCode> {
    if !check_admin_auth(&headers, &state).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let keys = state.key_manager.list_keys().await;
    Ok(Json(keys))
}

async fn create_key_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyRecord>), StatusCode> {
    if !check_admin_auth(&headers, &state).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let new_key = state
        .key_manager
        .create_key(payload.name, payload.expires_in_days)
        .await;
    info!(
        "Nueva API Key generada: {} ({}, expira: {:?})",
        new_key.id, new_key.name, new_key.expires_at
    );
    Ok((StatusCode::CREATED, Json(new_key)))
}

async fn toggle_key_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiKeyRecord>, StatusCode> {
    if !check_admin_auth(&headers, &state).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    state
        .key_manager
        .toggle_key(&id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn delete_key_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> StatusCode {
    if !check_admin_auth(&headers, &state).await {
        return StatusCode::UNAUTHORIZED;
    }
    if state.key_manager.delete_key(&id).await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn change_password_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<StatusCode, StatusCode> {
    if !check_admin_auth(&headers, &state).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Validar política de contraseña
    let policy = PasswordPolicy::default();
    let new_password = &payload.new_password;

    if new_password.len() < policy.min_length || new_password.len() > policy.max_length {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Cumplir requisitos de compleción
    if policy.require_uppercase && !new_password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if policy.require_lowercase && !new_password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if policy.require_number && !new_password.chars().any(|c| c.is_ascii_digit()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if policy.require_special && !new_password.chars().any(|c| !c.is_alphanumeric()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Generar nuevo hash con sal aleatoria
    let new_hash = AdminPasswordHash::new(new_password);

    // 1. Actualizar estado en memoria de forma inmediata
    {
        let mut auth_lock = state.admin_auth.write().await;
        *auth_lock = new_hash.clone();
    }

    // 2. Persistir el nuevo hash en disco (data/admin_auth.json)
    if let Err(e) = state.config.save_admin_auth(&state.config.admin_user, new_hash) {
        tracing::warn!("Advertencia: no se pudo persistir el hash de admin en disco: {}. Estará activo en memoria.", e);
    } else {
        info!("Contraseña de administrador actualizada y persistida en disco exitosamente");
    }

    Ok(StatusCode::OK)
}

async fn get_logs_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<log_buffer::LogEntry>>, StatusCode> {
    if !check_admin_auth(&headers, &state).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(state.log_buffer.get_recent()))
}

async fn openapi_spec_handler() -> Json<serde_json::Value> {
    Json(json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Email Validator API",
            "version": "1.0.0",
            "description": "Microservicio REST de validación de sintaxis RFC 5321/5322 y registros MX DNS asíncronos con soporte Null MX (RFC 7505) y fallback SMTP"
        },
        "paths": {
            "/api/v1/validate-email": {
                "post": {
                    "summary": "Validar sintaxis y registros MX de un correo",
                    "parameters": [{
                        "name": "X-API-Key",
                        "in": "header",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": { "email": { "type": "string", "example": "usuario@gmail.com" } },
                                    "required": ["email"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Resultado de la validación (incluye tanto correos válidos como inválidos)",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "email": { "type": "string" },
                                            "is_valid_syntax": { "type": "boolean" },
                                            "has_mx_records": { "type": "boolean" },
                                            "is_deliverable": { "type": "boolean" },
                                            "reason": { "type": "string" }
                                        }
                                    }
                                }
                            }
                        },
                        "400": { "description": "Cuerpo de solicitud malformado o JSON inválido" },
                        "401": { "description": "API Key inválida, ausente o expirada" },
                        "429": { "description": "Rate limit excedido por segundo" }
                    }
                }
            },
            "/health": {
                "get": {
                    "summary": "Healthcheck del microservicio",
                    "responses": { "200": { "description": "Servicio saludable" } }
                }
            }
        }
    }))
}

// ==================== SERVIDOR PRINCIPAL ====================

/// Construye el resolver DNS con timeouts, reintentos y caché ajustables.
///
/// Si `DNS_NAMESERVERS` está definido se usan esos servidores; de lo contrario
/// se parte de la configuración del sistema. Los TTL mínimos permiten que las
/// respuestas (incluidas las negativas) se reutilicen desde la caché en lugar
/// de volver a consultar la red en cada validación.
fn build_resolver(config: &AppConfig) -> Result<TokioAsyncResolver, Box<dyn std::error::Error>> {
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_millis(config.dns_timeout_ms.max(100));
    opts.attempts = config.dns_attempts.max(1);
    opts.cache_size = config.dns_cache_size;
    opts.positive_min_ttl = Some(Duration::from_secs(config.dns_positive_min_ttl));
    opts.negative_min_ttl = Some(Duration::from_secs(config.dns_negative_min_ttl));
    opts.negative_max_ttl = Some(Duration::from_secs(
        config
            .dns_negative_max_ttl
            .max(config.dns_negative_min_ttl),
    ));

    if config.dns_nameservers.is_empty() {
        let (sys_config, mut sys_opts) = hickory_resolver::system_conf::read_system_conf()?;
        sys_opts.timeout = opts.timeout;
        sys_opts.attempts = opts.attempts;
        sys_opts.cache_size = opts.cache_size;
        sys_opts.positive_min_ttl = opts.positive_min_ttl;
        sys_opts.negative_min_ttl = opts.negative_min_ttl;
        sys_opts.negative_max_ttl = opts.negative_max_ttl;
        Ok(TokioAsyncResolver::tokio(sys_config, sys_opts))
    } else {
        let ips: Vec<IpAddr> = config
            .dns_nameservers
            .iter()
            .filter_map(|server| server.parse::<IpAddr>().ok())
            .collect();
        if ips.is_empty() {
            return Err("DNS_NAMESERVERS no contiene direcciones IP válidas".into());
        }
        let name_servers = NameServerConfigGroup::from_ips_clear(&ips, 53, true);
        let resolver_config = ResolverConfig::from_parts(None, Vec::new(), name_servers);
        Ok(TokioAsyncResolver::tokio(resolver_config, opts))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Configuración de logging con buffer en memoria para el dashboard
    let log_buffer = LogBuffer::new(200);
    let log_layer = LogLayer {
        buffer: log_buffer.clone(),
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(log_layer)
        .init();

    let config = Arc::new(AppConfig::from_env());

    // 2. Resolver DNS, Gestor de Claves y Limitador de Tasa
    let resolver = build_resolver(&config)?;
    let key_manager = Arc::new(KeyManager::init(&config.data_dir, &config.api_key).await);
    let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit_qps));
    let metrics = Arc::new(AtomicMetrics::new());
    let domain_tracker = Arc::new(DomainTracker::new());
    let admin_auth = Arc::new(RwLock::new(config.admin_password_hash.clone()));

    let state = Arc::new(AppState {
        config: Arc::clone(&config),
        admin_auth,
        resolver,
        key_manager,
        rate_limiter,
        metrics,
        domain_tracker,
        log_buffer,
        start_time: Instant::now(),
    });

    println!("========================================");
    println!("🚀 Email Validator API (Versión con Panel Admin)");
    println!("========================================");
    println!("📡 Puerto: {}", config.port);
    println!("🔗 URL Base: http://0.0.0.0:{}", config.port);
    println!("📊 Panel Admin: http://0.0.0.0:{}/admin", config.port);
    println!("👤 Usuario Admin: {}", config.admin_user);
    println!("🔑 Clave Admin: ******** (hash seguro SHA-256 + salt)");
    println!("⚡ Rate Limit: {} QPS", config.rate_limit_qps);
    println!("📖 Swagger / OpenAPI: http://0.0.0.0:{}/api/v1/openapi.json", config.port);
    println!("📁 Directorio de Persistencia: {}", config.data_dir);
    println!("========================================");

    // 3. Configurar CORS
    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::HeaderName::from_static("x-api-key"),
        ]);

    if config.allowed_origins.is_empty() {
        cors = cors.allow_origin(Any);
    } else {
        let origins: Vec<HeaderValue> = config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse::<HeaderValue>().ok())
            .collect();
        cors = cors.allow_origin(origins);
    }

    // 4. Configuración del Router con Rutas de API y Admin protegidas
    let app = Router::new()
        // Raíz: redirige al panel administrativo (evita una página vacía)
        .route("/", get(root_redirect_handler))
        // API Pública
        .route("/health", get(health_check))
        .route("/api/v1/validate-email", post(validate_email_handler))
        .route("/api/v1/openapi.json", get(openapi_spec_handler))
        // Panel Administrativo & Métricas Protegidas
        .route("/admin", get(admin_dashboard_handler))
        .route("/api/v1/admin/password", post(change_password_handler))
        .route("/api/v1/admin/login", get(admin_login_check))
        .route("/api/v1/admin/metrics", get(get_metrics_handler))
        .route("/api/v1/admin/keys", get(list_keys_handler).post(create_key_handler))
        .route("/api/v1/admin/keys/:id/toggle", post(toggle_key_handler))
        .route("/api/v1/admin/keys/:id", delete(delete_key_handler))
        .route("/api/v1/admin/logs", get(get_logs_handler))
        .layer(cors)
        .layer(DefaultBodyLimit::max(2048))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).await?;

    info!("Servidor escuchando en http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}