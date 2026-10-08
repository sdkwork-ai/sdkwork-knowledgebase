use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use hkdf::Hkdf;
use sdkwork_utils_rust::is_blank;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;

/// Family prefix shared by every encrypted envelope version (`kbenc:v1:`/`kbenc:v2:`). Request
/// validation must reject user-supplied values that begin with it so a tenant cannot store an
/// envelope as a "secret" and permanently brick its config.
pub const ENCRYPTED_VALUE_PREFIX: &str = "kbenc:";
/// Legacy v1 envelope: `kbenc:v1:<nonce||ct base64>` with an unsalted SHA-256 key. Kept for
/// backward reading only; new writes use the salted v2 format.
const ENCRYPTED_V1_PREFIX: &str = "kbenc:v1:";
/// Current v2 envelope: `kbenc:v2:<salt_b64>:<nonce_b64>:<ct_b64>` with an HKDF-SHA256 key and a
/// per-blob random salt, so equal secrets under one master key never share a derived key.
const ENCRYPTED_V2_PREFIX: &str = "kbenc:v2:";
const NONCE_LEN: usize = 12;
const V2_SALT_LEN: usize = 16;
/// HKDF info label binding the v2 derived key to this cipher's purpose and version.
const V2_KEY_INFO: &[u8] = b"sdkwork-knowledgebase wechat secret cipher v2 aes-256-gcm";
const MAX_KEY_MATERIAL_FILE_BYTES: u64 = 4 * 1024;

#[cfg(test)]
pub(crate) static SECRET_CIPHER_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Debug, Error)]
pub enum SecretCipherError {
    #[error("secret encryption failed: {0}")]
    Encrypt(String),
    #[error("secret decryption failed: {0}")]
    Decrypt(String),
    #[error("secret encryption key is not configured")]
    MissingKey,
}

/// Returns true when a master key is available from env or key file.
pub fn encryption_key_configured() -> bool {
    resolve_key_material().is_some()
}

/// Encrypts a secret for at-rest storage. Requires a configured master key.
pub fn encrypt_secret(plaintext: &str) -> Result<String, SecretCipherError> {
    if is_blank(Some(plaintext)) {
        return Ok(String::new());
    }
    if plaintext.starts_with(ENCRYPTED_VALUE_PREFIX) {
        return Ok(plaintext.to_string());
    }
    let Some(key_material) = resolve_key_material() else {
        return Err(SecretCipherError::MissingKey);
    };

    let mut salt = [0u8; V2_SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let data_key = derive_v2_key(&key_material, &salt)
        .map_err(SecretCipherError::Encrypt)?;
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&data_key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|error| SecretCipherError::Encrypt(error.to_string()))?;

    Ok(format!(
        "{ENCRYPTED_V2_PREFIX}{}:{}:{}",
        encode_b64(&salt),
        encode_b64(&nonce_bytes),
        encode_b64(&ciphertext),
    ))
}

/// Decrypts a stored secret. Plaintext legacy values pass through unchanged; both the current
/// v2 envelope and the legacy unsalted v1 envelope decrypt through their own key derivation.
pub fn decrypt_secret(value: &str) -> Result<String, SecretCipherError> {
    if is_blank(Some(value)) {
        return Ok(String::new());
    }
    if let Some(payload) = value.strip_prefix(ENCRYPTED_V2_PREFIX) {
        return decrypt_v2(payload);
    }
    if !value.starts_with(ENCRYPTED_V1_PREFIX) {
        return Ok(value.to_string());
    }

    let Some(key_material) = resolve_key_material() else {
        return Err(SecretCipherError::MissingKey);
    };

    let encoded = value.strip_prefix(ENCRYPTED_V1_PREFIX).unwrap_or(value);
    let packed = decode_b64(encoded)?;
    if packed.len() <= NONCE_LEN {
        return Err(SecretCipherError::Decrypt(
            "ciphertext shorter than nonce length".to_string(),
        ));
    }

    let (nonce_bytes, ciphertext) = packed.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&derive_v1_key(&key_material)));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|error| SecretCipherError::Decrypt(error.to_string()))?;
    String::from_utf8(plaintext).map_err(|error| SecretCipherError::Decrypt(error.to_string()))
}

