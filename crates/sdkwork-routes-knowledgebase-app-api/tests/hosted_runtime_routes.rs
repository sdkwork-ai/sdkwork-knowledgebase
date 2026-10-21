use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    is_postgres_database_url, PostgresKnowledgeOkfConceptStore,
    SqlxKnowledgeEngineProviderBindingStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::{
    KnowledgeOkfConceptStore, UpsertKnowledgeOkfConceptRecord,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_binding_store::{
    KnowledgeEngineProviderBindingStore, KnowledgeEngineProviderScope,
    RecordKnowledgeEngineProviderTestResult,
};
use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineCapability;
use sdkwork_knowledgebase_contract::provider_binding::{
    CreateKnowledgeEngineProviderBindingRequest,
    CreateKnowledgeEngineProviderCredentialReferenceRequest, KnowledgeEngineProviderBinding,
};
use sdkwork_knowledgebase_contract::OkfConceptPublishState;
use sdkwork_knowledgebase_test_support::provider_execution::knowledge_execution_context;
use sdkwork_routes_knowledgebase_app_api::{dev_auth, KnowledgebaseRuntime};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, MutexGuard};
use tower::util::ServiceExt;

static EXTERNAL_ADAPTER_ENV_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

async fn lock_external_adapter_env() -> MutexGuard<'static, ()> {
    EXTERNAL_ADAPTER_ENV_TEST_LOCK.lock().await
}

