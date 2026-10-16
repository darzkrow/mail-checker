//! email-validator-shared - Modelos, errores y configuración compartidos
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use sha2::{Sha256, Digest};
/// Errores de validación de correo
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    #[error("Correo inválido: {0}")]
    InvalidEmail(String),

    #[error("API Key inválida")]
    InvalidApiKey,

    #[error("API Key expirada")]
    ExpiredApiKey,

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

/// Registro persistente de API Key con soporte de expiración
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub id: String,
    pub key: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: String,
    pub expires_at: Option<String>, // None = Nunca expira (Eterna)
    pub total_requests: u64,
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

/// Configuración de políticas de contraseña para el administrador
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    /// Longitud mínima
    pub min_length: usize,
    /// Longitud máxima
    pub max_length: usize,
    /// Requerir mayúscula
    pub require_uppercase: bool,
    /// Requerir minúscula
    pub require_lowercase: bool,
    /// Requerir número
    pub require_number: bool,
    /// Requerir carácter especial
    pub require_special: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_number: true,
            require_special: true,
        }
    }
}

/// Hash de contraseña con sal para autenticación administrador
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminPasswordHash {
    /// Sal base64-encoded (16 caracteres alfanuméricos)
    pub salt: String,
    /// Hash SHA-256 de salt + contraseña base64-encoded
    pub hash: String,
}

impl AdminPasswordHash {
    /// Generar un nuevo hash con sal aleatoria para la contraseña dada
    pub fn new(password: &str) -> Self {
        use rand::Rng;
        let salt: String = rand::thread_rng()
            .sample_iter(rand::distributions::Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        let mut hasher = Sha256::new();
        hasher.update(format!("{salt}{password}").as_bytes());
        let result = hasher.finalize();

        Self {
            salt: base64::encode(salt.as_bytes()),
            hash: base64::encode(result),
        }
    }

    /// Verificar si una contraseña coincide con este hash
    pub fn verify(&self, password: &str) -> bool {
        // Decodificar sal
        let salt_bytes = match base64::decode(&self.salt) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let mut hasher = Sha256::new();
        hasher.update(format!("{salt_bytes:?}{password}").as_bytes());
        // Usar los bytes crudos de la sal directamente
        // Convertimos a string legible tomando solo los bytes imprimibles ASCII (32-126)
        let salt_str: String = salt_bytes
            .iter()
            .filter(|&&b| b >= 32 && b <= 126)
            .map(|&b| b as char)
            .collect();

        hasher.update(salt_str.as_bytes());
        hasher.update(password.as_bytes());
        let result = hasher.finalize();

        base64::encode(result) == self.hash
    }
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

/// Configuración de la aplicación compartida
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub admin_user: String,
    pub admin_password: String,
    pub admin_password_hash: AdminPasswordHash,
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

        let admin_user = std::env::var("ADMIN_USER").unwrap_or_else(|_| "Administrator".to_string());

        // Intentar leer password hash y salt desde .env, si no existen, generar los nuevos
        let (admin_password, admin_password_hash) = {
            let stored_hash = std::env::var("ADMIN_PASSWORD_HASH").ok();
            let stored_salt = std::env::var("ADMIN_PASSWORD_SALT").ok();

            match (stored_hash, stored_salt) {
                (Some(hash), Some(salt)) => {
                    // Usar hash y salt existentes
                    let password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin12345".to_string());
                    // Crear AdminPasswordHash desde los valores almacenados
                    (password, AdminPasswordHash {
                        salt,
                        hash,
                    })
                }
                _ => {
                    // Generar nuevos hash con la contraseña actual del .env
                    let raw_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin12345".to_string());
                    let hash = AdminPasswordHash::new(&raw_password);
                    (raw_password, hash)
                }
            }
        };

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
            admin_user,
            admin_password,
            admin_password_hash,
            allowed_origins,
            rate_limit_qps,
            port,
            data_dir,
        }
    }

    /// Verificar credenciales del administrador (usando hash)
    pub fn verify_admin_password(&self, password: &str) -> bool {
        self.admin_password_hash.verify(password)
    }

    /// Obtener la contraseña en texto plano (solo para depuración/initial load)
    pub fn get_admin_password(&self) -> &str {
        &self.admin_password
    }
}