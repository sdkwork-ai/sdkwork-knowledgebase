use sdkwork_knowledgebase_contract::document::{
    CreateKnowledgeDocumentRequest, KnowledgeDocument, KnowledgeDocumentState,
    KnowledgeDocumentVersion, KnowledgeDocumentVersionState, KnowledgeDocumentVisibility,
};
use sdkwork_knowledgebase_contract::source::{KnowledgeSource, KnowledgeSourceType};

#[test]
fn source_contract_serializes_source_type_and_drive_projection_as_camel_case() {
    let source = KnowledgeSource {
        id: 1,
        space_id: 7,
        source_type: KnowledgeSourceType::Api,
        provider: Some("app-api".to_string()),
        drive_bucket: None,
        drive_prefix: Some("inbox/api/1".to_string()),
        connector_metadata_json: Some(r#"{"origin":"external_import"}"#.to_string()),
    };

    let json = serde_json::to_value(source).unwrap();

    assert_eq!(json["spaceId"], "7");
    assert_eq!(json["sourceType"], "api");
    assert_eq!(json["provider"], "app-api");
    assert_eq!(json["drivePrefix"], "inbox/api/1");
    assert_eq!(
        json["connectorMetadataJson"],
        r#"{"origin":"external_import"}"#
    );
}

#[test]
fn document_contract_keeps_metadata_and_drive_references_out_of_payload_bytes() {
    let document = KnowledgeDocument {
        id: 2,
        space_id: 7,
        source_id: Some(1),
        original_file_drive_node_id: Some("node-api-payload".to_string()),
        title: "API payload note".to_string(),
        mime_type: Some("text/markdown; charset=utf-8".to_string()),
        language: Some("en".to_string()),
        current_version_id: None,
        visibility: KnowledgeDocumentVisibility::Space,
        content_state: KnowledgeDocumentState::Ready,
        index_state: KnowledgeDocumentVersionState::Pending,
    };
    let version = KnowledgeDocumentVersion {
        id: 3,
        document_id: 2,
        version_no: 1,
        original_object_ref_id: 42,
        checksum_sha256_hex: Some("abc123".to_string()),
        size_bytes: 128,
        mime_type: Some("text/markdown; charset=utf-8".to_string()),
        parse_state: KnowledgeDocumentVersionState::Pending,
        index_state: KnowledgeDocumentVersionState::Pending,
    };

    let document_json = serde_json::to_value(document).unwrap();
    let version_json = serde_json::to_value(version).unwrap();

    assert_eq!(document_json["id"], "2");
    assert_eq!(document_json["spaceId"], "7");
    assert_eq!(document_json["currentVersionId"], serde_json::Value::Null);
    assert_eq!(document_json["originalFileDriveNodeId"], "node-api-payload");
    assert_eq!(document_json["visibility"], "space");
    assert_eq!(document_json["contentState"], "ready");
    assert_eq!(version_json["originalObjectRefId"], "42");
    assert_eq!(version_json["parseState"], "pending");
    assert!(document_json.get("payloadMarkdown").is_none());
    assert!(version_json.get("payloadBytes").is_none());
}

#[test]
fn create_document_request_accepts_missing_optional_int64_fields() {
    let request: CreateKnowledgeDocumentRequest = serde_json::from_value(serde_json::json!({
        "spaceId": "332872288108019712",
        "title": "Visibility update",
        "visibility": "public"
    }))
    .expect("deserialize create document request without optional ids");

    assert_eq!(request.space_id, 332_872_288_108_019_712);
    assert_eq!(request.source_id, None);
}