fn decrypt_v2(payload: &str) -> Result<String, SecretCipherError> {
    let Some(key_material) = resolve_key_material() else {
        return Err(SecretCipherError::MissingKey);
    };

    let segments: Vec<&str> = payload.split(':').collect();
    if segments.len() != 3 {
        return Err(SecretCipherError::Decrypt(
            "v2 envelope must be <salt>:<nonce>:<ciphertext> base64".to_string(),
        ));
    }
    let salt = decode_b64(segments[0])?;
    let nonce_bytes = decode_b64(segments[1])?;
    let ciphertext = decode_b64(segments[2])?;
    if salt.len() != V2_SALT_LEN {
        return Err(SecretCipherError::Decrypt(
            "v2 envelope salt has an unexpected length".to_string(),
        ));
    }
    if nonce_bytes.len() != NONCE_LEN {
        return Err(SecretCipherError::Decrypt(
            "v2 envelope nonce has an unexpected length".to_string(),
        ));
    }

    let data_key =
        derive_v2_key(&key_material, &salt).map_err(SecretCipherError::Decrypt)?;
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&data_key));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_slice())
        .map_err(|error| SecretCipherError::Decrypt(error.to_string()))?;
    String::from_utf8(plaintext).map_err(|error| SecretCipherError::Decrypt(error.to_string()))
}

pub fn encrypt_optional_secret(value: Option<String>) -> Result<Option<String>, SecretCipherError> {
    match value {
        None => Ok(None),
        Some(entry) => encrypt_secret(entry.as_str()).map(Some),
    }
}

pub fn decrypt_optional_secret(value: Option<String>) -> Result<Option<String>, SecretCipherError> {
    match value {
        None => Ok(None),
        Some(entry) => decrypt_secret(entry.as_str()).map(Some),
    }
}

fn resolve_key_material() -> Option<Vec<u8>> {
    if let Ok(path) = std::env::var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE") {
        if !is_blank(Some(path.as_str())) {
            if let Some(contents) = read_bounded_key_material(Path::new(path.trim())) {
                return Some(contents);
            }
        }
    }

    std::env::var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY")
        .ok()
        .filter(|value| !is_blank(Some(value.as_str())))
        .map(|value| value.trim().as_bytes().to_vec())
}

fn read_bounded_key_material(path: &Path) -> Option<Vec<u8>> {
    let file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_KEY_MATERIAL_FILE_BYTES {
        return None;
    }

    let mut contents = String::with_capacity(metadata.len() as usize);
    file.take(MAX_KEY_MATERIAL_FILE_BYTES + 1)
        .read_to_string(&mut contents)
        .ok()?;
    if contents.len() as u64 > MAX_KEY_MATERIAL_FILE_BYTES {
        return None;
    }
    let trimmed = contents.trim();
    (!is_blank(Some(trimmed))).then(|| trimmed.as_bytes().to_vec())
}

/// v2 key derivation: HKDF-SHA256 over the master key with a per-blob random salt, so the data
/// key is bound to both the key material and the envelope it encrypts.
fn derive_v2_key(material: &[u8], salt: &[u8]) -> Result<[u8; 32], String> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), material);
    let mut okm = [0u8; 32];
    hkdf.expand(V2_KEY_INFO, &mut okm)
        .map_err(|error| error.to_string())?;
    Ok(okm)
}

/// Legacy v1 key derivation (unsalted SHA-256). Used only to read existing v1 envelopes.
fn derive_v1_key(material: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(material);
    digest.into()
}

fn encode_b64(bytes: &[u8]) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
}

