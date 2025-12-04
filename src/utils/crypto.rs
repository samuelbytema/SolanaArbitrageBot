use sha2::{Sha256, Sha512, Digest};
use hmac::{Hmac, Mac};
use aes::{Aes256, Block};
use aes::cipher::{
    BlockEncrypt, BlockDecrypt,
    KeyInit, generic_array::GenericArray,
};
use rand::{Rng, RngCore};
use base64::{Engine as _, engine::general_purpose};

/// Cryptographic utility functions
pub struct CryptoUtils;

impl CryptoUtils {
    /// Compute SHA256 hash
    pub fn sha256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
    
    /// Compute SHA512 hash
    pub fn sha512(data: &[u8]) -> [u8; 64] {
        let mut hasher = Sha512::new();
        hasher.update(data);
        hasher.finalize().into()
    }
    
    /// Compute HMAC-SHA256
    pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
        let mut mac = <Hmac<Sha256> as hmac::Mac>::new_from_slice(key)
            .expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().into()
    }
    
    /// Compute HMAC-SHA512
    pub fn hmac_sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
        let mut mac = <Hmac<Sha512> as hmac::Mac>::new_from_slice(key)
            .expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().into()
    }
    
    /// Generate random bytes
    pub fn random_bytes(length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut bytes = vec![0u8; length];
        rng.fill_bytes(&mut bytes);
        bytes
    }
    
    /// Generate random string
    pub fn random_string(length: usize) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                abcdefghijklmnopqrstuvwxyz\
                                0123456789)(*&^%$#@!~";
        let mut rng = rand::thread_rng();
        
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Generate random UUID
    pub fn random_uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Base64 encode
    pub fn base64_encode(data: &[u8]) -> String {
        general_purpose::STANDARD.encode(data)
    }
    
    /// Base64 decode
    pub fn base64_decode(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
        general_purpose::STANDARD.decode(encoded)
    }
    
    /// Compute file hash
    pub fn file_hash(file_path: &str) -> Result<[u8; 32], std::io::Error> {
        use std::fs::File;
        use std::io::Read;
        
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        
        Ok(Self::sha256(&buffer))
    }
    
    /// Verify file integrity
    pub fn verify_file_integrity(
        file_path: &str,
        expected_hash: &[u8; 32],
    ) -> Result<bool, std::io::Error> {
        let actual_hash = Self::file_hash(file_path)?;
        Ok(actual_hash == *expected_hash)
    }
    
    /// Generate password hash (PBKDF2)
    pub fn hash_password(password: &str, salt: &[u8]) -> [u8; 32] {
        use pbkdf2::{pbkdf2, Pbkdf2};
        use sha2::Sha256;
        
        let mut hash = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, 10000, &mut hash);
        hash
    }
    
    /// Verify password
    pub fn verify_password(password: &str, salt: &[u8], hash: &[u8]) -> bool {
        let computed_hash = Self::hash_password(password, salt);
        computed_hash == *hash
    }
    
    /// Generate salt
    pub fn generate_salt() -> [u8; 32] {
        Self::random_bytes(32).try_into().unwrap()
    }
    
    /// Constant-time string comparison (prevent timing attacks)
    pub fn secure_compare(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        
        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        
        result == 0
    }
}

/// AES encryption utilities
pub struct AesUtils;

impl AesUtils {
    /// AES-256 encrypt
    pub fn encrypt_aes256(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, String> {
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        let cipher = Aes256::new_from_slice(key)
            .map_err(|e| format!("Failed to create AES cipher: {}", e))?;
        
        let mut encrypted = Vec::new();
        let mut chunks = data.chunks_exact(16);
        
        // Encrypt full blocks
        for chunk in chunks.by_ref() {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.encrypt_block(&mut block);
            encrypted.extend_from_slice(&block);
        }
        
        // Handle the last partial block (PKCS7 padding)
        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            let padding_len = 16 - remainder.len();
            let mut padded_block = [0u8; 16];
            padded_block[..remainder.len()].copy_from_slice(remainder);
            padded_block[remainder.len()..].fill(padding_len as u8);
            
            let mut block = GenericArray::from(padded_block);
            cipher.encrypt_block(&mut block);
            encrypted.extend_from_slice(&block);
        }
        
        Ok(encrypted)
    }
    
