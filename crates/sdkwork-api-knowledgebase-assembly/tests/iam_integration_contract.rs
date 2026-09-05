#[test]
fn host_neutral_assembly_excludes_dependency_owned_iam() {
    let bootstrap = include_str!("../src/bootstrap.rs");
    let cargo_toml = include_str!("../Cargo.toml");

    assert!(!bootstrap.contains("sdkwork_api_iam_assembly"));
    assert!(!cargo_toml.contains("sdkwork_api_iam_assembly"));
    assert!(!cargo_toml.contains("sdkwork-api-iam-assembly"));
    assert!(!cargo_toml.contains("sdkwork-routes-iam-app-api"));
}

#[test]
fn standalone_gateway_composes_iam_assembly_contribution() {
    let gateway_main =
        include_str!("../../sdkwork-api-knowledgebase-standalone-gateway/src/bin/app_main.rs");
    let gateway_cargo =
        include_str!("../../sdkwork-api-knowledgebase-standalone-gateway/Cargo.toml");

    assert!(gateway_main.contains("sdkwork_api_iam_assembly::assemble_app_api_contribution"));
    // The standalone gateway composes through the module registry API
    // (`ApiModuleRegistry::try_compose`), the current canonical composition
    // entrypoint (API_ASSEMBLY_SPEC §4.1.1).
    assert!(gateway_main.contains("module_registry.try_compose"));
    assert!(gateway_cargo.contains("sdkwork_api_iam_assembly.workspace = true"));
}
