// # Cryptographic Primitives for Blockchain Tables
//
// This module provides cryptographic functions for blockchain table operations including:
// - Hash computations (SHA-256, SHA-3)
// - Digital signatures (Ed25519-like)
// - Hash chain verification
// - Merkle tree operations
// - Zero-knowledge proof concepts
// - Cryptographic accumulators
// - Key derivation functions
// - Secure random generation

use std::fmt;
use sha2::{Sha256, Sha512, Digest};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::error::DbError;

// ============================================================================
// Type Aliases
// ============================================================================

// SHA-256 hash output (32 bytes)
pub type Hash256 = [u8; 32];

// SHA-512 hash output (64 bytes)
pub type Hash512 = [u8; 64];

// Digital signature (64 bytes for Ed25519-like)
pub type Signature = [u8; 64];

// Public key (32 bytes)
pub type PublicKey = [u8; 32];

// Private key (32 bytes)
pub type PrivateKey = [u8; 32];

// Nonce for cryptographic operations
pub type Nonce = [u8; 24];

// ============================================================================
// Hash Algorithms
// ============================================================================

// Supported hash algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashAlgorithm {
    // SHA-256 (default for blockchain)
    Sha256,
    // SHA-512 for enhanced security
    Sha512,
    // HMAC-SHA256 for keyed hashing
    HmacSha256,
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        HashAlgorithm::Sha256
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HashAlgorithm::Sha256 => write!(f, "SHA-256"),
            HashAlgorithm::Sha512 => write!(f, "SHA-512"),
            HashAlgorithm::HmacSha256 => write!(f, "HMAC-SHA256"),
        }
    }
}

// ============================================================================
// Cryptographic Hash Functions
// ============================================================================

// Compute SHA-256 hash of data
pub fn sha256(data: &[u8]) -> Hash256 {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

// Compute SHA-512 hash of data
pub fn sha512(data: &[u8]) -> Hash512 {
    let mut hasher = Sha512::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 64];
    hash.copy_from_slice(&result);
    hash
}

// Compute HMAC-SHA256 with a key
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Hash256> {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| DbError::Internal(format!("HMAC key error: {}", e)))?;
    mac.update(data);
    let result = mac.finalize();
    let bytes = result.into_bytes();

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}

// Compute hash using specified algorithm
pub fn compute_hash(algorithm: HashAlgorithm, data: &[u8], key: Option<&[u8]>) -> Result<Vec<u8>> {
    match algorithm {
        HashAlgorithm::Sha256 => Ok(sha256(data).to_vec()),
        HashAlgorithm::Sha512 => Ok(sha512(data).to_vec()),
        HashAlgorithm::HmacSha256 => {
            let key = key.ok_or_else(|| DbError::InvalidInput("HMAC requires a key".to_string()))?;
            Ok(hmac_sha256(key, data)?.to_vec())
        }
    }
}

// ============================================================================
// Hash Chaining
// ============================================================================

// Represents a link in a hash chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainLink {
    // Index in the chain
    pub index: u64,
    // Hash of previous link
    pub previous_hash: Hash256,
    // Data hash
    pub data_hash: Hash256,
    // Combined hash of this link
    pub link_hash: Hash256,
    // Timestamp
    pub timestamp: u64,
}

impl ChainLink {
    // Create a new chain link
    pub fn new(index: u64, previous_hash: Hash256, data: &[u8], timestamp: u64) -> Self {
        let data_hash = sha256(data);
        let link_hash = Self::compute_link_hash(index, &previous_hash, &data_hash, timestamp);

        Self {
            index,
            previous_hash,
            data_hash,
            link_hash,
            timestamp,
        }
    }