    /// AES-256 decrypt
    pub fn decrypt_aes256(key: &[u8; 32], encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
        if encrypted_data.is_empty() {
            return Ok(Vec::new());
        }
        
        if encrypted_data.len() % 16 != 0 {
            return Err("Encrypted data length must be multiple of 16".to_string());
        }
        
        let cipher = Aes256::new_from_slice(key)
            .map_err(|e| format!("Failed to create AES cipher: {}", e))?;
        
        let mut decrypted = Vec::new();
        
        for chunk in encrypted_data.chunks_exact(16) {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.decrypt_block(&mut block);
            decrypted.extend_from_slice(&block);
        }
        
        // Remove PKCS7 padding
        if let Some(&last_byte) = decrypted.last() {
            if last_byte <= 16 {
                let padding_len = last_byte as usize;
                if decrypted.len() >= padding_len {
                    let padding_start = decrypted.len() - padding_len;
                    let padding = &decrypted[padding_start..];
                    if padding.iter().all(|&b| b == last_byte) {
                        decrypted.truncate(padding_start);
                    }
                }
            }
        }
        
        Ok(decrypted)
    }
    
    /// Generate random bytes
    pub fn random_bytes(len: usize) -> Vec<u8> {
        use rand::RngCore;
        let mut bytes = vec![0u8; len];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes
    }

    /// Generate AES key
    pub fn generate_aes_key() -> [u8; 32] {
        Self::random_bytes(32).try_into().unwrap()
    }
}

/// Key derivation utilities
pub struct KeyDerivationUtils;

impl KeyDerivationUtils {
    /// Derive key from password
    pub fn derive_key_from_password(
        password: &str,
        salt: &[u8],
        iterations: u32,
        key_length: usize,
    ) -> Vec<u8> {
        use pbkdf2::{pbkdf2, Pbkdf2};
        use sha2::Sha256;
        
        let mut key = vec![0u8; key_length];
        pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, iterations, &mut key);
        key
    }
    
    /// Generate random salt
    pub fn generate_salt(length: usize) -> Vec<u8> {
        Self::random_bytes(length)
    }
    
    /// Secure key generation
    pub fn generate_secure_key(length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut key = vec![0u8; length];
        rng.fill_bytes(&mut key);
        key
    }
}

impl KeyDerivationUtils {
    fn random_bytes(length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut bytes = vec![0u8; length];
        rng.fill_bytes(&mut bytes);
        bytes
    }
}

/// Digital signature utilities
pub struct SignatureUtils;

impl SignatureUtils {
    /// Create digital signature
    pub fn create_signature(private_key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
        use ed25519_dalek::{Keypair, SecretKey, PublicKey, Signer};
        
        let secret = SecretKey::from_bytes(private_key)
            .map_err(|e| format!("Invalid private key: {}", e))?;
        let public = PublicKey::from(&secret);
        let keypair = Keypair { secret, public };
        
        let signature = keypair.sign(data);
        Ok(signature.to_bytes().to_vec())
    }
    
    /// Verify digital signature
    pub fn verify_signature(
        public_key: &[u8],
        data: &[u8],
        signature: &[u8],
    ) -> Result<bool, String> {
        use ed25519_dalek::{PublicKey, Verifier};
        
        let public_key = PublicKey::from_bytes(public_key)
            .map_err(|e| format!("Invalid public key: {}", e))?;
        
        let signature = ed25519_dalek::Signature::from_bytes(signature)
            .map_err(|e| format!("Invalid signature: {}", e))?;
        
        Ok(public_key.verify(data, &signature).is_ok())
    }
    
