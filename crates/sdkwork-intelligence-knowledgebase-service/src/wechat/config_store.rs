use crate::ports::knowledge_drive_storage::{
    HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeStorageError,
    PutKnowledgeObjectRequest,
};
use crate::wechat::secret_cipher::{decrypt_optional_secret, encrypt_optional_secret, ENCRYPTED_VALUE_PREFIX};
use sdkwork_knowledgebase_contract::wechat::{
    KnowledgeWechatApplet, KnowledgeWechatOfficialAccount,
};
use sdkwork_utils_rust::{is_blank, sha256_hash};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const CONFIG_LOGICAL_PATH: &str = "wechat/v1/config.json";
const CONFIG_OBJECT_ROLE: &str = "wechat_config";
const CALLBACK_RECEIPT_LOGICAL_PATH_PREFIX: &str = "wechat/v1/callback";
const CALLBACK_RECEIPT_OBJECT_ROLE: &str = "wechat_callback_receipt";
const MAX_WECHAT_CALLBACK_RECEIPT_BYTES: u64 = 128 * 1024;
const MAX_WECHAT_CONFIG_BYTES: u64 = 1024 * 1024;
const MAX_WECHAT_CONFIG_ENTRIES_PER_KIND: usize = 100;
const MAX_WECHAT_DOMAIN_VALUES: usize = 50;
const MAX_WECHAT_ID_CHARS: usize = 128;
const MAX_WECHAT_NAME_CHARS: usize = 128;
const MAX_WECHAT_AVATAR_CHARS: usize = 32;
const MAX_WECHAT_DESCRIPTION_CHARS: usize = 2048;
const MAX_WECHAT_APP_ID_CHARS: usize = 64;
const MAX_WECHAT_SECRET_CHARS: usize = 256;
const MAX_WECHAT_SERVER_URL_CHARS: usize = 2048;
const MAX_WECHAT_TOKEN_CHARS: usize = 256;
const MAX_WECHAT_AES_KEY_CHARS: usize = 43;
const MAX_WECHAT_VERIFY_FILE_NAME_CHARS: usize = 255;
const MAX_WECHAT_VERIFY_FILE_CONTENT_BYTES: usize = 65_536;
const MAX_WECHAT_OFFICIAL_ACCOUNT_DOMAIN_CHARS: usize = 255;
const MAX_WECHAT_APPLET_ENDPOINT_CHARS: usize = 2048;
const MAX_WECHAT_GROUP_CHARS: usize = 128;
const MAX_WECHAT_APPLET_ORIGINAL_ID_CHARS: usize = 128;
const MAX_WECHAT_APPLET_PATH_CHARS: usize = 1024;
const WECHAT_ACCOUNT_TYPES: &[&str] = &["subscription", "service"];
const WECHAT_ENCRYPT_MODES: &[&str] = &["plain", "compatible", "safe"];
const WECHAT_MESSAGE_DATA_FORMATS: &[&str] = &["json", "xml"];

/// Serializes the load-merge-save critical section for every tenant and config kind. One
/// process-wide async lock is intentionally coarse: wechat config writes are rare tenant-admin
/// operations, so per-tenant lock maps would add complexity for no real contention win, while
/// two concurrent replacements without the lock would silently drop one side's entries.
static CONFIG_WRITE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TenantWechatConfig {
    #[serde(default)]
    official_accounts: Vec<KnowledgeWechatOfficialAccount>,
    #[serde(default)]
    applets: Vec<KnowledgeWechatApplet>,
}

pub struct WechatConfigStore<'a> {
    drive: &'a dyn KnowledgeDriveStorage,
    tenant_space_uuid: String,
}

/// A verified WeChat callback delivery, persisted verbatim (the encrypted payload is
/// stored, not the decrypted content) so later processing re-derives the plaintext
/// under the tenant's own key material.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatCallbackReceiptArtifact {
    pub account_id: String,
    pub signature: String,
    pub timestamp: String,
    pub nonce: String,
    pub encrypt: String,
    pub msg_type: String,
    pub from_user_name: String,
    pub create_time: i64,
}

impl<'a> WechatConfigStore<'a> {
    pub fn new(drive: &'a dyn KnowledgeDriveStorage, tenant_id: &str) -> Self {
        Self {
            drive,
            tenant_space_uuid: tenant_config_space_uuid(tenant_id),
        }
    }

    pub async fn load_official_accounts(
        &self,
    ) -> Result<Vec<KnowledgeWechatOfficialAccount>, KnowledgeStorageError> {
        let config = self.load_config().await?;
        Ok(config
            .official_accounts
            .into_iter()
            .map(redact_official_account)
            .collect())
    }

    pub async fn replace_official_accounts(
        &self,
        accounts: Vec<KnowledgeWechatOfficialAccount>,
    ) -> Result<Vec<KnowledgeWechatOfficialAccount>, KnowledgeStorageError> {
        validate_official_accounts(&accounts)?;
        let _config_write_guard = CONFIG_WRITE_LOCK.lock().await;
        let existing = self.load_config().await?;
        let mut config = existing;
        config.official_accounts =
            merge_official_account_secrets(accounts, &config.official_accounts);
        self.save_config(&config).await?;
        Ok(config
            .official_accounts
            .into_iter()
            .map(redact_official_account)
            .collect())
    }

    pub async fn load_applets(&self) -> Result<Vec<KnowledgeWechatApplet>, KnowledgeStorageError> {
        let config = self.load_config().await?;
        Ok(config.applets.into_iter().map(redact_applet).collect())
    }

