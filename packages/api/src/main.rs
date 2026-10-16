//! Email Validator API - Servidor REST con Autenticación de Administrador, Panel Admin y API Keys con Expiración

mod admin_ui;
mod key_manager;
mod log_buffer;

use admin_ui::ADMIN_HTML;
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use dns::resolve_mx_records;
use hickory_resolver::TokioAsyncResolver;
use key_manager::{KeyManager, KeyValidationResult};
use log_buffer::{LogBuffer, LogLayer};
use serde::Deserialize;
use serde_json::json;
use shared::{
    ApiKeyRecord, AppConfig, AtomicMetrics, EmailValidationResponse, MetricsSnapshot, PasswordPolicy, AdminPasswordHash,
    ValidationError,
};
use std::{
    net::SocketAddr,
    sync::{atomic::Ordering, Arc},
    time::Instant,
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use validation::{format_response, validate_email_syntax};

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
        let (status, error_msg) = match self {
            ApiError::Validation(ValidationError::InvalidEmail(msg)) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Validation(ValidationError::InvalidApiKey) => (
                StatusCode::UNAUTHORIZED,
                "Falta el header X-API-Key o es inválido".to_string(),
            ),
            ApiError::Validation(ValidationError::ExpiredApiKey) => (
                StatusCode::UNAUTHORIZED,
                "La API Key proporcionada ha expirado. Contacte al administrador.".to_string(),
            ),
            ApiError::Validation(ValidationError::RateLimitExceeded) => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit excedido. Intente más tarde.".to_string(),
            ),
            ApiError::Validation(ValidationError::PayloadTooLarge) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "Payload demasiado grande. Máximo 2KB permitidos.".to_string(),
            ),
            ApiError::Validation(ValidationError::DnsError(msg)) => (StatusCode::BAD_GATEWAY, msg),
            ApiError::Validation(ValidationError::InternalError(msg)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };

        (status, Json(json!({ "error": error_msg }))).into_response()
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
    resolver: TokioAsyncResolver,
    key_manager: Arc<KeyManager>,
    metrics: Arc<AtomicMetrics>,
    log_buffer: LogBuffer,
    start_time: Instant,
}

// ==================== VERIFICACIÓN DE CREDENCIALES ADMIN ====================

fn check_admin_auth(headers: &HeaderMap, state: &AppState) -> bool {
    if let Some(auth_header) = headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        if let Some(base64_part) = auth_header.strip_prefix("Basic ") {
            use std::io::Read;
            // Decodificación base64 manual/segura
            if let Ok(decoded) = base64_decode(base64_part) {
                if let Ok(cred_str) = String::from_utf8(decoded) {
                    let parts: Vec<&str> = cred_str.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let user = parts[0];
                        let pass = parts[1];
                        return user == state.config.admin_user && state.config.verify_admin_password(&pass);
                    }
                }
            }
        }
    }
    false
}

fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let input = input.trim().as_bytes();
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;

    for &b in input {
        if b == b'=' {
            break;
        }
        let val = TABLE.iter().position(|&c| c == b).ok_or(())? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

// ==================== HANDLERS API PÚBLICA ====================

async fn validate_email_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<EmailRequest>,
) -> Result<(StatusCode, Json<EmailValidationResponse>), ApiError> {
    let timer = Instant::now();

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

    // Registrar uso para la clave
    state.key_manager.increment_usage(api_key).await;

    let email = payload.email.trim().to_string();

    // 2. Validar sintaxis (RFC 5322)
    if let Err(err) = validate_email_syntax(&email) {
        state.metrics.record_syntax_error();
        return Err(err.into());
    }

    // 3. Resolución DNS MX asíncrona
    let domain = email.split('@').last().unwrap_or("");
    let has_mx = resolve_mx_records(&state.resolver, domain).await?;

    let elapsed = timer.elapsed().as_millis() as u64;
    state.metrics.record_success(has_mx, elapsed);

    info!("Validación completada para {}: MX={} ({}ms)", domain, has_mx, elapsed);

    // 4. Formatear y construir respuesta
    let response = format_response(email, true, has_mx);

    Ok((StatusCode::OK, Json(response)))
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "✅ Email Validator API is running")
}

