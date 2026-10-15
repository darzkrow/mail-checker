//! email-validator-shared - Modelos y errores compartidos
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errores de validación de correo
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    #[error("Correo inválido: {0}")]
    InvalidEmail(String),

    #[error("API Key inválida")]
    InvalidApiKey,

    #[error("Rate limit excedido. Intente más tarde.")]
    RateLimitExceeded,

    #[error("Payload demasiado grande. Máximo 2KB permitidos.")]
    PayloadTooLarge,

    #[error("Error en la resolución DNS: {0}")]
    DnsError(String),

    #[error("Error interno del servidor: {0}")]
    InternalError(String),
}

/// Respuesta de validación de email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailValidationResponse {
    pub email: String,
    pub is_valid_syntax: bool,
    pub has_mx_records: bool,
    pub is_deliverable: bool,
    pub reason: String,
}

/// Configuración de la aplicación compartida
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub allowed_origins: Vec<String>,
    pub rate_limit_qps: u32,
    pub port: u16,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let api_key = std::env::var("API_KEY").unwrap_or_else(|_| {
            eprintln!("ADVERTENCIA: API_KEY no configurada, usando valor por defecto");
            "default-key-for-development".to_string()
        });

        let allowed_origins: Vec<String> = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let rate_limit_qps: u32 = std::env::var("RATE_LIMIT_QPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let port: u16 = std::env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3000);

        Self {
            api_key,
            allowed_origins,
            rate_limit_qps,
            port,
        }
    }
}