    pub async fn replace_applets(
        &self,
        applets: Vec<KnowledgeWechatApplet>,
    ) -> Result<Vec<KnowledgeWechatApplet>, KnowledgeStorageError> {
        validate_applets(&applets)?;
        let _config_write_guard = CONFIG_WRITE_LOCK.lock().await;
        let existing = self.load_config().await?;
        let mut config = existing;
        config.applets = merge_applet_secrets(applets, &config.applets);
        self.save_config(&config).await?;
        Ok(config.applets.into_iter().map(redact_applet).collect())
    }

    pub async fn find_official_account(
        &self,
        account_id: &str,
    ) -> Result<Option<KnowledgeWechatOfficialAccount>, KnowledgeStorageError> {
        let config = self.load_config().await?;
        Ok(config
            .official_accounts
            .into_iter()
            .find(|account| account.id == account_id))
    }

    /// Persists a verified callback's raw encrypted payload under the tenant's wechat
    /// config space. The object key is content-derived (`timestamp` + encrypt digest),
    /// so WeChat retransmissions overwrite the same artifact instead of accumulating.
    pub async fn store_callback_receipt(
        &self,
        artifact: &WechatCallbackReceiptArtifact,
    ) -> Result<String, KnowledgeStorageError> {
        // The account id lands in the object key, so path separators in it would
        // escape the callback prefix; ids are validated but never charset-restricted.
        if artifact.account_id.contains('/')
            || artifact.account_id.contains('\\')
            || artifact.account_id == "."
            || artifact.account_id == ".."
        {
            return Err(KnowledgeStorageError::InvalidRequest(
                "official account id cannot be used in a callback artifact path".to_string(),
            ));
        }
        if !artifact
            .timestamp
            .bytes()
            .all(|byte| byte.is_ascii_digit())
            || artifact.timestamp.is_empty()
        {
            return Err(KnowledgeStorageError::InvalidRequest(
                "callback receipt timestamp must be epoch-seconds digits".to_string(),
            ));
        }
        let body = serde_json::to_vec(artifact).map_err(|error| {
            KnowledgeStorageError::Internal(format!("failed to encode wechat callback receipt: {error}"))
        })?;
        if body.len() as u64 > MAX_WECHAT_CALLBACK_RECEIPT_BYTES {
            return Err(KnowledgeStorageError::InvalidRequest(format!(
                "wechat callback receipt exceeds {MAX_WECHAT_CALLBACK_RECEIPT_BYTES} bytes"
            )));
        }
        let digest = &sha256_hash(artifact.encrypt.as_bytes())[..16];
        let logical_path = format!(
            "{CALLBACK_RECEIPT_LOGICAL_PATH_PREFIX}/{}/{timestamp}-{digest}.json",
            artifact.account_id,
            timestamp = artifact.timestamp,
        );
        let checksum = format!("sha256:{}", sha256_hash(&body));
        self.drive
            .put_object(PutKnowledgeObjectRequest {
                logical_path: logical_path.clone(),
                object_role: CALLBACK_RECEIPT_OBJECT_ROLE.to_string(),
                content_type: "application/json; charset=utf-8".to_string(),
                body,
                checksum_sha256_hex: Some(checksum),
                space_uuid: Some(self.tenant_space_uuid.clone()),
            })
            .await?;
        Ok(logical_path)
    }

    async fn load_config(&self) -> Result<TenantWechatConfig, KnowledgeStorageError> {
        let head_request =
            HeadKnowledgeObjectRequest::managed_artifact(CONFIG_LOGICAL_PATH, CONFIG_OBJECT_ROLE)
                .with_space_uuid(self.tenant_space_uuid.as_str());
        let object_ref = match self.drive.head_object(head_request).await {
            Ok(object_ref) => object_ref,
            Err(KnowledgeStorageError::NotFound(_)) => return Ok(TenantWechatConfig::default()),
            Err(error) => return Err(error),
        };
        let body = self
            .drive
            .get_object_text_bounded(&object_ref, MAX_WECHAT_CONFIG_BYTES)
            .await?;
        let mut config: TenantWechatConfig = serde_json::from_str(&body).map_err(|error| {
            KnowledgeStorageError::Internal(format!("invalid wechat config json: {error}"))
        })?;
        decrypt_config_secrets(&mut config)?;
        Ok(config)
    }

    async fn save_config(&self, config: &TenantWechatConfig) -> Result<(), KnowledgeStorageError> {
        let mut encrypted = config.clone();
        encrypt_config_secrets(&mut encrypted)?;
        let body = serde_json::to_vec(&encrypted).map_err(|error| {
            KnowledgeStorageError::Internal(format!("failed to encode wechat config: {error}"))
        })?;
        if body.len() as u64 > MAX_WECHAT_CONFIG_BYTES {
            return Err(KnowledgeStorageError::InvalidRequest(format!(
                "wechat config exceeds {MAX_WECHAT_CONFIG_BYTES} bytes"
            )));
        }
        let checksum = format!("sha256:{}", sha256_hash(&body));
        self.drive
            .put_object(PutKnowledgeObjectRequest {
                logical_path: CONFIG_LOGICAL_PATH.to_string(),
                object_role: CONFIG_OBJECT_ROLE.to_string(),
                content_type: "application/json; charset=utf-8".to_string(),
                body,
                checksum_sha256_hex: Some(checksum),
                space_uuid: Some(self.tenant_space_uuid.clone()),
            })
            .await?;
        Ok(())
    }
}

fn tenant_config_space_uuid(tenant_id: &str) -> String {
    format!("tenant-{tenant_id}")
}