    /// Generate keypair
    pub fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
        use ed25519_dalek::{Keypair, SecretKey, PublicKey};
        use rand::RngCore;
        
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        let secret = SecretKey::from_bytes(&seed).map_err(|e| format!("Invalid secret key: {}", e)).unwrap();
        let public = PublicKey::from(&secret);
        let keypair = Keypair { secret, public };
        
        (
            keypair.secret.to_bytes().to_vec(),
            keypair.public.to_bytes().to_vec(),
        )
    }
}

/// Hash utilities
pub struct HashUtils;

impl HashUtils {
    /// Compute hash of a string
    pub fn hash_string(input: &str) -> String {
        let hash = CryptoUtils::sha256(input.as_bytes());
        hex::encode(hash)
    }
    
    /// Compute hash of a file
    pub fn hash_file(file_path: &str) -> Result<String, std::io::Error> {
        let hash = CryptoUtils::file_hash(file_path)?;
        Ok(hex::encode(hash))
    }
    
    /// Verify string hash
    pub fn verify_string_hash(input: &str, expected_hash: &str) -> bool {
        let actual_hash = Self::hash_string(input);
        actual_hash == expected_hash
    }
    
    /// Verify file hash
    pub fn verify_file_hash(file_path: &str, expected_hash: &str) -> Result<bool, std::io::Error> {
        let actual_hash = Self::hash_file(file_path)?;
        Ok(actual_hash == expected_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sha256() {
        let data = b"Hello, World!";
        let hash = CryptoUtils::sha256(data);
        assert_eq!(hash.len(), 32);
        
        // Verify the same input produces the same hash
        let hash2 = CryptoUtils::sha256(data);
        assert_eq!(hash, hash2);
    }
    
    #[test]
    fn test_hmac() {
        let key = b"secret_key";
        let data = b"Hello, World!";
        let hmac = CryptoUtils::hmac_sha256(key, data);
        assert_eq!(hmac.len(), 32);
    }
    
    #[test]
    fn test_random_bytes() {
        let bytes1 = CryptoUtils::random_bytes(32);
        let bytes2 = CryptoUtils::random_bytes(32);
        
        assert_eq!(bytes1.len(), 32);
        assert_eq!(bytes2.len(), 32);
        assert_ne!(bytes1, bytes2); // Randomness test
    }
    
    #[test]
    fn test_base64() {
        let data = b"Hello, World!";
        let encoded = CryptoUtils::base64_encode(data);
        let decoded = CryptoUtils::base64_decode(&encoded).unwrap();
        
        assert_eq!(data, decoded.as_slice());
    }
    
    #[test]
    fn test_aes_encryption() {
        let key = AesUtils::generate_aes_key();
        let data = b"This is a test message for AES encryption!";
        
        let encrypted = AesUtils::encrypt_aes256(&key, data).unwrap();
        let decrypted = AesUtils::decrypt_aes256(&key, &encrypted).unwrap();
        
        assert_eq!(data, decrypted.as_slice());
    }
    
    #[test]
    fn test_password_hashing() {
        let password = "my_password";
        let salt = CryptoUtils::generate_salt();
        
        let hash = CryptoUtils::hash_password(password, &salt);
        let is_valid = CryptoUtils::verify_password(password, &salt, &hash);
        
        assert!(is_valid);
    }
    
    #[test]
    fn test_secure_compare() {
        let a = b"hello";
        let b = b"hello";
        let c = b"world";
        
        assert!(CryptoUtils::secure_compare(a, b));
        assert!(!CryptoUtils::secure_compare(a, c));
    }
    
    #[test]
    fn test_signature() {
        let (private_key, public_key) = SignatureUtils::generate_keypair();
        let data = b"Hello, World!";
        
        let signature = SignatureUtils::create_signature(&private_key, data).unwrap();
        let is_valid = SignatureUtils::verify_signature(&public_key, data, &signature).unwrap();
        
        assert!(is_valid);
    }
}
