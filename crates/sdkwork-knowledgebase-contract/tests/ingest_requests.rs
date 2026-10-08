use sdkwork_knowledgebase_contract::ingest::{KnowledgeDriveImportRequest, KnowledgeIngestRequest};

#[test]
fn ingest_request_serializes_markdown_payload_without_storage_locator_leaks() {
    let request = KnowledgeIngestRequest {
        space_id: 7,
        title: "API payload note".to_string(),
        payload_markdown: "# API Note\n\nImportant source text.".to_string(),
        idempotency_key: "api-note-1".to_string(),
        source_url: None,
    };

    let json = serde_json::to_value(request).unwrap();

    assert_eq!(json["spaceId"], "7");
    assert_eq!(
        json["payloadMarkdown"],
        "# API Note\n\nImportant source text."
    );
    assert!(json.get("driveObjectKey").is_none());
    assert!(json.get("presignedUrl").is_none());
}

#[test]
fn ingest_request_serializes_source_url_for_server_side_fetch() {
    let request = KnowledgeIngestRequest {
        space_id: 7,
        title: "Web article".to_string(),
        payload_markdown: String::new(),
        source_url: Some("https://example.com/article".to_string()),
        idempotency_key: "web-link-1".to_string(),
    };

    let json = serde_json::to_value(request).unwrap();

    assert_eq!(json["sourceUrl"], "https://example.com/article");
    assert_eq!(json["payloadMarkdown"], "");
}

#[test]
fn drive_import_request_uses_opaque_drive_ids_without_storage_locator_leaks() {
    let request = KnowledgeDriveImportRequest {
        space_id: 7,
        title: "Quarterly Report".to_string(),
        drive_space_id: "drv-kb-001".to_string(),
        drive_node_id: "node-report".to_string(),
        idempotency_key: "drive-quarterly-report".to_string(),
        language: Some("en".to_string()),
    };

    let json = serde_json::to_value(request).unwrap();

    assert_eq!(json["spaceId"], "7");
    assert_eq!(json["driveSpaceId"], "drv-kb-001");
    assert_eq!(json["driveNodeId"], "node-report");
    assert_eq!(json["idempotencyKey"], "drive-quarterly-report");
    assert!(json.get("driveStorageProviderId").is_none());
    assert!(json.get("driveBucket").is_none());
    assert!(json.get("driveObjectKey").is_none());
    assert!(json.get("fileBytes").is_none());
}