fn validate_official_accounts(
    accounts: &[KnowledgeWechatOfficialAccount],
) -> Result<(), KnowledgeStorageError> {
    if accounts.len() > MAX_WECHAT_CONFIG_ENTRIES_PER_KIND {
        return Err(KnowledgeStorageError::InvalidRequest(format!(
            "official account count exceeds {MAX_WECHAT_CONFIG_ENTRIES_PER_KIND}"
        )));
    }
    let mut ids = HashSet::with_capacity(accounts.len());
    for account in accounts {
        validate_required_text(
            account.id.as_str(),
            "official account id",
            MAX_WECHAT_ID_CHARS,
        )?;
        validate_required_text(
            account.name.as_str(),
            "official account name",
            MAX_WECHAT_NAME_CHARS,
        )?;
        validate_enum(
            account.account_type.as_str(),
            "official account type",
            WECHAT_ACCOUNT_TYPES,
        )?;
        validate_avatar(account.avatar.as_str(), "official account avatar")?;
        validate_optional_text(
            account.description.as_deref(),
            "official account description",
            MAX_WECHAT_DESCRIPTION_CHARS,
        )?;
        validate_required_text(
            account.app_id.as_str(),
            "official account appId",
            MAX_WECHAT_APP_ID_CHARS,
        )?;
        validate_secret_text(
            account.app_secret.as_deref(),
            "official account appSecret",
            MAX_WECHAT_SECRET_CHARS,
        )?;
        validate_optional_text(
            account.server_url.as_deref(),
            "official account serverUrl",
            MAX_WECHAT_SERVER_URL_CHARS,
        )?;
        validate_secret_text(
            account.token.as_deref(),
            "official account token",
            MAX_WECHAT_TOKEN_CHARS,
        )?;
        validate_secret_text(
            account.encoding_aes_key.as_deref(),
            "official account encodingAesKey",
            MAX_WECHAT_AES_KEY_CHARS,
        )?;
        validate_optional_enum(
            account.encrypt_mode.as_deref(),
            "official account encryptMode",
            WECHAT_ENCRYPT_MODES,
        )?;
        validate_verification_file(
            account.domain_verify_file_name.as_deref(),
            account.domain_verify_file_content.as_deref(),
            "official account domainVerifyFileName",
            "official account domainVerifyFileContent",
        )?;
        validate_text_values(
            account.js_secure_domains.as_deref(),
            "official account jsSecureDomains",
            MAX_WECHAT_OFFICIAL_ACCOUNT_DOMAIN_CHARS,
        )?;
        validate_text_values(
            account.web_auth_domains.as_deref(),
            "official account webAuthDomains",
            MAX_WECHAT_OFFICIAL_ACCOUNT_DOMAIN_CHARS,
        )?;
        validate_text_values(
            account.business_domains.as_deref(),
            "official account businessDomains",
            MAX_WECHAT_OFFICIAL_ACCOUNT_DOMAIN_CHARS,
        )?;
        validate_optional_text(
            account.group.as_deref(),
            "official account group",
            MAX_WECHAT_GROUP_CHARS,
        )?;
        if !ids.insert(account.id.as_str()) {
            return Err(KnowledgeStorageError::InvalidRequest(format!(
                "duplicate official account id: {}",
                account.id
            )));
        }
    }
    Ok(())
}

fn validate_applets(applets: &[KnowledgeWechatApplet]) -> Result<(), KnowledgeStorageError> {
    if applets.len() > MAX_WECHAT_CONFIG_ENTRIES_PER_KIND {
        return Err(KnowledgeStorageError::InvalidRequest(format!(
            "applet count exceeds {MAX_WECHAT_CONFIG_ENTRIES_PER_KIND}"
        )));
    }
    let mut ids = HashSet::with_capacity(applets.len());
    for applet in applets {
        validate_required_text(applet.id.as_str(), "applet id", MAX_WECHAT_ID_CHARS)?;
        validate_required_text(applet.name.as_str(), "applet name", MAX_WECHAT_NAME_CHARS)?;
        validate_required_text(
            applet.app_id.as_str(),
            "applet appId",
            MAX_WECHAT_APP_ID_CHARS,
        )?;
        validate_optional_text(
            applet.original_id.as_deref(),
            "applet originalId",
            MAX_WECHAT_APPLET_ORIGINAL_ID_CHARS,
        )?;
        validate_secret_text(
            applet.app_secret.as_deref(),
            "applet appSecret",
            MAX_WECHAT_SECRET_CHARS,
        )?;
        validate_text_length(
            applet.path.as_str(),
            "applet path",
            MAX_WECHAT_APPLET_PATH_CHARS,
        )?;
        validate_avatar(applet.avatar.as_str(), "applet avatar")?;
        validate_optional_text(
            applet.group.as_deref(),
            "applet group",
            MAX_WECHAT_GROUP_CHARS,
        )?;
        validate_optional_text(
            applet.description.as_deref(),
            "applet description",
            MAX_WECHAT_DESCRIPTION_CHARS,
        )?;
        for (values, field) in [
            (applet.request_domain.as_deref(), "applet requestDomain"),
            (applet.socket_domain.as_deref(), "applet socketDomain"),
            (applet.upload_domain.as_deref(), "applet uploadDomain"),
            (applet.download_domain.as_deref(), "applet downloadDomain"),
            (applet.udp_domain.as_deref(), "applet udpDomain"),
            (applet.tcp_domain.as_deref(), "applet tcpDomain"),
            (applet.business_domain.as_deref(), "applet businessDomain"),
        ] {
            validate_text_values(values, field, MAX_WECHAT_APPLET_ENDPOINT_CHARS)?;
        }
        validate_verification_file(
            applet.domain_verify_file_name.as_deref(),
            applet.domain_verify_file_content.as_deref(),
            "applet domainVerifyFileName",
            "applet domainVerifyFileContent",
        )?;
        validate_secret_text(
            applet.msg_token.as_deref(),
            "applet msgToken",
            MAX_WECHAT_TOKEN_CHARS,
        )?;
        validate_secret_text(
            applet.msg_encoding_aes_key.as_deref(),
            "applet msgEncodingAESKey",
            MAX_WECHAT_AES_KEY_CHARS,
        )?;
        validate_optional_enum(
            applet.msg_data_format.as_deref(),
            "applet msgDataFormat",
            WECHAT_MESSAGE_DATA_FORMATS,
        )?;
        validate_optional_enum(
            applet.msg_encrypt_mode.as_deref(),
            "applet msgEncryptMode",
            WECHAT_ENCRYPT_MODES,
        )?;
        if !ids.insert(applet.id.as_str()) {
            return Err(KnowledgeStorageError::InvalidRequest(format!(
                "duplicate applet id: {}",
                applet.id
            )));
        }
    }
    Ok(())
}