    // Compute the hash for this link
    fn compute_link_hash(index: u64, prevhash: &Hash256, datahash: &Hash256, timestamp: u64) -> Hash256 {
        let mut hasher = Sha256::new();
        hasher.update(&index.to_le_bytes());
        hasher.update(prevhash);
        hasher.update(datahash);
        hasher.update(&timestamp.to_le_bytes());

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    // Verify this link's integrity
    pub fn verify(&self) -> bool {
        let computed = Self::compute_link_hash(self.index, &self.previous_hash, &self.data_hash, self.timestamp);
        computed == self.link_hash
    }

    // Verify this link connects to a previous link
    pub fn verify_connection(&self, previous: &ChainLink) -> bool {
        self.previous_hash == previous.link_hash && self.index == previous.index + 1
    }
}

// Hash chain for sequential data verification
#[derive(Debug, Clone)]
pub struct HashChain {
    // Chain links
    links: Vec<ChainLink>,
    // Algorithm used
    algorithm: HashAlgorithm,
}

impl HashChain {
    // Create a new hash chain
    pub fn new(algorithm: HashAlgorithm) -> Self {
        Self {
            links: Vec::new(),
            algorithm,
        }
    }

    // Get genesis hash (all zeros)
    pub fn genesis_hash() -> Hash256 {
        [0u8; 32]
    }

    // Append data to the chain
    pub fn append(&mut self, data: &[u8], timestamp: u64) -> ChainLink {
        let index = self.links.len() as u64;
        let previous_hash = if index == 0 {
            Self::genesis_hash()
        } else {
            self.links.last().unwrap().link_hash
        };

        let link = ChainLink::new(index, previous_hash, data, timestamp);
        self.links.push(link.clone());
        link
    }

    // Verify the entire chain
    pub fn verify(&self) -> Result<bool> {
        if self.links.is_empty() {
            return Ok(true);
        }

        // Verify first link
        if !self.links[0].verify() {
            return Ok(false);
        }

        if self.links[0].index != 0 {
            return Ok(false);
        }

        if self.links[0].previous_hash != Self::genesis_hash() {
            return Ok(false);
        }

        // Verify remaining links
        for i in 1..self.links.len() {
            if !self.links[i].verify() {
                return Ok(false);
            }

            if !self.links[i].verify_connection(&self.links[i - 1]) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    // Get chain length
    pub fn len(&self) -> usize {
        self.links.len()
    }

    // Check if chain is empty
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    // Get link at index
    pub fn get_link(&self, index: usize) -> Option<&ChainLink> {
        self.links.get(index)
    }

    // Get latest link
    pub fn latest_link(&self) -> Option<&ChainLink> {
        self.links.last()
    }
}

// ============================================================================
// Merkle Tree
// ============================================================================

// Merkle tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    // Hash of this node
    pub hash: Hash256,
    // Left child hash (if internal node)
    pub left: Option<Hash256>,
    // Right child hash (if internal node)
    pub right: Option<Hash256>,
}

impl MerkleNode {
    // Create a leaf node
    pub fn leaf(data: &[u8]) -> Self {
        Self {
            hash: sha256(data),
            left: None,
            right: None,
        }
    }

    // Create an internal node
    pub fn internal(lefthash: Hash256, righthash: Hash256) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&lefthash);
        hasher.update(&righthash);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);

        Self {
            hash,
            left: Some(lefthash),
            right: Some(righthash),
        }
    }
}

// Merkle proof for inclusion verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    // Index of the leaf
    pub leaf_index: usize,
    // Sibling hashes on path to root
    pub siblings: Vec<(Hash256, bool)>, // (hash, is_left)
    // Root hash
    pub root: Hash256,
}

impl MerkleProof {
    // Verify that data is included in the tree
    pub fn verify(&self, data: &[u8]) -> bool {
        let mut current_hash = sha256(data);

        for (sibling_hash, is_left) in &self.siblings {
            let mut hasher = Sha256::new();
            if *is_left {
                hasher.update(sibling_hash);
                hasher.update(&current_hash);
            } else {
                hasher.update(&current_hash);
                hasher.update(sibling_hash);
            }
            let result = hasher.finalize();
            current_hash.copy_from_slice(&result);
        }

        current_hash == self.root
    }
}

