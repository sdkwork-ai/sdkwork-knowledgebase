use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use keyring::Entry;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "sdkwork-knowledgebase-pc";
const LEGACY_SNAPSHOT_FILE: &str = "secure-session.json";
const KEY_INDEX_FILE: &str = "secure-session-keys.json";
const SESSION_STORAGE_KEY: &str = "sdkwork-knowledgebase-pc-session";
const WECHAT_CREDENTIAL_PREFIX: &str = "sdkwork.knowledgebase.pc.wechat.credentials.v1.";
const MAX_SECURE_KEY_BYTES: usize = 256;
const MAX_SECURE_VALUE_BYTES: usize = 256 * 1024;
const MAX_TRACKED_KEYS: usize = 512;
const MAX_KEY_INDEX_BYTES: u64 = 64 * 1024;

#[derive(Debug, Default, Serialize, Deserialize)]
struct SecureSessionSnapshot {
    values: HashMap<String, String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SecureSessionKeyIndex {
    keys: Vec<String>,
}

pub struct SecureSessionState {
    keys_path: PathBuf,
    keys: Mutex<Vec<String>>,
}

impl SecureSessionState {
    fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&app_data_dir).map_err(|error| error.to_string())?;

        let keys_path = app_data_dir.join(KEY_INDEX_FILE);
        let legacy_path = app_data_dir.join(LEGACY_SNAPSHOT_FILE);
        let mut keys = load_key_index(&keys_path);

        if legacy_path.exists() {
            // Migration failure must NOT abort startup (an unreadable or
            // oversized legacy file would otherwise make the app refuse to
            // launch on every start). Skip and retry on the next launch; the
            // plaintext snapshot stays until the migration succeeds.
            match migrate_legacy_snapshot(&legacy_path, &mut keys) {
                Ok(()) => {
                    if fs::remove_file(&legacy_path).is_err() {
                        // Deletion can fail (AV lock, permissions). Rename first
                        // so the migrated plaintext is out of the way; a stale
                        // leftover from an interrupted earlier run is removed
                        // first (Windows rename fails onto an existing file);
                        // if even the rename fails, keep the file and warn —
                        // the values are already safely in the OS keychain.
                        let renamed = legacy_path.with_extension("json.migrated");
                        let _ = fs::remove_file(&renamed);
                        if fs::rename(&legacy_path, &renamed).is_err() {
                            eprintln!(
                                "warning: legacy secure-session snapshot could not be removed after migration: {}",
                                legacy_path.display()
                            );
                        }
                    }
                }
                Err(error) => {
                    eprintln!(
                        "warning: legacy secure-session migration failed and will retry on next launch: {error}"
                    );
                }
            }
        }

        persist_key_index(&keys_path, &keys)?;
        Ok(Self {
            keys_path,
            keys: Mutex::new(keys),
        })
    }

    fn track_key(&self, key: &str) -> Result<(), String> {
        let mut keys = self
            .keys
            .lock()
            .map_err(|_| "secure session lock poisoned".to_string())?;
        if !keys.iter().any(|existing| existing == key) {
            if keys.len() >= MAX_TRACKED_KEYS {
                return Err("secure session key limit exceeded".to_string());
            }
            keys.push(key.to_string());
            persist_key_index(&self.keys_path, &keys)?;
        }
        Ok(())
    }

    fn untrack_key(&self, key: &str) -> Result<(), String> {
        let mut keys = self
            .keys
            .lock()
            .map_err(|_| "secure session lock poisoned".to_string())?;
        let original_len = keys.len();
        keys.retain(|existing| existing != key);
        if keys.len() != original_len {
            persist_key_index(&self.keys_path, &keys)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecureSessionKeyRequest {
    key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecureSessionWriteRequest {
    key: String,
    value: String,
}

fn keyring_entry(key: &str) -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, key).map_err(|error| error.to_string())
}

fn validate_secure_key(key: &str) -> Result<&str, String> {
    if key.is_empty() || key.len() > MAX_SECURE_KEY_BYTES || key.trim() != key {
        return Err("secure session key is invalid".to_string());
    }
    let is_allowed_namespace =
        key == SESSION_STORAGE_KEY || key.starts_with(WECHAT_CREDENTIAL_PREFIX);
    let has_safe_characters = key
        .bytes()
        .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'.' | b'-' | b'_' | b':'));
    if !is_allowed_namespace || !has_safe_characters {
        return Err("secure session key is outside the allowed namespace".to_string());
    }
    Ok(key)
}

