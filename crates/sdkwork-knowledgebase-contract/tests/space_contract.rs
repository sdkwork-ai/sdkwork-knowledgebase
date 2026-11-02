use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::{KnowledgeSpace, KnowledgeSpaceStatus};

fn snowflake_space() -> KnowledgeSpace {
    KnowledgeSpace {
        id: 332_473_351_878_475_800,
        uuid: "kb-test-uuid".to_string(),
        created_by: 42,
        name: "Snowflake Space".to_string(),
        description: None,
        drive_space_id: Some("kb-drive-space".to_string()),
        status: KnowledgeSpaceStatus::Active,
        okf_bundle_initialized: true,
        knowledge_mode: KnowledgeAgentKnowledgeMode::Rag,
    }
}

#[test]
fn knowledge_space_serializes_snowflake_id_as_json_string() {
    let space = snowflake_space();

    let json = serde_json::to_value(&space).expect("serialize knowledge space");
    assert_eq!(
        json.get("id").and_then(|value| value.as_str()),
        Some("332473351878475800")
    );
}

#[test]
fn knowledge_space_never_serializes_server_side_ownership() {
    // `created_by` is object-level authorization state. It must never reach an
    // API response, so the public wire shape and generated SDK models stay
    // unchanged even though the field exists on the shared contract type.
    let space = snowflake_space();

    let json = serde_json::to_value(&space).expect("serialize knowledge space");
    assert!(
        json.get("createdBy").is_none(),
        "server-side ownership metadata must not be exposed on the wire: {json}"
    );
}

#[test]
fn knowledge_space_deserializes_snowflake_id_from_json_string() {
    let json = serde_json::json!({
        "id": "332473351878475800",
        "uuid": "kb-test-uuid",
        "name": "Snowflake Space",
        "status": "active",
        "okfBundleInitialized": true,
        "knowledgeMode": "rag"
    });

    let space: KnowledgeSpace =
        serde_json::from_value(json).expect("deserialize knowledge space from string id");
    assert_eq!(space.id, 332_473_351_878_475_800);
    // A response payload never carries ownership, so decoding leaves it at the
    // default instead of trusting a client-supplied owner.
    assert_eq!(space.created_by, 0);
}