// Merkle tree for efficient integrity verification
#[derive(Debug, Clone)]
pub struct MerkleTree {
    // Leaf nodes (data hashes)
    leaves: Vec<Hash256>,
    // All tree levels (bottom-up)
    levels: Vec<Vec<Hash256>>,
    // Root hash
    root: Hash256,
}

impl MerkleTree {
    // Build a Merkle tree from data items
    pub fn build(data_items: &[&[u8]]) -> Result<Self> {
        if data_items.is_empty() {
            return Err(DbError::InvalidInput("Cannot build Merkle tree from empty data".to_string()));
        }

        // Create leaf hashes
        let leaves: Vec<Hash256> = data_items.iter().map(|data| sha256(data)).collect();

        let mut levels = vec![leaves.clone()];
        let mut current_level = leaves.clone();

        // Build tree bottom-up
        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    // Duplicate last hash if odd number
                    current_level[i]
                };

                let node = MerkleNode::internal(left, right);
                next_level.push(node.hash);
            }

            levels.push(next_level.clone());
            current_level = next_level;
        }

        let root = current_level[0];

        Ok(Self {
            leaves,
            levels,
            root,
        })
    }

    // Get the root hash
    pub fn root(&self) -> Hash256 {
        self.root
    }

    // Generate a proof of inclusion for a leaf
    pub fn generate_proof(&self, leafindex: usize) -> Result<MerkleProof> {
        if leafindex >= self.leaves.len() {
            return Err(DbError::InvalidInput(format!("Leaf index {} out of bounds", leafindex)));
        }

        let mut siblings = Vec::new();
        let mut index = leafindex;

        // Traverse from leaf to root
        for level in &self.levels[..self.levels.len() - 1] {
            let sibling_index = if index % 2 == 0 {
                index + 1
            } else {
                index - 1
            };

            let sibling_hash = if sibling_index < level.len() {
                level[sibling_index]
            } else {
                level[index] // Duplicate if at end
            };

            let is_left = index % 2 == 1;
            siblings.push((sibling_hash, is_left));

            index /= 2;
        }

        Ok(MerkleProof {
            leaf_index: leafindex,
            siblings,
            root: self.root,
        })
    }

    // Verify a proof
    pub fn verify_proof(&self, proof: &MerkleProof, data: &[u8]) -> bool {
        proof.verify(data) && proof.root == self.root
    }

    // Get number of leaves
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }
}

// ============================================================================
// Digital Signatures (Simplified Ed25519-like)
// ============================================================================

// Key pair for digital signatures
#[derive(Debug, Clone)]
pub struct KeyPair {
    // Public key
    pub public_key: PublicKey,
    // Private key (should be kept secret)
    private_key: PrivateKey,
}

impl KeyPair {
    // Generate a new key pair (simplified - not production-ready)
    pub fn generate() -> Self {
        let mut private_key = [0u8; 32];

        // Use secure random generation
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut private_key);

        // Derive public key from private (simplified)
        let public_key = sha256(&private_key);

        Self {
            public_key,
            private_key,
        }
    }

    // Sign a message (simplified - not production-ready)
    pub fn sign(&self, message: &[u8]) -> Signature {
        let mut signature = [0u8; 64];

        // Create signature by combining message hash with private key hash
        let message_hash = sha256(message);
        let key_hash = sha256(&self.private_key);

        signature[..32].copy_from_slice(&message_hash);
        signature[32..].copy_from_slice(&key_hash);

        signature
    }

    // Get public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
}

// Verify a signature (simplified - not production-ready)
pub fn verify_signature(_public_key: &PublicKey, message: &[u8], signature: &Signature) -> bool {
    // Extract message hash from signature
    let mut claimed_message_hash = [0u8; 32];
    claimed_message_hash.copy_from_slice(&signature[..32]);

    // Compute actual message hash
    let actual_message_hash = sha256(message);

    // Verify message hash matches
    claimed_message_hash == actual_message_hash
}

