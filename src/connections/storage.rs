#![allow(dead_code)]
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub encrypted_password: Option<String>,
    pub dbname: String,
    pub ssl_mode: SslMode,
    pub created_at: String,
    pub last_used: Option<String>,
    pub sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SslMode {
    #[default]
    Prefer,
    Disable,
    Require,
}

pub struct ConnectionManager {
    pub profiles: Vec<ConnectionProfile>,
    pub active_index: Option<usize>,
    pub is_connected: bool,
    storage_path: PathBuf,
    crypto: MachineCipher,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let crypto = Self::load_or_create_key();
        let storage_path = Self::storage_path();
        let mut mgr = ConnectionManager {
            profiles: Vec::new(),
            active_index: None,
            is_connected: false,
            storage_path,
            crypto,
        };
        mgr.load_profiles();
        mgr
    }

    fn key_path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("lite-pg").join("master.key")
    }

    fn storage_path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("lite-pg").join("connections.json")
    }

    fn load_or_create_key() -> MachineCipher {
        let path = Self::key_path();
        if path.exists() {
            let bytes = std::fs::read(&path).unwrap_or_default();
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return MachineCipher::new_with_key(key);
            }
        }
        let cipher = MachineCipher::new();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Warning: failed to create key directory: {e}");
            }
        }
        if let Err(e) = std::fs::write(&path, cipher.as_bytes()) {
            eprintln!("Warning: failed to write master key: {e}");
        }
        cipher
    }

    fn load_profiles(&mut self) {
        if !self.storage_path.exists() {
            return;
        }
        let content = match std::fs::read_to_string(&self.storage_path) {
            Ok(c) => c,
            Err(_) => return,
        };
        self.profiles = serde_json::from_str(&content).unwrap_or_default();
    }

    pub fn save_profiles(&self) {
        if let Some(parent) = self.storage_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Warning: failed to create config directory: {e}");
            }
        }
        let content = serde_json::to_string_pretty(&self.profiles).unwrap_or_default();
        if let Err(e) = std::fs::write(&self.storage_path, content) {
            eprintln!("Warning: failed to save profiles: {e}");
        }
    }

    pub fn add_profile(
        &mut self,
        name: &str,
        host: &str,
        port: u16,
        user: &str,
        password: Option<&str>,
        dbname: &str,
    ) {
        let encrypted = password.and_then(|p| self.crypto.encrypt(p));
        let profile = ConnectionProfile {
            name: name.to_string(),
            host: host.to_string(),
            port,
            user: user.to_string(),
            encrypted_password: encrypted,
            dbname: dbname.to_string(),
            ssl_mode: SslMode::Prefer,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_used: None,
            sort_order: self.profiles.len() as u32,
        };
        self.profiles.push(profile);
        self.save_profiles();
    }

    pub fn remove_profile(&mut self, index: usize) {
        if index < self.profiles.len() {
            self.profiles.remove(index);
            if self.active_index == Some(index) {
                self.active_index = None;
            } else if let Some(active) = self.active_index {
                if index < active {
                    self.active_index = Some(active - 1);
                }
            }
            self.save_profiles();
        }
    }

    pub fn update_profile(
        &mut self,
        index: usize,
        name: &str,
        host: &str,
        port: u16,
        user: &str,
        password: Option<&str>,
        dbname: &str,
    ) {
        if index >= self.profiles.len() {
            return;
        }
        let encrypted = password.and_then(|p| self.crypto.encrypt(p));
        self.profiles[index].name = name.to_string();
        self.profiles[index].host = host.to_string();
        self.profiles[index].port = port;
        self.profiles[index].user = user.to_string();
        self.profiles[index].dbname = dbname.to_string();
        self.profiles[index].encrypted_password = encrypted;
        self.save_profiles();
    }

    pub fn get_decrypted_password(&self, index: usize) -> Option<String> {
        let profile = self.profiles.get(index)?;
        profile
            .encrypted_password
            .as_ref()
            .and_then(|enc| self.crypto.decrypt(enc))
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.profiles.len() {
            self.active_index = Some(index);
            self.profiles[index].last_used = Some(chrono::Utc::now().to_rfc3339());
            self.save_profiles();
        }
    }

}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

struct MachineCipher {
    key: [u8; 32],
}

impl MachineCipher {
    fn new() -> Self {
        let mut key = [0u8; 32];
        use rand::Rng;
        rand::rng().fill_bytes(&mut key);
        MachineCipher { key }
    }

    fn new_with_key(key: [u8; 32]) -> Self {
        MachineCipher { key }
    }

    fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }

    fn encrypt(&self, plaintext: &str) -> Option<String> {
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .ok()?;
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        use base64::Engine;
        Some(base64::engine::general_purpose::STANDARD.encode(&combined))
    }

    fn decrypt(&self, encoded: &str) -> Option<String> {
        use base64::Engine;
        let combined = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()?;
        if combined.len() < 12 {
            return None;
        }
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
        String::from_utf8(plaintext).ok()
    }
}
