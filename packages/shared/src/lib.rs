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

/// Códigos de error numéricos estandarizados para mapeo directo por clientes de la API
pub mod error_codes {
    /// 0: Correo válido y entregable
    pub const ERR_SUCCESS: u32 = 0;
    /// 1: Error desconocido o interno no clasificado
    pub const ERR_UNKNOWN: u32 = 1;
    /// 2: Sintaxis de correo electrónico inválida (RFC 5321/5322)
    pub const ERR_INVALID_SYNTAX: u32 = 2;
    /// 3: El dominio no posee servidores de correo (registros MX ni fallback A/AAAA)
    pub const ERR_NO_MX_RECORDS: u32 = 3;
    /// 4: Dominio con Null MX explícito según RFC 7505 (no acepta correos)
    pub const ERR_NULL_MX_REJECTED: u32 = 4;
    /// 5: Error en la consulta de resolución DNS externa o timeout
    pub const ERR_DNS_ERROR: u32 = 5;
    /// 6: Autenticación rechazada (API Key inválida, faltante o expirada)
    pub const ERR_UNAUTHORIZED: u32 = 6;
    /// 7: Límite de peticiones por segundo excedido (Rate limit)
    pub const ERR_RATE_LIMITED: u32 = 7;
    /// 8: Tamaño de carga útil excedido (Máximo 2KB)
    pub const ERR_PAYLOAD_TOO_LARGE: u32 = 8;
}

/// Respuesta de validación de email con código numérico estándar de error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailValidationResponse {
    pub email: String,
    pub is_valid_syntax: bool,
    pub has_mx_records: bool,
    pub is_deliverable: bool,
    /// Código numérico de error para mapeo directo (0 = Éxito/Válido, >0 = Error)
    pub error_code: u32,
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

/// Entrada de dominio problemático recurrente detectado en las solicitudes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblematicDomainEntry {
    pub domain: String,
    pub occurrences: u64,
    pub reason: String,
    pub error_category: String, // "sintaxis", "sin_mx", "dns_fallo"
}

/// Snapshot de métricas operativas de la API para serializar al Dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Total de llamadas API recibidas por el servicio
    pub total_requests: u64,
    /// Total de solicitudes procesadas y respondidas exitosamente por la API
    pub processed_validations: u64,
    /// Tasa de éxito operativo de la API (% de llamadas procesadas sin caídas de servidor)
    pub api_success_rate: f64,
    /// Latencia promedio de procesamiento en milisegundos
    pub average_latency_ms: f64,
    /// Solicitudes rechazadas por autenticación (API Key ausente, inválida o expirada)
    pub auth_failures: u64,
    /// Solicitudes rechazadas por Rate Limiting (HTTP 429)
    pub rate_limit_hits: u64,
    /// Solicitudes con sintaxis de correo inválida enviadas por clientes
    pub syntax_errors: u64,
    /// Solicitudes con sintaxis válida pero dominio sin registros MX / rechazo
    pub no_mx_count: u64,
    /// Solicitudes con fallos en consulta DNS externa
    pub dns_errors: u64,
    /// Tiempo de actividad continuo del servidor en segundos
    pub uptime_seconds: u64,
    /// Lista de dominios problemáticos más recurrentes recibidos en la API
    pub top_problematic_domains: Vec<ProblematicDomainEntry>,
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

use base64::Engine;

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
            salt: base64::engine::general_purpose::STANDARD.encode(salt.as_bytes()),
            hash: base64::engine::general_purpose::STANDARD.encode(result),
        }
    }

    /// Verificar si una contraseña coincide con este hash (de forma determinista e inmune a discrepancias de codificación)
    pub fn verify(&self, password: &str) -> bool {
        // Decodificar la sal original desde base64
        let salt_bytes = match base64::engine::general_purpose::STANDARD.decode(&self.salt) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let salt_str = match std::str::from_utf8(&salt_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let mut hasher = Sha256::new();
        hasher.update(format!("{salt_str}{password}").as_bytes());
        let result = hasher.finalize();

        base64::engine::general_purpose::STANDARD.encode(result) == self.hash
    }
}

/// Estructura de almacenamiento persistente para credenciales del Administrador
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAuthStore {
    pub username: String,
    pub password_hash: AdminPasswordHash,
    pub updated_at: String,
}

