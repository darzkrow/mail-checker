//! Cache y Connection Pool para optimización de rendimiento

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use crate::MxRecord;

/// Entrada de cache MX con timestamp
#[derive(Debug, Clone)]
pub struct MxCacheEntry {
    pub records: Vec<MxRecord>,
    pub created_at: Instant,
}

/// Cache LRU para registros MX con TTL
pub struct MxCache {
    cache: Arc<RwLock<HashMap<String, MxCacheEntry>>>,
    ttl: Duration,
    max_entries: usize,
}

impl MxCache {
    /// Crear nuevo cache MX
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            max_entries,
        }
    }

    /// Obtener registros MX del cache
    pub async fn get(&self, domain: &str) -> Option<Vec<MxRecord>> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(domain) {
            if entry.created_at.elapsed() < self.ttl {
                return Some(entry.records.clone());
            }
        }
        None
    }

    /// Guardar registros MX en cache
    pub async fn set(&self, domain: String, records: Vec<MxRecord>) {
        let mut cache = self.cache.write().await;
        
        // LRU eviction si se excede el máximo
        if cache.len() >= self.max_entries && !cache.contains_key(&domain) {
            // Eliminar entrada más antigua (simple FIFO para simplicidad)
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        
        cache.insert(domain, MxCacheEntry {
            records,
            created_at: Instant::now(),
        });
    }

    /// Limpiar entradas expiradas
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let now = Instant::now();
        cache.retain(|_, entry| now.duration_since(entry.created_at) < self.ttl);
    }

    /// Obtener estadísticas del cache
    pub async fn stats(&self) -> MxCacheStats {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|e| e.created_at.elapsed() >= self.ttl).count();
        MxCacheStats {
            total_entries: total,
            expired_entries: expired,
            hit_rate: 0.0, // Se calcula externamente
        }
    }
}

/// Estadísticas del cache MX
#[derive(Debug, Clone, serde::Serialize)]
pub struct MxCacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub hit_rate: f64,
}

/// Pool de conexiones SMTP por host
pub struct SmtpConnectionPool {
    pools: Arc<RwLock<HashMap<String, Vec<SmtpPooledConnection>>>>,
    max_per_host: usize,
    max_idle_time: Duration,
}

/// Conexión SMTP en pool (simplificada - en producción sería una conexión real)
#[derive(Debug, Clone)]
pub struct SmtpPooledConnection {
    pub host: String,
    pub port: u16,
    pub created_at: Instant,
    pub last_used: Instant,
    pub in_use: bool,
}

impl SmtpConnectionPool {
    /// Crear nuevo pool de conexiones
    pub fn new(max_per_host: usize, max_idle_time: Duration) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            max_per_host,
            max_idle_time,
        }
    }

    /// Obtener una conexión del pool (o crear nueva)
    pub async fn get_connection(&self, host: &str, port: u16) -> Option<SmtpPooledConnection> {
        let key = format!("{}:{}", host, port);
        let mut pools = self.pools.write().await;
        
        if let Some(pool) = pools.get_mut(&key) {
            // Buscar conexión disponible
            for conn in pool.iter_mut() {
                if !conn.in_use && conn.last_used.elapsed() < self.max_idle_time {
                    conn.in_use = true;
                    conn.last_used = Instant::now();
                    return Some(conn.clone());
                }
            }
            
            // Crear nueva si hay espacio
            if pool.len() < self.max_per_host {
                let conn = SmtpPooledConnection {
                    host: host.to_string(),
                    port,
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    in_use: true,
                };
                pool.push(conn.clone());
                return Some(conn);
            }
        } else {
            // Nuevo pool para este host
            let conn = SmtpPooledConnection {
                host: host.to_string(),
                port,
                created_at: Instant::now(),
                last_used: Instant::now(),
                in_use: true,
            };
            pools.insert(key, vec![conn.clone()]);
            return Some(conn);
        }
        
        None
    }

    /// Devolver conexión al pool
    pub async fn return_connection(&self, host: &str, port: u16) {
        let key = format!("{}:{}", host, port);
        let mut pools = self.pools.write().await;
        
        if let Some(pool) = pools.get_mut(&key) {
            for conn in pool.iter_mut() {
                if conn.in_use && conn.host == host && conn.port == port {
                    conn.in_use = false;
                    conn.last_used = Instant::now();
                    break;
                }
            }
        }
    }

    /// Limpiar conexiones idle
    pub async fn cleanup_idle(&self) {
        let mut pools = self.pools.write().await;
        let now = Instant::now();
        
        for pool in pools.values_mut() {
            pool.retain(|conn| !conn.in_use && now.duration_since(conn.last_used) < self.max_idle_time);
        }
        
        // Eliminar pools vacíos
        pools.retain(|_, pool| !pool.is_empty());
    }

    /// Obtener estadísticas del pool
    pub async fn stats(&self) -> SmtpPoolStats {
        let pools = self.pools.read().await;
        let total_pools = pools.len();
        let total_connections: usize = pools.values().map(|p| p.len()).sum();
        let active_connections: usize = pools.values()
            .flat_map(|p| p.iter())
            .filter(|c| c.in_use)
            .count();
        
        SmtpPoolStats {
            total_pools,
            total_connections,
            active_connections,
            idle_connections: total_connections - active_connections,
        }
    }
}

