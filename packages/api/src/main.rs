//! Email Validator API - Versión Modular Funcional y Segura

use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use dns::resolve_mx_records;
use hickory_resolver::TokioAsyncResolver;
use serde::Deserialize;
use serde_json::json;
use shared::{AppConfig, EmailValidationResponse, ValidationError};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
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

// ==================== ESTADO ====================

#[derive(Clone)]
struct AppState {
    config: Arc<AppConfig>,
    resolver: TokioAsyncResolver,
}

// ==================== HANDLERS ====================

async fn validate_email_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<EmailRequest>,
) -> Result<(StatusCode, Json<EmailValidationResponse>), ApiError> {
    // Validar autenticación por API Key
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(ValidationError::InvalidApiKey)?;

    if api_key != state.config.api_key {
        return Err(ValidationError::InvalidApiKey.into());
    }

    let email = payload.email.trim().to_string();

    // 1. Validar sintaxis (RFC 5322 simplificada)
    validate_email_syntax(&email)?;

    // 2. Extraer dominio y resolver registros MX vía DNS
    let domain = email.split('@').last().unwrap_or("");
    let has_mx = resolve_mx_records(&state.resolver, domain).await?;

    // 3. Formatear y construir respuesta
    let response = format_response(email, true, has_mx);

    Ok((StatusCode::OK, Json(response)))
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "✅ Email Validator API is running")
}

// ==================== SERVIDOR ====================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializar tracing / logging
    tracing_subscriber::fmt::init();

    let config = Arc::new(AppConfig::from_env());

    // Crear resolver DNS asíncrono con Tokio
    let resolver = TokioAsyncResolver::tokio_from_system_conf()?;

    let state = Arc::new(AppState {
        config: Arc::clone(&config),
        resolver,
    });

    println!("========================================");
    println!("🚀 Email Validator API (Arquitectura Modular)");
    println!("========================================");
    println!("📡 Puerto: {}", config.port);
    println!("🔗 URL: http://0.0.0.0:{}", config.port);
    println!("📖 Endpoints:");
    println!("  • POST /api/v1/validate-email");
    println!("  • GET  /health");
    println!("🔒 Seguridad:");
    println!("  • API Key requerida en header X-API-Key");
    println!("  • Límite de Payload: 2KB");
    println!("========================================");

    // Configuración de CORS
    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
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

    // Configurar router con límites de seguridad y estado
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/validate-email", post(validate_email_handler))
        .layer(cors)
        .layer(DefaultBodyLimit::max(2048)) // 2KB máximo para protección
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).await?;

    println!("✅ Servidor escuchando en http://{}", addr);

    // Servidor axum 0.7+
    axum::serve(listener, app).await?;

    Ok(())
}