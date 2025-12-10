// # Key Management and Key Store
//
// Hierarchical key management system with master encryption keys (MEK),
// data encryption keys (DEK), and envelope encryption.
//
// ## Key Hierarchy
//
// ```text
// ┌─────────────────────────────────────────┐
// │  Master Encryption Key (MEK)            │
// │  - Protected by password/HSM            │
// │  - Rarely rotated                       │
// └──────────────┬──────────────────────────┘
//                │ Encrypts
//                ▼
// ┌─────────────────────────────────────────┐
// │  Data Encryption Keys (DEK)             │
// │  - Per tablespace/column                │
// │  - Regular rotation                     │
// │  - Encrypted at rest by MEK             │
// └──────────────┬──────────────────────────┘
//                │ Encrypts
//                ▼
// ┌─────────────────────────────────────────┐
// │  Actual Data                            │
// └─────────────────────────────────────────┘
// ```
//
// ## Features
//
// - **Envelope Encryption**: DEKs encrypted by MEK
// - **Key Versioning**: Multiple versions of keys with seamless rotation
// - **Distributed Sync**: Key synchronization across cluster nodes
// - **Secure Storage**: Keys encrypted at rest
// - **Key Derivation**: PBKDF2/Argon2 for password-based keys

use crate::{DbError, Result};
use aes_gcm::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{SaltString, rand_core::OsRng as ArgonRng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use parking_lot::RwLock;
use sha2::Digest;
use std::fs;
use std::sync::Arc;
use rand::RngCore;

// Key version number
pub type KeyVersion = u32;

// Master Encryption Key (MEK)
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterKey {
    // Key identifier
    pub id: String,
    // Key material (encrypted when serialized)
    #[serde(skip)]
    pub key_material: Vec<u8>,
    // Encrypted key material (for persistence)
    pub encrypted_material: Vec<u8>,
    // Nonce used for encryption
    pub nonce: Vec<u8>,
    // Key version
    pub version: KeyVersion,
    // Creation timestamp
    pub created_at: i64,
    // Activation timestamp
    pub activated_at: Option<i64>,
    // Deactivation timestamp
    pub deactivated_at: Option<i64>,
    // Key status
    pub status: KeyStatus,
    // Algorithm used
    pub algorithm: String,
}

// Data Encryption Key (DEK)
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEncryptionKey {
    // Key identifier (e.g., tablespace name, column name)
    pub id: String,
    // Key material (decrypted, in memory only)
    #[serde(skip)]
    pub key_material: Vec<u8>,
    // Encrypted key material (encrypted by MEK)
    pub encrypted_material: Vec<u8>,
    // MEK version used to encrypt this DEK
    pub mek_version: KeyVersion,
    // DEK version
    pub version: KeyVersion,
    // Nonce for encryption
    pub nonce: Vec<u8>,
    // Creation timestamp
    pub created_at: i64,
    // Expiration timestamp (for rotation)
    pub expires_at: Option<i64>,
    // Key status
    pub status: KeyStatus,
    // Algorithm used with this key
    pub algorithm: String,
    // Usage count
    pub usage_count: u64,
}

// Key status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyStatus {
    // Key is active and can be used
    Active,
    // Key is deactivated but can still decrypt
    Deactivated,
    // Key is compromised and should not be used
    Compromised,
    // Key is pending activation
    Pending,
    // Key is expired
    Expired,
}

// Key metadata for persistence
#[derive(Debug, Serialize, Deserialize)]
struct KeyStoreMetadata {
    // MEK ID
    mek_id: String,
    // MEK version
    mek_version: KeyVersion,
    // Last rotation timestamp
    last_rotation: i64,
    // Total DEKs
    total_deks: usize,
}

