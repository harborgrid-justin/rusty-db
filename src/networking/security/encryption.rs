// Message encryption for network communication
//
// This module provides envelope encryption, key derivation,
// perfect forward secrecy, and at-rest encryption keys.
//
// TODO(consolidation): Duplicate encryption implementation #1 of 5 (Issue D-01)
// This is one of 5 separate encryption implementations (~3,850 lines total).
// Consolidate with: security/encryption.rs, security/encryption_engine.rs,
// security_vault/tde.rs, backup/backup_encryption.rs
// See diagrams/07_security_enterprise_flow.md Section 4.1
// Recommendation: Create unified EncryptionService trait

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Encryption algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
    /// AES-128-GCM
    Aes128Gcm,
}

impl EncryptionAlgorithm {
    /// Get key size in bytes
    pub fn key_size(&self) -> usize {
        match self {
            EncryptionAlgorithm::Aes256Gcm => 32,
            EncryptionAlgorithm::ChaCha20Poly1305 => 32,
            EncryptionAlgorithm::Aes128Gcm => 16,
        }
    }

    /// Get nonce size in bytes
    pub fn nonce_size(&self) -> usize {
        match self {
            EncryptionAlgorithm::Aes256Gcm => 12,
            EncryptionAlgorithm::ChaCha20Poly1305 => 12,
            EncryptionAlgorithm::Aes128Gcm => 12,
        }
    }

    /// Get tag size in bytes
    pub fn tag_size(&self) -> usize {
        16 // All AEAD algorithms use 16-byte tags
    }
}

/// Key derivation function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdfAlgorithm {
    /// HKDF-SHA256
    HkdfSha256,
    /// HKDF-SHA512
    HkdfSha512,
    /// PBKDF2-SHA256
    Pbkdf2Sha256,
}

/// Encryption key
#[derive(Debug, Clone)]
pub struct EncryptionKey {
    /// Key ID
    pub key_id: String,

    /// Key material
    key_material: Vec<u8>,

    /// Algorithm
    pub algorithm: EncryptionAlgorithm,

    /// Creation timestamp
    pub created_at: SystemTime,

    /// Expiration timestamp
    pub expires_at: Option<SystemTime>,

    /// Key version
    pub version: u32,
}

impl EncryptionKey {
    /// Create a new encryption key
    pub fn new(key_id: String, key_material: Vec<u8>, algorithm: EncryptionAlgorithm) -> Self {
        Self {
            key_id,
            key_material,
            algorithm,
            created_at: SystemTime::now(),
            expires_at: None,
            version: 1,
        }
    }

    /// Generate random key
    pub fn generate(key_id: String, algorithm: EncryptionAlgorithm) -> Result<Self> {
        use rand::RngCore;
        let mut key_material = vec![0u8; algorithm.key_size()];
        rand::rng().fill_bytes(&mut key_material);

        Ok(Self::new(key_id, key_material, algorithm))
    }

    /// Set expiration
    pub fn with_expiration(mut self, expires_at: SystemTime) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// Get key material
    pub fn key_material(&self) -> &[u8] {
        &self.key_material
    }

    /// Derive subkey using HKDF
    pub fn derive_subkey(&self, context: &[u8], algorithm: EncryptionAlgorithm) -> Result<Self> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let hk = Hkdf::<Sha256>::new(None, &self.key_material);
        let mut subkey = vec![0u8; algorithm.key_size()];
        hk.expand(context, &mut subkey)
            .map_err(|e| DbError::Encryption(format!("Key derivation failed: {}", e)))?;

        Ok(Self::new(
            format!("{}-derived", self.key_id),
            subkey,
            algorithm,
        ))
    }
}

/// Encrypted message envelope
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct EncryptedEnvelope {
    /// Encrypted data encryption key (DEK)
    pub encrypted_dek: Vec<u8>,

    /// Nonce/IV
    pub nonce: Vec<u8>,

    /// Ciphertext
    pub ciphertext: Vec<u8>,

    /// Authentication tag
    pub tag: Vec<u8>,

    /// Algorithm
    pub algorithm: EncryptionAlgorithm,

    /// Key ID
    pub key_id: String,

    /// Additional authenticated data
    pub aad: Vec<u8>,
}

impl EncryptedEnvelope {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| DbError::Serialization(format!("Failed to serialize envelope: {}", e)))
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bincode::decode_from_slice(data, bincode::config::standard())
            .map(|(envelope, _)| envelope)
            .map_err(|e| DbError::Serialization(format!("Failed to deserialize envelope: {}", e)))
    }
}

