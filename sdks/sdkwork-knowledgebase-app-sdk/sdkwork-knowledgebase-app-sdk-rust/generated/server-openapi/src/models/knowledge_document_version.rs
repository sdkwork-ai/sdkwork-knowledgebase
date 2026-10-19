use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeDocumentVersion {
    pub id: String,

    #[serde(rename = "documentId")]
    pub document_id: String,

    #[serde(rename = "versionNo")]
    pub version_no: String,

    #[serde(rename = "originalObjectRefId")]
    pub original_object_ref_id: String,

    #[serde(rename = "checksumSha256Hex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum_sha256_hex: Option<String>,

    #[serde(rename = "sizeBytes")]
    pub size_bytes: String,

    #[serde(rename = "mimeType")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    #[serde(rename = "parseState")]
    pub parse_state: String,

    #[serde(rename = "indexState")]
    pub index_state: String,
}