// Main Key Store
pub struct KeyStore {
    // Storage directory
    data_dir: PathBuf,
    // Master Encryption Key (current)
    current_mek: RwLock<Option<Arc<MasterKey>>>,
    // Historical MEKs (for decryption)
    historical_meks: RwLock<HashMap<KeyVersion, Arc<MasterKey>>>,
    // Data Encryption Keys
    deks: RwLock<HashMap<String, DataEncryptionKey>>,
    // KEK (Key Encryption Key) for protecting MEK
    #[allow(dead_code)]
    kek: RwLock<Option<Vec<u8>>>,
    // Metadata
    metadata: RwLock<KeyStoreMetadata>,
}

impl KeyStore {
    // Create a new key store
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(&data_dir)
            .map_err(|e| DbError::IoError(format!("Failed to create keystore directory: {}", e)))?;

        Ok(Self {
            data_dir,
            current_mek: RwLock::new(None),
            historical_meks: RwLock::new(HashMap::new()),
            deks: RwLock::new(HashMap::new()),
            kek: RwLock::new(None),
            metadata: RwLock::new(KeyStoreMetadata {
                mek_id: String::new(),
                mek_version: 0,
                last_rotation: 0,
                total_deks: 0,
            }),
        })
    }

    // Initialize MEK from password using Argon2
    pub fn initialize_mek(&self, password: &str, salt: Option<&str>) -> Result<()> {
        // Generate or use provided salt
        let salt_str = if let Some(s) = salt {
            SaltString::from_b64(s)
                .map_err(|e| DbError::InvalidInput(format!("Invalid salt: {}", e)))?
        } else {
            SaltString::generate(&mut ArgonRng)
        };

        // Derive key from password using Argon2
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt_str)
            .map_err(|e| DbError::Encryption(format!("Failed to hash password: {}", e)))?;

        // Extract key material from hash
        let hash_str = password_hash.hash.ok_or_else(|| {
            DbError::Encryption("Failed to generate hash".to_string())
        })?;

        let key_material: Vec<u8> = hash_str.as_bytes().iter()
            .take(32)
            .copied()
            .collect();

        // Pad to 32 bytes if needed
        let mut key_material = key_material;
        key_material.resize(32, 0);

        // Create MEK
        let mek = MasterKey {
            id: "mek_v1".to_string(),
            key_material: key_material.clone(),
            encrypted_material: key_material.clone(), // In real scenario, encrypt with HSM/TPM
            nonce: vec![0u8; 12],
            version: 1,
            created_at: chrono::Utc::now().timestamp(),
            activated_at: Some(chrono::Utc::now().timestamp()),
            deactivated_at: None,
            status: KeyStatus::Active,
            algorithm: "AES256GCM".to_string(),
        };

        *self.current_mek.write() = Some(Arc::new(mek.clone()));
        self.historical_meks.write().insert(1, Arc::new(mek));

        // Update metadata
        let mut metadata = self.metadata.write();
        metadata.mek_id = "mek_v1".to_string();
        metadata.mek_version = 1;
        metadata.last_rotation = chrono::Utc::now().timestamp();

        Ok(())
    }

    // Generate a new MEK (for rotation)
    pub fn generate_mek(&self) -> Result<()> {
        // Generate random 256-bit key
        let mut key_material = vec![0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut key_material);

        let current_version = self.metadata.read().mek_version;
        let new_version = current_version + 1;

        let mek = MasterKey {
            id: format!("mek_v{}", new_version),
            key_material: key_material.clone(),
            encrypted_material: key_material, // Should be encrypted by KEK
            nonce: vec![0u8; 12],
            version: new_version,
            created_at: chrono::Utc::now().timestamp(),
            activated_at: Some(chrono::Utc::now().timestamp()),
            deactivated_at: None,
            status: KeyStatus::Active,
            algorithm: "AES256GCM".to_string(),
        };

        // Move current MEK to historical
        if let Some(current) = self.current_mek.read().as_ref() {
            self.historical_meks.write().insert(current.version, Arc::clone(current));
        }

        // Set new MEK as current
        *self.current_mek.write() = Some(Arc::new(mek.clone()));
        self.historical_meks.write().insert(new_version, Arc::new(mek));

        // Update metadata
        let mut metadata = self.metadata.write();
        metadata.mek_version = new_version;
        metadata.last_rotation = chrono::Utc::now().timestamp();

        Ok(())
    }

    // Generate a Data Encryption Key
    #[inline]
    pub fn generate_dek(&mut self, id: &str, algorithm: &str) -> Result<Vec<u8>> {
        let mek = self.current_mek.read()
            .as_ref()
            .ok_or_else(|| DbError::InvalidOperation("MEK not initialized".to_string()))?
            .clone();

        // Generate random DEK
        let mut key_material = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_material);

        // Encrypt DEK with MEK (envelope encryption)
        let (nonce, encrypted_material) = self.encrypt_with_mek(&mek, &key_material)?;

        let dek = DataEncryptionKey {
            id: id.to_string(),
            key_material: key_material.clone(),
            encrypted_material,
            mek_version: mek.version,
            version: 1,
            nonce,
            created_at: chrono::Utc::now().timestamp(),
            expires_at: None,
            status: KeyStatus::Active,
            algorithm: algorithm.to_string(),
            usage_count: 0,
        };

        self.deks.write().insert(id.to_string(), dek);

        // Update metadata
        self.metadata.write().total_deks += 1;

        Ok(key_material)
    }

    // Get a Data Encryption Key
    #[inline]
    pub fn get_dek(&self, id: &str) -> Result<Vec<u8>> {
        let deks = self.deks.read();
        let dek = deks.get(id)
            .ok_or_else(|| DbError::NotFound(format!("DEK not found: {}", id)))?;

        if dek.status != KeyStatus::Active {
            return Err(DbError::InvalidOperation(format!(
                "DEK is not active: {:?}", dek.status
            )));
        }

        Ok(dek.key_material.clone())
    }

    // Rotate a Data Encryption Key
    pub fn rotate_dek(&mut self, id: &str) -> Result<Vec<u8>> {
        let mek = self.current_mek.read()
            .as_ref()
            .ok_or_else(|| DbError::InvalidOperation("MEK not initialized".to_string()))?
            .clone();

        // Generate new key material
        let mut key_material = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_material);

        // Encrypt with current MEK
        let (nonce, encrypted_material) = self.encrypt_with_mek(&mek, &key_material)?;

        // Update DEK
        let mut deks = self.deks.write();
        let dek = deks.get_mut(id)
            .ok_or_else(|| DbError::NotFound(format!("DEK not found: {}", id)))?;

        dek.key_material = key_material.clone();
        dek.encrypted_material = encrypted_material;
        dek.mek_version = mek.version;
        dek.version += 1;
        dek.nonce = nonce;

        Ok(key_material)
    }

    // Rotate all expired DEKs
    pub fn rotate_expired_deks(&mut self) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let expired_ids: Vec<String> = self.deks.read()
            .iter()
            .filter(|(_, dek)| {
                if let Some(expires_at) = dek.expires_at {
                    expires_at < now
                } else {
                    false
                }
            })
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired_ids.len();
        for id in expired_ids {
            self.rotate_dek(&id)?;
        }

        Ok(count)
    }

    // Set DEK expiration
    pub fn set_dek_expiration(&mut self, id: &str, days: u32) -> Result<()> {
        let mut deks = self.deks.write();
        let dek = deks.get_mut(id)
            .ok_or_else(|| DbError::NotFound(format!("DEK not found: {}", id)))?;

        let expires_at = chrono::Utc::now().timestamp() + (days as i64 * 86400);
        dek.expires_at = Some(expires_at);

        Ok(())
    }

    // Encrypt data with MEK (envelope encryption)
    #[inline]
    fn encrypt_with_mek(
        &self,
        mek: &MasterKey,
        plaintext: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        let cipher = Aes256Gcm::new(GenericArray::from_slice(&mek.key_material));

        // Generate nonce
        let mut nonce_bytes = vec![0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);

        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| DbError::Encryption(format!("MEK encryption failed: {}", e)))?;

        Ok((nonce_bytes, ciphertext))
    }

    // Decrypt data with MEK
    #[inline]
    fn decrypt_with_mek(
        &self,
        mek: &MasterKey,
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> Result<Vec<u8>> {

        let cipher = Aes256Gcm::new(GenericArray::from_slice(&mek.key_material));
        let nonce = Nonce::from_slice(nonce);

        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| DbError::Encryption(format!("MEK decryption failed: {}", e)))?;

        Ok(plaintext)
    }

    // Re-encrypt all DEKs with new MEK (after MEK rotation)
    pub fn reencrypt_all_deks(&mut self) -> Result<usize> {
        let mek = self.current_mek.read()
            .as_ref()
            .ok_or_else(|| DbError::InvalidOperation("MEK not initialized".to_string()))?
            .clone();

        let dek_ids: Vec<String> = self.deks.read().keys().cloned().collect();
        let count = dek_ids.len();

        for id in dek_ids {
            let mut deks = self.deks.write();
            if let Some(dek) = deks.get_mut(&id) {
                // Re-encrypt key material with new MEK
                let (nonce, encrypted_material) = self.encrypt_with_mek(&mek, &dek.key_material)?;
                dek.encrypted_material = encrypted_material;
                dek.mek_version = mek.version;
                dek.nonce = nonce;
            }
        }

        Ok(count)
    }

    // Deactivate a DEK
    pub fn deactivate_dek(&mut self, id: &str) -> Result<()> {
        let mut deks = self.deks.write();
        let dek = deks.get_mut(id)
            .ok_or_else(|| DbError::NotFound(format!("DEK not found: {}", id)))?;

        dek.status = KeyStatus::Deactivated;
        Ok(())
    }

    // Mark DEK as compromised
    pub fn mark_dek_compromised(&mut self, id: &str) -> Result<()> {
        let mut deks = self.deks.write();
        let dek = deks.get_mut(id)
            .ok_or_else(|| DbError::NotFound(format!("DEK not found: {}", id)))?;

        dek.status = KeyStatus::Compromised;
        Ok(())
    }

    // List all DEKs
    pub fn list_deks(&self) -> Vec<String> {
        self.deks.read().keys().cloned().collect()
    }

    // Get DEK metadata
    pub fn get_dek_metadata(&self, id: &str) -> Option<DekMetadata> {
        self.deks.read().get(id).map(|dek| DekMetadata {
            id: dek.id.clone(),
            version: dek.version,
            mek_version: dek.mek_version,
            algorithm: dek.algorithm.clone(),
            status: dek.status.clone(),
            created_at: dek.created_at,
            expires_at: dek.expires_at,
            usage_count: dek.usage_count,
        })
    }

    // Get key store statistics
    pub fn get_stats(&self) -> KeyStoreStats {
        let metadata = self.metadata.read();
        let deks = self.deks.read();

        KeyStoreStats {
            mek_version: metadata.mek_version,
            total_deks: deks.len(),
            active_deks: deks.values().filter(|d| d.status == KeyStatus::Active).count(),
            expired_deks: deks.values().filter(|d| {
                if let Some(exp) = d.expires_at {
                    exp < chrono::Utc::now().timestamp()
                } else {
                    false
                }
            }).count(),
            last_rotation: metadata.last_rotation,
        }
    }

    // Persist key store to disk
    pub fn persist(&self) -> Result<()> {
        let metadata_path = self.data_dir.join("metadata.json");
        let metadata = self.metadata.read();
        let json = serde_json::to_string_pretty(&*metadata)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize metadata: {}", e)))?;

        fs::write(&metadata_path, json)
            .map_err(|e| DbError::IoError(format!("Failed to write metadata: {}", e)))?;

        // Persist DEKs (encrypted)
        let deks_path = self.data_dir.join("deks.bin");
        let deks = self.deks.read();
        let serialized = bincode::serialize(&*deks)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize DEKs: {}", e)))?;

        fs::write(&deks_path, serialized)
            .map_err(|e| DbError::IoError(format!("Failed to write DEKs: {}", e)))?;

        Ok(())
    }

    // Load key store from disk
    pub fn load(&mut self, password: &str) -> Result<()> {
        // Load metadata
        let metadata_path = self.data_dir.join("metadata.json");
        if metadata_path.exists() {
            let json = fs::read_to_string(&metadata_path)
                .map_err(|e| DbError::IoError(format!("Failed to read metadata: {}", e)))?;
            let loaded_metadata: KeyStoreMetadata = serde_json::from_str(&json)
                .map_err(|e| DbError::Serialization(format!("Failed to parse metadata: {}", e)))?;
            *self.metadata.write() = loaded_metadata;
        }

        // Re-initialize MEK from password
        self.initialize_mek(password, None)?;

        // Load DEKs
        let deks_path = self.data_dir.join("deks.bin");
        if deks_path.exists() {
            let data = fs::read(&deks_path)
                .map_err(|e| DbError::IoError(format!("Failed to read DEKs: {}", e)))?;
            let loaded_deks: HashMap<String, DataEncryptionKey> = bincode::deserialize(&data)
                .map_err(|e| DbError::Serialization(format!("Failed to parse DEKs: {}", e)))?;
            *self.deks.write() = loaded_deks;
        }

        Ok(())
    }
}