fn optional_postgres_database_url() -> Option<String> {
    std::env::var("SDKWORK_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|url| is_postgres_database_url(url))
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_app_router_lists_documents() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let space_id = create_space(&app, "Document List Space").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/app/v3/api/knowledge/documents?spaceId={space_id}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_ne!(
        response.status(),
        StatusCode::NOT_IMPLEMENTED,
        "hosted app api must not return operation_unsupported for documents.list"
    );

    let body = response_body_json(response).await;
    assert!(body["items"].is_array());
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_app_router_creates_manual_document_without_client_source_id() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let space_id = create_space(&app, "Manual Document Space").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/documents")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "spaceId": space_id,
                        "title": "manual note",
                        "mimeType": "text/markdown"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response_body_json(response).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "manual document create failed: {body}"
    );
    assert_eq!(body["title"], "manual note");
    assert_eq!(json_u64_field(&body, "spaceId"), Some(space_id));
    assert!(
        json_u64_field(&body, "sourceId").is_some(),
        "backend-created manual document must receive an internal API source: {body}"
    );
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_router_serves_provider_health() {
    let _env_guard = lock_external_adapter_env().await;
    clear_external_adapter_env();
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/backend/v3/api/knowledge/provider_health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_ne!(
        response.status(),
        StatusCode::NOT_IMPLEMENTED,
        "hosted backend must not return operation_unsupported for providerHealth.list"
    );

    let body = response_body_json(response).await;
    assert_eq!(body["status"], "ok");
    let provider_id = body["providerId"].as_str().expect("providerId");
    assert!(
        provider_id.contains("engine.knowledge.okf.native"),
        "providerId must include okf native engine: {provider_id}"
    );
    assert!(
        provider_id.contains("engine.knowledge.rag.native"),
        "providerId must include rag native engine: {provider_id}"
    );
    assert!(
        !provider_id.contains("engine.knowledge.external."),
        "unconfigured external engines must not be reported as active health providers: {provider_id}"
    );
    assert!(
        body["checkedAt"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "provider health must report the check timestamp: {body}"
    );
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_audit_legal_hold_lifecycle_is_operator_scoped() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let subject_id = format!("http-legal-hold-actor-{nonce}");

    let create_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/compliance/audit_legal_holds")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "subjectType": "actor",
                        "subjectId": subject_id,
                        "reason": "HTTP retention drill",
                        "caseReference": "CASE-HTTP"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created = response_body_json(create_response).await;
    let hold = created.clone();
    let hold_id = json_u64_field(&hold, "holdId").expect("holdId");
    assert_eq!(hold["status"], "active");
    assert_eq!(hold["createdBy"], "99");
    assert_eq!(hold["subjectId"], subject_id);

    let list_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/backend/v3/api/knowledge/compliance/audit_legal_holds?page_size=200")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let listed = response_body_json(list_response).await;
    let listed_ids = listed["items"]
        .as_array()
        .expect("legal hold items")
        .iter()
        .filter_map(|item| json_u64_field(item, "holdId"))
        .collect::<Vec<_>>();
    assert!(
        listed_ids.contains(&hold_id),
        "created hold must be listed: {listed}"
    );

    let release_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/compliance/audit_legal_holds/{hold_id}/release"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(release_response.status(), StatusCode::CREATED);
    let released = response_body_json(release_response).await;
    assert_eq!(released["status"], "released");
    assert_eq!(released["releasedBy"], "99");

    let duplicate_release = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/compliance/audit_legal_holds/{hold_id}/release"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(duplicate_release.status(), StatusCode::CONFLICT);

    sqlx::query("DELETE FROM kb_audit_legal_hold WHERE id = $1")
        .bind(hold_id as i64)
        .execute(runtime.pool())
        .await
        .expect("cleanup legal hold");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_provider_management_is_scoped_versioned_and_secret_safe() {
    let _env_guard = lock_external_adapter_env().await;
    clear_external_adapter_env();
    let _dify_base_url =
        TempEnvVar::set("SDKWORK_KNOWLEDGEBASE_DIFY_BASE_URL", "http://127.0.0.1:9");
    let _managed_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_DIFY_MANAGED_KEY",
        "managed-test-secret",
    );
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));
    let space_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Managed Provider Space","description":"Provider management contract","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "managed Provider space id").await;

    let invalid_credential_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/provider_credential_references")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"implementationId":"engine.knowledge.external.dify","displayName":"Invalid credential","referenceLocator":"secret://unapproved/provider/key"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        invalid_credential_response.status(),
        StatusCode::BAD_REQUEST
    );

    let credential_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/provider_credential_references")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"implementationId":"engine.knowledge.external.dify","displayName":"Dify managed credential","referenceLocator":"env://SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_DIFY_MANAGED_KEY"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let credential_status = credential_response.status();
    let credential_raw = response_raw_json(credential_response).await;
    assert_eq!(
        credential_status,
        StatusCode::CREATED,
        "credential create failed: {credential_raw}"
    );
    assert_eq!(credential_raw["code"], 0);
    assert!(credential_raw["traceId"].as_str().is_some());
    let credential = credential_raw["data"]["item"].clone();
    let credential_id = json_u64_field(&credential, "id").expect("credential id");
    assert!(credential["id"].is_string());
    assert!(credential["version"].is_string());
    assert!(credential.get("referenceLocator").is_none());
    assert!(!credential_raw
        .to_string()
        .contains("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_DIFY_MANAGED_KEY"));

    let credential_list_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/backend/v3/api/knowledge/provider_credential_references?page_size=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(credential_list_response.status(), StatusCode::OK);
    let credential_page = response_body_json(credential_list_response).await;
    assert_eq!(credential_page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(credential_page["pageInfo"]["mode"], "cursor");

    let binding_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/spaces/{space_id}/provider_bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "implementationId": "engine.knowledge.external.dify",
                        "remoteResourceType": "dataset",
                        "remoteResourceId": "managed-dataset",
                        "credentialReferenceId": credential_id.to_string()
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::CREATED);
    let binding = response_body_json(binding_response).await;
    let binding_id = json_u64_field(&binding, "id").expect("binding id");
    assert!(binding["spaceId"].is_string());
    assert_eq!(binding["lifecycleState"], "draft");

    let wrong_space_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/backend/v3/api/knowledge/spaces/{}/provider_bindings/{binding_id}",
                    space_id + 1
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wrong_space_response.status(), StatusCode::FORBIDDEN);

    let updated_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!(
                    "/backend/v3/api/knowledge/spaces/{space_id}/provider_bindings/{binding_id}"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "remoteResourceId": "managed-dataset-v2",
                        "clearCredentialReference": false,
                        "expectedVersion": binding["version"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated_response.status(), StatusCode::OK);
    let updated = response_body_json(updated_response).await;
    assert_eq!(updated["remoteResourceId"], "managed-dataset-v2");

    let stale_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!(
                    "/backend/v3/api/knowledge/spaces/{space_id}/provider_bindings/{binding_id}"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "remoteResourceId": "stale-dataset",
                        "clearCredentialReference": false,
                        "expectedVersion": binding["version"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stale_response.status(), StatusCode::CONFLICT);

    let invalid_rotate_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/provider_credential_references/{credential_id}/rotate"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"referenceLocator":"secret://unapproved/rotated/key","expectedVersion":"0"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_rotate_response.status(), StatusCode::BAD_REQUEST);

    let rotate_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/provider_credential_references/{credential_id}/rotate"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"referenceLocator":"env://SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_DIFY_ROTATED_MANAGED_KEY","expectedVersion":"0"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rotate_response.status(), StatusCode::OK);
    let rotate_command = response_body_json(rotate_response).await;
    assert_eq!(rotate_command["accepted"], true);
    assert_eq!(rotate_command["resourceId"], credential_id.to_string());
    assert_eq!(rotate_command["status"], "current");

    let revoke_response = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/provider_credential_references/{credential_id}/revoke"
                ))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"expectedVersion":"1"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke_response.status(), StatusCode::OK);
    let revoke_command = response_body_json(revoke_response).await;
    assert_eq!(revoke_command["accepted"], true);
    assert_eq!(revoke_command["status"], "revoked");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_provider_migration_is_scoped_recoverable_and_reversible() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));
    let space_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Provider Migration Space","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "Provider migration space id").await;

    let store = SqlxKnowledgeEngineProviderBindingStore::new(runtime.pool().clone());
    let scope = KnowledgeEngineProviderScope {
        tenant_id: runtime.tenant_id(),
        organization_id: runtime.organization_id(),
    };
    let source = create_tested_provider_binding(
        &store,
        scope,
        space_id,
        "engine.knowledge.external.dify",
        "migration-source",
    )
    .await;
    let source = store
        .activate_binding(scope, source.id, "migration-test", source.version)
        .await
        .expect("activate migration source");
    let target = create_tested_provider_binding(
        &store,
        scope,
        space_id,
        "engine.knowledge.external.ragflow",
        "migration-target",
    )
    .await;

    let create_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/spaces/{space_id}/provider_migrations"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "sourceBindingId": source.id.to_string(),
                        "targetBindingId": target.id.to_string(),
                        "idempotencyKey": "hosted-provider-migration-001",
                        "expectedSourceVersion": source.version.to_string(),
                        "expectedTargetVersion": target.version.to_string(),
                        "observationSeconds": 60
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let create_status = create_response.status();
    let create_body = response_raw_json(create_response).await;
    assert_eq!(create_status, StatusCode::CREATED, "{create_body}");
    let operation = create_body["data"]["item"].clone();
    let operation_id = json_u64_field(&operation, "id").expect("migration operation id");
    assert_eq!(operation["operationState"], "dry_run");
    assert!(operation["version"].is_string());

    let list_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/backend/v3/api/knowledge/spaces/{space_id}/provider_migrations?operation_state=dry_run&page_size=1"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let page = response_body_json(list_response).await;
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(page["pageInfo"]["mode"], "cursor");

    let wrong_space = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/backend/v3/api/knowledge/spaces/{}/provider_migrations/{operation_id}",
                    space_id + 1
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wrong_space.status(), StatusCode::NOT_FOUND);

    let processed = runtime
        .process_provider_migrations(
            "hosted-provider-migration-worker",
            std::time::Duration::from_secs(30),
            4,
        )
        .await
        .expect("process migration through cutover");
    assert_eq!(processed.processed, 4);
    let observing_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/backend/v3/api/knowledge/spaces/{space_id}/provider_migrations/{operation_id}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(observing_response.status(), StatusCode::OK);
    let observing = response_body_json(observing_response).await;
    assert_eq!(observing["operationState"], "observing");
    assert_eq!(
        store
            .get_active_binding_for_space(scope, space_id)
            .await
            .expect("retrieve active target")
            .expect("target active")
            .id,
        target.id
    );

    let rollback_response = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/spaces/{space_id}/provider_migrations/{operation_id}/rollback"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "expectedVersion": observing["version"] }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rollback_response.status(), StatusCode::OK);
    let rollback = response_body_json(rollback_response).await;
    assert_eq!(rollback["accepted"], true);
    assert_eq!(rollback["status"], "rolling_back");

    let rolled_back = runtime
        .process_provider_migrations(
            "hosted-provider-migration-worker",
            std::time::Duration::from_secs(30),
            1,
        )
        .await
        .expect("process Provider rollback");
    assert_eq!(rolled_back.rolled_back, 1);
    assert_eq!(
        store
            .get_active_binding_for_space(scope, space_id)
            .await
            .expect("retrieve restored source")
            .expect("source restored")
            .id,
        source.id
    );
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_provider_health_degrades_for_failed_external_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    clear_external_adapter_env();
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/datasets/ds-health"))
        .and(wiremock::matchers::header(
            "authorization",
            "Bearer health-test-key",
        ))
        .respond_with(wiremock::ResponseTemplate::new(503))
        .expect(3)
        .mount(&mock_server)
        .await;

    let _dify_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_DIFY_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _dify_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_DIFY_API_KEY",
        "health-test-key",
    );
    let _dify_dataset = TempEnvVar::set("SDKWORK_KNOWLEDGEBASE_DIFY_DATASET_ID", "ds-health");

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));
    let space_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Health Binding Space","description":"Binding-aware health","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "health space id").await;
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.dify",
        "ds-health",
    )
    .await;

    let response = backend
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/backend/v3/api/knowledge/provider_health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_json(response).await;
    assert_eq!(body["status"], "degraded");
    let provider_id = body["providerId"].as_str().expect("providerId");
    assert!(
        provider_id.contains("engine.knowledge.external.dify"),
        "configured Dify adapter must participate in aggregate health: {provider_id}"
    );
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_router_lists_sources() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/backend/v3/api/knowledge/sources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_json(response).await;
    assert!(body["items"].is_array());
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_app_router_upserts_okf_concept() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"OKF Test Space","description":"Hosted upsert integration"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space = response_body_json(space_response).await;
    let space_id = json_u64_field(&space, "id").expect("created space id");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/app/v3/api/knowledge/okf/concepts/upsert")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r##"{{"spaceId":{space_id},"conceptId":"tables/users","markdown":"---\ntype: Entity\ntitle: Users\n---\n# Users\n","actor":"author","publish":false}}"##
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_ne!(
        response.status(),
        StatusCode::NOT_IMPLEMENTED,
        "hosted app api must not return operation_unsupported for okf.concepts.update"
    );

    let body = response_body_json(response).await;
    assert_eq!(body["conceptId"], "tables/users");
    assert_eq!(body["title"], "Users");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_app_lists_okf_concepts_for_space() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));

    let space_id = create_space(&app, "Concept List Space").await;
    publish_okf_concept(
        &app,
        space_id,
        "tables/users",
        "---\ntype: Entity\ntitle: Users\n---\n# Users\n\nUser table.\n\n# Citations\n\n[1] [Src](https://example.com)\n",
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/app/v3/api/knowledge/okf/concepts?spaceId={space_id}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_json(response).await;
    assert!(body["items"]
        .as_array()
        .expect("concept items")
        .iter()
        .any(|item| item["conceptId"] == "tables/users"));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_app_pages_okf_concepts_and_revisions_with_standard_list_envelopes() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let space_id = create_space(&app, "OKF Pagination Space").await;
    let concepts = PostgresKnowledgeOkfConceptStore::new(runtime.pool().clone(), 1, 0);
    let mut revision_concept_id = None;
    let mut other_concept_id = None;

    for index in (0..205).rev() {
        let concept = concepts
            .upsert_concept(UpsertKnowledgeOkfConceptRecord {
                space_id,
                concept_id: format!("topics/concept-{index:04}"),
                title: format!("Concept {index:04}"),
                concept_type: "Topic".to_string(),
                logical_path: format!("okf/topics/concept-{index:04}.md"),
                description: format!("Concept summary {index:04}"),
                source_count: 0,
                tags: vec![],
                publish_state: OkfConceptPublishState::Published,
            })
            .await
            .expect("insert paginated concept fixture");
        if index == 0 {
            revision_concept_id = Some(concept.id);
        } else if index == 1 {
            other_concept_id = Some(concept.id);
        }
    }
    let revision_concept_id = revision_concept_id.expect("revision concept id");
    let other_concept_id = other_concept_id.expect("other concept id");
    for revision_no in 1..=205_u64 {
        insert_okf_revision_fixture(runtime.pool(), revision_concept_id, revision_no).await;
    }

    let first_concepts = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/app/v3/api/knowledge/okf/concepts?spaceId={space_id}&page_size=200"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first_concepts.status(), StatusCode::OK);
    let first_concepts = response_raw_json(first_concepts).await;
    assert_standard_cursor_page(&first_concepts, 200, true);
    assert_eq!(
        first_concepts["data"]["items"][0]["conceptId"],
        "topics/concept-0000"
    );
    assert_eq!(
        first_concepts["data"]["items"][199]["conceptId"],
        "topics/concept-0199"
    );
    assert!(first_concepts["data"].get("item").is_none());
    let concept_cursor = first_concepts["data"]["pageInfo"]["nextCursor"]
        .as_str()
        .expect("concept next cursor")
        .to_string();
    assert_ne!(concept_cursor, "topics/concept-0199");

    let second_concepts = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/app/v3/api/knowledge/okf/concepts?spaceId={space_id}&page_size=200&cursor={concept_cursor}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second_concepts.status(), StatusCode::OK);
    let second_concepts = response_raw_json(second_concepts).await;
    assert_standard_cursor_page(&second_concepts, 5, false);
    assert_eq!(
        second_concepts["data"]["items"][0]["conceptId"],
        "topics/concept-0200"
    );
    assert_eq!(
        second_concepts["data"]["items"][4]["conceptId"],
        "topics/concept-0204"
    );

    let first_revisions = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/app/v3/api/knowledge/okf/concepts/{revision_concept_id}/revisions?page_size=200"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first_revisions.status(), StatusCode::OK);
    let first_revisions = response_raw_json(first_revisions).await;
    assert_standard_cursor_page(&first_revisions, 200, true);
    assert_eq!(first_revisions["data"]["items"][0]["revisionNo"], 1);
    assert_eq!(first_revisions["data"]["items"][199]["revisionNo"], 200);
    assert!(first_revisions["data"].get("item").is_none());
    let revision_cursor = first_revisions["data"]["pageInfo"]["nextCursor"]
        .as_str()
        .expect("revision next cursor")
        .to_string();
    assert_ne!(revision_cursor, "200");

    let second_revisions = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/app/v3/api/knowledge/okf/concepts/{revision_concept_id}/revisions?page_size=200&cursor={revision_cursor}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second_revisions.status(), StatusCode::OK);
    let second_revisions = response_raw_json(second_revisions).await;
    assert_standard_cursor_page(&second_revisions, 5, false);
    assert_eq!(second_revisions["data"]["items"][0]["revisionNo"], 201);
    assert_eq!(second_revisions["data"]["items"][4]["revisionNo"], 205);

    for uri in [
        format!(
            "/app/v3/api/knowledge/okf/concepts/{revision_concept_id}/revisions?cursor={concept_cursor}"
        ),
        format!(
            "/app/v3/api/knowledge/okf/concepts?spaceId={space_id}&cursor={revision_cursor}"
        ),
        format!(
            "/app/v3/api/knowledge/okf/concepts/{other_concept_id}/revisions?cursor={revision_cursor}"
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_invalid_parameter_problem(response).await;
    }

    let other_space_id = create_space(&app, "Other OKF Pagination Space").await;
    let replay = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/app/v3/api/knowledge/okf/concepts?spaceId={other_space_id}&cursor={concept_cursor}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_invalid_parameter_problem(replay).await;
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_app_okf_lists_reject_invalid_and_noncanonical_pagination_queries() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let space_id = create_space(&app, "OKF Invalid Pagination Space").await;
    let concepts = PostgresKnowledgeOkfConceptStore::new(runtime.pool().clone(), 1, 0);
    let concept = concepts
        .upsert_concept(UpsertKnowledgeOkfConceptRecord {
            space_id,
            concept_id: "topics/query-validation".to_string(),
            title: "Query validation".to_string(),
            concept_type: "Topic".to_string(),
            logical_path: "okf/topics/query-validation.md".to_string(),
            description: "Query validation fixture".to_string(),
            source_count: 0,
            tags: vec![],
            publish_state: OkfConceptPublishState::Published,
        })
        .await
        .expect("insert query validation concept");

    for invalid_query in [
        "page_size=0",
        "page_size=201",
        "page_size=-1",
        "page_size=not-a-number",
        "pageSize=20",
        "%70ageSize=20",
        "limit=20",
        "page_no=1",
        "pageNo=1",
        "per_page=20",
        "size=20",
    ] {
        for uri in [
            format!("/app/v3/api/knowledge/okf/concepts?spaceId={space_id}&{invalid_query}"),
            format!(
                "/app/v3/api/knowledge/okf/concepts/{}/revisions?{invalid_query}",
                concept.id
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::GET)
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_invalid_parameter_problem(response).await;
        }
    }
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_lists_okf_candidates_for_space() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_id = create_space(&app, "Candidate Space").await;
    let response = backend
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/backend/v3/api/knowledge/okf/candidates?spaceId={space_id}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_ne!(response.status(), StatusCode::NOT_IMPLEMENTED);
    let body = response_body_json(response).await;
    assert!(body["items"].is_array());
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_registers_okf_profile_and_rebuilds_index() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_id = create_space(&app, "Profile Space").await;
    publish_okf_concept(
        &app,
        space_id,
        "entities/widget",
        "---\ntype: Entity\ntitle: Widget\n---\n# Widget\n\nWidget entity.\n\n# Citations\n\n[1] [Src](https://example.com)\n",
    )
    .await;

    let profile_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/okf/profile")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"profileVersion":"2026-06-19"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);

    let rebuild_response = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/okf/index/rebuild")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"spaceId":{space_id}}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rebuild_response.status(), StatusCode::OK);
    let rebuild_body = response_body_json(rebuild_response).await;
    assert!(rebuild_body["markdown"]
        .as_str()
        .unwrap_or("")
        .contains("/entities/index.md"));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_imports_staged_export_bundle() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_id = create_space(&app, "Backend Import Space").await;
    publish_okf_concept(
        &app,
        space_id,
        "entities/gadget",
        "---\ntype: Entity\ntitle: Gadget\n---\n# Gadget\n\nA gadget.\n\n# Citations\n\n[1] [Src](https://example.com)\n",
    )
    .await;

    let export_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/okf/exports")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"exportType":"okf_strict","stageForImport":true,"importId":"backend-import"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export_response.status(), StatusCode::CREATED);

    let import_response = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/okf/imports")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"importType":"okf_strict","importId":"backend-import"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::CREATED);
    let import_body = response_body_json(import_response).await;
    assert!(import_body["importedConceptCount"].as_u64().unwrap_or(0) >= 1);
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_app_export_stages_bundle_for_import_roundtrip() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));

    let source_space = create_space(&app, "Export Source").await;
    publish_okf_concept(
        &app,
        source_space,
        "entities/widget",
        "---\ntype: Entity\ntitle: Widget\n---\n# Widget\n\nA durable widget.\n\n# Citations\n\n[1] [Src](https://example.com)\n",
    )
    .await;

    let export_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/okf/exports")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{source_space},"exportType":"okf_strict","stageForImport":true,"importId":"api-roundtrip"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export_response.status(), StatusCode::CREATED);
    let export_body = response_body_json(export_response).await;
    assert_eq!(export_body["importId"], "api-roundtrip");
    assert_eq!(
        export_body["stagedImportRoot"],
        "inbox/drive-imports/api-roundtrip"
    );

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/okf/imports")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{source_space},"importType":"okf_strict","importId":"api-roundtrip"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let import_status = import_response.status();
    let import_body = response_body_json(import_response).await;
    if import_status != StatusCode::CREATED {
        panic!("import staged bundle failed: {import_body}");
    }
    assert!(import_body["importedConceptCount"].as_u64().unwrap_or(0) >= 1);

    let concepts_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/app/v3/api/knowledge/okf/concepts?spaceId={source_space}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(concepts_response.status(), StatusCode::OK);
    let concepts_body = response_body_json(concepts_response).await;
    assert!(concepts_body["items"]
        .as_array()
        .expect("concept list")
        .iter()
        .any(|item| item["conceptId"] == "entities/widget"));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_export_stages_bundle_for_import() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let source_space = create_space(&app, "Backend Export Source").await;
    publish_okf_concept(
        &app,
        source_space,
        "entities/widget",
        "---\ntype: Entity\ntitle: Widget\n---\n# Widget\n\nA durable widget.\n\n# Citations\n\n[1] [Src](https://example.com)\n",
    )
    .await;

    let export_response = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/okf/exports")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{source_space},"exportType":"okf_strict","stageForImport":true,"importId":"backend-roundtrip"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export_response.status(), StatusCode::CREATED);
    let export_body = response_body_json(export_response).await;
    assert_eq!(export_body["importId"], "backend-roundtrip");
    assert_eq!(
        export_body["stagedImportRoot"],
        "inbox/drive-imports/backend-roundtrip"
    );
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_runs_okf_lint_job_for_space() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_id = create_space(&app, "Lint Space").await;
    publish_okf_concept(
        &app,
        space_id,
        "entities/lintable",
        "---\ntype: Entity\ntitle: Lintable\n---\n# Lintable\n\nLint target.\n\n# Citations\n\n[1] [Src](https://example.com)\n",
    )
    .await;

    let response = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/okf/lint_runs")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"spaceId":{space_id}}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_body_json(response).await;
    assert_eq!(body["state"], "succeeded");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_runs_okf_compile_job_for_space() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_id = create_space(&app, "Compile Space").await;
    publish_okf_concept(
        &app,
        space_id,
        "entities/compilable",
        "---\ntype: Entity\ntitle: Compilable\n---\n# Compilable\n\nCompile target.\n\n# Citations\n\n[1] [Src](https://example.com)\n",
    )
    .await;

    let response = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/okf/compile_jobs")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"spaceId":{space_id}}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_body_json(response).await;
    assert_eq!(body["state"], "succeeded");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_approves_okf_candidate() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_id = create_space(&app, "Approve Space").await;
    stage_okf_concept(
        &app,
        space_id,
        "entities/approve-me",
        "---\ntype: Entity\ntitle: Approve Me\n---\n# Approve Me\n\nPending review.\n",
    )
    .await;

    let candidates = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/backend/v3/api/knowledge/okf/candidates?spaceId={space_id}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(candidates.status(), StatusCode::OK);
    let candidate_items = response_body_json(candidates).await["items"]
        .as_array()
        .expect("candidate items")
        .clone();
    let candidate_id = candidate_items
        .iter()
        .find(|item| item["state"] == "candidate_ready")
        .and_then(|item| item["id"].as_u64())
        .expect("open candidate id");

    let approve_response = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/okf/candidates/{candidate_id}/approve"
                ))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reviewerId":99,"note":"approved in test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(approve_response.status(), StatusCode::OK);
    let approve_body = response_body_json(approve_response).await;
    assert_eq!(approve_body["state"], "published");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_rejects_okf_candidate() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_id = create_space(&app, "Reject Space").await;
    stage_okf_concept(
        &app,
        space_id,
        "entities/reject-me",
        "---\ntype: Entity\ntitle: Reject Me\n---\n# Reject Me\n\nNot ready.\n",
    )
    .await;

    let candidates = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/backend/v3/api/knowledge/okf/candidates?spaceId={space_id}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(candidates.status(), StatusCode::OK);
    let candidate_id = response_body_json(candidates).await["items"]
        .as_array()
        .expect("candidate items")
        .iter()
        .find(|item| item["state"] == "candidate_ready")
        .and_then(|item| item["id"].as_u64())
        .expect("open candidate id");

    let reject_response = backend
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/backend/v3/api/knowledge/okf/candidates/{candidate_id}/reject"
                ))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reviewerId":99,"note":"rejected in test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reject_response.status(), StatusCode::OK);
    let reject_body = response_body_json(reject_response).await;
    assert_eq!(reject_body["state"], "rejected");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_dify_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"External Hosted Space","description":"External knowledge mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"dify"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.dify",
        "ds-unconfigured",
    )
    .await;
    let source_body = response_body_json(source_response).await;
    assert_eq!(source_body["provider"], "dify");
    assert_eq!(source_body["sourceType"], "connector");

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.dify");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_ragflow_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"RAGFlow Hosted Space","description":"External RAGFlow mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"ragflow"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.ragflow",
        "resolver-ragflow",
    )
    .await;
    let source_body = response_body_json(source_response).await;
    assert_eq!(source_body["provider"], "ragflow");
    assert_eq!(source_body["sourceType"], "connector");

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.ragflow");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_onyx_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Onyx Hosted Space","description":"External Onyx mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"onyx"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.onyx",
        "resolver-onyx",
    )
    .await;

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.onyx");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_anythingllm_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"AnythingLLM Hosted Space","description":"External AnythingLLM mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"anythingllm"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.anythingllm",
        "resolver-anythingllm",
    )
    .await;

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.anythingllm");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_open_webui_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Open WebUI Hosted Space","description":"External Open WebUI mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"open-webui"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.open-webui",
        "resolver-open-webui",
    )
    .await;

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.open-webui");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_flowise_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Flowise Hosted Space","description":"External Flowise mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"flowise"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.flowise",
        "resolver-flowise",
    )
    .await;

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.flowise");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_chroma_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Chroma Hosted Space","description":"External Chroma mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"chroma"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.chroma",
        "resolver-chroma",
    )
    .await;

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.chroma");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_qdrant_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Qdrant Hosted Space","description":"External Qdrant mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"qdrant"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.qdrant",
        "resolver-qdrant",
    )
    .await;

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.qdrant");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_weaviate_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Weaviate Hosted Space","description":"External Weaviate mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"weaviate"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.weaviate",
        "resolver-weaviate",
    )
    .await;

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.weaviate");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_backend_resolves_unconfigured_haystack_without_startup_credential_access() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Haystack Hosted Space","description":"External Haystack mode","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"haystack"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.haystack",
        "resolver-haystack",
    )
    .await;

    let implementation_id = runtime
        .resolve_knowledge_engine_implementation_id_for_space(space_id)
        .await
        .expect("Provider selection must not resolve credentials");
    assert_eq!(implementation_id, "engine.knowledge.external.haystack");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_rejects_unconfigured_external_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    clear_dify_adapter_env();
    clear_ragflow_adapter_env();
    clear_onyx_adapter_env();
    clear_anythingllm_adapter_env();
    clear_open_webui_adapter_env();
    clear_flowise_adapter_env();
    clear_chroma_adapter_env();
    clear_qdrant_adapter_env();
    clear_weaviate_adapter_env();
    clear_haystack_adapter_env();
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"External Agent Space","description":"External agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"dify"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.dify",
        "ds-unconfigured",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"External Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the external knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::BAD_REQUEST,
        "unconfigured external adapter must fail agent chat before LLM invocation"
    );
    let chat_body = response_body_json(chat_response).await;
    let detail = chat_body["detail"]
        .as_str()
        .or_else(|| chat_body["title"].as_str())
        .unwrap_or("");
    assert!(
        detail.to_ascii_lowercase().contains("unsupported")
            || detail.to_ascii_lowercase().contains("adapter")
            || detail.to_ascii_lowercase().contains("catalog")
            || detail.to_ascii_lowercase().contains("dify")
            || detail.to_ascii_lowercase().contains("dataset"),
        "expected unconfigured adapter rejection detail, got: {chat_body}"
    );
}

