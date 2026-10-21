$ErrorActionPreference = "Stop"

$specPaths = @(
    "sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json",
    "sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json"
)

$openApiSpecPath = "sdks/sdkwork-knowledgebase-sdk/openapi/knowledgebase-open-api.openapi.json"

$requiredOpenOperations = @(
    "retrievals.create",
    "retrievals.retrieve",
    "contextPacks.create",
    "ingests.create",
    "ingests.retrieve",
    "documents.list",
    "documents.retrieve",
    "spaces.browser.list"
)

$required = @(
    "spaces.create",
    "spaces.retrieve",
    "driveImports.create",
    "ingests.create",
    "ingests.retrieve",
    "spaces.browser.list",
    "retrievals.create",
    "retrievals.retrieve",
    "contextPacks.create",
    "agentProfiles.create",
    "agentProfiles.retrieve",
    "agentProfiles.update",
    "agentProfiles.delete",
    "agentProfiles.bindings.list",
    "agentProfiles.bindings.create",
    "agentProfiles.bindings.update",
    "agentProfiles.bindings.delete",
    "agentProfiles.retrievalPreview.create",
    "sources.list",
    "sources.create",
    "documents.list",
    "documents.create",
    "documents.retrieve",
    "documents.content.list",
    "documents.versions.create",
    "documents.versions.list",
    "okf.bundle.index.retrieve",
    "okf.bundle.index.rebuild",
    "okf.bundle.log.retrieve",
    "okf.log.entries.create",
    "okf.bundle.profile.retrieve",
    "okf.profile.create",
    "okf.queries.fileAnswer",
    "indexes.create",
    "indexes.retrieve",
    "indexes.rebuild",
    "retrievalProfiles.create",
    "retrievalProfiles.retrieve",
    "retrievalProfiles.update",
    "retrievalTraces.list",
    "retrievalTraces.retrieve",
    "providerHealth.list"
)

$operationIds = New-Object System.Collections.Generic.List[string]

foreach ($specPath in $specPaths) {
    if (!(Test-Path $specPath)) {
        throw "Missing OpenAPI spec: $specPath"
    }

    $spec = Get-Content -Raw $specPath | ConvertFrom-Json
    if (!$spec.components -or !$spec.components.schemas -or $spec.components.schemas.PSObject.Properties.Count -eq 0) {
        throw "OpenAPI spec has no component schemas: $specPath"
    }

    $securitySchemes = $spec.components.securitySchemes
    if (!$securitySchemes) {
        throw "OpenAPI spec must define SDKWork v3 security schemes: $specPath"
    }

    $authToken = $securitySchemes.AuthToken
    if (!$authToken -or $authToken.type -ne "http" -or $authToken.scheme -ne "bearer") {
        throw "OpenAPI spec must define AuthToken as HTTP bearer security: $specPath"
    }

    $accessToken = $securitySchemes.AccessToken
    if (
        !$accessToken `
        -or $accessToken.type -ne "apiKey" `
        -or $accessToken.in -ne "header" `
        -or $accessToken.name -ne "Access-Token"
    ) {
        throw "OpenAPI spec must define AccessToken as Access-Token header apiKey security: $specPath"
    }

    foreach ($pathProperty in $spec.paths.PSObject.Properties) {
        foreach ($methodProperty in $pathProperty.Value.PSObject.Properties) {
            $operationId = $methodProperty.Value.operationId
            if ($operationId) {
                $operation = $methodProperty.Value

                if (!$methodProperty.Value.responses) {
                    throw "OpenAPI operation has no responses: $operationId"
                }

                $operationSecurity = $operation.security
                # API_SPEC §4.5.2: operations that mirror a third-party wire protocol may declare
                # explicit anonymous security (`security: []`) when the upstream caller cannot
                # present SDKWork tokens; the route must also carry the external wire marker.
                $externalWireProtocol = $operation.'x-sdkwork-wire-protocol'
                $declaresSecurity = $operation.PSObject.Properties['security'] -ne $null
                $isExternalAnonymous = $declaresSecurity `
                    -and $operationSecurity.Count -eq 0 `
                    -and $externalWireProtocol -eq "external"

                if (-not $isExternalAnonymous) {
                    if (!$operationSecurity -or $operationSecurity.Count -eq 0) {
                        throw "OpenAPI operation must declare SDKWork v3 security or explicit anonymous security: $operationId"
                    }

                    $firstSecurity = $operationSecurity[0]
                    if (
                        !$firstSecurity.PSObject.Properties["AuthToken"] `
                        -or !$firstSecurity.PSObject.Properties["AccessToken"]
                    ) {
                        throw "OpenAPI operation must require both AuthToken and AccessToken: $operationId"
                    }
                }

                foreach ($errorStatus in @("400", "404")) {
                    $responseProperty = $operation.responses.PSObject.Properties[$errorStatus]
                    if ($responseProperty) {
                        $content = $responseProperty.Value.content
                        if (!$content -or !$content.PSObject.Properties["application/problem+json"]) {
                            throw "OpenAPI error response $errorStatus must include application/problem+json: $operationId"
                        }
                    }
                }

                if ($pathProperty.Name.Contains("{") -and !$methodProperty.Value.parameters) {
                    throw "OpenAPI operation with path parameters has no parameter definitions: $operationId"
                }

                $methodName = [string]$methodProperty.Name
                if (($methodName -eq "post" -or $methodName -eq "patch") -and !$methodProperty.Value.requestBody) {
                    throw "OpenAPI mutating operation has no requestBody: $operationId"
                }

                if ($specPath -like "*backend-api*") {
                    $permission = $operation.'x-sdkwork-permission'
                    if ($permission -ne "knowledge.platform.manage") {
                        throw "Backend OpenAPI operation must declare x-sdkwork-permission knowledge.platform.manage: $operationId"
                    }
                }

                [void]$operationIds.Add([string]$operationId)
            }
        }
    }
}

