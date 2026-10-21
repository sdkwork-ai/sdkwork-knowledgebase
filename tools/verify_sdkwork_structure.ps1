$ErrorActionPreference = "Stop"

function Assert-PathExists {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [string]$Message = "Missing required path"
    )

    if (!(Test-Path -LiteralPath $Path)) {
        throw "${Message}: ${Path}"
    }
}

function Assert-PathAbsent {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [string]$Message = "Forbidden path exists"
    )

    if (Test-Path -LiteralPath $Path) {
        throw "${Message}: ${Path}"
    }
}

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Content,
        [Parameter(Mandatory = $true)]
        [string]$Expected,
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if (!$Content.Contains($Expected)) {
        throw "${Path} must contain: ${Expected}"
    }
}

function Assert-NotContains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Content,
        [Parameter(Mandatory = $true)]
        [string]$Forbidden,
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if ($Content.Contains($Forbidden)) {
        throw "${Path} must not contain: ${Forbidden}"
    }
}

function Get-JsonFile {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    return Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
}

$requiredSpecPaths = @(
    "../sdkwork-specs/README.md",
    "../sdkwork-specs/SOUL.md",
    "../sdkwork-specs/AGENTS_SPEC.md",
    "../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md",
    "../sdkwork-specs/CODE_STYLE_SPEC.md",
    "../sdkwork-specs/NAMING_SPEC.md",
    "../sdkwork-specs/RUST_CODE_SPEC.md"
)

foreach ($path in $requiredSpecPaths) {
    Assert-PathExists $path "Required SDKWORK spec path does not resolve"
}

Assert-PathExists "AGENTS.md" "Missing SDKWORK agent entrypoint"
Assert-PathExists "sdkwork.app.config.json" "Missing application manifest"
Assert-PathExists ".sdkwork/README.md" "Missing SDKWORK workspace README"
Assert-PathExists ".sdkwork/skills/README.md" "Missing SDKWORK skills README"
Assert-PathExists ".sdkwork/plugins/README.md" "Missing SDKWORK plugins README"
Assert-PathExists ".sdkwork/.gitignore" "Missing SDKWORK workspace .gitignore"

Assert-PathExists "specs/topology.spec.json" "Missing topology spec"
Assert-PathExists "docs/architecture/tech/TECH-topology-standard.md" "Missing topology standard doc"
Assert-PathExists "scripts/lib/knowledgebase-topology.mjs" "Missing topology adapter"
Assert-PathExists "scripts/knowledgebase-dev.mjs" "Missing topology dev orchestrator"

$topologySpec = Get-JsonFile "specs/topology.spec.json"
if ($topologySpec.schemaVersion -ne 5) {
    throw "specs/topology.spec.json schemaVersion must be 5"
}
if ($topologySpec.kind -ne "sdkwork.app.topology") {
    throw "specs/topology.spec.json kind must be sdkwork.app.topology"
}
foreach ($configFile in $topologySpec.packaging.cloudConfigFiles) {
    Assert-PathExists (Join-Path "etc" $configFile) "Missing cloud gateway config bundle"
}

$agent = Get-Content -Raw -LiteralPath "AGENTS.md"
Assert-Contains $agent "sdkwork.app.config.json" "AGENTS.md"
Assert-NotContains $agent "No `sdkwork.app.config.json` is present" "AGENTS.md"

foreach ($path in @(".sdkwork/README.md", ".sdkwork/skills/README.md", ".sdkwork/plugins/README.md")) {
    $content = Get-Content -Raw -LiteralPath $path
    Assert-NotContains $content '$name' $path
    Assert-NotContains $content '$specPath' $path
}

$requiredRootDirectories = @(
    "apis",
    "apps",
    "crates",
    "sdks",
    "jobs",
    "tools",
    "plugins",
    "examples",
    "etc",
    "deployments",
    "scripts",
    "docs",
    "tests"
)

foreach ($directory in $requiredRootDirectories) {
    Assert-PathExists $directory "Missing standard root directory"
    Assert-PathExists (Join-Path $directory "README.md") "Missing standard root directory README"
}

Assert-PathAbsent "services" "Nonstandard top-level services directory must be removed"

$rootManifest = Get-JsonFile "sdkwork.app.config.json"
if ($rootManifest.publish.config.workspaceRoot -ne ".") {
    throw "sdkwork.app.config.json publish.config.workspaceRoot must be '.': $($rootManifest.publish.config.workspaceRoot)"
}
if ($rootManifest.devApp.sourceRoot -ne ".") {
    throw "sdkwork.app.config.json devApp.sourceRoot must be '.': $($rootManifest.devApp.sourceRoot)"
}