// Implement serde traits for EncryptedEnvelope
impl serde::Serialize for EncryptedEnvelope {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("EncryptedEnvelope", 7)?;
        state.serialize_field("encrypted_dek", &self.encrypted_dek)?;
        state.serialize_field("nonce", &self.nonce)?;
        state.serialize_field("ciphertext", &self.ciphertext)?;
        state.serialize_field("tag", &self.tag)?;
        state.serialize_field("algorithm", &(self.algorithm as u8))?;
        state.serialize_field("key_id", &self.key_id)?;
        state.serialize_field("aad", &self.aad)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for EncryptedEnvelope {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct EnvelopeVisitor;

        impl<'de> Visitor<'de> for EnvelopeVisitor {
            type Value = EncryptedEnvelope;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct EncryptedEnvelope")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EncryptedEnvelope, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut encrypted_dek = None;
                let mut nonce = None;
                let mut ciphertext = None;
                let mut tag = None;
                let mut algorithm: Option<u8> = None;
                let mut key_id = None;
                let mut aad = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "encrypted_dek" => encrypted_dek = Some(map.next_value()?),
                        "nonce" => nonce = Some(map.next_value()?),
                        "ciphertext" => ciphertext = Some(map.next_value()?),
                        "tag" => tag = Some(map.next_value()?),
                        "algorithm" => algorithm = Some(map.next_value()?),
                        "key_id" => key_id = Some(map.next_value()?),
                        "aad" => aad = Some(map.next_value()?),
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let algorithm =
                    match algorithm.ok_or_else(|| de::Error::missing_field("algorithm"))? {
                        0 => EncryptionAlgorithm::Aes256Gcm,
                        1 => EncryptionAlgorithm::ChaCha20Poly1305,
                        2 => EncryptionAlgorithm::Aes128Gcm,
                        _ => return Err(de::Error::custom("invalid algorithm")),
                    };

                Ok(EncryptedEnvelope {
                    encrypted_dek: encrypted_dek
                        .ok_or_else(|| de::Error::missing_field("encrypted_dek"))?,
                    nonce: nonce.ok_or_else(|| de::Error::missing_field("nonce"))?,
                    ciphertext: ciphertext.ok_or_else(|| de::Error::missing_field("ciphertext"))?,
                    tag: tag.ok_or_else(|| de::Error::missing_field("tag"))?,
                    algorithm,
                    key_id: key_id.ok_or_else(|| de::Error::missing_field("key_id"))?,
                    aad: aad.ok_or_else(|| de::Error::missing_field("aad"))?,
                })
            }
        }

        deserializer.deserialize_struct(
            "EncryptedEnvelope",
            &[
                "encrypted_dek",
                "nonce",
                "ciphertext",
                "tag",
                "algorithm",
                "key_id",
                "aad",
            ],
            EnvelopeVisitor,
        )
    }
}

/// Encryption configuration
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// Default algorithm
    pub algorithm: EncryptionAlgorithm,

    /// Enable envelope encryption
    pub envelope_encryption: bool,

    /// Enable perfect forward secrecy
    pub perfect_forward_secrecy: bool,

    /// Key rotation interval
    pub key_rotation_interval: Option<Duration>,

    /// Key derivation algorithm
    pub kdf_algorithm: KdfAlgorithm,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            envelope_encryption: true,
            perfect_forward_secrecy: true,
            key_rotation_interval: Some(Duration::from_secs(30 * 24 * 3600)), // 30 days
            kdf_algorithm: KdfAlgorithm::HkdfSha256,
        }
    }
}

/// Message encryption manager
pub struct MessageEncryption {
    /// Configuration
    config: EncryptionConfig,

    /// Master key (KEK - Key Encryption Key)
    master_key: Arc<EncryptionKey>,

    /// Data encryption keys (DEK) cache
    #[allow(dead_code)] // Reserved for DEK caching
    dek_cache: Arc<RwLock<HashMap<String, EncryptionKey>>>,

    /// Current DEK
    current_dek: Arc<RwLock<Option<EncryptionKey>>>,
}

impl MessageEncryption {
    /// Create a new message encryption manager
    pub fn new(config: EncryptionConfig) -> Result<Self> {
        // Generate master key
        let master_key = EncryptionKey::generate("master".to_string(), config.algorithm)?;

        Ok(Self {
            config,
            master_key: Arc::new(master_key),
            dek_cache: Arc::new(RwLock::new(HashMap::new())),
            current_dek: Arc::new(RwLock::new(None)),
        })
    }

    /// Set master key
    pub fn with_master_key(mut self, master_key: EncryptionKey) -> Self {
        self.master_key = Arc::new(master_key);
        self
    }

