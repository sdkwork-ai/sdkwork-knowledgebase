use sdkwork_knowledgebase_contract::agent_chat::{
    KnowledgeAgentChatMessage, KnowledgeAgentChatRequest,
};

#[test]
fn chat_request_without_history_deserializes_as_an_empty_history() {
    let request: KnowledgeAgentChatRequest = serde_json::from_value(serde_json::json!({
        "message": "how do refunds work?"
    }))
    .expect("legacy chat request without history must deserialize");

    assert!(request.history.is_empty());
}

#[test]
fn chat_history_round_trips_as_camel_case_turns() {
    let request = KnowledgeAgentChatRequest {
        tenant_id: 100_001,
        actor_id: Some(7),
        message: "and the second one?".to_string(),
        history: vec![
            KnowledgeAgentChatMessage {
                role: "user".to_string(),
                content: "what is the refund policy?".to_string(),
            },
            KnowledgeAgentChatMessage {
                role: "assistant".to_string(),
                content: "Refunds are accepted within 30 days.".to_string(),
            },
        ],
        mode: None,
        session_id: None,
        model_provider_id: None,
        model_id: None,
        agent_implementation_id: None,
    };

    let value = serde_json::to_value(&request).expect("chat request serializes");
    assert_eq!(value["history"][0]["role"], "user");
    assert_eq!(value["history"][1]["role"], "assistant");
    assert_eq!(value["message"], "and the second one?");

    let decoded: KnowledgeAgentChatRequest =
        serde_json::from_value(value).expect("chat request round-trips");
    assert_eq!(decoded, request);
}

#[test]
fn empty_history_serializes_as_an_empty_array() {
    let value = serde_json::to_value(KnowledgeAgentChatRequest {
        tenant_id: 1,
        actor_id: None,
        message: "hello".to_string(),
        history: Vec::new(),
        mode: None,
        session_id: None,
        model_provider_id: None,
        model_id: None,
        agent_implementation_id: None,
    })
    .expect("chat request serializes");
    assert_eq!(value["history"], serde_json::json!([]));
}