fn validate_required_text(
    value: &str,
    field: &str,
    max_chars: usize,
) -> Result<(), KnowledgeStorageError> {
    if is_blank(Some(value)) {
        return Err(invalid_config(format!("{field} is required")));
    }
    validate_text_length(value, field, max_chars)
}

fn validate_optional_text(
    value: Option<&str>,
    field: &str,
    max_chars: usize,
) -> Result<(), KnowledgeStorageError> {
    if let Some(value) = value {
        validate_text_length(value, field, max_chars)?;
    }
    Ok(())
}

/// Secrets must arrive as plaintext. A value already shaped like the store's own `kbenc:`
/// envelope would be stored verbatim (the cipher passes prefixed values through) and later
/// decrypt to garbage, permanently bricking the tenant config until manual repair.
fn validate_secret_text(
    value: Option<&str>,
    field: &str,
    max_chars: usize,
) -> Result<(), KnowledgeStorageError> {
    validate_optional_text(value, field, max_chars)?;
    if value.is_some_and(|secret| secret.starts_with(ENCRYPTED_VALUE_PREFIX)) {
        return Err(invalid_config(format!(
            "{field} must be the plaintext secret, not an encrypted {ENCRYPTED_VALUE_PREFIX} envelope"
        )));
    }
    Ok(())
}

fn validate_text_length(
    value: &str,
    field: &str,
    max_chars: usize,
) -> Result<(), KnowledgeStorageError> {
    if value.chars().count() > max_chars {
        return Err(invalid_config(format!(
            "{field} exceeds {max_chars} characters"
        )));
    }
    Ok(())
}

fn validate_enum(value: &str, field: &str, allowed: &[&str]) -> Result<(), KnowledgeStorageError> {
    if !allowed.contains(&value) {
        return Err(invalid_config(format!("{field} is not supported")));
    }
    Ok(())
}

fn validate_optional_enum(
    value: Option<&str>,
    field: &str,
    allowed: &[&str],
) -> Result<(), KnowledgeStorageError> {
    if let Some(value) = value {
        validate_enum(value, field, allowed)?;
    }
    Ok(())
}

fn validate_avatar(value: &str, field: &str) -> Result<(), KnowledgeStorageError> {
    validate_required_text(value, field, MAX_WECHAT_AVATAR_CHARS)?;
    let value = value.trim();
    if value.contains('/') || value.contains('\\') || has_uri_scheme(value) {
        return Err(invalid_config(format!(
            "{field} must be an icon value, not a media URI"
        )));
    }
    Ok(())
}

fn has_uri_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic())
        && characters.all(|value| value.is_ascii_alphanumeric() || matches!(value, '+' | '-' | '.'))
}

fn validate_verification_file(
    file_name: Option<&str>,
    content: Option<&str>,
    file_name_field: &str,
    content_field: &str,
) -> Result<(), KnowledgeStorageError> {
    validate_optional_text(
        file_name,
        file_name_field,
        MAX_WECHAT_VERIFY_FILE_NAME_CHARS,
    )?;
    if content.is_some_and(|value| value.len() > MAX_WECHAT_VERIFY_FILE_CONTENT_BYTES) {
        return Err(invalid_config(format!(
            "{content_field} exceeds {MAX_WECHAT_VERIFY_FILE_CONTENT_BYTES} UTF-8 bytes"
        )));
    }
    Ok(())
}

fn validate_text_values(
    values: Option<&[String]>,
    field: &str,
    max_item_chars: usize,
) -> Result<(), KnowledgeStorageError> {
    let Some(values) = values else {
        return Ok(());
    };
    if values.len() > MAX_WECHAT_DOMAIN_VALUES {
        return Err(invalid_config(format!(
            "{field} count exceeds {MAX_WECHAT_DOMAIN_VALUES}"
        )));
    }
    for value in values {
        validate_required_text(value, field, max_item_chars)?;
    }
    Ok(())
}

fn invalid_config(detail: String) -> KnowledgeStorageError {
    KnowledgeStorageError::InvalidRequest(detail)
}