/// Estadísticas del pool SMTP
#[derive(Debug, Clone, serde::Serialize)]
pub struct SmtpPoolStats {
    pub total_pools: usize,
    pub total_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
}

/// Detector de dominios desechables
pub struct DisposableEmailDetector {
    domains: Arc<Mutex<HashMap<String, bool>>>,
}

impl DisposableEmailDetector {
    /// Crear detector con lista inicial de dominios desechables conocidos
    pub fn new() -> Self {
        let mut domains = HashMap::new();
        
        // Lista de dominios desechables conocidos (ejemplos)
        let disposable_domains = [
            "temp-mail.org", "guerrillamail.com", "10minutemail.com",
            "mailinator.com", "throwawaymail.com", "trashmail.com",
            "yopmail.com", "maildrop.cc", "dispostable.com",
            "tempmail.net", "fakeinbox.com", "spamgourmet.com",
            "getnada.com", "mohmal.com", "tempmailaddress.com",
        ];
        
        for domain in disposable_domains {
            domains.insert(domain.to_string(), true);
        }
        
        Self {
            domains: Arc::new(Mutex::new(domains)),
        }
    }

    /// Verificar si un dominio es desechable
    pub async fn is_disposable(&self, domain: &str) -> bool {
        let domains = self.domains.lock().await;
        domains.contains_key(domain)
    }

    /// Agregar dominio a la lista
    pub async fn add_domain(&self, domain: String) {
        let mut domains = self.domains.lock().await;
        domains.insert(domain, true);
    }

    /// Obtener lista de dominios
    pub async fn list_domains(&self) -> Vec<String> {
        let domains = self.domains.lock().await;
        domains.keys().cloned().collect()
    }
}

/// Detector de emails basados en roles
pub struct RoleBasedEmailDetector {
    role_prefixes: Vec<String>,
}

impl RoleBasedEmailDetector {
    /// Crear detector con prefijos de roles conocidos
    pub fn new() -> Self {
        Self {
            role_prefixes: vec![
                "admin".to_string(),
                "administrator".to_string(),
                "info".to_string(),
                "support".to_string(),
                "help".to_string(),
                "sales".to_string(),
                "marketing".to_string(),
                "billing".to_string(),
                "accounts".to_string(),
                "finance".to_string(),
                "hr".to_string(),
                "legal".to_string(),
                "security".to_string(),
                "abuse".to_string(),
                "postmaster".to_string(),
                "webmaster".to_string(),
                "hostmaster".to_string(),
                "noreply".to_string(),
                "no-reply".to_string(),
                "donotreply".to_string(),
                "contact".to_string(),
                "hello".to_string(),
                "team".to_string(),
                "office".to_string(),
            ],
        }
    }

    /// Verificar si un email es role-based
    pub fn is_role_based(&self, email: &str) -> bool {
        if let Some(local_part) = email.split('@').next() {
            let local_lower = local_part.to_lowercase();
            for prefix in &self.role_prefixes {
                if local_lower == *prefix || local_lower.starts_with(&format!("{}.", prefix)) || local_lower.starts_with(&format!("{}_", prefix)) || local_lower.starts_with(&format!("{}-", prefix)) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mx_cache() {
        let cache = MxCache::new(Duration::from_secs(60), 100);
        
        // Test miss
        assert!(cache.get("example.com").await.is_none());
        
        // Test set and hit
        let records = vec![crate::MxRecord::new_port25("mail.example.com".to_string(), 10)];
        cache.set("example.com".to_string(), records.clone()).await;
        let cached = cache.get("example.com").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_disposable_detector() {
        let detector = DisposableEmailDetector::new();
        
        assert!(detector.is_disposable("temp-mail.org").await);
        assert!(detector.is_disposable("guerrillamail.com").await);
        assert!(!detector.is_disposable("gmail.com").await);
        assert!(!detector.is_disposable("example.com").await);
    }

    #[test]
    fn test_role_based_detector() {
        let detector = RoleBasedEmailDetector::new();
        
        assert!(detector.is_role_based("admin@example.com"));
        assert!(detector.is_role_based("support@company.com"));
        assert!(detector.is_role_based("info@test.org"));
        assert!(detector.is_role_based("noreply@service.com"));
        assert!(detector.is_role_based("sales.department@corp.com"));
        assert!(!detector.is_role_based("john.doe@example.com"));
        assert!(!detector.is_role_based("user123@gmail.com"));
    }

    #[tokio::test]
    async fn test_smtp_pool() {
        let pool = SmtpConnectionPool::new(5, Duration::from_secs(300));
        
        let conn = pool.get_connection("mail.example.com", 25).await;
        assert!(conn.is_some());
        
        let conn = conn.unwrap();
        pool.return_connection(&conn.host, conn.port).await;
        
        let stats = pool.stats().await;
        assert_eq!(stats.total_pools, 1);
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.active_connections, 0);
    }
}