$expectedPackages = @(
    "sdkwork-knowledgebase-contract",
    "sdkwork-knowledgebase-agent-provider",
    "sdkwork-intelligence-knowledgebase-object-key-service",
    "sdkwork-knowledgebase-drive",
    "sdkwork-knowledgebase-memory",
    "sdkwork-knowledgebase-test-support",
    "sdkwork-knowledgebase-observability",
    "sdkwork-api-knowledgebase-standalone-gateway",
    "sdkwork-knowledgebase-contract-tests",
    "sdkwork-knowledgebase-worker",
    "sdkwork-routes-knowledgebase-app-api",
    "sdkwork-routes-knowledgebase-open-api",
    "sdkwork-routes-knowledgebase-backend-api",
    "sdkwork-intelligence-knowledgebase-service",
    "sdkwork-intelligence-knowledgebase-repository-sqlx"
)

$legacyForbiddenPackages = @(
    "sdkwork-knowledgebase-core",
    "sdkwork-knowledgebase-product",
    "sdkwork-knowledgebase-storage-sqlx",
    "sdkwork-knowledgebase-app-api",
    "sdkwork-knowledgebase-backend-api"
)

# Enumerate tracked manifests through git so generated `target/`, `node_modules/`,
# `.sdkwork/manual-backups/` and broken junctions cannot break the recursive walk
# (Windows long-path and symlink failures abort `Get-ChildItem -Recurse` mid-tree).
$cargoTomls = git ls-files -- '*Cargo.toml' |
    ForEach-Object { Get-Item -LiteralPath $_ -ErrorAction SilentlyContinue } |
    Where-Object { $_ -and $_.FullName -notmatch "\\(target|node_modules|\.sdkwork)\\" } |
    Sort-Object FullName