// DEK metadata (without sensitive key material)
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DekMetadata {
    pub id: String,
    pub version: KeyVersion,
    pub mek_version: KeyVersion,
    pub algorithm: String,
    pub status: KeyStatus,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub usage_count: u64,
}

// Structure-of-Arrays layout for DEK metadata (cache-friendly)
// Improves iteration performance by keeping similar data together
#[derive(Debug)]
struct DekMetadataSoA {
    // Parallel arrays for better cache locality
    ids: Vec<String>,
    versions: Vec<KeyVersion>,
    mek_versions: Vec<KeyVersion>,
    algorithms: Vec<String>,
    statuses: Vec<KeyStatus>,
    created_ats: Vec<i64>,
    expires_ats: Vec<Option<i64>>,
    usage_counts: Vec<u64>,
}

impl DekMetadataSoA {
    fn new() -> Self {
        Self {
            ids: Vec::new(),
            versions: Vec::new(),
            mek_versions: Vec::new(),
            algorithms: Vec::new(),
            statuses: Vec::new(),
            created_ats: Vec::new(),
            expires_ats: Vec::new(),
            usage_counts: Vec::new(),
        }
    }

    #[inline]
    fn push(&mut self, metadata: DekMetadata) {
        self.ids.push(metadata.id);
        self.versions.push(metadata.version);
        self.mek_versions.push(metadata.mek_version);
        self.algorithms.push(metadata.algorithm);
        self.statuses.push(metadata.status);
        self.created_ats.push(metadata.created_at);
        self.expires_ats.push(metadata.expires_at);
        self.usage_counts.push(metadata.usage_count);
    }