// ============================================================================
// Cryptographic Accumulator
// ============================================================================

// Cryptographic accumulator for set membership proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Accumulator {
    // Current accumulator value
    value: Hash256,
    // Set of accumulated elements
    elements: Vec<Hash256>,
}

impl Accumulator {
    // Create a new empty accumulator
    pub fn new() -> Self {
        Self {
            value: [0u8; 32],
            elements: Vec::new(),
        }
    }

    // Add an element to the accumulator
    pub fn add(&mut self, element: &[u8]) {
        let element_hash = sha256(element);

        // Combine current value with new element
        let mut hasher = Sha256::new();
        hasher.update(&self.value);
        hasher.update(&element_hash);
        let result = hasher.finalize();
        self.value.copy_from_slice(&result);

        self.elements.push(element_hash);
    }

    // Get current accumulator value
    pub fn value(&self) -> &Hash256 {
        &self.value
    }

    // Generate membership proof for an element
    pub fn generate_membership_proof(&self, element: &[u8]) -> Option<Vec<Hash256>> {
        let element_hash = sha256(element);

        if !self.elements.contains(&element_hash) {
            return None;
        }

        // Simple proof: all other elements
        Some(self.elements.iter()
            .filter(|&&h| h != element_hash)
            .copied()
            .collect())
    }

    // Verify membership proof
    pub fn verify_membership(&self, element: &[u8], proof: &[Hash256]) -> bool {
        let element_hash = sha256(element);

        // Reconstruct accumulator value
        let mut value = [0u8; 32];
        let mut hasher = Sha256::new();
        hasher.update(&value);
        hasher.update(&element_hash);
        let result = hasher.finalize();
        value.copy_from_slice(&result);

        for proof_hash in proof {
            hasher = Sha256::new();
            hasher.update(&value);
            hasher.update(proof_hash);
            let result = hasher.finalize();
            value.copy_from_slice(&result);
        }

        value == self.value
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Key Derivation
// ============================================================================

// Derive a key from a master key and context
pub fn derive_key(masterkey: &[u8], context: &[u8], index: u64) -> Hash256 {
    let mut hasher = Sha256::new();
    hasher.update(masterkey);
    hasher.update(context);
    hasher.update(&index.to_le_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

// HKDF-like key derivation
pub fn hkdf_expand(prk: &[u8], info: &[u8], outputlen: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(outputlen);
    let mut counter = 1u8;

    while output.len() < outputlen {
        let mut hasher = Sha256::new();
        hasher.update(prk);
        hasher.update(info);
        hasher.update(&[counter]);
        let result = hasher.finalize();

        let bytes_needed = outputlen - output.len();
        let bytes_to_copy = bytes_needed.min(32);
        output.extend_from_slice(&result[..bytes_to_copy]);

        counter += 1;
    }

    output
}

// ============================================================================
// Zero-Knowledge Proof Concepts
// ============================================================================

// Simple commitment scheme for zero-knowledge proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    // Commitment value
    pub value: Hash256,
    // Blinding factor
    blinding: Hash256,
}

impl Commitment {
    // Create a commitment to a value
    pub fn commit(value: &[u8]) -> (Self, Hash256) {
        // Generate random blinding factor
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut blinding = [0u8; 32];
        rng.fill_bytes(&mut blinding);

        // Compute commitment
        let mut hasher = Sha256::new();
        hasher.update(value);
        hasher.update(&blinding);
        let result = hasher.finalize();
        let mut commitment_value = [0u8; 32];
        commitment_value.copy_from_slice(&result);

        let commitment = Self {
            value: commitment_value,
            blinding,
        };

        (commitment, blinding)
    }

    // Verify a commitment opening
    pub fn verify(&self, value: &[u8], blinding: &Hash256) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(value);
        hasher.update(blinding);
        let result = hasher.finalize();
        let mut computed = [0u8; 32];
        computed.copy_from_slice(&result);

        computed == self.value
    }
}

// Range proof concept (simplified)
#[derive(Debug, Clone)]
pub struct RangeProof {
    // Claimed range
    pub min: u64,
    pub max: u64,
    // Proof data
    proof_data: Vec<u8>,
}

impl RangeProof {
    // Create a range proof (simplified)
    pub fn create(value: u64, min: u64, max: u64) -> Result<Self> {
        if value < min || value > max {
            return Err(DbError::InvalidInput("Value outside range".to_string()));
        }

        // Simplified proof: just hash the value with range
        let mut hasher = Sha256::new();
        hasher.update(&value.to_le_bytes());
        hasher.update(&min.to_le_bytes());
        hasher.update(&max.to_le_bytes());
        let result = hasher.finalize();

        Ok(Self {
            min,
            max,
            proof_data: result.to_vec(),
        })
    }

    // Verify range proof (simplified)
    pub fn verify(&self) -> bool {
        // In a real implementation, this would verify without knowing the value
        !self.proof_data.is_empty()
    }
}

// ============================================================================
// Secure Random Generation
// ============================================================================

// Generate secure random bytes
pub fn secure_random(size: usize) -> Vec<u8> {
    use rand::{thread_rng, RngCore};

    let mut bytes = vec![0u8; size];
    let mut rng = thread_rng();
    rng.fill_bytes(&mut bytes);
    bytes
}

// Generate a secure random hash
pub fn random_hash256() -> Hash256 {
    let random_bytes = secure_random(32);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&random_bytes);
    hash
}

// Generate a nonce for cryptographic operations
pub fn generate_nonce() -> Nonce {
    let random_bytes = secure_random(24);
    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&random_bytes);
    nonce
}

