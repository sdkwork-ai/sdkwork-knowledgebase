//! WeChat official-account server callback verification and receipt (safe/encrypted mode).
//!
//! This surface consumes the stored callback credentials (`token`, `encodingAesKey`,
//! `encryptMode: "safe"`, `serverUrl`) so they are no longer dead weight: WeChat's
//! servers call the callback directly, the `msg_signature` scheme authenticates the
//! caller, and verified payloads are persisted as bounded Drive artifacts for later
//! processing. Business message handling is explicitly out of scope here.
//!
//! Only official accounts are served: `serverUrl`/`token`/`encodingAesKey` belong to
//! the official-account server config. The applet `msgToken`/`msgEncodingAESKey`
//! fields belong to the mini-program push wire, which is a different surface.
//!
//! Wire format per the WeChat public-platform spec:
//! `msg_signature = sha1(lexicographic_sort(token, timestamp, nonce, encrypt))`, and
//! the encrypted blob is `base64(AES-256-CBC(pkcs7(random(16) || msg_len(4, BE) ||
//! msg || receive_id)))` with `key = base64decode(encodingAesKey + "=")` (43 chars
//! + pad) and `iv = key[..16]`. The `receive_id` of an official account is its appid.

use std::collections::VecDeque;

use aes::Aes256;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use cbc::cipher::{BlockDecryptMut, KeyIvInit};
use sdkwork_utils_rust::secure_compare;
use sha1::Digest as Sha1Digest;
use thiserror::Error;

use crate::ports::knowledge_drive_storage::KnowledgeDriveStorage;
use crate::wechat::config_store::WechatConfigStore;

type Aes256CbcDec = cbc::Decryptor<Aes256>;

/// Mirrors the Drive webhook replay window (`wiki_event_consumer.rs`).
pub const CALLBACK_TIMESTAMP_WINDOW_SECONDS: i64 = 300;
/// Decrypted WeChat messages above this bound are rejected before field parsing.
pub const MAX_DECRYPTED_MESSAGE_BYTES: usize = 64 * 1024;
/// Base64 expansion of the largest accepted ciphertext plus slack.
const MAX_ENCRYPT_B64_CHARS: usize = 96 * 1024;
const RANDOM_PREFIX_BYTES: usize = 16;
const MESSAGE_LENGTH_PREFIX_BYTES: usize = 4;
/// Top-level callback field values (MsgType, FromUserName, MsgId, ...) are small;
/// larger "values" mean a malformed or hostile payload.
const MAX_CALLBACK_FIELD_BYTES: usize = 256;
const MAX_CALLBACK_SIGNATURE_CHARS: usize = 64;
const MAX_CALLBACK_NONCE_CHARS: usize = 64;
const MAX_CALLBACK_ECHO_CHARS: usize = 2048;
const MAX_CALLBACK_TIMESTAMP_CHARS: usize = 20;
const MAX_CALLBACK_ACCOUNT_ID_CHARS: usize = 128;
const MAX_CALLBACK_BODY_BYTES: usize = 100 * 1024;
const BOUNDED_REPLAY_SEEN_CAPACITY: usize = 4096;
const MAX_REPLAY_KEY_CHARS: usize = 256;