fn decode_b64(value: &str) -> Result<Vec<u8>, SecretCipherError> {
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value)
        .map_err(|error| SecretCipherError::Decrypt(error.to_string()))
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::SECRET_CIPHER_TEST_LOCK;

    pub struct TestEncryptionKeyGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TestEncryptionKeyGuard {
        pub fn with_key(key: &str) -> Self {
            let lock = SECRET_CIPHER_TEST_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            std::env::set_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY", key);
            Self { _lock: lock }
        }

        pub fn without_key() -> Self {
            let lock = SECRET_CIPHER_TEST_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            std::env::remove_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY");
            std::env::remove_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE");
            Self { _lock: lock }
        }
    }

    impl Drop for TestEncryptionKeyGuard {
        fn drop(&mut self) {
            std::env::remove_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY");
            std::env::remove_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::TestEncryptionKeyGuard;
    use super::*;

    #[test]
    fn encrypt_roundtrip_when_key_configured() {
        let _guard = TestEncryptionKeyGuard::with_key("integration-test-master-key");
        let encrypted = encrypt_secret("super-secret").expect("encrypt secret");
        assert!(encrypted.starts_with(ENCRYPTED_V2_PREFIX));
        let decrypted = decrypt_secret(&encrypted).expect("decrypt secret");
        assert_eq!(decrypted, "super-secret");
    }

    #[test]
    fn encrypt_requires_master_key() {
        let _guard = TestEncryptionKeyGuard::without_key();
        let error = encrypt_secret("super-secret").expect_err("encrypt without key");
        assert!(matches!(error, SecretCipherError::MissingKey));
    }

    #[test]
    fn key_file_is_bounded_before_reading() {
        let _guard = TestEncryptionKeyGuard::without_key();
        let path = std::env::temp_dir().join(format!(
            "sdkwork-knowledgebase-secret-key-{}",
            std::process::id()
        ));
        std::fs::write(&path, "bounded-integration-test-master-key").unwrap();
        std::env::set_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE", &path);
        assert!(encryption_key_configured());

        std::fs::write(&path, vec![b'x'; MAX_KEY_MATERIAL_FILE_BYTES as usize + 1]).unwrap();
        assert!(!encryption_key_configured());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn plaintext_legacy_values_pass_through_on_decrypt() {
        let _guard = TestEncryptionKeyGuard::without_key();
        let value = "legacy-plain-secret";
        assert_eq!(decrypt_secret(value).expect("legacy decrypt"), value);
    }

    #[test]
    fn equal_secrets_never_share_a_derived_v2_key() {
        let _guard = TestEncryptionKeyGuard::with_key("salt-uniqueness-master-key");
        let first = encrypt_secret("same-secret").expect("first encrypt");
        let second = encrypt_secret("same-secret").expect("second encrypt");
        assert_ne!(first, second, "random salt and nonce must change the envelope");

        assert_eq!(
            decrypt_secret(&first).expect("first decrypt"),
            "same-secret"
        );
        assert_eq!(
            decrypt_secret(&second).expect("second decrypt"),
            "same-secret"
        );
    }

    #[test]
    fn legacy_v1_envelopes_still_decrypt_with_unsalted_key() {
        let _guard = TestEncryptionKeyGuard::with_key("v1-compat-master-key");
        let cipher = Aes256Gcm::new(GenericArray::from_slice(&derive_v1_key(
            b"v1-compat-master-key",
        )));
        let nonce_bytes = [7u8; NONCE_LEN];
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), b"v1-secret".as_slice())
            .expect("v1 encrypt");
        let mut packed = nonce_bytes.to_vec();
        packed.extend(ciphertext);
        let value = format!("{ENCRYPTED_V1_PREFIX}{}", encode_b64(&packed));

        assert_eq!(decrypt_secret(&value).expect("v1 decrypt"), "v1-secret");
    }

    #[test]
    fn malformed_v2_envelopes_fail_without_panicking() {
        let _guard = TestEncryptionKeyGuard::with_key("malformed-v2-master-key");
        for malformed in [
            "kbenc:v2:notbase64:notbase64:notbase64",
            "kbenc:v2:AAAA",
            "kbenc:v2:AAAA:AAAA:AAAA",
            "kbenc:v2:::",
        ] {
            assert!(
                decrypt_secret(malformed).is_err(),
                "{malformed} must be rejected"
            );
        }
    }
}