// ============================================================================
// Utility Functions
// ============================================================================

// Convert hash to hex string
pub fn hash_to_hex(hash: &[u8]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

// Parse hex string to hash
pub fn hex_to_hash(hex: &str) -> Result<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return Err(DbError::InvalidInput("Hex string must have even length".to_string()));
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex[i..i + 2], 16)
            .map_err(|e| DbError::InvalidInput(format!("Invalid hex string: {}", e)))?;
        bytes.push(byte);
    }

    Ok(bytes)
}

// Constant-time comparison to prevent timing attacks
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let data = b"Hello, World!";
        let hash = sha256(data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_hash_chain() {
        let mut chain = HashChain::new(HashAlgorithm::Sha256);

        chain.append(b"block1", 1000);
        chain.append(b"block2", 2000);
        chain.append(b"block3", 3000);

        assert_eq!(chain.len(), 3);
        assert!(chain.verify().unwrap());
    }

    #[test]
    fn test_merkle_tree() {
        let data = vec![b"data1".as_ref(), b"data2", b"data3", b"data4"];
        let tree = MerkleTree::build(&data).unwrap();

        assert_eq!(tree.leaf_count(), 4);

        let proof = tree.generate_proof(1).unwrap();
        assert!(proof.verify(b"data2"));
    }

    #[test]
    fn test_key_pair() {
        let keypair = KeyPair::generate();
        let message = b"test message";

        let signature = keypair.sign(message);
        assert!(verify_signature(&keypair.public_key, message, &signature));
    }

    #[test]
    fn test_accumulator() {
        let mut acc = Accumulator::new();

        acc.add(b"element1");
        acc.add(b"element2");
        acc.add(b"element3");

        let proof = acc.generate_membership_proof(b"element2").unwrap();
        assert!(acc.verify_membership(b"element2", &proof));
    }

    #[test]
    fn test_commitment() {
        let value = b"secret value";
        let (commitment, blinding) = Commitment::commit(value);

        assert!(commitment.verify(value, &blinding));
        assert!(!commitment.verify(b"wrong value", &blinding));
    }
}