/// F-16 regression: an agent profile with zero enabled space bindings has no
/// space ACL to consult, so only the authenticated creator may read, update, or
/// delete it. A second tenant user must receive `403` on every operation, while
/// the creator keeps full access.
#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_agent_profile_without_bindings_is_owner_only() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let owner = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let other_user = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(99));

    let profile_body = |name: &str| {
        format!(
            r#"{{"tenantId":"1","name":"{name}","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","knowledgeMode":"rag","status":"active"}}"#
        )
    };

    let create_response = owner
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(profile_body("Owner Only Agent")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created = response_body_json(create_response).await;
    let profile_id = json_u64_field(&created, "profileId").expect("created profile id");
    assert_eq!(
        created.get("createdBy"),
        None,
        "server-side ownership metadata must not be exposed on the wire: {created}"
    );

    let profile_uri = format!("/app/v3/api/knowledge/agent_profiles/{profile_id}");

    let other_retrieve = other_user
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&profile_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        other_retrieve.status(),
        StatusCode::FORBIDDEN,
        "user B must not retrieve user A's zero-binding profile"
    );
    let problem = response_body_json(other_retrieve).await;
    assert_eq!(
        problem["code"], 40301,
        "denial must be a typed permission problem: {problem}"
    );

    let other_update = other_user
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(&profile_uri)
                .header("content-type", "application/json")
                .body(Body::from(profile_body("Hijacked Agent")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        other_update.status(),
        StatusCode::FORBIDDEN,
        "user B must not update user A's zero-binding profile"
    );

    let other_delete = other_user
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(&profile_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        other_delete.status(),
        StatusCode::FORBIDDEN,
        "user B must not delete user A's zero-binding profile"
    );

    let owner_retrieve = owner
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&profile_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        owner_retrieve.status(),
        StatusCode::OK,
        "the creator must retain access to its own zero-binding profile"
    );

    let owner_update = owner
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(&profile_uri)
                .header("content-type", "application/json")
                .body(Body::from(profile_body("Owner Only Agent Updated")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        owner_update.status(),
        StatusCode::OK,
        "the creator must be able to update its own zero-binding profile"
    );

    let owner_delete = owner
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(&profile_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        owner_delete.status(),
        StatusCode::NO_CONTENT,
        "the creator must be able to delete its own zero-binding profile"
    );
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_dify_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/datasets/ds-hosted-e2e/retrieve"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "records": [{
                    "segment": {
                        "id": "seg-hosted",
                        "content": "hosted external knowledge snippet",
                        "document": { "name": "Hosted External Doc" }
                    },
                    "score": 0.95
                }]
            })),
        )
        .mount(&mock_server)
        .await;

    let _dify_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_DIFY_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _dify_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_DIFY_API_KEY",
        "hosted-test-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured External Space","description":"Configured external agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"dify","connectorMetadataJson":"{{\"datasetId\":\"ds-hosted-e2e\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.dify",
        "ds-hosted-e2e",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured External Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the external knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured external engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted External Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted External Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_read_resolves_configured_dify_citation_document() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/datasets/ds-read-e2e/retrieve"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "records": [{
                    "segment": {
                        "id": "seg-read",
                        "document_id": "doc-read",
                        "content": "hosted citation snippet",
                        "document": { "name": "Hosted Read Doc" }
                    },
                    "score": 0.95
                }]
            })),
        )
        .mount(&mock_server)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(
            "/datasets/ds-read-e2e/documents/doc-read/segments/seg-read",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "content": "hosted segment full body",
                    "document": { "name": "Hosted Read Doc" }
                }
            })),
        )
        .mount(&mock_server)
        .await;

    let _dify_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_DIFY_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _dify_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_DIFY_API_KEY",
        "hosted-read-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"External Read Space","description":"External read E2E","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"dify","connectorMetadataJson":"{{\"datasetId\":\"ds-read-e2e\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.dify",
        "ds-read-e2e",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"External Read Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the external knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chat_response.status(), StatusCode::OK);
    let chat_body = response_body_json(chat_response).await;
    let logical_path = chat_body["citations"][0]["logicalPath"]
        .as_str()
        .expect("citation logical path");
    let (_, local_document_id) = logical_path
        .split_once('/')
        .expect("scoped citation logical path");
    assert_eq!(local_document_id, "doc-read#seg-read");

    let document = runtime
        .read_knowledge_engine_document_for_space(
            &knowledge_execution_context(
                runtime.tenant_id(),
                runtime.organization_id(),
                space_id,
                None,
                "trace-hosted-runtime-read",
            ),
            space_id,
            local_document_id,
        )
        .await
        .expect("read citation document through SPI");
    assert_eq!(document.content, "hosted segment full body");
    assert_eq!(document.title, "Hosted Read Doc");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_read_resolves_configured_ragflow_citation_document() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/api/v1/retrieval"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": 0,
                "data": {
                    "chunks": [{
                        "id": "chunk-read",
                        "content": "hosted citation snippet",
                        "document_id": "doc-read",
                        "document_keyword": "Hosted RAGFlow Read Doc",
                        "similarity": 0.95
                    }]
                }
            })),
        )
        .mount(&mock_server)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(
            "/api/v1/datasets/ds-read-ragflow/documents/doc-read/chunks/chunk-read",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": 0,
                "data": {
                    "content": "hosted ragflow chunk full body",
                    "document_keyword": "Hosted RAGFlow Read Doc"
                }
            })),
        )
        .mount(&mock_server)
        .await;

    let _ragflow_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_RAGFLOW_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _ragflow_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_RAGFLOW_API_KEY",
        "hosted-ragflow-read-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"RAGFlow Read Space","description":"RAGFlow read E2E","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"ragflow","connectorMetadataJson":"{{\"datasetId\":\"ds-read-ragflow\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.ragflow",
        "ds-read-ragflow",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"RAGFlow Read Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the RAGFlow knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chat_response.status(), StatusCode::OK);
    let chat_body = response_body_json(chat_response).await;
    let logical_path = chat_body["citations"][0]["logicalPath"]
        .as_str()
        .expect("citation logical path");
    let (_, local_document_id) = logical_path
        .split_once('/')
        .expect("scoped citation logical path");
    assert_eq!(local_document_id, "doc-read#chunk-read");

    let document = runtime
        .read_knowledge_engine_document_for_space(
            &knowledge_execution_context(
                runtime.tenant_id(),
                runtime.organization_id(),
                space_id,
                None,
                "trace-hosted-runtime-read",
            ),
            space_id,
            local_document_id,
        )
        .await
        .expect("read citation document through SPI");
    assert_eq!(document.content, "hosted ragflow chunk full body");
    assert_eq!(document.title, "Hosted RAGFlow Read Doc");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_read_resolves_configured_open_webui_citation_document() {
    const SNIPPET: &str = "hosted openwebui read snippet";
    use sdkwork_knowledgebase_engine_open_webui::chunk_id_from_content;

    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/api/v1/retrieval/query/collection",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "distances": [[0.95]],
                "documents": [[SNIPPET]],
                "metadatas": [[{
                    "source": "Hosted Open WebUI Read Doc",
                    "url": "file://read.txt"
                }]]
            })),
        )
        .mount(&mock_server)
        .await;

    let _open_webui_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_OPEN_WEBUI_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _open_webui_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_OPEN_WEBUI_API_KEY",
        "hosted-open-webui-read-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Open WebUI Read Space","description":"Open WebUI read E2E","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"open-webui","connectorMetadataJson":"{{\"datasetId\":\"kb-read-open-webui\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.open-webui",
        "kb-read-open-webui",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Open WebUI Read Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Open WebUI knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chat_response.status(), StatusCode::OK);
    let chat_body = response_body_json(chat_response).await;
    let logical_path = chat_body["citations"][0]["logicalPath"]
        .as_str()
        .expect("citation logical path");
    let (_, local_document_id) = logical_path
        .split_once('/')
        .expect("scoped citation logical path");
    let chunk_id = chunk_id_from_content(SNIPPET);
    assert_eq!(
        local_document_id,
        format!("Hosted Open WebUI Read Doc#{chunk_id}")
    );

    let document = runtime
        .read_knowledge_engine_document_for_space(
            &knowledge_execution_context(
                runtime.tenant_id(),
                runtime.organization_id(),
                space_id,
                None,
                "trace-hosted-runtime-read",
            ),
            space_id,
            local_document_id,
        )
        .await
        .expect("read citation document through SPI");
    assert_eq!(document.content, SNIPPET);
    assert_eq!(document.title, "Hosted Open WebUI Read Doc");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_read_resolves_configured_flowise_citation_document() {
    const SNIPPET: &str = "hosted flowise read snippet";
    use sdkwork_knowledgebase_engine_flowise::chunk_id_from_content;

    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/api/v1/document-store/vectorstore/query",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "timeTaken": 8,
                "docs": [{
                    "pageContent": SNIPPET,
                    "metadata": {
                        "source": "Hosted Flowise Read Doc",
                        "url": "file://read.txt"
                    }
                }]
            })),
        )
        .mount(&mock_server)
        .await;

    let _flowise_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_FLOWISE_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _flowise_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_FLOWISE_API_KEY",
        "hosted-flowise-read-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Flowise Read Space","description":"Flowise read E2E","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"flowise","connectorMetadataJson":"{{\"datasetId\":\"store-read-flowise\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.flowise",
        "store-read-flowise",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Flowise Read Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Flowise knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chat_response.status(), StatusCode::OK);
    let chat_body = response_body_json(chat_response).await;
    let logical_path = chat_body["citations"][0]["logicalPath"]
        .as_str()
        .expect("citation logical path");
    let (_, local_document_id) = logical_path
        .split_once('/')
        .expect("scoped citation logical path");
    let chunk_id = chunk_id_from_content(SNIPPET);
    assert_eq!(
        local_document_id,
        format!("Hosted Flowise Read Doc#{chunk_id}")
    );

    let document = runtime
        .read_knowledge_engine_document_for_space(
            &knowledge_execution_context(
                runtime.tenant_id(),
                runtime.organization_id(),
                space_id,
                None,
                "trace-hosted-runtime-read",
            ),
            space_id,
            local_document_id,
        )
        .await
        .expect("read citation document through SPI");
    assert_eq!(document.content, SNIPPET);
    assert_eq!(document.title, "Hosted Flowise Read Doc");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_read_resolves_configured_qdrant_citation_document() {
    const SNIPPET: &str = "hosted qdrant read snippet";
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    let collection_name = "kb-read-qdrant";
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(format!(
            "/collections/{collection_name}/points/query"
        )))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "points": [{
                        "id": "point-read",
                        "score": 0.95,
                        "payload": {
                            "title": "Hosted Qdrant Read Doc",
                            "text": SNIPPET,
                            "source": "file://read.txt"
                        }
                    }]
                },
                "status": "ok"
            })),
        )
        .mount(&mock_server)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(format!(
            "/collections/{collection_name}/points"
        )))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [{
                    "id": "point-read",
                    "payload": {
                        "title": "Hosted Qdrant Read Doc",
                        "text": SNIPPET,
                        "source": "file://read.txt"
                    }
                }],
                "status": "ok"
            })),
        )
        .mount(&mock_server)
        .await;

    let _qdrant_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_QDRANT_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _qdrant_query_model = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_QDRANT_QUERY_MODEL",
        "sentence-transformers/all-minilm-l6-v2",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Qdrant Read Space","description":"Qdrant read E2E","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"qdrant","connectorMetadataJson":"{{\"datasetId\":\"{collection_name}\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.qdrant",
        collection_name,
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Qdrant Read Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Qdrant knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chat_response.status(), StatusCode::OK);
    let chat_body = response_body_json(chat_response).await;
    let logical_path = chat_body["citations"][0]["logicalPath"]
        .as_str()
        .expect("citation logical path");
    let (_, local_document_id) = logical_path
        .split_once('/')
        .expect("scoped citation logical path");
    assert_eq!(local_document_id, "Hosted Qdrant Read Doc#point-read");

    let document = runtime
        .read_knowledge_engine_document_for_space(
            &knowledge_execution_context(
                runtime.tenant_id(),
                runtime.organization_id(),
                space_id,
                None,
                "trace-hosted-runtime-read",
            ),
            space_id,
            local_document_id,
        )
        .await
        .expect("read citation document through SPI");
    assert_eq!(document.content, SNIPPET);
    assert_eq!(document.title, "Hosted Qdrant Read Doc");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_ragflow_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/api/v1/retrieval"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": 0,
                "data": {
                    "chunks": [{
                        "id": "chunk-hosted",
                        "content": "hosted ragflow knowledge snippet",
                        "document_id": "doc-hosted",
                        "document_keyword": "Hosted RAGFlow Doc",
                        "similarity": 0.95
                    }]
                }
            })),
        )
        .mount(&mock_server)
        .await;

    let _ragflow_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_RAGFLOW_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _ragflow_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_RAGFLOW_API_KEY",
        "hosted-ragflow-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured RAGFlow Space","description":"Configured RAGFlow agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"ragflow","connectorMetadataJson":"{{\"datasetId\":\"ds-ragflow-hosted\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.ragflow",
        "ds-ragflow-hosted",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured RAGFlow Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the RAGFlow knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured RAGFlow engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted RAGFlow Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted RAGFlow Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_onyx_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/search"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [{
                    "title": "Hosted Onyx Doc",
                    "url": "https://example.com/onyx-hosted",
                    "content": "hosted onyx knowledge snippet",
                    "source_type": "web"
                }]
            })),
        )
        .mount(&mock_server)
        .await;

    let _onyx_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_ONYX_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _onyx_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_ONYX_API_KEY",
        "hosted-onyx-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured Onyx Space","description":"Configured Onyx agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"onyx"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.onyx",
        "onyx-hosted",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured Onyx Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Onyx knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured Onyx engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted Onyx Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted Onyx Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_anythingllm_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/api/v1/workspace/ws-hosted/vector-search",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [{
                    "id": "chunk-hosted",
                    "text": "hosted anythingllm knowledge snippet",
                    "score": 0.95,
                    "metadata": {
                        "title": "Hosted AnythingLLM Doc",
                        "url": "file://hosted.txt"
                    }
                }]
            })),
        )
        .mount(&mock_server)
        .await;

    let _anythingllm_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_ANYTHINGLLM_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _anythingllm_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_ANYTHINGLLM_API_KEY",
        "hosted-anythingllm-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured AnythingLLM Space","description":"Configured AnythingLLM agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"anythingllm","connectorMetadataJson":"{{\"workspaceSlug\":\"ws-hosted\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.anythingllm",
        "ws-hosted",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured AnythingLLM Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the AnythingLLM knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured AnythingLLM engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted AnythingLLM Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted AnythingLLM Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_open_webui_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/api/v1/retrieval/query/collection",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "distances": [[0.95]],
                "documents": [["hosted openwebui knowledge snippet"]],
                "metadatas": [[{
                    "source": "Hosted Open WebUI Doc",
                    "url": "file://hosted.txt"
                }]]
            })),
        )
        .mount(&mock_server)
        .await;

    let _open_webui_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_OPEN_WEBUI_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _open_webui_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_OPEN_WEBUI_API_KEY",
        "hosted-open-webui-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured Open WebUI Space","description":"Configured Open WebUI agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"open-webui","connectorMetadataJson":"{{\"datasetId\":\"kb-hosted\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.open-webui",
        "kb-hosted",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured Open WebUI Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Open WebUI knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured Open WebUI engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted Open WebUI Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted Open WebUI Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_flowise_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(
            "/api/v1/document-store/vectorstore/query",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "timeTaken": 10,
                "docs": [{
                    "pageContent": "hosted flowise knowledge snippet",
                    "metadata": {
                        "source": "Hosted Flowise Doc",
                        "url": "file://hosted.txt"
                    }
                }]
            })),
        )
        .mount(&mock_server)
        .await;

    let _flowise_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_FLOWISE_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _flowise_credential = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_FLOWISE_API_KEY",
        "hosted-flowise-key",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured Flowise Space","description":"Configured Flowise agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"flowise","connectorMetadataJson":"{{\"datasetId\":\"store-hosted\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.flowise",
        "store-hosted",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured Flowise Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Flowise knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured Flowise engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted Flowise Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted Flowise Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_chroma_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    let collection_id = "603a7b51-ae7c-4b0a-8865-e454ed2f6766";
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(format!(
            "/api/v2/tenants/default_tenant/databases/default_database/collections/{collection_id}/query"
        )))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ids": [["rec-hosted"]],
                "documents": [["hosted chroma knowledge snippet"]],
                "metadatas": [[{
                    "title": "Hosted Chroma Doc",
                    "source": "file://hosted.txt"
                }]],
                "distances": [[0.08]]
            })),
        )
        .mount(&mock_server)
        .await;

    let _chroma_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_CHROMA_BASE_URL",
        mock_server.uri().as_str(),
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured Chroma Space","description":"Configured Chroma agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"chroma","connectorMetadataJson":"{{\"datasetId\":\"{collection_id}\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.chroma",
        collection_id,
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured Chroma Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Chroma knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured Chroma engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted Chroma Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted Chroma Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_read_resolves_configured_chroma_citation_document() {
    const SNIPPET: &str = "hosted chroma read snippet";
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    let collection_id = "603a7b51-ae7c-4b0a-8865-e454ed2f6766";
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(format!(
            "/api/v2/tenants/default_tenant/databases/default_database/collections/{collection_id}/query"
        )))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ids": [["rec-read"]],
                "documents": [[SNIPPET]],
                "metadatas": [[{
                    "title": "Hosted Chroma Read Doc",
                    "source": "file://read.txt"
                }]],
                "distances": [[0.08]]
            })),
        )
        .mount(&mock_server)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(format!(
            "/api/v2/tenants/default_tenant/databases/default_database/collections/{collection_id}/get"
        )))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ids": ["rec-read"],
                "documents": [SNIPPET],
                "metadatas": [{
                    "title": "Hosted Chroma Read Doc",
                    "source": "file://read.txt"
                }]
            })),
        )
        .mount(&mock_server)
        .await;

    let _chroma_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_CHROMA_BASE_URL",
        mock_server.uri().as_str(),
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Chroma Read Space","description":"Chroma read E2E","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"chroma","connectorMetadataJson":"{{\"datasetId\":\"{collection_id}\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.chroma",
        collection_id,
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Chroma Read Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Chroma knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chat_response.status(), StatusCode::OK);
    let chat_body = response_body_json(chat_response).await;
    let logical_path = chat_body["citations"][0]["logicalPath"]
        .as_str()
        .expect("citation logical path");
    let (_, local_document_id) = logical_path
        .split_once('/')
        .expect("scoped citation logical path");
    assert_eq!(local_document_id, "Hosted Chroma Read Doc#rec-read");

    let document = runtime
        .read_knowledge_engine_document_for_space(
            &knowledge_execution_context(
                runtime.tenant_id(),
                runtime.organization_id(),
                space_id,
                None,
                "trace-hosted-runtime-read",
            ),
            space_id,
            local_document_id,
        )
        .await
        .expect("read citation document through SPI");
    assert_eq!(document.content, SNIPPET);
    assert_eq!(document.title, "Hosted Chroma Read Doc");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_qdrant_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    let collection_name = "policies-hosted";
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(format!(
            "/collections/{collection_name}/points/query"
        )))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "points": [{
                        "id": "pt-hosted",
                        "score": 0.95,
                        "payload": {
                            "title": "Hosted Qdrant Doc",
                            "text": "hosted qdrant knowledge snippet",
                            "source": "file://hosted.txt"
                        }
                    }]
                },
                "status": "ok"
            })),
        )
        .mount(&mock_server)
        .await;

    let _qdrant_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_QDRANT_BASE_URL",
        mock_server.uri().as_str(),
    );
    let _qdrant_query_model = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_QDRANT_QUERY_MODEL",
        "sentence-transformers/all-minilm-l6-v2",
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured Qdrant Space","description":"Configured Qdrant agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"qdrant","connectorMetadataJson":"{{\"datasetId\":\"{collection_name}\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.qdrant",
        collection_name,
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured Qdrant Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Qdrant knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured Qdrant engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted Qdrant Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted Qdrant Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_weaviate_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    let class_name = "KnowledgeChunk";
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/v1/graphql"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "Get": {
                        class_name: [{
                            "title": "Hosted Weaviate Doc",
                            "content": "hosted weaviate knowledge snippet",
                            "_additional": {
                                "id": "obj-hosted",
                                "certainty": 0.95
                            }
                        }]
                    }
                }
            })),
        )
        .mount(&mock_server)
        .await;

    let _weaviate_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_WEAVIATE_BASE_URL",
        mock_server.uri().as_str(),
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured Weaviate Space","description":"Configured Weaviate agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"weaviate","connectorMetadataJson":"{{\"datasetId\":\"{class_name}\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.weaviate",
        class_name,
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured Weaviate Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Weaviate knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured Weaviate engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted Weaviate Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted Weaviate Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_read_resolves_configured_weaviate_citation_document() {
    const SNIPPET: &str = "hosted weaviate read snippet";
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    let class_name = "KnowledgeChunk";
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/v1/graphql"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "Get": {
                        class_name: [{
                            "title": "Hosted Weaviate Read Doc",
                            "content": SNIPPET,
                            "_additional": {
                                "id": "obj-read",
                                "certainty": 0.95
                            }
                        }]
                    }
                }
            })),
        )
        .mount(&mock_server)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(format!(
            "/v1/objects/{class_name}/obj-read"
        )))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "properties": {
                    "title": "Hosted Weaviate Read Doc",
                    "content": SNIPPET,
                    "source": "file://read.txt"
                }
            })),
        )
        .mount(&mock_server)
        .await;

    let _weaviate_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_WEAVIATE_BASE_URL",
        mock_server.uri().as_str(),
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Weaviate Read Space","description":"Weaviate read E2E","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"weaviate","connectorMetadataJson":"{{\"datasetId\":\"{class_name}\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.weaviate",
        class_name,
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Weaviate Read Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Weaviate knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chat_response.status(), StatusCode::OK);
    let chat_body = response_body_json(chat_response).await;
    let logical_path = chat_body["citations"][0]["logicalPath"]
        .as_str()
        .expect("citation logical path");
    let (_, local_document_id) = logical_path
        .split_once('/')
        .expect("scoped citation logical path");
    assert_eq!(local_document_id, "Hosted Weaviate Read Doc#obj-read");

    let document = runtime
        .read_knowledge_engine_document_for_space(
            &knowledge_execution_context(
                runtime.tenant_id(),
                runtime.organization_id(),
                space_id,
                None,
                "trace-hosted-runtime-read",
            ),
            space_id,
            local_document_id,
        )
        .await
        .expect("read citation document through SPI");
    assert_eq!(document.content, SNIPPET);
    assert_eq!(document.title, "Hosted Weaviate Read Doc");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_agent_chat_succeeds_with_configured_haystack_adapter() {
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    let pipeline_name = "retrieval_pipeline";
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(format!("/{pipeline_name}/run")))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retriever": {
                    "documents": [{
                        "id": "doc-hosted",
                        "content": "hosted haystack knowledge snippet",
                        "meta": {
                            "title": "Hosted Haystack Doc",
                            "source": "file://hosted.txt"
                        },
                        "score": 0.93
                    }]
                }
            })),
        )
        .mount(&mock_server)
        .await;

    let _haystack_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_HAYSTACK_BASE_URL",
        mock_server.uri().as_str(),
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Configured Haystack Space","description":"Configured Haystack agent chat","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"haystack","connectorMetadataJson":"{{\"datasetId\":\"{pipeline_name}\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.haystack",
        pipeline_name,
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Configured Haystack Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Haystack knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "configured Haystack engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "external");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Hosted Haystack Doc");
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Hosted Haystack Doc")));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_external_read_resolves_configured_haystack_citation_document() {
    const SNIPPET: &str = "hosted haystack read snippet";
    let _env_guard = lock_external_adapter_env().await;
    let mock_server = wiremock::MockServer::start().await;
    let pipeline_name = "retrieval_pipeline";
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path(format!("/{pipeline_name}/run")))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "retriever": {
                    "documents": [{
                        "id": "doc-read",
                        "content": SNIPPET,
                        "meta": {
                            "title": "Hosted Haystack Read Doc",
                            "source": "file://read.txt"
                        },
                        "score": 0.95
                    }]
                }
            })),
        )
        .mount(&mock_server)
        .await;

    let _haystack_base_url = TempEnvVar::set(
        "SDKWORK_KNOWLEDGEBASE_HAYSTACK_BASE_URL",
        mock_server.uri().as_str(),
    );

    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let backend = dev_auth::with_dev_backend_auth(runtime.build_backend_router(), 1, Some(99));

    let space_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Haystack Read Space","description":"Haystack read E2E","knowledgeMode":"external"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(space_response.status(), StatusCode::CREATED);
    let space_id = response_id_field(space_response, "external space id").await;

    let source_response = backend
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/backend/v3/api/knowledge/sources")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"spaceId":{space_id},"sourceType":"connector","provider":"haystack","connectorMetadataJson":"{{\"datasetId\":\"{pipeline_name}\"}}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    activate_provider_binding(
        &runtime,
        space_id,
        "engine.knowledge.external.haystack",
        pipeline_name,
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"Haystack Read Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"external","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is in the Haystack knowledge base?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chat_response.status(), StatusCode::OK);
    let chat_body = response_body_json(chat_response).await;
    let logical_path = chat_body["citations"][0]["logicalPath"]
        .as_str()
        .expect("citation logical path");
    let (_, local_document_id) = logical_path
        .split_once('/')
        .expect("scoped citation logical path");
    assert_eq!(local_document_id, "Hosted Haystack Read Doc#doc-read");

    let document = runtime
        .read_knowledge_engine_document_for_space(
            &knowledge_execution_context(
                runtime.tenant_id(),
                runtime.organization_id(),
                space_id,
                None,
                "trace-hosted-runtime-read",
            ),
            space_id,
            local_document_id,
        )
        .await
        .expect("read citation document through SPI");
    assert_eq!(document.content, SNIPPET);
    assert_eq!(document.title, "Hosted Haystack Read Doc");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_okf_agent_chat_succeeds_with_published_concept_citations() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));

    let space_id = create_space(&app, "OKF Agent Chat Space").await;
    publish_okf_concept(
        &app,
        space_id,
        "concepts/agent-target",
        "---\ntype: Knowledge Concept\ntitle: Agent Target\ndescription: Hosted OKF agent citation target for chat\n---\n# Agent Target\n\nHosted OKF knowledge for agent chat.\n\n# Citations\n\n[1] [Src](https://example.com)\n",
    )
    .await;

    let profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/agent_profiles")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","name":"OKF Agent","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.knowledgebase-contract","modelId":"contract","agentImplementationId":"plugin.intelligence.knowledgebase-contract","knowledgeMode":"okf_bundle","status":"active"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(profile_response.status(), StatusCode::CREATED);
    let profile_body = response_body_json(profile_response).await;
    let profile_id = json_u64_field(&profile_body, "profileId").expect("created profile id");

    let binding_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings"
                ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"tenantId":"1","profileId":"{profile_id}","spaceId":"{space_id}","priority":0,"enabled":true}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);

    let chat_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"tenantId":"1","message":"What is the Agent Target concept?"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        chat_response.status(),
        StatusCode::OK,
        "OKF native engine must complete agent chat"
    );
    let chat_body = response_body_json(chat_response).await;
    assert_eq!(chat_body["mode"], "okf_bundle");
    assert!(chat_body["citations"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert_eq!(chat_body["citations"][0]["title"], "Agent Target");
    assert_eq!(
        chat_body["citations"][0]["conceptId"],
        "concepts/agent-target"
    );
    assert_eq!(
        chat_body["citations"][0]["logicalPath"],
        format!("{space_id}/concepts/agent-target")
    );
    assert_eq!(
        chat_body["citations"][0]["locator"],
        format!("okf:{space_id}:concepts/agent-target")
    );
    assert!(chat_body["answer"]
        .as_str()
        .is_some_and(|answer| answer.contains("Agent Target")));

    let logical_path = chat_body["citations"][0]["logicalPath"]
        .as_str()
        .expect("citation logical path");
    let (_, local_document_id) = logical_path
        .split_once('/')
        .expect("scoped citation logical path");
    assert_eq!(local_document_id, "concepts/agent-target");

    let document = runtime
        .read_knowledge_engine_document_for_space(
            &knowledge_execution_context(
                runtime.tenant_id(),
                runtime.organization_id(),
                space_id,
                None,
                "trace-hosted-runtime-read",
            ),
            space_id,
            local_document_id,
        )
        .await
        .expect("read OKF citation document through SPI");
    assert!(document
        .content
        .contains("Hosted OKF knowledge for agent chat"));
    assert_eq!(document.title, "Agent Target");
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_app_runs_okf_lint_job_for_space() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));

    let space_id = create_space(&app, "App Lint Space").await;
    publish_okf_concept(
        &app,
        space_id,
        "entities/app-lint",
        "---\ntype: Entity\ntitle: App Lint\n---\n# App Lint\n\nApp lint target.\n\n# Citations\n\n[1] [Src](https://example.com)\n",
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/okf/lint_runs")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"spaceId":{space_id}}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_body_json(response).await;
    assert_eq!(body["state"], "succeeded");
}