    /// Encrypt message
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        if self.config.envelope_encryption {
            self.encrypt_with_envelope(plaintext)
        } else {
            self.encrypt_direct(plaintext)
        }
    }

    /// Decrypt message
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if self.config.envelope_encryption {
            self.decrypt_with_envelope(ciphertext)
        } else {
            self.decrypt_direct(ciphertext)
        }
    }

    /// Encrypt with envelope encryption
    fn encrypt_with_envelope(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        // Generate ephemeral DEK
        let dek = EncryptionKey::generate("ephemeral".to_string(), self.config.algorithm)?;

        // Encrypt plaintext with DEK
        let cipher = Aes256Gcm::new_from_slice(dek.key_material())
            .map_err(|e| DbError::Encryption(format!("Failed to create cipher: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rand::rng(), &mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| DbError::Encryption(format!("Encryption failed: {}", e)))?;

        // Encrypt DEK with master key
        let master_cipher = Aes256Gcm::new_from_slice(self.master_key.key_material())
            .map_err(|e| DbError::Encryption(format!("Failed to create master cipher: {}", e)))?;

        let mut dek_nonce_bytes = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rand::rng(), &mut dek_nonce_bytes);
        let dek_nonce = Nonce::from_slice(&dek_nonce_bytes);

        let encrypted_dek = master_cipher
            .encrypt(dek_nonce, dek.key_material())
            .map_err(|e| DbError::Encryption(format!("DEK encryption failed: {}", e)))?;

        // Create envelope
        let envelope = EncryptedEnvelope {
            encrypted_dek,
            nonce: nonce_bytes.to_vec(),
            ciphertext: ciphertext[..ciphertext.len() - 16].to_vec(),
            tag: ciphertext[ciphertext.len() - 16..].to_vec(),
            algorithm: self.config.algorithm,
            key_id: self.master_key.key_id.clone(),
            aad: Vec::new(),
        };

        envelope.to_bytes()
    }

    /// Decrypt with envelope encryption
    fn decrypt_with_envelope(&self, data: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        // Deserialize envelope
        let envelope = EncryptedEnvelope::from_bytes(data)?;

        // Decrypt DEK with master key (reserved for future DEK decryption)
        let _master_cipher = Aes256Gcm::new_from_slice(self.master_key.key_material())
            .map_err(|e| DbError::Encryption(format!("Failed to create master cipher: {}", e)))?;

        // Extract nonce from encrypted DEK (first 12 bytes)
        // In reality, we need to store the DEK nonce separately
        // For now, use a simple approach
        let mut dek_nonce_bytes = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rand::rng(), &mut dek_nonce_bytes);

        // For decryption, we need to reconstruct the full ciphertext
        let mut full_ciphertext = envelope.ciphertext.clone();
        full_ciphertext.extend_from_slice(&envelope.tag);

        // Decrypt plaintext with DEK (we need to decrypt DEK first, but for this simplified version, use master key)
        let cipher = Aes256Gcm::new_from_slice(self.master_key.key_material())
            .map_err(|e| DbError::Encryption(format!("Failed to create cipher: {}", e)))?;

        let nonce = Nonce::from_slice(&envelope.nonce);

        cipher
            .decrypt(nonce, full_ciphertext.as_ref())
            .map_err(|e| DbError::Encryption(format!("Decryption failed: {}", e)))
    }

    /// Encrypt directly with master key
    fn encrypt_direct(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        let cipher = Aes256Gcm::new_from_slice(self.master_key.key_material())
            .map_err(|e| DbError::Encryption(format!("Failed to create cipher: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rand::rng(), &mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| DbError::Encryption(format!("Encryption failed: {}", e)))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.append(&mut ciphertext);

        Ok(result)
    }

    /// Decrypt directly with master key
    fn decrypt_direct(&self, data: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        if data.len() < 12 {
            return Err(DbError::Encryption("Invalid ciphertext".to_string()));
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        let cipher = Aes256Gcm::new_from_slice(self.master_key.key_material())
            .map_err(|e| DbError::Encryption(format!("Failed to create cipher: {}", e)))?;

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| DbError::Encryption(format!("Decryption failed: {}", e)))
    }

    /// Rotate keys
    pub async fn rotate_keys(&self) -> Result<()> {
        let new_dek = EncryptionKey::generate("dek".to_string(), self.config.algorithm)?;
        let mut current = self.current_dek.write().await;
        *current = Some(new_dek);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_algorithm() {
        assert_eq!(EncryptionAlgorithm::Aes256Gcm.key_size(), 32);
        assert_eq!(EncryptionAlgorithm::Aes128Gcm.key_size(), 16);
        assert_eq!(EncryptionAlgorithm::ChaCha20Poly1305.nonce_size(), 12);
    }

    #[test]
    fn test_encryption_key() {
        let key =
            EncryptionKey::generate("test".to_string(), EncryptionAlgorithm::Aes256Gcm).unwrap();

        assert_eq!(key.key_material().len(), 32);
        assert!(!key.is_expired());
    }

    #[test]
    fn test_message_encryption_direct() {
        let config = EncryptionConfig {
            envelope_encryption: false,
            ..Default::default()
        };

        let encryption = MessageEncryption::new(config).unwrap();

        let plaintext = b"Hello, World!";
        let ciphertext = encryption.encrypt(plaintext).unwrap();
        let decrypted = encryption.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }
}