fn merge_official_account_secrets(
    incoming: Vec<KnowledgeWechatOfficialAccount>,
    existing: &[KnowledgeWechatOfficialAccount],
) -> Vec<KnowledgeWechatOfficialAccount> {
    incoming
        .into_iter()
        .map(|mut account| {
            if let Some(previous) = existing.iter().find(|item| item.id == account.id) {
                if !secret_present(account.app_secret.as_deref()) {
                    account.app_secret = previous.app_secret.clone();
                }
                if !secret_present(account.token.as_deref()) {
                    account.token = previous.token.clone();
                }
                if !secret_present(account.encoding_aes_key.as_deref()) {
                    account.encoding_aes_key = previous.encoding_aes_key.clone();
                }
            }
            account
        })
        .collect()
}

fn merge_applet_secrets(
    incoming: Vec<KnowledgeWechatApplet>,
    existing: &[KnowledgeWechatApplet],
) -> Vec<KnowledgeWechatApplet> {
    incoming
        .into_iter()
        .map(|mut applet| {
            if let Some(previous) = existing.iter().find(|item| item.id == applet.id) {
                if !secret_present(applet.app_secret.as_deref()) {
                    applet.app_secret = previous.app_secret.clone();
                }
                if !secret_present(applet.msg_token.as_deref()) {
                    applet.msg_token = previous.msg_token.clone();
                }
                if !secret_present(applet.msg_encoding_aes_key.as_deref()) {
                    applet.msg_encoding_aes_key = previous.msg_encoding_aes_key.clone();
                }
            }
            applet
        })
        .collect()
}

fn secret_present(value: Option<&str>) -> bool {
    value.is_some_and(|secret| !is_blank(Some(secret)))
}

fn redact_official_account(
    mut account: KnowledgeWechatOfficialAccount,
) -> KnowledgeWechatOfficialAccount {
    account.app_secret = None;
    account.token = None;
    account.encoding_aes_key = None;
    account
}

fn redact_applet(mut applet: KnowledgeWechatApplet) -> KnowledgeWechatApplet {
    applet.app_secret = None;
    applet.msg_token = None;
    applet.msg_encoding_aes_key = None;
    applet
}

fn decrypt_config_secrets(config: &mut TenantWechatConfig) -> Result<(), KnowledgeStorageError> {
    for account in &mut config.official_accounts {
        account.app_secret =
            decrypt_optional_secret(account.app_secret.clone()).map_err(cipher_storage_error)?;
        account.token =
            decrypt_optional_secret(account.token.clone()).map_err(cipher_storage_error)?;
        account.encoding_aes_key = decrypt_optional_secret(account.encoding_aes_key.clone())
            .map_err(cipher_storage_error)?;
    }
    for applet in &mut config.applets {
        applet.app_secret =
            decrypt_optional_secret(applet.app_secret.clone()).map_err(cipher_storage_error)?;
        applet.msg_token =
            decrypt_optional_secret(applet.msg_token.clone()).map_err(cipher_storage_error)?;
        applet.msg_encoding_aes_key = decrypt_optional_secret(applet.msg_encoding_aes_key.clone())
            .map_err(cipher_storage_error)?;
    }
    Ok(())
}

fn encrypt_config_secrets(config: &mut TenantWechatConfig) -> Result<(), KnowledgeStorageError> {
    for account in &mut config.official_accounts {
        account.app_secret =
            encrypt_optional_secret(account.app_secret.clone()).map_err(cipher_storage_error)?;
        account.token =
            encrypt_optional_secret(account.token.clone()).map_err(cipher_storage_error)?;
        account.encoding_aes_key = encrypt_optional_secret(account.encoding_aes_key.clone())
            .map_err(cipher_storage_error)?;
    }
    for applet in &mut config.applets {
        applet.app_secret =
            encrypt_optional_secret(applet.app_secret.clone()).map_err(cipher_storage_error)?;
        applet.msg_token =
            encrypt_optional_secret(applet.msg_token.clone()).map_err(cipher_storage_error)?;
        applet.msg_encoding_aes_key = encrypt_optional_secret(applet.msg_encoding_aes_key.clone())
            .map_err(cipher_storage_error)?;
    }
    Ok(())
}