async fn create_space(app: &axum::Router, name: &str) -> u64 {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/app/v3/api/knowledge/spaces")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"{name}","description":"Hosted OKF integration"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    response_id_field(response, "created space id").await
}

async fn publish_okf_concept(app: &axum::Router, space_id: u64, concept_id: &str, markdown: &str) {
    upsert_okf_concept(app, space_id, concept_id, markdown, true).await;
}

async fn stage_okf_concept(app: &axum::Router, space_id: u64, concept_id: &str, markdown: &str) {
    upsert_okf_concept(app, space_id, concept_id, markdown, false).await;
}

async fn upsert_okf_concept(
    app: &axum::Router,
    space_id: u64,
    concept_id: &str,
    markdown: &str,
    publish: bool,
) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/app/v3/api/knowledge/okf/concepts/upsert")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r##"{{"spaceId":{space_id},"conceptId":"{concept_id}","markdown":{markdown_json},"actor":"author","publish":{publish}}}"##,
                    markdown_json = serde_json::to_string(markdown).expect("serialize markdown")
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    if response.status() != StatusCode::OK {
        let body = response_body_json(response).await;
        panic!("upsert okf concept failed: {body}");
    }
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn hosted_open_router_lists_documents() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_open_auth(runtime.build_open_api_router(), 1, Some(42));
    let app_router = dev_auth::with_dev_app_auth(runtime.build_full_app_router(), 1, Some(42));
    let space_id = create_space(&app_router, "Open Document List Space").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/knowledge/v3/api/documents?spaceId={space_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_ne!(
        response.status(),
        StatusCode::NOT_IMPLEMENTED,
        "hosted open api must not return operation_unsupported for documents.list"
    );

    let body = response_body_json(response).await;
    assert!(body["items"].is_array());
}