fn validate_secure_value(value: &str) -> Result<(), String> {
    if value.len() > MAX_SECURE_VALUE_BYTES {
        return Err("secure session value exceeds the maximum allowed size".to_string());
    }
    Ok(())
}

fn load_key_index(path: &Path) -> Vec<String> {
    if !path.exists() {
        return Vec::new();
    }
    if fs::metadata(path)
        .map(|metadata| metadata.len() > MAX_KEY_INDEX_BYTES)
        .unwrap_or(true)
    {
        return Vec::new();
    }
    let raw = fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str::<SecureSessionKeyIndex>(&raw)
        .map(|index| {
            index
                .keys
                .into_iter()
                .filter(|key| validate_secure_key(key).is_ok())
                .take(MAX_TRACKED_KEYS)
                .collect()
        })
        .unwrap_or_default()
}

fn persist_key_index(path: &Path, keys: &[String]) -> Result<(), String> {
    let payload = SecureSessionKeyIndex {
        keys: keys.to_vec(),
    };
    let serialized = serde_json::to_string_pretty(&payload)
        .map_err(|error: serde_json::Error| error.to_string())?;
    fs::write(path, serialized).map_err(|error| error.to_string())
}

fn migrate_legacy_snapshot(legacy_path: &Path, keys: &mut Vec<String>) -> Result<(), String> {
    if fs::metadata(legacy_path)
        .map_err(|error| error.to_string())?
        .len()
        > MAX_SECURE_VALUE_BYTES as u64
    {
        return Err("legacy secure session snapshot exceeds the maximum allowed size".to_string());
    }
    let raw = fs::read_to_string(legacy_path).map_err(|error| error.to_string())?;
    let snapshot = serde_json::from_str::<SecureSessionSnapshot>(&raw).unwrap_or_default();
    for (key, value) in snapshot.values {
        if validate_secure_key(&key).is_err() || validate_secure_value(&value).is_err() {
            continue;
        }
        keyring_entry(&key)?
            .set_password(&value)
            .map_err(|error| error.to_string())?;
        if !keys.iter().any(|existing| existing == &key) {
            keys.push(key);
        }
    }
    Ok(())
}

pub fn init_secure_session_state(app: &AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let state = SecureSessionState::new(app_data_dir)?;
    app.manage(state);
    Ok(())
}

#[tauri::command]
pub fn write_secure_session_value(
    state: tauri::State<'_, SecureSessionState>,
    request: SecureSessionWriteRequest,
) -> Result<(), String> {
    let key = validate_secure_key(&request.key)?;
    validate_secure_value(&request.value)?;
    keyring_entry(key)?
        .set_password(&request.value)
        .map_err(|error| error.to_string())?;
    state.track_key(key)
}

#[tauri::command]
pub fn remove_secure_session_value(
    state: tauri::State<'_, SecureSessionState>,
    request: SecureSessionKeyRequest,
) -> Result<(), String> {
    let key = validate_secure_key(&request.key)?;
    let entry = keyring_entry(key).ok();
    // Entry::new failing (platform-level) is undeletable: no keychain
    // operation succeeded, so the key must stay tracked for retry.
    if entry.is_none() {
        return Err(
            "failed to access the OS keychain; the credential stays tracked for retry".to_string(),
        );
    }
    let deleted = entry
        .as_ref()
        .and_then(|entry| entry.delete_credential().ok())
        .is_some();
    // Only keyring::Error::NoEntry proves absence: a transient platform/IPC
    // error on a still-present secret must keep the key tracked for retry
    // instead of silently orphaning it.
    let missing = !deleted
        && entry
            .as_ref()
            .map(|entry| matches!(entry.get_password(), Err(keyring::Error::NoEntry)))
            .unwrap_or(false);
    // Failed deletions stay tracked so a later clear can retry instead of
    // orphaning the OS-keychain secret.
    if deleted || missing {
        state.untrack_key(key)
    } else {
        Err("failed to delete secure session credential; it stays tracked for retry".to_string())
    }
}

