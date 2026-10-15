//! KeyManager - Gestor persistente de claves API con expiración

use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use shared::ApiKeyRecord;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyStoreFile {
    keys: Vec<ApiKeyRecord>,
}

#[derive(Debug, Clone)]
pub struct KeyManager {
    file_path: PathBuf,
    // Key -> Record
    keys: Arc<RwLock<HashMap<String, ApiKeyRecord>>>,
}

pub enum KeyValidationResult {
    Valid,
    Expired,
    Invalid,
}

impl KeyManager {
    pub async fn init(data_dir: &str, master_key: &str) -> Self {
        let dir = Path::new(data_dir);
        let _ = fs::create_dir_all(dir);
        let file_path = dir.join("keys.json");

        let mut map: HashMap<String, ApiKeyRecord> = HashMap::new();

        // 1. Cargar claves existentes del archivo
        if file_path.exists() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Ok(store) = serde_json::from_str::<KeyStoreFile>(&content) {
                    for key_record in store.keys {
                        map.insert(key_record.key.clone(), key_record);
                    }
                }
            }
        }

        // 2. Asegurar que la master key del .env esté registrada (eterna)
        if !map.contains_key(master_key) {
            let master_record = ApiKeyRecord {
                id: "master-key".to_string(),
                key: master_key.to_string(),
                name: "Clave Maestra (.env)".to_string(),
                is_active: true,
                created_at: Utc::now().to_rfc3339(),
                expires_at: None, // Eterna
                total_requests: 0,
            };
            map.insert(master_key.to_string(), master_record);
        }

        let manager = Self {
            file_path,
            keys: Arc::new(RwLock::new(map)),
        };

        // Guardar estado inicial persistido
        manager.persist().await;

        manager
    }

    /// Validar estado de la clave (activa y no expirada)
    pub async fn validate_key(&self, key: &str) -> KeyValidationResult {
        let read = self.keys.read().await;
        if let Some(record) = read.get(key) {
            if !record.is_active {
                return KeyValidationResult::Invalid;
            }

            // Comprobar si ha expirado
            if let Some(ref exp_str) = record.expires_at {
                if let Ok(exp_time) = DateTime::parse_from_rfc3339(exp_str) {
                    if Utc::now() > exp_time.with_timezone(&Utc) {
                        return KeyValidationResult::Expired;
                    }
                }
            }

            KeyValidationResult::Valid
        } else {
            KeyValidationResult::Invalid
        }
    }

    /// Incrementar contador de peticiones por clave
    pub async fn increment_usage(&self, key: &str) {
        let mut write = self.keys.write().await;
        if let Some(record) = write.get_mut(key) {
            record.total_requests = record.total_requests.saturating_add(1);
        }
    }

    /// Listar todas las claves registradas
    pub async fn list_keys(&self) -> Vec<ApiKeyRecord> {
        let read = self.keys.read().await;
        let mut list: Vec<ApiKeyRecord> = read.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    /// Crear una nueva clave con expiración configurable (días o None para eterna)
    pub async fn create_key(&self, name: String, expires_in_days: Option<u32>) -> ApiKeyRecord {
        let random_part: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let generated_key = format!("ev_{}", random_part);
        let id = format!("key_{}", &random_part[0..8]);

        let created_at = Utc::now();
        let expires_at = expires_in_days.and_then(|days| {
            if days > 0 {
                Some((created_at + Duration::days(days as i64)).to_rfc3339())
            } else {
                None // 0 días = Eterna
            }
        });

        let record = ApiKeyRecord {
            id,
            key: generated_key.clone(),
            name,
            is_active: true,
            created_at: created_at.to_rfc3339(),
            expires_at,
            total_requests: 0,
        };

        {
            let mut write = self.keys.write().await;
            write.insert(generated_key, record.clone());
        }

        self.persist().await;
        record
    }

    /// Activar / Desactivar clave
    pub async fn toggle_key(&self, id: &str) -> Option<ApiKeyRecord> {
        let mut target: Option<ApiKeyRecord> = None;
        {
            let mut write = self.keys.write().await;
            for record in write.values_mut() {
                if record.id == id {
                    record.is_active = !record.is_active;
                    target = Some(record.clone());
                    break;
                }
            }
        }
        if target.is_some() {
            self.persist().await;
        }
        target
    }

    /// Eliminar clave por ID (no permite borrar master-key)
    pub async fn delete_key(&self, id: &str) -> bool {
        let mut found_key: Option<String> = None;
        {
            let read = self.keys.read().await;
            for (key, record) in read.iter() {
                if record.id == id && record.id != "master-key" {
                    found_key = Some(key.clone());
                    break;
                }
            }
        }

        if let Some(key) = found_key {
            let mut write = self.keys.write().await;
            write.remove(&key);
            drop(write);
            self.persist().await;
            true
        } else {
            false
        }
    }

    /// Guardar a disco atómicamente
    async fn persist(&self) {
        let read = self.keys.read().await;
        let keys_vec: Vec<ApiKeyRecord> = read.values().cloned().collect();
        let store = KeyStoreFile { keys: keys_vec };

        if let Ok(json_str) = serde_json::to_string_pretty(&store) {
            let tmp_path = self.file_path.with_extension("tmp");
            if fs::write(&tmp_path, json_str).is_ok() {
                let _ = fs::rename(tmp_path, &self.file_path);
            }
        }
    }
}