struct TempEnvVar {
    key: &'static str,
    previous: Option<String>,
}

impl TempEnvVar {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for TempEnvVar {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

fn clear_dify_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_DIFY_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_DIFY_API_KEY",
        "SDKWORK_KNOWLEDGEBASE_DIFY_DATASET_ID",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_ragflow_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_RAGFLOW_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_RAGFLOW_API_KEY",
        "SDKWORK_KNOWLEDGEBASE_RAGFLOW_DATASET_ID",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_onyx_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_ONYX_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_ONYX_API_KEY",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_anythingllm_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_ANYTHINGLLM_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_ANYTHINGLLM_API_KEY",
        "SDKWORK_KNOWLEDGEBASE_ANYTHINGLLM_WORKSPACE_SLUG",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_open_webui_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_OPEN_WEBUI_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_OPEN_WEBUI_API_KEY",
        "SDKWORK_KNOWLEDGEBASE_OPEN_WEBUI_KNOWLEDGE_ID",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_flowise_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_FLOWISE_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_FLOWISE_API_KEY",
        "SDKWORK_KNOWLEDGEBASE_FLOWISE_STORE_ID",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_chroma_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_CHROMA_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_CHROMA_API_KEY",
        "SDKWORK_KNOWLEDGEBASE_CHROMA_COLLECTION_ID",
        "SDKWORK_KNOWLEDGEBASE_CHROMA_TENANT",
        "SDKWORK_KNOWLEDGEBASE_CHROMA_DATABASE",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_qdrant_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_QDRANT_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_QDRANT_API_KEY",
        "SDKWORK_KNOWLEDGEBASE_QDRANT_COLLECTION_NAME",
        "SDKWORK_KNOWLEDGEBASE_QDRANT_QUERY_MODEL",
        "SDKWORK_KNOWLEDGEBASE_QDRANT_USING_VECTOR",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_weaviate_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_WEAVIATE_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_WEAVIATE_API_KEY",
        "SDKWORK_KNOWLEDGEBASE_WEAVIATE_CLASS_NAME",
        "SDKWORK_KNOWLEDGEBASE_WEAVIATE_TITLE_PROPERTY",
        "SDKWORK_KNOWLEDGEBASE_WEAVIATE_CONTENT_PROPERTY",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_haystack_adapter_env() {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_HAYSTACK_BASE_URL",
        "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_HAYSTACK_API_KEY",
        "SDKWORK_KNOWLEDGEBASE_HAYSTACK_PIPELINE",
        "SDKWORK_KNOWLEDGEBASE_HAYSTACK_WORKSPACE",
        "SDKWORK_KNOWLEDGEBASE_HAYSTACK_DEPLOYMENT_MODE",
        "SDKWORK_KNOWLEDGEBASE_HAYSTACK_QUERY_FIELD",
    ] {
        std::env::remove_var(key);
    }
}

fn clear_external_adapter_env() {
    clear_dify_adapter_env();
    clear_ragflow_adapter_env();
    clear_onyx_adapter_env();
    clear_anythingllm_adapter_env();
    clear_open_webui_adapter_env();
    clear_flowise_adapter_env();
    clear_chroma_adapter_env();
    clear_qdrant_adapter_env();
    clear_weaviate_adapter_env();
    clear_haystack_adapter_env();
}

async fn test_runtime() -> Option<KnowledgebaseRuntime> {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return None;
    };
    std::env::set_var("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID", "42");
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let work_dir = std::env::current_dir().expect("current directory");
    let test_root = work_dir
        .join("target")
        .join("hosted-runtime-tests")
        .join(format!("{}-{}-{}", std::process::id(), nanos, sequence));
    std::fs::create_dir_all(&test_root).expect("create hosted runtime test directory");

