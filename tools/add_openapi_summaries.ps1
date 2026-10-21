$summaries = @{
    # app-api
    "spaces.create" = "Create a knowledge space"
    "spaces.retrieve" = "Retrieve a knowledge space"
    "driveImports.create" = "Import a drive object into knowledgebase"
    "ingests.create" = "Create an ingestion job"
    "ingests.retrieve" = "Retrieve an ingestion job"
    "documents.list" = "List knowledge documents"
    "documents.create" = "Create a knowledge document"
    "documents.retrieve" = "Retrieve a knowledge document"
    "documents.content.list" = "Retrieve authoritative knowledge document content"
    "documents.update" = "Update a knowledge document"
    "documents.delete" = "Delete a knowledge document"
    "documents.versions.list" = "List document versions"
    "documents.versions.versions" = "Create a document version"
    "okf.concepts.list" = "List OKF concepts"
    "okf.concepts.retrieve" = "Retrieve an OKF concept"
    "okf.concepts.revisions.list" = "List OKF concept revisions"
    "okf.bundle.index.retrieve" = "Retrieve the OKF bundle index"
    "okf.bundle.log.retrieve" = "Retrieve the OKF bundle log"
    "okf.bundle.profile.retrieve" = "Retrieve the OKF bundle profile"
    "okf.queries.create" = "Create an OKF query"
    "okf.queries.fileAnswer" = "File an answer for an OKF query"
    "okf.contextPacks.create" = "Create an OKF context pack"
    "spaces.browser.list" = "List browser view of a knowledge space"
    "retrievals.create" = "Create a knowledge retrieval"
    "retrievals.retrieve" = "Retrieve a knowledge retrieval result"
    "contextPacks.create" = "Create a knowledge context pack"
    "agentProfiles.create" = "Create a knowledge agent profile"
    "agentProfiles.retrieve" = "Retrieve a knowledge agent profile"
    "agentProfiles.update" = "Update a knowledge agent profile"
    "agentProfiles.delete" = "Delete a knowledge agent profile"
    "agentProfiles.bindings.list" = "List agent profile bindings"
    "agentProfiles.bindings.bindings" = "Create an agent profile binding"
    "agentProfiles.bindings.update" = "Update an agent profile binding"
    "agentProfiles.bindings.delete" = "Delete an agent profile binding"
    "agentProfiles.retrievalPreview.retrievalPreview" = "Preview retrieval for an agent profile"
    # backend-api
    "sources.list" = "List knowledge sources"
    "sources.create" = "Create a knowledge source"
    "okf.compileJobs.create" = "Create an OKF compile job"
    "okf.candidates.list" = "List OKF candidates"
    "okf.candidates.approve" = "Approve an OKF candidate"
    "okf.candidates.reject" = "Reject an OKF candidate"
    "okf.concepts.publish" = "Publish an OKF concept"
    "okf.profile.create" = "Create an OKF profile"
    "okf.profile.update" = "Update an OKF profile"
    "okf.bundle.index.rebuild" = "Rebuild the OKF bundle index"
    "okf.log.entries.create" = "Create an OKF log entry"
    "okf.bundle.export.create" = "Create an OKF bundle export"
    "okf.bundle.export.retrieve" = "Retrieve an OKF bundle export"
    "okf.bundle.files.list" = "List OKF bundle files"
    "okf.lintRuns.create" = "Create an OKF lint run"
    "okf.evalRuns.create" = "Create an OKF eval run"
    "indexes.create" = "Create a knowledge index"
    "indexes.retrieve" = "Retrieve a knowledge index"
    "indexes.rebuild" = "Rebuild a knowledge index"
    "retrievalProfiles.create" = "Create a retrieval profile"
    "retrievalProfiles.retrieve" = "Retrieve a retrieval profile"
    "retrievalProfiles.update" = "Update a retrieval profile"
    "retrievalTraces.list" = "List retrieval traces"
    "retrievalTraces.retrieve" = "Retrieve a retrieval trace"
    "providerHealth.list" = "Retrieve provider health status"
}

$files = @(
    "sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json",
    "sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json",
    "sdks/sdkwork-knowledgebase-sdk/openapi/knowledgebase-open-api.openapi.json"
)

foreach ($file in $files) {
    $fullPath = Join-Path $PSScriptRoot "..\$file"
    if (-not (Test-Path $fullPath)) {
        $fullPath = $file
    }
    
    $content = Get-Content $fullPath -Raw
    $json = $content | ConvertFrom-Json
    
    $modified = $false
    foreach ($path in $json.paths.PSObject.Properties) {
        foreach ($method in @("get", "post", "put", "patch", "delete")) {
            if ($path.Value.PSObject.Properties[$method]) {
                $op = $path.Value.$method
                if ($op.operationId -and -not $op.summary) {
                    $opId = $op.operationId
                    if ($summaries.ContainsKey($opId)) {
                        $op | Add-Member -NotePropertyName "summary" -NotePropertyValue $summaries[$opId] -Force
                        $modified = $true
                        Write-Host "Added summary to $opId : $($summaries[$opId])"
                    } else {
                        Write-Host "WARNING: No summary mapping for $opId"
                    }
                }
            }
        }
    }
    
    if ($modified) {
        $json | ConvertTo-Json -Depth 100 | Set-Content $fullPath -Encoding UTF8NoBOM
        Write-Host "Updated $file"
    }
}
