use axum::body::Body;
use axum::http::{Request, StatusCode};
use sdkwork_routes_knowledgebase_app_api::{
    build_router_with_shared_app_api_and_readiness, paths, KnowledgeAppApi,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::util::ServiceExt;

struct UnimplementedAppApi;

#[async_trait::async_trait]
impl KnowledgeAppApi for UnimplementedAppApi {}

#[tokio::test]
async fn contract_health_probe_is_not_exposed_on_business_router() {
    let app = build_router_with_shared_app_api_and_readiness(Arc::new(UnimplementedAppApi), None);

    let response = app
        .oneshot(
            Request::builder()
                .uri(paths::HEALTHZ)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn contract_context_binding_operations_are_declared() {
    let spec: Value = serde_json::from_str(include_str!(
        "../../../sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json"
    ))
    .unwrap();

    for operation_id in [
        "documents.content.list",
        "spaces.contextBindings.list",
        "spaces.contextBindings.create",
        "contextBindings.retrieve",
        "contextBindings.update",
        "contextBindings.delete",
    ] {
        assert!(
            spec["paths"].as_object().unwrap().values().any(|methods| {
                methods
                    .as_object()
                    .unwrap()
                    .values()
                    .any(|operation| operation["operationId"] == operation_id)
            }),
            "missing operationId in authority OpenAPI: {operation_id}"
        );
    }
}

#[test]
fn contract_wechat_configuration_inputs_are_bounded() {
    let specs = [
        (
            "authored app-api",
            serde_json::from_str::<Value>(include_str!(
                "../../../apis/app-api/knowledgebase-app-api.openapi.json"
            ))
            .unwrap(),
        ),
        (
            "materialized app SDK",
            serde_json::from_str::<Value>(include_str!(
                "../../../sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json"
            ))
            .unwrap(),
        ),
    ];

    for (source, spec) in specs {
        let schemas = &spec["components"]["schemas"];
        let account = &schemas["KnowledgeWechatOfficialAccount"];
        let applet = &schemas["KnowledgeWechatApplet"];

        assert_eq!(account["additionalProperties"], json!(false), "{source}");
        assert_eq!(account["properties"]["id"]["maxLength"], json!(128));
        assert_eq!(
            account["properties"]["type"]["enum"],
            json!(["subscription", "service"])
        );
        assert_eq!(account["properties"]["avatar"]["maxLength"], json!(32));
        assert!(account["properties"]["avatar"]["pattern"]
            .as_str()
            .is_some_and(|pattern| pattern.contains("A-Za-z0-9+.-")));
        assert_eq!(account["properties"]["appSecret"]["writeOnly"], json!(true));
        assert_eq!(
            account["properties"]["domainVerifyFileContent"]["maxLength"],
            json!(65_536)
        );
        assert_eq!(
            account["properties"]["jsSecureDomains"]["maxItems"],
            json!(50)
        );

        assert_eq!(applet["additionalProperties"], json!(false), "{source}");
        assert_eq!(applet["properties"]["path"]["maxLength"], json!(1024));
        assert_eq!(applet["properties"]["requestDomain"]["maxItems"], json!(50));
        assert_eq!(
            applet["properties"]["msgDataFormat"]["enum"],
            json!(["json", "xml"])
        );
        assert_eq!(
            applet["properties"]["msgEncryptMode"]["enum"],
            json!(["plain", "compatible", "safe"])
        );
        assert_eq!(applet["properties"]["msgToken"]["writeOnly"], json!(true));

        for wrapper in [
            "KnowledgeWechatOfficialAccountList",
            "KnowledgeWechatReplaceOfficialAccountsRequest",
            "KnowledgeWechatAppletList",
            "KnowledgeWechatReplaceAppletsRequest",
        ] {
            assert_eq!(
                schemas[wrapper]["additionalProperties"],
                json!(false),
                "{source} {wrapper}"
            );
        }
        assert_eq!(
            schemas["KnowledgeWechatReplaceOfficialAccountsRequest"]["properties"]["accounts"]
                ["maxItems"],
            json!(100)
        );
        assert_eq!(
            schemas["KnowledgeWechatReplaceAppletsRequest"]["properties"]["applets"]["maxItems"],
            json!(100)
        );
    }
}