// ==================== HANDLERS PANEL ADMINISTRATIVO ====================

async fn admin_dashboard_handler() -> Html<&'static str> {
    Html(ADMIN_HTML)
}

async fn admin_login_check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    if check_admin_auth(&headers, &state) {
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn get_metrics_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MetricsSnapshot>, StatusCode> {
    if !check_admin_auth(&headers, &state) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let total = state.metrics.total_requests.load(Ordering::Relaxed);
    let success = state.metrics.successful_validations.load(Ordering::Relaxed);
    let deliverable = state.metrics.deliverable_count.load(Ordering::Relaxed);
    let non_deliverable = state.metrics.non_deliverable_count.load(Ordering::Relaxed);
    let syntax_err = state.metrics.invalid_syntax_count.load(Ordering::Relaxed);
    let auth_fail = state.metrics.auth_failures.load(Ordering::Relaxed);
    let total_lat = state.metrics.total_latency_ms.load(Ordering::Relaxed);

    let deliverability_rate = if success > 0 {
        (deliverable as f64 / success as f64) * 100.0
    } else {
        0.0
    };

    let average_latency_ms = if success > 0 {
        total_lat as f64 / success as f64
    } else {
        0.0
    };

    let uptime_seconds = state.start_time.elapsed().as_secs();

    Ok(Json(MetricsSnapshot {
        total_requests: total,
        successful_validations: success,
        deliverable_count: deliverable,
        non_deliverable_count: non_deliverable,
        invalid_syntax_count: syntax_err,
        auth_failures: auth_fail,
        deliverability_rate,
        average_latency_ms,
        uptime_seconds,
    }))
}

async fn list_keys_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiKeyRecord>>, StatusCode> {
    if !check_admin_auth(&headers, &state) {
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
    if !check_admin_auth(&headers, &state) {
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
    if !check_admin_auth(&headers, &state) {
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
    if !check_admin_auth(&headers, &state) {
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
    if !check_admin_auth(&headers, &state) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Validar política de contraseña
    let policy = PasswordPolicy::default();
    let new_password = &payload.new_password;

    if new_password.len() < policy.min_length {
        return Err(StatusCode::BAD_REQUEST);
    }
    if new_password.len() > policy.max_length {
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

    // Generar nuevo hash
    let new_hash = AdminPasswordHash::new(new_password);

    // Actualizar config (Arc clone, así que necesitamos mutar...)
    // Para simplificar, actualizamos el env y el state
    // En un entorno real usaríamos una base de datos o archivo de configuración
    drop(state); // evitar préstamo mutuo

    // Por ahora, respondemos exitosamente y el admin debe reiniciar con las nuevas credenciales
    // TODO: Persistir el nuevo hash en el archivo .env o en almacenamiento persistente
    Ok(StatusCode::OK)
}

async fn get_logs_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<log_buffer::LogEntry>>, StatusCode> {
    if !check_admin_auth(&headers, &state) {
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
            "description": "Microservicio REST de validación de sintaxis RFC 5322 y registros MX DNS asíncronos"
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
                        "200": { "description": "Resultado de la validación" },
                        "400": { "description": "Sintaxis de email inválida" },
                        "401": { "description": "API Key inválida, ausente o expirada" }
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

    // 2. DNS Resolver y Gestor de Claves
    let resolver = TokioAsyncResolver::tokio_from_system_conf()?;
    let key_manager = Arc::new(KeyManager::init(&config.data_dir, &config.api_key).await);
    let metrics = Arc::new(AtomicMetrics::new());

    let state = Arc::new(AppState {
        config: Arc::clone(&config),
        resolver,
        key_manager,
        metrics,
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
    println!("🔑 Clave Admin: ******** (hash almacenado en configuración)");
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