/// Estructura atómica para recolección de métricas operativas de la API sin bloqueo
#[derive(Debug, Default)]
pub struct AtomicMetrics {
    pub total_requests: AtomicU64,
    pub processed_validations: AtomicU64,
    pub auth_failures: AtomicU64,
    pub rate_limit_hits: AtomicU64,
    pub syntax_errors: AtomicU64,
    pub no_mx_count: AtomicU64,
    pub dns_errors: AtomicU64,
    pub internal_errors: AtomicU64,
    pub total_latency_ms: AtomicU64,
}

impl AtomicMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registrar nueva llamada HTTP entrante a la API
    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Registrar validación procesada con éxito por la API
    pub fn record_processed(&self, has_mx: bool, latency_ms: u64) {
        self.processed_validations.fetch_add(1, Ordering::Relaxed);
        if !has_mx {
            self.no_mx_count.fetch_add(1, Ordering::Relaxed);
        }
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
    }

    /// Registrar correo con error de sintaxis RFC enviado por el cliente
    pub fn record_syntax_error(&self) {
        self.syntax_errors.fetch_add(1, Ordering::Relaxed);
        self.processed_validations.fetch_add(1, Ordering::Relaxed);
    }

    /// Registrar rechazo de autenticación (API Key inválida, ausente o expirada)
    pub fn record_auth_failure(&self) {
        self.auth_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Registrar rechazo por límite de solicitudes por segundo (Rate Limit HTTP 429)
    pub fn record_rate_limit_hit(&self) {
        self.rate_limit_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Registrar error en consulta a servidores DNS externos
    pub fn record_dns_error(&self) {
        self.dns_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Registrar error interno del servidor
    pub fn record_internal_error(&self) {
        self.internal_errors.fetch_add(1, Ordering::Relaxed);
    }
}

/// Tracker thread-safe para identificar dominios mal escritos y problemáticos recurrentes
#[derive(Debug, Default)]
pub struct DomainTracker {
    // domain -> (occurrences, last_reason, error_category)
    entries: std::sync::Mutex<std::collections::HashMap<String, (u64, String, String)>>,
}

impl DomainTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registrar un dominio problemático (mal escrito, sin MX o con error DNS)
    pub fn record(&self, domain: &str, reason: &str, category: &str) {
        let domain_clean = domain.trim().trim_end_matches('.').to_lowercase();
        if domain_clean.is_empty() {
            return;
        }

        if let Ok(mut map) = self.entries.lock() {
            let entry = map.entry(domain_clean).or_insert((0, String::new(), String::new()));
            entry.0 += 1;
            entry.1 = reason.to_string();
            entry.2 = category.to_string();

            // Auto-mantenimiento para evitar consumo excesivo de memoria: tope de 2000 dominios
            if map.len() > 2000 {
                let avg = map.values().map(|(c, _, _)| *c).sum::<u64>() / map.len() as u64;
                map.retain(|_, (c, _, _)| *c >= avg);
            }
        }
    }

    /// Obtener ranking de los top N dominios problemáticos más recurrentes
    pub fn get_top(&self, limit: usize) -> Vec<ProblematicDomainEntry> {
        if let Ok(map) = self.entries.lock() {
            let mut list: Vec<ProblematicDomainEntry> = map
                .iter()
                .map(|(domain, (occurrences, reason, category))| ProblematicDomainEntry {
                    domain: domain.clone(),
                    occurrences: *occurrences,
                    reason: reason.clone(),
                    error_category: category.clone(),
                })
                .collect();
            list.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
            list.truncate(limit);
            list
        } else {
            Vec::new()
        }
    }
}

/// Configuración de la aplicación compartida
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub admin_user: String,
    pub admin_password_hash: AdminPasswordHash,
    pub allowed_origins: Vec<String>,
    pub rate_limit_qps: u32,
    pub port: u16,
    pub data_dir: String,
    /// Servidores DNS explícitos (si está vacío se usa la configuración del sistema)
    pub dns_nameservers: Vec<String>,
    /// Timeout por intento de consulta DNS (ms)
    pub dns_timeout_ms: u64,
    /// Número de reintentos por consulta DNS
    pub dns_attempts: usize,
    /// Tamaño de la caché DNS en memoria
    pub dns_cache_size: usize,
    /// TTL mínimo aplicado a respuestas positivas (segundos)
    pub dns_positive_min_ttl: u64,
    /// TTL mínimo aplicado a respuestas negativas (segundos)
    pub dns_negative_min_ttl: u64,
    /// TTL máximo aplicado a respuestas negativas (segundos)
    pub dns_negative_max_ttl: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let api_key = std::env::var("API_KEY").unwrap_or_else(|_| {
            eprintln!("ADVERTENCIA: API_KEY no configurada, usando valor por defecto");
            "default-key-for-development".to_string()
        });

        let mut admin_user = std::env::var("ADMIN_USER").unwrap_or_else(|_| "Administrator".to_string());
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());

        // 1. Intentar cargar credenciales previamente persistidas en data/admin_auth.json
        let auth_file = std::path::Path::new(&data_dir).join("admin_auth.json");
        let admin_password_hash = if auth_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&auth_file) {
                if let Ok(store) = serde_json::from_str::<AdminAuthStore>(&content) {
                    admin_user = store.username;
                    store.password_hash
                } else {
                    Self::load_hash_from_env()
                }
            } else {
                Self::load_hash_from_env()
            }
        } else {
            Self::load_hash_from_env()
        };

        let allowed_origins: Vec<String> = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:4000,http://localhost:5173".to_string())
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
            .unwrap_or(3001);

        // ── Configuración de DNS (rendimiento) ──
        let dns_nameservers: Vec<String> = std::env::var("DNS_NAMESERVERS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let env_num = |key: &str, default: u64| -> u64 {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };

        let dns_timeout_ms = env_num("DNS_TIMEOUT_MS", 2000);
        let dns_attempts = env_num("DNS_ATTEMPTS", 2) as usize;
        let dns_cache_size = env_num("DNS_CACHE_SIZE", 5000) as usize;
        let dns_positive_min_ttl = env_num("DNS_POSITIVE_MIN_TTL", 60);
        let dns_negative_min_ttl = env_num("DNS_NEGATIVE_MIN_TTL", 60);
        let dns_negative_max_ttl = env_num("DNS_NEGATIVE_MAX_TTL", 300);

        Self {
            api_key,
            admin_user,
            admin_password_hash,
            allowed_origins,
            rate_limit_qps,
            port,
            data_dir,
            dns_nameservers,
            dns_timeout_ms,
            dns_attempts,
            dns_cache_size,
            dns_positive_min_ttl,
            dns_negative_min_ttl,
            dns_negative_max_ttl,
        }
    }

    fn load_hash_from_env() -> AdminPasswordHash {
        let stored_hash = std::env::var("ADMIN_PASSWORD_HASH").ok();
        let stored_salt = std::env::var("ADMIN_PASSWORD_SALT").ok();

        match (stored_hash, stored_salt) {
            (Some(hash), Some(salt)) => AdminPasswordHash { salt, hash },
            _ => {
                let raw_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin12345".to_string());
                AdminPasswordHash::new(&raw_password)
            }
        }
    }

    /// Verificar credenciales del administrador usando hash con sal
    pub fn verify_admin_password(&self, password: &str) -> bool {
        self.admin_password_hash.verify(password)
    }

    /// Guardar nuevo hash de credenciales en archivo persistente
    pub fn save_admin_auth(&self, username: &str, new_hash: AdminPasswordHash) -> Result<(), String> {
        let dir = std::path::Path::new(&self.data_dir);
        let _ = std::fs::create_dir_all(dir);
        let auth_file = dir.join("admin_auth.json");

        let store = AdminAuthStore {
            username: username.to_string(),
            password_hash: new_hash,
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default(),
        };

        let json = serde_json::to_string_pretty(&store)
            .map_err(|e| format!("Error serializando auth store: {}", e))?;

        let tmp_file = dir.join("admin_auth.tmp");
        std::fs::write(&tmp_file, json)
            .map_err(|e| format!("Error escribiendo archivo temporal: {}", e))?;
        std::fs::rename(tmp_file, auth_file)
            .map_err(|e| format!("Error reemplazando archivo final: {}", e))?;

        Ok(())
    }
}