fn cipher_storage_error(
    error: crate::wechat::secret_cipher::SecretCipherError,
) -> KnowledgeStorageError {
    KnowledgeStorageError::Internal(format!("wechat secret cipher error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FailingHeadDrive {
        put_calls: AtomicUsize,
    }

    struct NoIoDrive {
        head_calls: AtomicUsize,
        put_calls: AtomicUsize,
    }

    struct OversizeConfigDrive;

    #[async_trait]
    impl KnowledgeDriveStorage for OversizeConfigDrive {
        async fn put_object(
            &self,
            _request: PutKnowledgeObjectRequest,
        ) -> Result<crate::ports::knowledge_drive_storage::KnowledgeObjectRef, KnowledgeStorageError>
        {
            Err(KnowledgeStorageError::Internal(
                "unexpected config write".to_string(),
            ))
        }

        async fn head_object(
            &self,
            _request: HeadKnowledgeObjectRequest,
        ) -> Result<crate::ports::knowledge_drive_storage::KnowledgeObjectRef, KnowledgeStorageError>
        {
            Ok(crate::ports::knowledge_drive_storage::KnowledgeObjectRef {
                storage_provider_id: "test".to_string(),
                bucket: "test".to_string(),
                object_key: CONFIG_LOGICAL_PATH.to_string(),
                logical_path: CONFIG_LOGICAL_PATH.to_string(),
                object_role: CONFIG_OBJECT_ROLE.to_string(),
                content_type: "application/json".to_string(),
                size_bytes: MAX_WECHAT_CONFIG_BYTES + 1,
                checksum_sha256_hex: None,
                etag: None,
                version_id: None,
            })
        }

        async fn get_object_text(
            &self,
            _object_ref: &crate::ports::knowledge_drive_storage::KnowledgeObjectRef,
        ) -> Result<String, KnowledgeStorageError> {
            panic!("oversize config must be rejected before reading its body")
        }
    }

    #[async_trait]
    impl KnowledgeDriveStorage for FailingHeadDrive {
        async fn put_object(
            &self,
            _request: PutKnowledgeObjectRequest,
        ) -> Result<crate::ports::knowledge_drive_storage::KnowledgeObjectRef, KnowledgeStorageError>
        {
            self.put_calls.fetch_add(1, Ordering::SeqCst);
            Err(KnowledgeStorageError::Internal(
                "unexpected config write".to_string(),
            ))
        }

        async fn head_object(
            &self,
            _request: HeadKnowledgeObjectRequest,
        ) -> Result<crate::ports::knowledge_drive_storage::KnowledgeObjectRef, KnowledgeStorageError>
        {
            Err(KnowledgeStorageError::Upstream(
                "test config read failure".to_string(),
            ))
        }

        async fn get_object_text(
            &self,
            _object_ref: &crate::ports::knowledge_drive_storage::KnowledgeObjectRef,
        ) -> Result<String, KnowledgeStorageError> {
            Err(KnowledgeStorageError::Internal(
                "unexpected config body read".to_string(),
            ))
        }
    }

    #[async_trait]
    impl KnowledgeDriveStorage for NoIoDrive {
        async fn put_object(
            &self,
            _request: PutKnowledgeObjectRequest,
        ) -> Result<crate::ports::knowledge_drive_storage::KnowledgeObjectRef, KnowledgeStorageError>
        {
            self.put_calls.fetch_add(1, Ordering::SeqCst);
            Err(KnowledgeStorageError::Internal(
                "unexpected config write".to_string(),
            ))
        }

        async fn head_object(
            &self,
            _request: HeadKnowledgeObjectRequest,
        ) -> Result<crate::ports::knowledge_drive_storage::KnowledgeObjectRef, KnowledgeStorageError>
        {
            self.head_calls.fetch_add(1, Ordering::SeqCst);
            Err(KnowledgeStorageError::NotFound(
                "test config is absent".to_string(),
            ))
        }

        async fn get_object_text(
            &self,
            _object_ref: &crate::ports::knowledge_drive_storage::KnowledgeObjectRef,
        ) -> Result<String, KnowledgeStorageError> {
            panic!("invalid config must be rejected before reading Drive")
        }
    }

    fn official_account(id: &str) -> KnowledgeWechatOfficialAccount {
        KnowledgeWechatOfficialAccount {
            id: id.to_string(),
            name: "Account".to_string(),
            account_type: "subscription".to_string(),
            avatar: "OA".to_string(),
            description: None,
            app_id: format!("wx-{id}"),
            app_secret: None,
            server_url: None,
            token: None,
            encoding_aes_key: None,
            encrypt_mode: Some("safe".to_string()),
            domain_verify_file_name: None,
            domain_verify_file_content: None,
            js_secure_domains: None,
            web_auth_domains: None,
            business_domains: None,
            group: None,
        }
    }

    fn applet(id: &str) -> KnowledgeWechatApplet {
        KnowledgeWechatApplet {
            id: id.to_string(),
            name: "Applet".to_string(),
            app_id: format!("wx-{id}"),
            original_id: None,
            app_secret: None,
            path: "pages/index".to_string(),
            avatar: "AP".to_string(),
            group: None,
            description: None,
            request_domain: None,
            socket_domain: None,
            upload_domain: None,
            download_domain: None,
            udp_domain: None,
            tcp_domain: None,
            business_domain: None,
            domain_verify_file_name: None,
            domain_verify_file_content: None,
            msg_token: None,
            msg_encoding_aes_key: None,
            msg_data_format: Some("json".to_string()),
            msg_encrypt_mode: Some("safe".to_string()),
        }
    }

    #[test]
    fn secret_merge_preserves_omitted_tokens_when_app_secret_rotates() {
        let mut existing_account = official_account("oa-secret-rotation");
        existing_account.app_secret = Some("old-account-secret".to_string());
        existing_account.token = Some("account-token".to_string());
        existing_account.encoding_aes_key = Some("A".repeat(MAX_WECHAT_AES_KEY_CHARS));

        let mut incoming_account = official_account("oa-secret-rotation");
        incoming_account.app_secret = Some("new-account-secret".to_string());
        let merged_accounts =
            merge_official_account_secrets(vec![incoming_account], &[existing_account]);
        assert_eq!(
            merged_accounts[0].app_secret.as_deref(),
            Some("new-account-secret")
        );
        assert_eq!(merged_accounts[0].token.as_deref(), Some("account-token"));
        assert_eq!(
            merged_accounts[0].encoding_aes_key.as_deref(),
            Some("A".repeat(MAX_WECHAT_AES_KEY_CHARS).as_str())
        );

        let mut existing_applet = applet("applet-secret-rotation");
        existing_applet.app_secret = Some("old-applet-secret".to_string());
        existing_applet.msg_token = Some("applet-token".to_string());
        existing_applet.msg_encoding_aes_key = Some("B".repeat(MAX_WECHAT_AES_KEY_CHARS));

        let mut incoming_applet = applet("applet-secret-rotation");
        incoming_applet.app_secret = Some("new-applet-secret".to_string());
        let merged_applets = merge_applet_secrets(vec![incoming_applet], &[existing_applet]);
        assert_eq!(
            merged_applets[0].app_secret.as_deref(),
            Some("new-applet-secret")
        );
        assert_eq!(merged_applets[0].msg_token.as_deref(), Some("applet-token"));
        assert_eq!(
            merged_applets[0].msg_encoding_aes_key.as_deref(),
            Some("B".repeat(MAX_WECHAT_AES_KEY_CHARS).as_str())
        );
    }

    #[tokio::test]
    async fn replacements_do_not_overwrite_config_when_existing_config_read_fails() {
        let drive = FailingHeadDrive {
            put_calls: AtomicUsize::new(0),
        };
        let store = WechatConfigStore::new(&drive, "tenant-1");

        for error in [
            store
                .replace_official_accounts(Vec::new())
                .await
                .expect_err("official account replacement must fail closed"),
            store
                .replace_applets(Vec::new())
                .await
                .expect_err("applet replacement must fail closed"),
        ] {
            assert!(matches!(error, KnowledgeStorageError::Upstream(_)));
        }
        assert_eq!(drive.put_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn config_load_rejects_declared_oversize_before_body_read() {
        let store = WechatConfigStore::new(&OversizeConfigDrive, "tenant-1");

        let error = store
            .load_official_accounts()
            .await
            .expect_err("oversize config must be rejected");

        assert!(matches!(error, KnowledgeStorageError::InvalidRequest(_)));
    }

    #[test]
    fn config_validation_accepts_declared_boundaries() {
        let accounts = (0..MAX_WECHAT_CONFIG_ENTRIES_PER_KIND)
            .map(|index| official_account(format!("oa-{index}").as_str()))
            .collect::<Vec<_>>();
        validate_official_accounts(&accounts).expect("100 official accounts should be accepted");

        let mut account = official_account("oa-boundary");
        account.avatar = "A".repeat(MAX_WECHAT_AVATAR_CHARS);
        account.domain_verify_file_name = Some("f".repeat(MAX_WECHAT_VERIFY_FILE_NAME_CHARS));
        account.domain_verify_file_content = Some("x".repeat(MAX_WECHAT_VERIFY_FILE_CONTENT_BYTES));
        account.js_secure_domains = Some(
            (0..MAX_WECHAT_DOMAIN_VALUES)
                .map(|index| format!("{index}.example.com"))
                .collect(),
        );
        validate_official_accounts(&[account])
            .expect("official account field boundaries should be accepted");

        let mut applet = applet("applet-boundary");
        applet.request_domain = Some(
            (0..MAX_WECHAT_DOMAIN_VALUES)
                .map(|index| format!("https://api-{index}.example.com/path"))
                .collect(),
        );
        applet.domain_verify_file_content = Some("x".repeat(MAX_WECHAT_VERIFY_FILE_CONTENT_BYTES));
        validate_applets(&[applet]).expect("applet field boundaries should be accepted");
    }

    #[test]
    fn config_validation_rejects_collection_and_text_overflow() {
        let accounts = (0..=MAX_WECHAT_CONFIG_ENTRIES_PER_KIND)
            .map(|index| official_account(format!("oa-{index}").as_str()))
            .collect::<Vec<_>>();
        assert!(matches!(
            validate_official_accounts(&accounts),
            Err(KnowledgeStorageError::InvalidRequest(_))
        ));

        let mut account = official_account("oa-overflow");
        account.name = "n".repeat(MAX_WECHAT_NAME_CHARS + 1);
        assert!(matches!(
            validate_official_accounts(&[account]),
            Err(KnowledgeStorageError::InvalidRequest(_))
        ));

        let mut domain_overflow_applet = applet("applet-domain-overflow");
        domain_overflow_applet.request_domain = Some(vec![
            "api.example.com".to_string();
            MAX_WECHAT_DOMAIN_VALUES + 1
        ]);
        assert!(matches!(
            validate_applets(&[domain_overflow_applet]),
            Err(KnowledgeStorageError::InvalidRequest(_))
        ));

        let mut content_overflow_applet = applet("applet-content-overflow");
        content_overflow_applet.domain_verify_file_content =
            Some("界".repeat(MAX_WECHAT_VERIFY_FILE_CONTENT_BYTES / 3 + 1));
        assert!(matches!(
            validate_applets(&[content_overflow_applet]),
            Err(KnowledgeStorageError::InvalidRequest(_))
        ));
    }

    #[test]
    fn config_validation_rejects_unsupported_enums_and_media_uris() {
        for avatar in [
            "data:image/png;base64,AAAA",
            "https://cdn.example.com/avatar.png",
            "images/avatar.png",
        ] {
            let mut account = official_account("oa-media");
            account.avatar = avatar.to_string();
            assert!(matches!(
                validate_official_accounts(&[account]),
                Err(KnowledgeStorageError::InvalidRequest(_))
            ));
        }

        let mut account = official_account("oa-enum");
        account.account_type = "enterprise".to_string();
        assert!(matches!(
            validate_official_accounts(&[account]),
            Err(KnowledgeStorageError::InvalidRequest(_))
        ));

        let mut applet = applet("applet-enum");
        applet.msg_data_format = Some("yaml".to_string());
        assert!(matches!(
            validate_applets(&[applet]),
            Err(KnowledgeStorageError::InvalidRequest(_))
        ));
    }

    #[tokio::test]
    async fn rejected_config_performs_no_drive_io() {
        let drive = NoIoDrive {
            head_calls: AtomicUsize::new(0),
            put_calls: AtomicUsize::new(0),
        };
        let store = WechatConfigStore::new(&drive, "tenant-1");

        let mut account = official_account("oa-invalid");
        account.avatar = "data:image/png;base64,AAAA".to_string();
        let error = store
            .replace_official_accounts(vec![account])
            .await
            .expect_err("invalid account must be rejected");
        assert!(matches!(error, KnowledgeStorageError::InvalidRequest(_)));

        let mut applet = applet("applet-invalid");
        applet.msg_encrypt_mode = Some("unsupported".to_string());
        let error = store
            .replace_applets(vec![applet])
            .await
            .expect_err("invalid applet must be rejected");
        assert!(matches!(error, KnowledgeStorageError::InvalidRequest(_)));

        assert_eq!(drive.head_calls.load(Ordering::SeqCst), 0);
        assert_eq!(drive.put_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn config_validation_rejects_duplicate_ids() {
        let account = KnowledgeWechatOfficialAccount {
            id: "duplicate".to_string(),
            name: "A".to_string(),
            account_type: "subscription".to_string(),
            avatar: "A".to_string(),
            description: None,
            app_id: "wx1".to_string(),
            app_secret: None,
            server_url: None,
            token: None,
            encoding_aes_key: None,
            encrypt_mode: None,
            domain_verify_file_name: None,
            domain_verify_file_content: None,
            js_secure_domains: None,
            web_auth_domains: None,
            business_domains: None,
            group: None,
        };

        let error = validate_official_accounts(&[account.clone(), account])
            .expect_err("duplicate account ids must be rejected");
        assert!(matches!(error, KnowledgeStorageError::InvalidRequest(_)));
    }

    #[test]
    fn merge_official_account_secrets_preserves_existing_secret() {
        let existing = vec![KnowledgeWechatOfficialAccount {
            id: "1".to_string(),
            name: "A".to_string(),
            account_type: "subscription".to_string(),
            avatar: "A".to_string(),
            description: None,
            app_id: "wx1".to_string(),
            app_secret: Some("secret".to_string()),
            server_url: None,
            token: Some("token".to_string()),
            encoding_aes_key: None,
            encrypt_mode: None,
            domain_verify_file_name: None,
            domain_verify_file_content: None,
            js_secure_domains: None,
            web_auth_domains: None,
            business_domains: None,
            group: None,
        }];
        let incoming = vec![KnowledgeWechatOfficialAccount {
            id: "1".to_string(),
            name: "A".to_string(),
            account_type: "subscription".to_string(),
            avatar: "A".to_string(),
            description: None,
            app_id: "wx1".to_string(),
            app_secret: None,
            server_url: None,
            token: None,
            encoding_aes_key: None,
            encrypt_mode: None,
            domain_verify_file_name: None,
            domain_verify_file_content: None,
            js_secure_domains: None,
            web_auth_domains: None,
            business_domains: None,
            group: None,
        }];
        let merged = merge_official_account_secrets(incoming, &existing);
        assert_eq!(merged[0].app_secret.as_deref(), Some("secret"));
        assert_eq!(merged[0].token.as_deref(), Some("token"));
    }

    #[test]
    fn encrypt_config_secrets_writes_encrypted_prefix_when_key_configured() {
        let _guard = crate::wechat::secret_cipher::test_support::TestEncryptionKeyGuard::with_key(
            "config-store-test-key",
        );
        let mut config = TenantWechatConfig {
            official_accounts: vec![KnowledgeWechatOfficialAccount {
                id: "1".to_string(),
                name: "A".to_string(),
                account_type: "subscription".to_string(),
                avatar: "A".to_string(),
                description: None,
                app_id: "wx1".to_string(),
                app_secret: Some("secret".to_string()),
                server_url: None,
                token: None,
                encoding_aes_key: None,
                encrypt_mode: None,
                domain_verify_file_name: None,
                domain_verify_file_content: None,
                js_secure_domains: None,
                web_auth_domains: None,
                business_domains: None,
                group: None,
            }],
            applets: vec![],
        };
        encrypt_config_secrets(&mut config).expect("encrypt config secrets");
        assert!(config.official_accounts[0]
            .app_secret
            .as_deref()
            .unwrap()
            .starts_with("kbenc:v2:"));
        decrypt_config_secrets(&mut config).expect("decrypt config secrets");
        assert_eq!(
            config.official_accounts[0].app_secret.as_deref(),
            Some("secret")
        );
    }

    #[test]
    fn config_validation_rejects_encrypted_envelope_values_as_secrets() {
        for envelope in [
            format!("kbenc:v1:{}", "A".repeat(32)),
            "kbenc:v2:c2FsdA==:bm9uY2U=:Y2lwaGVydGV4dA==".to_string(),
        ] {
            let mut account = official_account("oa-envelope-secret");
            account.app_secret = Some(envelope.clone());
            assert!(matches!(
                validate_official_accounts(&[account]),
                Err(KnowledgeStorageError::InvalidRequest(_))
            ));

            let mut account_token = official_account("oa-envelope-token");
            account_token.token = Some(envelope.clone());
            assert!(matches!(
                validate_official_accounts(&[account_token]),
                Err(KnowledgeStorageError::InvalidRequest(_))
            ));

            let mut applet = applet("applet-envelope-secret");
            applet.msg_token = Some(envelope);
            assert!(matches!(
                validate_applets(&[applet]),
                Err(KnowledgeStorageError::InvalidRequest(_))
            ));
        }
    }
}