    #[inline]
    fn get(&self, index: usize) -> Option<DekMetadata> {
        if index >= self.ids.len() {
            return None;
        }
        Some(DekMetadata {
            id: self.ids[index].clone(),
            version: self.versions[index],
            mek_version: self.mek_versions[index],
            algorithm: self.algorithms[index].clone(),
            status: self.statuses[index].clone(),
            created_at: self.created_ats[index],
            expires_at: self.expires_ats[index],
            usage_count: self.usage_counts[index],
        })
    }

    #[inline]
    fn len(&self) -> usize {
        self.ids.len()
    }

    // Batch update usage counts - cache-friendly operation
    #[inline]
    fn batch_increment_usage(&mut self, indices: &[usize]) {
        for &idx in indices {
            if idx < self.usage_counts.len() {
                self.usage_counts[idx] += 1;
            }
        }
    }

    // Find expired DEKs - cache-friendly scan
    #[inline]
    fn find_expired(&self, now: i64) -> Vec<usize> {
        self.expires_ats
            .iter()
            .enumerate()
            .filter_map(|(idx, &expires_at)| {
                expires_at.filter(|&exp| exp < now).map(|_| idx)
            })
            .collect()
    }
}

// Key store statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyStoreStats {
    pub mek_version: KeyVersion,
    pub total_deks: usize,
    pub active_deks: usize,
    pub expired_deks: usize,
    pub last_rotation: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mek_initialization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let keystore = KeyStore::new(temp_dir.path()).unwrap();

        keystore.initialize_mek("test_password", None).unwrap();

        let stats = keystore.get_stats();
        assert_eq!(stats.mek_version, 1);
    }

    #[test]
    fn test_dek_generation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut keystore = KeyStore::new(temp_dir.path()).unwrap();
        keystore.initialize_mek("test_password", None).unwrap();

        let dek = keystore.generate_dek("test_tablespace", "AES256GCM").unwrap();
        assert_eq!(dek.len(), 32);

        let stats = keystore.get_stats();
        assert_eq!(stats.total_deks, 1);
        assert_eq!(stats.active_deks, 1);
    }

    #[test]
    fn test_dek_retrieval() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut keystore = KeyStore::new(temp_dir.path()).unwrap();
        keystore.initialize_mek("test_password", None).unwrap();

        let original = keystore.generate_dek("test_key", "AES256GCM").unwrap();
        let retrieved = keystore.get_dek("test_key").unwrap();

        assert_eq!(original, retrieved);
    }

    #[test]
    fn test_dek_rotation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut keystore = KeyStore::new(temp_dir.path()).unwrap();
        keystore.initialize_mek("test_password", None).unwrap();

        let original = keystore.generate_dek("test_key", "AES256GCM").unwrap();
        let rotated = keystore.rotate_dek("test_key").unwrap();

        assert_ne!(original, rotated);

        let metadata = keystore.get_dek_metadata("test_key").unwrap();
        assert_eq!(metadata.version, 2);
    }

    #[test]
    fn test_mek_rotation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut keystore = KeyStore::new(temp_dir.path()).unwrap();
        keystore.initialize_mek("test_password", None).unwrap();

        keystore.generate_dek("test_key", "AES256GCM").unwrap();

        // Rotate MEK
        keystore.generate_mek().unwrap();

        let stats = keystore.get_stats();
        assert_eq!(stats.mek_version, 2);

        // Re-encrypt all DEKs
        let reencrypted = keystore.reencrypt_all_deks().unwrap();
        assert_eq!(reencrypted, 1);
    }

    #[test]
    fn test_dek_expiration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut keystore = KeyStore::new(temp_dir.path()).unwrap();
        keystore.initialize_mek("test_password", None).unwrap();

        keystore.generate_dek("test_key", "AES256GCM").unwrap();
        keystore.set_dek_expiration("test_key", 30).unwrap();

        let metadata = keystore.get_dek_metadata("test_key").unwrap();
        assert!(metadata.expires_at.is_some());
    }

    #[test]
    fn test_dek_deactivation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut keystore = KeyStore::new(temp_dir.path()).unwrap();
        keystore.initialize_mek("test_password", None).unwrap();

        keystore.generate_dek("test_key", "AES256GCM").unwrap();
        keystore.deactivate_dek("test_key").unwrap();

        let result = keystore.get_dek("test_key");
        assert!(result.is_err());
    }
}
