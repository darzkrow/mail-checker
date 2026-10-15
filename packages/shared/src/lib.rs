//! email-validator-shared - Modelos y errores compartidos
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Registro persistente de API Key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub id: String,
    pub key: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: String,
    pub total_requests: u64,
}

/// Estructura atómica para recolección de métricas sin bloqueo
#[derive(Debug, Default)]
pub struct AtomicMetrics {
    pub total_requests: AtomicU64,
    pub successful_validations: AtomicU64,
    pub deliverable_count: AtomicU64,
    pub non_deliverable_count: AtomicU64,
    pub invalid_syntax_count: AtomicU64,
    pub auth_failures: AtomicU64,
    pub total_latency_ms: AtomicU64,
}

impl AtomicMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_success(&self, is_deliverable: bool, latency_ms: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_validations.fetch_add(1, Ordering::Relaxed);
        if is_deliverable {
            self.deliverable_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.non_deliverable_count.fetch_add(1, Ordering::Relaxed);
        }
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
    }

    pub fn record_syntax_error(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.invalid_syntax_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_auth_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.auth_failures.fetch_add(1, Ordering::Relaxed);
    }
}

/// Snapshot de métricas para serializar al Dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub successful_validations: u64,
    pub deliverable_count: u64,
    pub non_deliverable_count: u64,
    pub invalid_syntax_count: u64,
    pub auth_failures: u64,
    pub deliverability_rate: f64,
    pub average_latency_ms: f64,
    pub uptime_seconds: u64,
}

/// Configuración de la aplicación compartida
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub allowed_origins: Vec<String>,
    pub rate_limit_qps: u32,
    pub port: u16,
    pub data_dir: String,
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

        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());

        Self {
            api_key,
            allowed_origins,
            rate_limit_qps,
            port,
            data_dir,
        }
    }
}