$schemaNames = New-Object System.Collections.Generic.HashSet[string]
foreach ($specPath in $specPaths) {
    $spec = Get-Content -Raw $specPath | ConvertFrom-Json
    foreach ($schemaProperty in $spec.components.schemas.PSObject.Properties) {
        [void]$schemaNames.Add([string]$schemaProperty.Name)
    }
}

foreach ($operationId in $operationIds) {
    if ($operationId.Contains("_")) {
        throw "operationId contains underscore: $operationId"
    }

    if ($operationId -match "^(okfBundleIndex|okfBundleLog|okfBundleProfile|okfConcepts)") {
        throw "operationId uses flattened okf resource name: $operationId"
    }
}

foreach ($requiredId in $required) {
    if (!$operationIds.Contains($requiredId)) {
        throw "Missing required operationId: $requiredId"
    }
}

$requiredSchemas = @(
    "CreateKnowledgeSpaceRequest",
    "KnowledgeSpace",
    "KnowledgeIngestRequest",
    "IngestionJob",
    "KnowledgeDriveImportRequest",
    "ListKnowledgeBrowserRequest",
    "KnowledgeBrowserListData",
    "KnowledgeBrowserNode",
    "KnowledgeBrowserView",
    "KnowledgeBrowserNodeType",
    "KnowledgeBrowserNodePermissions",
    "KnowledgeDocument",
    "KnowledgeDocumentVersion",
    "KnowledgeSource",
    "KnowledgeOkfBundleFile",
    "KnowledgeRetrievalRequest",
    "KnowledgeRetrievalResult",
    "KnowledgeContextPackRequest",
    "KnowledgeContextPack",
    "KnowledgeMemoryContextFragment",
    "KnowledgeAgentProfile",
    "KnowledgeAgentBinding",
    "KnowledgeIndex",
    "KnowledgeRetrievalProfile",
    "KnowledgeRetrievalTrace",
    "KnowledgeProviderHealth",
    "ProblemDetails"
)

foreach ($requiredSchema in $requiredSchemas) {
    if (!$schemaNames.Contains($requiredSchema)) {
        throw "Missing required OpenAPI schema: $requiredSchema"
    }
}

if (!(Test-Path $openApiSpecPath)) {
    throw "Missing OpenAPI spec: $openApiSpecPath"
}

$openSpec = Get-Content -Raw $openApiSpecPath | ConvertFrom-Json
$openSecuritySchemes = $openSpec.components.securitySchemes
$apiKey = $openSecuritySchemes.ApiKey
if (!$apiKey -or $apiKey.type -ne "apiKey" -or $apiKey.in -ne "header" -or $apiKey.name -ne "X-API-Key") {
    throw "Open API spec must define ApiKey as X-API-Key header security: $openApiSpecPath"
}

$openOperationIds = New-Object System.Collections.Generic.List[string]
foreach ($pathProperty in $openSpec.paths.PSObject.Properties) {
    foreach ($methodProperty in $pathProperty.Value.PSObject.Properties) {
        $operationId = $methodProperty.Value.operationId
        if (!$operationId) {
            continue
        }
        $operation = $methodProperty.Value
        $operationSecurity = $operation.security
        if (!$operationSecurity -or $operationSecurity.Count -eq 0) {
            throw "Open API operation must declare api-key security: $operationId"
        }
        $firstSecurity = $operationSecurity[0]
        if (!$firstSecurity.PSObject.Properties["ApiKey"]) {
            throw "Open API operation must require ApiKey security: $operationId"
        }
        if ($operation.'x-sdkwork-auth-mode' -ne "api-key") {
            throw "Open API operation must declare x-sdkwork-auth-mode api-key: $operationId"
        }
        [void]$openOperationIds.Add([string]$operationId)
    }
}

foreach ($requiredOpenId in $requiredOpenOperations) {
    if (!$openOperationIds.Contains($requiredOpenId)) {
        throw "Missing required open-api operationId: $requiredOpenId"
    }
}

Write-Host "Verified $($operationIds.Count) app/backend OpenAPI operationIds, $($openOperationIds.Count) open-api operationIds, and $($schemaNames.Count) schemas."