$packageNames = New-Object System.Collections.Generic.List[string]
foreach ($cargoToml in $cargoTomls) {
    $relativePath = $cargoToml.FullName.Substring((Get-Location).Path.Length + 1).Replace("\", "/")
    $isAllowedAppSurfaceTauriHost = $relativePath -match '^apps/[^/]+/packages/[^/]+/src-tauri/Cargo\.toml$'
    $isAuthoredSdkRustFamily = $relativePath -match '^sdks/[^/]+/[^/]+-rust/Cargo\.toml$'
    $isGeneratedSdkTransport = $relativePath -match '^sdks/[^/]+/[^/]+-rust/generated/server-openapi/Cargo\.toml$'
    if ($relativePath -ne "Cargo.toml" -and !$relativePath.StartsWith("crates/") -and !$isAllowedAppSurfaceTauriHost -and !$isAuthoredSdkRustFamily -and !$isGeneratedSdkTransport) {
        throw "Rust package manifest must live under crates/, an app-surface Tauri host, an SDK Rust family, or a generator-owned SDK transport: $relativePath"
    }

    $match = Select-String -LiteralPath $cargoToml.FullName -Pattern '^name\s*=\s*"([^"]+)"' | Select-Object -First 1
    if ($null -ne $match) {
        $packageName = $match.Matches.Groups[1].Value
        $packageNames.Add($packageName)
    }
}

foreach ($expectedPackage in $expectedPackages) {
    if (!$packageNames.Contains($expectedPackage)) {
        throw "Expected Cargo package is missing: $expectedPackage"
    }
}

foreach ($forbiddenPackage in $legacyForbiddenPackages) {
    if ($packageNames.Contains($forbiddenPackage)) {
        throw "Forbidden legacy Cargo package remains: $forbiddenPackage"
    }
}

$rootCargo = Get-Content -Raw -LiteralPath "Cargo.toml"
foreach ($forbiddenPackage in $legacyForbiddenPackages) {
    Assert-NotContains $rootCargo $forbiddenPackage "Cargo.toml"
}
foreach ($memberMatch in [regex]::Matches($rootCargo, '"([^"]+)"')) {
    $memberPath = $memberMatch.Groups[1].Value
    if ($memberPath.StartsWith("services/")) {
        throw "Cargo workspace member must not live under services/: $memberPath"
    }
}

$componentSpecs = git ls-files -- '*component.spec.json' |
    ForEach-Object { Get-Item -LiteralPath $_ -ErrorAction SilentlyContinue } |
    Where-Object { $_ -and $_.FullName -notmatch "\\(target|node_modules)\\" } |
    Sort-Object FullName

foreach ($componentSpec in $componentSpecs) {
    $relativePath = $componentSpec.FullName.Substring((Get-Location).Path.Length + 1).Replace("\", "/")
    $json = Get-JsonFile $componentSpec.FullName
    $componentRoot = [string]$json.component.root
    if ($componentRoot.Contains("/services/")) {
        throw "Component spec root must not reference services/: $relativePath"
    }
    if ($relativePath.StartsWith("crates/")) {
        $expectedRoot = "sdkwork-knowledgebase/" + ($relativePath -replace "/specs/component.spec.json$", "")
        if ($componentRoot -ne $expectedRoot) {
            throw "Component spec root mismatch in ${relativePath}: expected ${expectedRoot}, got ${componentRoot}"
        }
    }
}

$routeManifests = @(
    @{
        Path = "sdks/_route-manifests/app-api/sdkwork-routes-knowledgebase-app-api.route-manifest.json"
        PackageName = "sdkwork-routes-knowledgebase-app-api"
        Surface = "app-api"
        Prefix = "/app/v3/api"
        ApiAuthority = "sdkwork-knowledgebase-app-api"
        SdkFamily = "sdkwork-knowledgebase-app-sdk"
    },
    @{
        Path = "sdks/_route-manifests/backend-api/sdkwork-routes-knowledgebase-backend-api.route-manifest.json"
        PackageName = "sdkwork-routes-knowledgebase-backend-api"
        Surface = "backend-api"
        Prefix = "/backend/v3/api"
        ApiAuthority = "sdkwork-knowledgebase-backend-api"
        SdkFamily = "sdkwork-knowledgebase-backend-sdk"
    },
    @{
        Path = "sdks/_route-manifests/open-api/sdkwork-routes-knowledgebase-open-api.route-manifest.json"
        PackageName = "sdkwork-routes-knowledgebase-open-api"
        Surface = "open-api"
        Prefix = "/knowledge/v3/api"
        ApiAuthority = "sdkwork-knowledgebase-open-api"
        SdkFamily = "sdkwork-knowledgebase-sdk"
    }
)

foreach ($manifestExpectation in $routeManifests) {
    $path = $manifestExpectation.Path
    Assert-PathExists $path "Missing normalized route manifest"
    $manifest = Get-JsonFile $path
    if ($manifest.kind -ne "sdkwork.route.manifest") {
        throw "Route manifest kind mismatch in ${path}: $($manifest.kind)"
    }
    foreach ($field in @("PackageName", "Surface", "Prefix", "ApiAuthority", "SdkFamily")) {
        $jsonField = $field.Substring(0, 1).ToLowerInvariant() + $field.Substring(1)
        if ($manifest.$jsonField -ne $manifestExpectation.$field) {
            throw "Route manifest ${jsonField} mismatch in ${path}: expected $($manifestExpectation.$field), got $($manifest.$jsonField)"
        }
    }
    if (!$manifest.routes -or $manifest.routes.Count -eq 0) {
        throw "Route manifest must declare at least one route: $path"
    }
}

$activeSearchRoots = @(
    "AGENTS.md",
    "Cargo.toml",
    "README.md",
    "sdkwork.app.config.json",
    ".sdkwork",
    "apis",
    "apps",
    "etc",
    "crates",
    "deployments",
    "examples",
    "jobs",
    "plugins",
    "scripts",
    "sdks",
    "specs",
    "tests",
    "tools"
)

$forbiddenImportNames = @(
    "sdkwork_knowledgebase_core",
    "sdkworkuct",
    "sdkwork_knowledgebase_storage_sqlx",
    "sdkwork_knowledgebase_app_api",
    "sdkwork_knowledgebase_backend_api"
)

$contentForbiddenPatterns = $forbiddenImportNames
$excludedHistoricalDocs = @(
    "docs/architecture/tech/TECH-2026-06-01-knowledgebase-backend-design.md",
    "docs/architecture/tech/TECH-2026-06-01-knowledgebase-backend-phase1-implementation.md",
    "docs/architecture/tech/TECH-2026-06-09-knowledgebase-agent-rag-implementation.md",
    "tools/verify_sdkwork_structure.ps1"
)

$filesToScan = New-Object System.Collections.Generic.List[System.IO.FileInfo]
foreach ($root in $activeSearchRoots) {
    if (!(Test-Path -LiteralPath $root)) {
        continue
    }
    $item = Get-Item -LiteralPath $root
    if ($item.PSIsContainer) {
        Get-ChildItem -LiteralPath $root -Recurse -File |
            Where-Object { $_.FullName -notmatch "\\target\\" } |
            ForEach-Object { $filesToScan.Add($_) }
    } else {
        $filesToScan.Add($item)
    }
}

foreach ($file in $filesToScan) {
    $relativePath = $file.FullName.Substring((Get-Location).Path.Length + 1).Replace("\", "/")
    if ($excludedHistoricalDocs -contains $relativePath) {
        continue
    }
    $content = Get-Content -Raw -LiteralPath $file.FullName -ErrorAction SilentlyContinue
    if ($null -eq $content) {
        continue
    }
    foreach ($pattern in $contentForbiddenPatterns) {
        if ($content.Contains($pattern)) {
            throw "Legacy structure or package reference remains in ${relativePath}: ${pattern}"
        }
    }
}

Write-Host "SDKWork structure verification passed."