    let drive_root = test_root.join("drive-objects");
    std::fs::create_dir_all(&drive_root).expect("create hosted runtime drive storage root");
    std::env::set_var(
        "SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_ROOT",
        drive_root.to_string_lossy().as_ref(),
    );

    KnowledgebaseRuntime::connect(&database_url, 1)
        .await
        .expect("initialize hosted runtime")
        .into()
}

async fn activate_provider_binding(
    runtime: &KnowledgebaseRuntime,
    space_id: u64,
    implementation_id: &str,
    remote_resource_id: &str,
) {
    let store = SqlxKnowledgeEngineProviderBindingStore::new(runtime.pool().clone());
    let scope = KnowledgeEngineProviderScope {
        tenant_id: 1,
        organization_id: 42,
    };
    let actor_id = "hosted-provider-binding-test";
    let credential_reference_id = match provider_credential_environment(implementation_id) {
        Some(variable) if std::env::var(variable).is_ok() => Some(
            store
                .create_credential_reference(
                    scope,
                    actor_id,
                    CreateKnowledgeEngineProviderCredentialReferenceRequest {
                        implementation_id: implementation_id.to_string(),
                        display_name: format!("{implementation_id} hosted test credential"),
                        reference_locator: format!("env://{variable}"),
                    },
                )
                .await
                .expect("create Provider credential reference")
                .id,
        ),
        _ => None,
    };
    let created = store
        .create_binding(
            scope,
            actor_id,
            CreateKnowledgeEngineProviderBindingRequest {
                space_id,
                implementation_id: implementation_id.to_string(),
                remote_resource_type: "knowledge_resource".to_string(),
                remote_resource_id: remote_resource_id.to_string(),
                credential_reference_id,
            },
        )
        .await
        .expect("create explicit Provider binding");
    let testing = store
        .begin_binding_test(scope, created.id, actor_id, created.version)
        .await
        .expect("begin Provider binding test");
    let tested = store
        .record_binding_test_result(
            scope,
            created.id,
            RecordKnowledgeEngineProviderTestResult {
                expected_version: testing.version,
                capabilities: vec![
                    KnowledgeEngineCapability::Health,
                    KnowledgeEngineCapability::Search,
                    KnowledgeEngineCapability::ReadDocument,
                ],
                error_category: None,
                updated_by: actor_id.to_string(),
            },
        )
        .await
        .expect("record Provider binding test result");
    store
        .activate_binding(scope, created.id, actor_id, tested.version)
        .await
        .expect("activate explicit Provider binding");
}