#[derive(Debug, Error)]
pub enum WechatCallbackError {
    #[error("invalid wechat callback request: {0}")]
    InvalidRequest(String),
    #[error("wechat callback signature verification failed")]
    VerificationFailed,
    #[error("wechat callback account is not configured for encrypted callbacks: {0}")]
    NotConfigured(String),
    #[error(transparent)]
    Crypto(#[from] WechatCallbackCryptoError),
    #[error(transparent)]
    Storage(#[from] crate::ports::knowledge_drive_storage::KnowledgeStorageError),
}

#[derive(Debug, Error)]
pub enum WechatCallbackCryptoError {
    #[error("wechat callback EncodingAESKey is invalid: {0}")]
    InvalidAesKey(String),
    #[error("wechat callback payload base64 encoding is invalid")]
    InvalidEncoding,
    #[error("wechat callback payload failed AES-CBC decryption or PKCS#7 padding validation")]
    Decrypt,
    #[error("wechat callback payload structure is invalid: {0}")]
    MalformedPayload(String),
    #[error("wechat callback payload exceeds the 65536 byte decrypted limit")]
    PayloadTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WechatCallbackVerificationRequest {
    /// Runtime tenant the callback is addressed to; verified against the deployment's
    /// tenant before any account lookup because WeChat callers arrive unauthenticated.
    pub tenant_id: u64,
    pub account_id: String,
    pub signature: String,
    pub timestamp: String,
    pub nonce: String,
    pub echostr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WechatCallbackReceiptRequest {
    pub tenant_id: u64,
    pub account_id: String,
    pub signature: String,
    pub timestamp: String,
    pub nonce: String,
    /// Raw request body (XML with the `Encrypt` element).
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WechatCallbackReceipt {
    pub msg_type: String,
    pub from_user_name: String,
    pub create_time: i64,
    /// Set when this delivery repeats an already-persisted message: the artifact is
    /// kept from the first delivery and the caller still acks so WeChat stops retrying.
    pub duplicate: bool,
    pub artifact_logical_path: Option<String>,
}

/// Computes WeChat's callback signature: SHA-1 over the lexicographically sorted
/// concatenation of the four wire values, lowercase hex.
pub fn callback_signature(token: &str, timestamp: &str, nonce: &str, encrypt: &str) -> String {
    let mut parts = [token, timestamp, nonce, encrypt];
    parts.sort_unstable();
    let digest = sha1::Sha1::digest(parts.concat().as_bytes());
    let mut hex = String::with_capacity(40);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Timing-safe callback signature comparison against WeChat's hex digest.
pub fn verify_callback_signature(
    token: &str,
    timestamp: &str,
    nonce: &str,
    encrypt: &str,
    signature: &str,
) -> bool {
    if signature.is_empty() || signature.len() > MAX_CALLBACK_SIGNATURE_CHARS {
        return false;
    }
    let expected = callback_signature(token, timestamp, nonce, encrypt);
    secure_compare(&expected, &signature.to_ascii_lowercase())
}

pub fn timestamp_outside_window(timestamp_secs: i64, now_secs: i64) -> bool {
    (now_secs - timestamp_secs).abs() > CALLBACK_TIMESTAMP_WINDOW_SECONDS
}

pub fn unix_now_secs() -> Option<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedWechatMessage {
    /// The inner message bytes (the XML WeChat encrypted).
    pub message: Vec<u8>,
    /// The trailing receive id; for official accounts this is the appid.
    pub receive_id: String,
}

/// Decrypts a WeChat safe-mode callback blob: base64 decode, AES-256-CBC with the
/// 43-char EncodingAESKey (padded to 44 chars), PKCS#7 unpad, then split
/// `random(16) || msg_len(4, BE) || msg || receive_id`.
pub fn decrypt_wechat_message(
    encoding_aes_key_b64: &str,
    encrypt_b64: &str,
) -> Result<DecryptedWechatMessage, WechatCallbackCryptoError> {
    let key = decode_encoding_aes_key(encoding_aes_key_b64)?;
    if encrypt_b64.is_empty() || encrypt_b64.len() > MAX_ENCRYPT_B64_CHARS {
        return Err(WechatCallbackCryptoError::PayloadTooLarge);
    }
    let mut ciphertext =
        BASE64_STANDARD
            .decode(encrypt_b64)
            .map_err(|_| WechatCallbackCryptoError::InvalidEncoding)?;
    if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
        return Err(WechatCallbackCryptoError::MalformedPayload(
            "ciphertext is not a multiple of the AES block size".to_string(),
        ));
    }
    let plain = Aes256CbcDec::new_from_slices(&key, &key[..16])
        .map_err(|_| WechatCallbackCryptoError::InvalidAesKey("bad key or iv length".to_string()))?
        .decrypt_padded_mut::<cbc::cipher::block_padding::Pkcs7>(&mut ciphertext)
        .map_err(|_| WechatCallbackCryptoError::Decrypt)?;
    split_wechat_plain(plain)
}

fn decode_encoding_aes_key(encoding_aes_key_b64: &str) -> Result<Vec<u8>, WechatCallbackCryptoError> {
    let trimmed = encoding_aes_key_b64.trim();
    // WeChat issues a 43-char key; the standard encoding needs one "=" pad char.
    let padded: String = match trimmed.chars().count() {
        43 => format!("{trimmed}="),
        44 => trimmed.to_string(),
        _ => {
            return Err(WechatCallbackCryptoError::InvalidAesKey(
                "EncodingAESKey must be 43 base64 characters".to_string(),
            ))
        }
    };
    let key = BASE64_STANDARD
        .decode(padded.as_bytes())
        .map_err(|_| WechatCallbackCryptoError::InvalidAesKey("key is not valid base64".to_string()))?;
    if key.len() != 32 {
        return Err(WechatCallbackCryptoError::InvalidAesKey(
            "key must decode to 32 bytes".to_string(),
        ));
    }
    Ok(key)
}

fn split_wechat_plain(plain: &[u8]) -> Result<DecryptedWechatMessage, WechatCallbackCryptoError> {
    if plain.len() < RANDOM_PREFIX_BYTES + MESSAGE_LENGTH_PREFIX_BYTES {
        return Err(WechatCallbackCryptoError::MalformedPayload(
            "decrypted payload is shorter than the WeChat envelope".to_string(),
        ));
    }
    let message_len = u32::from_be_bytes([
        plain[RANDOM_PREFIX_BYTES],
        plain[RANDOM_PREFIX_BYTES + 1],
        plain[RANDOM_PREFIX_BYTES + 2],
        plain[RANDOM_PREFIX_BYTES + 3],
    ]) as usize;
    let message_end = RANDOM_PREFIX_BYTES + MESSAGE_LENGTH_PREFIX_BYTES + message_len;
    if message_len > MAX_DECRYPTED_MESSAGE_BYTES || message_end > plain.len() {
        return Err(WechatCallbackCryptoError::PayloadTooLarge);
    }
    let message = plain[RANDOM_PREFIX_BYTES + MESSAGE_LENGTH_PREFIX_BYTES..message_end].to_vec();
    let receive_id = String::from_utf8_lossy(&plain[message_end..])
        .trim_end_matches(char::from(0))
        .to_string();
    Ok(DecryptedWechatMessage {
        message,
        receive_id,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedWechatMessageFields {
    pub msg_type: String,
    pub from_user_name: String,
    pub create_time: i64,
    /// Present for passive messages, absent for most event pushes; the replay key
    /// falls back to `(msg_type, from_user_name, create_time)` when absent.
    pub msg_id: Option<String>,
}

/// Extracts only the bounded top-level fields needed for receipt bookkeeping.
/// Deliberately not a general XML parser: hostile input must never reach business
/// logic, and no XML dependency is warranted for four fixed tags.
pub fn extract_bounded_message_fields(
    xml: &str,
) -> Result<BoundedWechatMessageFields, WechatCallbackCryptoError> {
    let msg_type = extract_xml_tag_value(xml, "MsgType", MAX_CALLBACK_FIELD_BYTES)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            WechatCallbackCryptoError::MalformedPayload("MsgType is missing".to_string())
        })?;
    let from_user_name = extract_xml_tag_value(xml, "FromUserName", MAX_CALLBACK_FIELD_BYTES)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            WechatCallbackCryptoError::MalformedPayload("FromUserName is missing".to_string())
        })?;
    let create_time_raw = extract_xml_tag_value(xml, "CreateTime", MAX_CALLBACK_FIELD_BYTES)?
        .ok_or_else(|| {
            WechatCallbackCryptoError::MalformedPayload("CreateTime is missing".to_string())
        })?;
    let create_time = create_time_raw.parse::<i64>().map_err(|_| {
        WechatCallbackCryptoError::MalformedPayload("CreateTime is not epoch seconds".to_string())
    })?;
    let msg_id = extract_xml_tag_value(xml, "MsgId", MAX_CALLBACK_FIELD_BYTES)?;
    Ok(BoundedWechatMessageFields {
        msg_type,
        from_user_name,
        create_time,
        msg_id,
    })
}

/// Extracts the `Encrypt` element from a safe-mode callback request body.
pub fn extract_encrypted_payload(body: &str) -> Result<String, WechatCallbackCryptoError> {
    if body.len() > MAX_CALLBACK_BODY_BYTES {
        return Err(WechatCallbackCryptoError::PayloadTooLarge);
    }
    extract_xml_tag_value(body, "Encrypt", MAX_ENCRYPT_B64_CHARS)?.ok_or_else(|| {
        WechatCallbackCryptoError::MalformedPayload("Encrypt element is missing".to_string())
    })
}

fn extract_xml_tag_value(
    xml: &str,
    tag: &str,
    max_bytes: usize,
) -> Result<Option<String>, WechatCallbackCryptoError> {
    let Some(open) = xml.find(&format!("<{tag}>")) else {
        return Ok(None);
    };
    let value_start = open + tag.len() + 2;
    let closing = format!("</{tag}>");
    let Some(close) = xml[value_start..].find(&closing) else {
        return Err(WechatCallbackCryptoError::MalformedPayload(format!(
            "{tag} element is not closed"
        )));
    };
    let raw = &xml[value_start..value_start + close];
    let value = decode_simple_xml_text(raw);
    if value.len() > max_bytes || value.contains('<') {
        return Err(WechatCallbackCryptoError::MalformedPayload(format!(
            "{tag} value is out of bounds"
        )));
    }
    Ok(Some(value))
}

/// Decodes CDATA and the five predefined XML entities; numeric character references
/// are rejected by the caller's `<` guard and unknown entities pass through so a
/// hostile payload cannot smuggle markup past the bound check.
fn decode_simple_xml_text(raw: &str) -> String {
    let inner = raw
        .strip_prefix("<![CDATA[")
        .and_then(|value| value.strip_suffix("]]>"));
    let value = match inner {
        Some(cdata) => cdata.to_string(),
        None => raw
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
    };
    value.trim().to_string()
}

/// Process-wide bounded seen-set for callback replay protection: at most
/// `BOUNDED_REPLAY_SEEN_CAPACITY` keys, oldest evicted first. Returns `true` when
/// the key was already recorded (a replay).
pub fn callback_already_seen(key: &str) -> bool {
    static REPLAY_SEEN: std::sync::Mutex<VecDeque<String>> = std::sync::Mutex::new(VecDeque::new());
    let key = truncate_char_bound(key, MAX_REPLAY_KEY_CHARS);
    let mut seen = REPLAY_SEEN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if seen.iter().any(|entry| entry == &key) {
        return true;
    }
    if seen.len() >= BOUNDED_REPLAY_SEEN_CAPACITY {
        seen.pop_front();
    }
    seen.push_back(key);
    false
}

fn truncate_char_bound(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

/// Resolves and verifies WeChat server callbacks for one tenant. Read-only on the
/// config store for verification; receipts persist through the store's Drive helper.
pub struct WechatCallbackService<'a> {
    config_store: WechatConfigStore<'a>,
    tenant_id: String,
}

impl<'a> WechatCallbackService<'a> {
    pub fn new(drive: &'a dyn KnowledgeDriveStorage, tenant_id: &str) -> Self {
        Self {
            config_store: WechatConfigStore::new(drive, tenant_id),
            tenant_id: tenant_id.to_string(),
        }
    }

    /// GET URL verification: verify the signature over `(token, timestamp, nonce,
    /// echostr)`, decrypt the echo blob, and confirm its receive id is this
    /// account's appid. Returns the decrypted echo string that must go back as the
    /// raw response body.
    pub async fn verify_url(
        &self,
        request: &WechatCallbackVerificationRequest,
    ) -> Result<String, WechatCallbackError> {
        ensure_request_tenant(request.tenant_id, &self.tenant_id)?;
        validate_callback_query(
            &request.account_id,
            &request.signature,
            &request.timestamp,
            &request.nonce,
        )?;
        if request.echostr.is_empty() || request.echostr.len() > MAX_CALLBACK_ECHO_CHARS {
            return Err(WechatCallbackError::InvalidRequest(
                "echostr must be 1 to 2048 characters".to_string(),
            ));
        }
        let account = self.resolve_callback_account(&request.account_id).await?;
        verify_timestamp_window(&request.timestamp)?;
        if !verify_callback_signature(
            account.token.as_deref().unwrap_or_default(),
            &request.timestamp,
            &request.nonce,
            &request.echostr,
            &request.signature,
        ) {
            return Err(WechatCallbackError::VerificationFailed);
        }
        let decrypted = decrypt_wechat_message(
            account.encoding_aes_key.as_deref().unwrap_or_default(),
            &request.echostr,
        )?;
        ensure_receive_id_matches(&decrypted.receive_id, &account.app_id)?;
        String::from_utf8(decrypted.message).map_err(|_| {
            WechatCallbackError::Crypto(WechatCallbackCryptoError::MalformedPayload(
                "echo string is not UTF-8".to_string(),
            ))
        })
    }

    /// POST message receipt: verify the signature over the encrypted payload,
    /// decrypt, run the replay seen-set, extract the bounded fields, and persist
    /// the raw encrypted payload as a bounded Drive artifact for later processing.
    pub async fn receive_message(
        &self,
        request: &WechatCallbackReceiptRequest,
    ) -> Result<WechatCallbackReceipt, WechatCallbackError> {
        ensure_request_tenant(request.tenant_id, &self.tenant_id)?;
        validate_callback_query(
            &request.account_id,
            &request.signature,
            &request.timestamp,
            &request.nonce,
        )?;
        let account = self.resolve_callback_account(&request.account_id).await?;
        verify_timestamp_window(&request.timestamp)?;
        let encrypt = extract_encrypted_payload(&request.body)?;
        if !verify_callback_signature(
            account.token.as_deref().unwrap_or_default(),
            &request.timestamp,
            &request.nonce,
            &encrypt,
            &request.signature,
        ) {
            return Err(WechatCallbackError::VerificationFailed);
        }
        let decrypted = decrypt_wechat_message(
            account.encoding_aes_key.as_deref().unwrap_or_default(),
            &encrypt,
        )?;
        ensure_receive_id_matches(&decrypted.receive_id, &account.app_id)?;
        let xml = std::str::from_utf8(&decrypted.message).map_err(|_| {
            WechatCallbackError::Crypto(WechatCallbackCryptoError::MalformedPayload(
                "message XML is not UTF-8".to_string(),
            ))
        })?;
        let fields = extract_bounded_message_fields(xml)?;
        let replay_key = fields
            .msg_id
            .as_deref()
            .map(|msg_id| format!("{}:{msg_id}", self.tenant_id))
            .unwrap_or_else(|| {
                format!(
                    "{}:{}:{}:{}",
                    self.tenant_id, fields.msg_type, fields.from_user_name, fields.create_time
                )
            });
        let duplicate = callback_already_seen(&replay_key);
        let artifact_path = if duplicate {
            None
        } else {
            Some(
                self.config_store
                    .store_callback_receipt(&crate::wechat::config_store::WechatCallbackReceiptArtifact {
                        account_id: request.account_id.clone(),
                        signature: request.signature.clone(),
                        timestamp: request.timestamp.clone(),
                        nonce: request.nonce.clone(),
                        encrypt,
                        msg_type: fields.msg_type.clone(),
                        from_user_name: fields.from_user_name.clone(),
                        create_time: fields.create_time,
                    })
                    .await?,
            )
        };
        Ok(WechatCallbackReceipt {
            msg_type: fields.msg_type,
            from_user_name: fields.from_user_name,
            create_time: fields.create_time,
            duplicate,
            artifact_logical_path: artifact_path,
        })
    }

    async fn resolve_callback_account(
        &self,
        account_id: &str,
    ) -> Result<sdkwork_knowledgebase_contract::wechat::KnowledgeWechatOfficialAccount, WechatCallbackError>
    {
        let account = self
            .config_store
            .find_official_account(account_id)
            .await?
            .ok_or_else(|| {
                WechatCallbackError::InvalidRequest(
                    "wechat callback account was not found".to_string(),
                )
            })?;
        if account.token.as_deref().unwrap_or_default().is_empty()
            || account
                .encoding_aes_key
                .as_deref()
                .unwrap_or_default()
                .is_empty()
        {
            return Err(WechatCallbackError::NotConfigured(format!(
                "official account {account_id} is missing token or encodingAesKey"
            )));
        }
        if account.encrypt_mode.as_deref() != Some("safe") {
            return Err(WechatCallbackError::NotConfigured(format!(
                "official account {account_id} must set encryptMode to safe"
            )));
        }
        Ok(account)
    }
}

fn validate_callback_query(
    account_id: &str,
    signature: &str,
    timestamp: &str,
    nonce: &str,
) -> Result<(), WechatCallbackError> {
    if account_id.is_empty() || account_id.chars().count() > MAX_CALLBACK_ACCOUNT_ID_CHARS {
        return Err(WechatCallbackError::InvalidRequest(
            "account_id must be 1 to 128 characters".to_string(),
        ));
    }
    if signature.is_empty() || signature.len() > MAX_CALLBACK_SIGNATURE_CHARS {
        return Err(WechatCallbackError::InvalidRequest(
            "signature is missing or too long".to_string(),
        ));
    }
    validate_timestamp_param(timestamp)?;
    if nonce.is_empty() || nonce.chars().count() > MAX_CALLBACK_NONCE_CHARS {
        return Err(WechatCallbackError::InvalidRequest(
            "nonce must be 1 to 64 characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_timestamp_param(timestamp: &str) -> Result<(), WechatCallbackError> {
    if timestamp.is_empty() || timestamp.len() > MAX_CALLBACK_TIMESTAMP_CHARS {
        return Err(WechatCallbackError::InvalidRequest(
            "timestamp must be epoch seconds".to_string(),
        ));
    }
    timestamp
        .parse::<i64>()
        .map_err(|_| WechatCallbackError::InvalidRequest("timestamp must be epoch seconds".to_string()))?;
    Ok(())
}

fn verify_timestamp_window(timestamp: &str) -> Result<(), WechatCallbackError> {
    let timestamp_secs = timestamp.parse::<i64>().unwrap_or_default();
    let now = unix_now_secs().ok_or_else(|| {
        WechatCallbackError::InvalidRequest("system clock is before Unix epoch".to_string())
    })?;
    if timestamp_outside_window(timestamp_secs, now) {
        return Err(WechatCallbackError::InvalidRequest(
            "callback timestamp is outside the replay window".to_string(),
        ));
    }
    Ok(())
}

fn ensure_receive_id_matches(receive_id: &str, app_id: &str) -> Result<(), WechatCallbackError> {
    if receive_id.trim() != app_id.trim() {
        return Err(WechatCallbackError::VerificationFailed);
    }
    Ok(())
}

/// Callback callers are unauthenticated, so the tenant query parameter is the only
/// scope key until the signature passes; both the hosted adapter and this service
/// reject a mismatch before any account lookup or Drive IO.
fn ensure_request_tenant(request_tenant: u64, service_tenant: &str) -> Result<(), WechatCallbackError> {
    if request_tenant.to_string() != service_tenant {
        return Err(WechatCallbackError::InvalidRequest(
            "callback tenant is not served by this runtime".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbc::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
    type Aes256CbcEnc = cbc::Encryptor<Aes256>;

    const TEST_AES_KEY_43: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ";

    /// Numeric form matches how the runtime resolves `tenant_id_str` for the service.
    const CALLBACK_TEST_TENANT: u64 = 1;

    fn encode_b64(bytes: &[u8]) -> String {
        BASE64_STANDARD.encode(bytes)
    }

    fn test_aes_key32() -> [u8; 32] {
        decode_encoding_aes_key(TEST_AES_KEY_43)
            .expect("test key")
            .try_into()
            .expect("32 byte key")
    }

    fn encrypt_wechat_blob(key: &[u8; 32], message: &str, receive_id: &str) -> Vec<u8> {
        let mut plain = [0u8; RANDOM_PREFIX_BYTES].to_vec();
        plain.extend_from_slice(&(message.len() as u32).to_be_bytes());
        plain.extend_from_slice(message.as_bytes());
        plain.extend_from_slice(receive_id.as_bytes());
        let mut padded = vec![0u8; plain.len() + 16];
        padded[..plain.len()].copy_from_slice(&plain);
        Aes256CbcEnc::new_from_slices(key, &key[..16])
            .expect("test cipher key")
            .encrypt_padded_mut::<Pkcs7>(&mut padded, plain.len())
            .expect("test padding")
            .to_vec()
    }

    #[test]
    fn signature_matches_independently_computed_sorted_sha1() {
        let token = "know-token";
        let timestamp = "1700000000";
        let nonce = "nonce-1";
        let encrypt = "ENCRYPTED-BLOB";
        // Golden vector computed outside this module (sorted join -> sha1 hex).
        let expected = "b5a65d13e3104d002743345b5bbab58fb679000c";
        assert_eq!(
            callback_signature(token, timestamp, nonce, encrypt),
            expected
        );
        assert!(verify_callback_signature(
            token,
            timestamp,
            nonce,
            encrypt,
            expected
        ));
        assert!(verify_callback_signature(
            token,
            timestamp,
            nonce,
            encrypt,
            &expected.to_uppercase()
        ));
    }

    #[test]
    fn signature_verification_rejects_tampering() {
        let token = "know-token";
        assert!(!verify_callback_signature(
            token,
            "1700000000",
            "nonce-1",
            "ENCRYPTED-BLOB",
            "b5a65d13e3104d002743345b5bbab58fb679000d"
        ));
        assert!(!verify_callback_signature(
            "other-token",
            "1700000000",
            "nonce-1",
            "ENCRYPTED-BLOB",
            "b5a65d13e3104d002743345b5bbab58fb679000c"
        ));
        assert!(!verify_callback_signature(
            token,
            "1700000000",
            "nonce-1",
            "ENCRYPTED-BLOB",
            ""
        ));
    }

    #[test]
    fn decrypt_roundtrips_wechat_envelope_and_receive_id() {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&decode_encoding_aes_key(TEST_AES_KEY_43).expect("test key"));
        let message = "<xml><MsgType><![CDATA[text]]></MsgType></xml>";
        let blob = encode_b64(&encrypt_wechat_blob(&key_bytes, message, "wx-receive-id"));

        let decrypted =
            decrypt_wechat_message(TEST_AES_KEY_43, &blob).expect("decrypt wechat blob");
        assert_eq!(decrypted.message, message.as_bytes());
        assert_eq!(decrypted.receive_id, "wx-receive-id");
    }

    #[test]
    fn decrypt_rejects_bad_padding_and_structure() {
        let key = decode_encoding_aes_key(TEST_AES_KEY_43).expect("test key");
        // Valid base64, wrong size and content.
        assert!(matches!(
            decrypt_wechat_message(TEST_AES_KEY_43, &BASE64_STANDARD.encode([0u8; 8])),
            Err(WechatCallbackCryptoError::MalformedPayload(_))
        ));
        // Not base64.
        assert!(matches!(
            decrypt_wechat_message(TEST_AES_KEY_43, "not-base64!!"),
            Err(WechatCallbackCryptoError::InvalidEncoding)
        ));
        // Wrong key length.
        assert!(matches!(
            decrypt_wechat_message("short", "AAAA"),
            Err(WechatCallbackCryptoError::InvalidAesKey(_))
        ));
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn decrypt_rejects_oversize_message_length() {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&decode_encoding_aes_key(TEST_AES_KEY_43).expect("test key"));
        let mut plain = [0u8; RANDOM_PREFIX_BYTES].to_vec();
        // Declared length far beyond the decrypted cap; bounds must reject it.
        plain.extend_from_slice(&(u32::MAX).to_be_bytes());
        plain.extend_from_slice(b"trailing");
        let mut padded = vec![0u8; plain.len() + 16];
        padded[..plain.len()].copy_from_slice(&plain);
        let ciphertext = Aes256CbcEnc::new_from_slices(&key_bytes, &key_bytes[..16])
            .expect("test cipher key")
            .encrypt_padded_mut::<Pkcs7>(&mut padded, plain.len())
            .expect("test padding")
            .to_vec();
        let blob = encode_b64(&ciphertext);
        assert!(matches!(
            decrypt_wechat_message(TEST_AES_KEY_43, &blob),
            Err(WechatCallbackCryptoError::PayloadTooLarge)
        ));
    }

    #[test]
    fn timestamp_window_mirrors_the_drive_webhook_window() {
        assert!(!timestamp_outside_window(1_700_000_000, 1_700_000_000 + 300));
        assert!(timestamp_outside_window(1_700_000_000, 1_700_000_000 + 301));
        assert!(timestamp_outside_window(1_700_000_000 + 301, 1_700_000_000));
    }

    #[test]
    fn bounded_field_extraction_reads_only_declared_tags() {
        let xml = "<xml><ToUserName><![CDATA[to]]></ToUserName>\
<FromUserName><![CDATA[from-user]]></FromUserName>\
<CreateTime>1700000000</CreateTime>\
<MsgType><![CDATA[text]]></MsgType>\
<Content><![CDATA[hello]]></Content>\
<MsgId>1234567890123456</MsgId></xml>";
        let fields = extract_bounded_message_fields(xml).expect("bounded fields");
        assert_eq!(fields.msg_type, "text");
        assert_eq!(fields.from_user_name, "from-user");
        assert_eq!(fields.create_time, 1_700_000_000);
        assert_eq!(fields.msg_id.as_deref(), Some("1234567890123456"));
    }

    #[test]
    fn bounded_field_extraction_rejects_missing_or_hostile_values() {
        assert!(extract_bounded_message_fields("<xml><CreateTime>1</CreateTime></xml>").is_err());
        assert!(extract_bounded_message_fields(
            "<xml><MsgType>text</MsgType><FromUserName>f</FromUserName><CreateTime>bad</CreateTime></xml>"
        )
        .is_err());
        // A nested tag inside a value must be rejected, not parsed.
        assert!(extract_bounded_message_fields(
            "<xml><MsgType>te<xt></MsgType><FromUserName>f</FromUserName><CreateTime>1</CreateTime></xml>"
        )
        .is_err());
        // Oversized value must be rejected.
        let oversized = format!(
            "<xml><MsgType>{}</MsgType><FromUserName>f</FromUserName><CreateTime>1</CreateTime></xml>",
            "x".repeat(MAX_CALLBACK_FIELD_BYTES + 1)
        );
        assert!(extract_bounded_message_fields(&oversized).is_err());
    }

    #[test]
    fn encrypted_payload_extraction_bounds_the_body() {
        let body = "<xml><ToUserName><![CDATA[to]]></ToUserName><Encrypt><![CDATA[aGVsbG8=]]></Encrypt></xml>";
        assert_eq!(
            extract_encrypted_payload(body).expect("encrypt element"),
            "aGVsbG8="
        );
        assert!(extract_encrypted_payload("<xml></xml>").is_err());
        let oversize_body = format!("<Encrypt>{}</Encrypt>", "e".repeat(MAX_CALLBACK_BODY_BYTES + 1));
        assert!(matches!(
            extract_encrypted_payload(&oversize_body),
            Err(WechatCallbackCryptoError::PayloadTooLarge)
        ));
    }

    #[test]
    fn replay_seen_set_marks_duplicates_within_capacity() {
        let key = format!("replay-test-{}", std::process::id());
        assert!(!callback_already_seen(&key));
        assert!(callback_already_seen(&key));
        // Distinct keys keep working after the duplicate.
        let other = format!("{key}-other");
        assert!(!callback_already_seen(&other));
    }

    struct MapDrive {
        objects: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    }

    impl MapDrive {
        fn new() -> Self {
            Self {
                objects: std::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl crate::ports::knowledge_drive_storage::KnowledgeDriveStorage for MapDrive {
        async fn put_object(
            &self,
            request: crate::ports::knowledge_drive_storage::PutKnowledgeObjectRequest,
        ) -> Result<
            crate::ports::knowledge_drive_storage::KnowledgeObjectRef,
            crate::ports::knowledge_drive_storage::KnowledgeStorageError,
        > {
            self.objects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(request.logical_path.clone(), request.body);
            Ok(crate::ports::knowledge_drive_storage::KnowledgeObjectRef {
                storage_provider_id: "test".to_string(),
                bucket: "test".to_string(),
                object_key: request.logical_path.clone(),
                logical_path: request.logical_path,
                object_role: request.object_role,
                content_type: request.content_type,
                size_bytes: 0,
                checksum_sha256_hex: None,
                etag: None,
                version_id: None,
            })
        }

        async fn head_object(
            &self,
            request: crate::ports::knowledge_drive_storage::HeadKnowledgeObjectRequest,
        ) -> Result<
            crate::ports::knowledge_drive_storage::KnowledgeObjectRef,
            crate::ports::knowledge_drive_storage::KnowledgeStorageError,
        > {
            let objects = self
                .objects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let logical_path = request.logical_path.clone().unwrap_or_default();
            let size = objects.get(&logical_path).map(Vec::len).unwrap_or(0) as u64;
            drop(objects);
            if size == 0 {
                return Err(crate::ports::knowledge_drive_storage::KnowledgeStorageError::NotFound(
                    "absent".to_string(),
                ));
            }
            Ok(crate::ports::knowledge_drive_storage::KnowledgeObjectRef {
                storage_provider_id: "test".to_string(),
                bucket: "test".to_string(),
                object_key: logical_path.clone(),
                logical_path,
                object_role: request.object_role,
                content_type: "application/json".to_string(),
                size_bytes: size,
                checksum_sha256_hex: None,
                etag: None,
                version_id: None,
            })
        }

        async fn get_object_text(
            &self,
            object_ref: &crate::ports::knowledge_drive_storage::KnowledgeObjectRef,
        ) -> Result<String, crate::ports::knowledge_drive_storage::KnowledgeStorageError> {
            let objects = self
                .objects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            objects
                .get(&object_ref.logical_path)
                .cloned()
                .map(|bytes| String::from_utf8(bytes).expect("stored utf8"))
                .ok_or_else(|| {
                    crate::ports::knowledge_drive_storage::KnowledgeStorageError::NotFound(
                        "absent".to_string(),
                    )
                })
        }
    }

    fn safe_official_account(id: &str) -> sdkwork_knowledgebase_contract::wechat::KnowledgeWechatOfficialAccount {
        sdkwork_knowledgebase_contract::wechat::KnowledgeWechatOfficialAccount {
            id: id.to_string(),
            name: "Callback Account".to_string(),
            account_type: "service".to_string(),
            avatar: "CA".to_string(),
            description: None,
            app_id: "wx-callback-appid".to_string(),
            app_secret: None,
            server_url: Some("https://knowledgebase.example.com/app/v3/api/knowledge/wechat/callback".to_string()),
            token: Some("know-callback-token".to_string()),
            encoding_aes_key: Some(TEST_AES_KEY_43.to_string()),
            encrypt_mode: Some("safe".to_string()),
            domain_verify_file_name: None,
            domain_verify_file_content: None,
            js_secure_domains: None,
            web_auth_domains: None,
            business_domains: None,
            group: None,
        }
    }

    fn signed_params(token: &str, payload: &str, nonce: &str) -> (String, String, String) {
        let timestamp = unix_now_secs().expect("test clock").to_string();
        let signature = callback_signature(token, &timestamp, nonce, payload);
        (signature, timestamp, nonce.to_string())
    }

    #[tokio::test]
    async fn callback_service_verifies_echo_and_persists_receipt_with_replay_dedup() {
        let _key_guard =
            crate::wechat::secret_cipher::test_support::TestEncryptionKeyGuard::with_key(
                "callback-service-test-key",
            );
        let drive = MapDrive::new();
        let store = WechatConfigStore::new(&drive, "tenant-callback");
        store
            .replace_official_accounts(vec![safe_official_account("acct-callback")])
            .await
            .expect("seed callback account");
        let service = WechatCallbackService::new(&drive, "1");
        let token = "know-callback-token";

        // URL verification: signature + encrypted echo must round-trip. The GET
        // signature covers the encrypted echostr, exactly like the real wire.
        let echo = "random-echo-7381";
        let echostr = encode_b64(&encrypt_wechat_blob(&test_aes_key32(), echo, "wx-callback-appid"));
        let (signature, timestamp, nonce) = signed_params(token, &echostr, "verify-nonce");
        let verified = service
            .verify_url(&WechatCallbackVerificationRequest {
                tenant_id: CALLBACK_TEST_TENANT,
                account_id: "acct-callback".to_string(),
                signature,
                timestamp,
                nonce,
                echostr,
            })
            .await
            .expect("verify callback url");
        assert_eq!(verified, echo);

        // Message receipt: first delivery persists an artifact.
        let message_xml = "<xml><ToUserName><![CDATA[wx-callback-appid]]></ToUserName>\
<FromUserName><![CDATA[fan-openid]]></FromUserName>\
<CreateTime>1700000000</CreateTime>\
<MsgType><![CDATA[text]]></MsgType>\
<MsgId>9007199254740993</MsgId></xml>";
        let encrypt =
            encode_b64(&encrypt_wechat_blob(&test_aes_key32(), message_xml, "wx-callback-appid"));
        let body = format!("<xml><Encrypt><![CDATA[{encrypt}]]></Encrypt></xml>");
        let (signature, timestamp, nonce) = signed_params(token, &encrypt, "receipt-nonce");
        let receipt = service
            .receive_message(&WechatCallbackReceiptRequest {
                tenant_id: CALLBACK_TEST_TENANT,
                account_id: "acct-callback".to_string(),
                signature,
                timestamp,
                nonce,
                body,
            })
            .await
            .expect("receive callback message");
        assert_eq!(receipt.msg_type, "text");
        assert_eq!(receipt.from_user_name, "fan-openid");
        assert_eq!(receipt.create_time, 1_700_000_000);
        assert!(!receipt.duplicate);
        let artifact_path = receipt.artifact_logical_path.expect("artifact path");
        assert!(artifact_path.starts_with("wechat/v1/callback/acct-callback/"));

        // Retransmission: ack stays successful, no second artifact is written.
        let encrypt_again =
            encode_b64(&encrypt_wechat_blob(&test_aes_key32(), message_xml, "wx-callback-appid"));
        let body_again = format!("<xml><Encrypt><![CDATA[{encrypt_again}]]></Encrypt></xml>");
        let (signature_again, timestamp_again, nonce_again) =
            signed_params(token, &encrypt_again, "receipt-nonce-2");
        let duplicate = service
            .receive_message(&WechatCallbackReceiptRequest {
                tenant_id: CALLBACK_TEST_TENANT,
                account_id: "acct-callback".to_string(),
                signature: signature_again,
                timestamp: timestamp_again,
                nonce: nonce_again,
                body: body_again,
            })
            .await
            .expect("duplicate receipt still succeeds");
        assert!(duplicate.duplicate);
        assert!(duplicate.artifact_logical_path.is_none());
        let objects = drive
            .objects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_eq!(
            objects
                .keys()
                .filter(|path| path.starts_with("wechat/v1/callback/"))
                .count(),
            1,
            "retransmission must not accumulate artifacts"
        );
    }

    #[tokio::test]
    async fn callback_service_rejects_bad_signature_without_persisting() {
        let _key_guard =
            crate::wechat::secret_cipher::test_support::TestEncryptionKeyGuard::with_key(
                "callback-service-negative-test-key",
            );
        let drive = MapDrive::new();
        let store = WechatConfigStore::new(&drive, "tenant-callback-negative");
        store
            .replace_official_accounts(vec![safe_official_account("acct-negative")])
            .await
            .expect("seed callback account");
        let service = WechatCallbackService::new(&drive, "1");

        let echo = "rejected-echo";
        let (_, timestamp, nonce) = signed_params("know-callback-token", echo, "neg-nonce");
        let error = service
            .verify_url(&WechatCallbackVerificationRequest {
                tenant_id: CALLBACK_TEST_TENANT,
                account_id: "acct-negative".to_string(),
                signature: "0".repeat(40),
                timestamp,
                nonce,
                echostr: encode_b64(&encrypt_wechat_blob(
                    &test_aes_key32(),
                    echo,
                    "wx-callback-appid",
                )),
            })
            .await
            .expect_err("bad signature must fail");
        assert!(matches!(error, WechatCallbackError::VerificationFailed));

        // An account without safe-mode credentials is rejected before any crypto.
        let mut plain_account = safe_official_account("acct-plain");
        plain_account.encrypt_mode = Some("plain".to_string());
        let store = WechatConfigStore::new(&drive, "tenant-callback-negative");
        store
            .replace_official_accounts(vec![plain_account])
            .await
            .expect("seed plain account");
        let error = service
            .verify_url(&WechatCallbackVerificationRequest {
                tenant_id: CALLBACK_TEST_TENANT,
                account_id: "acct-plain".to_string(),
                signature: "0".repeat(40),
                timestamp: unix_now_secs().expect("test clock").to_string(),
                nonce: "neg-nonce-2".to_string(),
                echostr: "echo".to_string(),
            })
            .await
            .expect_err("non-safe encrypt mode must be rejected");
        assert!(matches!(error, WechatCallbackError::NotConfigured(_)));
    }
}
