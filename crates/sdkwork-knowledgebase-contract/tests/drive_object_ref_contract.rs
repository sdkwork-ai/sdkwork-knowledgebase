use sdkwork_knowledgebase_contract::KnowledgeDriveObjectRef;

#[test]
fn drive_object_ref_contract_serializes_public_identity_without_storage_locators() {
    let object_ref = KnowledgeDriveObjectRef {
        id: 91,
        space_id: 7,
        drive_space_id: Some("drv-kb-001".to_string()),
        drive_node_id: Some("node-001".to_string()),
        logical_path: Some("raw/documents/report.md".to_string()),
        drive_provider_kind: "sdkwork-drive".to_string(),
        drive_storage_provider_id: "provider-kb".to_string(),
        drive_bucket: "knowledgebase-source".to_string(),
        drive_object_key: "incoming/quarterly-report.md".to_string(),
        drive_object_version: Some("v1".to_string()),
        drive_etag: Some("etag".to_string()),
        content_type: Some("text/markdown; charset=utf-8".to_string()),
        size_bytes: 128,
        checksum_sha256_hex: Some("abc123".to_string()),
        object_role: "original_document".to_string(),
        access_mode: "managed".to_string(),
    };

    let json = serde_json::to_value(object_ref).unwrap();

    assert_eq!(json["spaceId"], "7");
    assert_eq!(json["driveSpaceId"], "drv-kb-001");
    assert_eq!(json["driveNodeId"], "node-001");
    assert_eq!(json["logicalPath"], "raw/documents/report.md");
    assert!(json.get("driveProviderKind").is_none());
    assert!(json.get("driveStorageProviderId").is_none());
    assert!(json.get("driveBucket").is_none());
    assert!(json.get("driveObjectKey").is_none());
    assert!(json.get("driveObjectVersion").is_none());
    assert!(json.get("driveEtag").is_none());
    assert_eq!(json["objectRole"], "original_document");
    assert_eq!(json["accessMode"], "managed");
    assert!(json.get("presignedUrl").is_none());
    assert!(json.get("providerCredentials").is_none());
    assert!(json.get("payloadBytes").is_none());
}