async fn create_tested_provider_binding(
    store: &SqlxKnowledgeEngineProviderBindingStore,
    scope: KnowledgeEngineProviderScope,
    space_id: u64,
    implementation_id: &str,
    remote_resource_id: &str,
) -> KnowledgeEngineProviderBinding {
    let actor_id = "migration-test";
    let created = store
        .create_binding(
            scope,
            actor_id,
            CreateKnowledgeEngineProviderBindingRequest {
                space_id,
                implementation_id: implementation_id.to_string(),
                remote_resource_type: "dataset".to_string(),
                remote_resource_id: remote_resource_id.to_string(),
                credential_reference_id: None,
            },
        )
        .await
        .expect("create migration Provider binding");
    let testing = store
        .begin_binding_test(scope, created.id, actor_id, created.version)
        .await
        .expect("begin migration Provider binding test");
    store
        .record_binding_test_result(
            scope,
            created.id,
            RecordKnowledgeEngineProviderTestResult {
                expected_version: testing.version,
                capabilities: vec![
                    KnowledgeEngineCapability::Health,
                    KnowledgeEngineCapability::Search,
                    KnowledgeEngineCapability::ReadDocument,
                ],
                error_category: None,
                updated_by: actor_id.to_string(),
            },
        )
        .await
        .expect("record migration Provider binding test")
}