#[tauri::command]
pub fn clear_secure_session_values(
    state: tauri::State<'_, SecureSessionState>,
) -> Result<(), String> {
    let scanned_keys = state
        .keys
        .lock()
        .map_err(|_| "secure session lock poisoned".to_string())?
        .clone();
    let scanned: std::collections::HashSet<String> = scanned_keys.iter().cloned().collect();
    let mut undeletable: Vec<String> = Vec::new();
    for key in &scanned_keys {
        let entry = keyring_entry(key).ok();
        // Entry::new failing (platform-level) is undeletable: no keychain
        // operation was attempted for this key.
        if entry.is_none() {
            undeletable.push(key.clone());
            continue;
        }
        let deleted = entry
            .as_ref()
            .and_then(|entry| entry.delete_credential().ok())
            .is_some();
        // The missing probe runs only after a failed deletion: only
        // keyring::Error::NoEntry proves absence — a transient platform error
        // on a still-present secret keeps the key tracked for retry.
        let missing = !deleted
            && entry
                .as_ref()
                .map(|entry| matches!(entry.get_password(), Err(keyring::Error::NoEntry)))
                .unwrap_or(false);
        // Keys whose OS-keychain deletion failed stay tracked so a later clear
        // can retry; dropping them from the index would orphan the secret in
        // the OS credential store forever despite a successful logout.
        if deleted || missing {
            continue;
        }
        undeletable.push(key.clone());
    }
    {
        let mut keys = state
            .keys
            .lock()
            .map_err(|_| "secure session lock poisoned".to_string())?;
        // Union with the live index: keys written while deletions ran (the
        // clone→replace window) were never deletion targets and must stay
        // tracked, or the persisted index would orphan their just-written
        // OS-keychain secrets.
        keys.retain(|key| {
            if scanned.contains(key) {
                undeletable.contains(key)
            } else {
                true
            }
        });
        for key in &undeletable {
            if !keys.contains(key) {
                keys.push(key.clone());
            }
        }
        persist_key_index(&state.keys_path, &keys)?;
    }
    if !undeletable.is_empty() {
        eprintln!(
            "warning: {} secure session credential(s) could not be deleted from the OS keychain and stay tracked for retry",
            undeletable.len()
        );
    }
    Ok(())
}

#[tauri::command]
pub fn read_secure_session_value(
    request: SecureSessionKeyRequest,
) -> Result<Option<String>, String> {
    let key = validate_secure_key(&request.key)?;
    match keyring_entry(key)?.get_password() {
        Ok(value) => {
            validate_secure_value(&value)?;
            Ok(Some(value))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_key_validation_allows_only_owned_namespaces() {
        assert!(validate_secure_key(SESSION_STORAGE_KEY).is_ok());
        assert!(validate_secure_key(
            "sdkwork.knowledgebase.pc.wechat.credentials.v1.applet.42.appSecret"
        )
        .is_ok());
        assert!(validate_secure_key("untrusted.secret").is_err());
        assert!(validate_secure_key("sdkwork-knowledgebase-pc-session\nother").is_err());
    }

    #[test]
    fn secure_value_validation_is_bounded() {
        assert!(validate_secure_value(&"x".repeat(MAX_SECURE_VALUE_BYTES)).is_ok());
        assert!(validate_secure_value(&"x".repeat(MAX_SECURE_VALUE_BYTES + 1)).is_err());
    }
}