fn provider_credential_environment(implementation_id: &str) -> Option<&'static str> {
    match implementation_id {
        "engine.knowledge.external.dify" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_DIFY_API_KEY")
        }
        "engine.knowledge.external.ragflow" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_RAGFLOW_API_KEY")
        }
        "engine.knowledge.external.onyx" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_ONYX_API_KEY")
        }
        "engine.knowledge.external.anythingllm" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_ANYTHINGLLM_API_KEY")
        }
        "engine.knowledge.external.open-webui" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_OPEN_WEBUI_API_KEY")
        }
        "engine.knowledge.external.flowise" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_FLOWISE_API_KEY")
        }
        "engine.knowledge.external.chroma" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_CHROMA_API_KEY")
        }
        "engine.knowledge.external.qdrant" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_QDRANT_API_KEY")
        }
        "engine.knowledge.external.weaviate" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_WEAVIATE_API_KEY")
        }
        "engine.knowledge.external.haystack" => {
            Some("SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_HAYSTACK_API_KEY")
        }
        _ => None,
    }
}

async fn insert_okf_revision_fixture(pool: &sqlx::AnyPool, concept_row_id: u64, revision_no: u64) {
    let id = 9_000_000_i64 + revision_no as i64;
    sqlx::query(
        r#"
        INSERT INTO kb_okf_concept_revision (
            id, uuid, tenant_id, concept_row_id, revision_no,
            markdown_object_ref_id, content_hash, review_state, status,
            created_at, updated_at, version
        )
        VALUES ($1, $2, 1, $3, $4, $1, $5, 'approved', 1, $6, $6, 0)
        "#,
    )
    .bind(id)
    .bind(format!("route-pagination-revision-{revision_no}"))
    .bind(concept_row_id as i64)
    .bind(revision_no as i64)
    .bind(format!("route-pagination-hash-{revision_no}"))
    .bind("2026-07-10T00:00:00Z")
    .execute(pool)
    .await
    .expect("insert revision pagination fixture");
}

fn assert_standard_cursor_page(body: &Value, expected_items: usize, has_more: bool) {
    assert_eq!(body["code"], 0);
    assert_eq!(
        body["data"]["items"]
            .as_array()
            .expect("standard list items")
            .len(),
        expected_items
    );
    assert_eq!(body["data"]["pageInfo"]["mode"], "cursor");
    assert_eq!(body["data"]["pageInfo"]["pageSize"], 200);
    assert_eq!(body["data"]["pageInfo"]["hasMore"], has_more);
    if has_more {
        assert!(body["data"]["pageInfo"]["nextCursor"].is_string());
    } else {
        assert!(body["data"]["pageInfo"]["nextCursor"].is_null());
    }
    let trace_id = body["traceId"].as_str().expect("success traceId");
    uuid::Uuid::parse_str(trace_id).expect("success traceId UUID");
}

async fn assert_invalid_parameter_problem(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );
    let body = response_raw_json(response).await;
    assert_eq!(body["code"].as_i64(), Some(40003));
    let trace_id = body["traceId"].as_str().expect("problem traceId");
    uuid::Uuid::parse_str(trace_id).expect("problem traceId UUID");
}

async fn response_raw_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    serde_json::from_slice(&bytes).expect("parse response json")
}

async fn response_body_json(response: axum::response::Response) -> Value {
    let value = response_raw_json(response).await;
    sdkwork_knowledgebase_test_support::api_envelope::unwrap_payload_or_envelope(&value)
}

async fn response_id_field(response: axum::response::Response, expectation: &str) -> u64 {
    let body = response_body_json(response).await;
    json_u64_field(&body, "id").unwrap_or_else(|| panic!("{expectation}: {body}"))
}

fn json_u64_field(body: &Value, field: &str) -> Option<u64> {
    body.get(field)
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
}
