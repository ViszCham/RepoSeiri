param(
    [string] $Path = ".",
    [string] $Profile = "common",
    [ValidateSet("json", "markdown")]
    [string] $Format = "markdown",
    [ValidateSet("repository", "workspace", "subtree")]
    [string] $Scope = "repository",
    [ValidateSet("summary", "routes", "evidence", "documents", "governance", "patches", "linter", "actions", "remote", "pr-body")]
    [string] $Query = "summary"
)

$ErrorActionPreference = "Stop"
$ExpectedSchema = "seiri.codex.v2"
$ExpectedContractSchema = "seiri.contract.v3"
$ExpectedRevisions = [ordered]@{
    repository_identity = "seiri.repository-identity.v3"
    source_session = "seiri.source-session.v1"
    stable_digest = "seiri.stable-digest.v2"
    markdown_parser = "seiri.markdown-parser.v3"
    path_classification = "seiri.path-classification.v2"
    document_selection = "seiri.document-selection.v2"
    coverage = "seiri.coverage.v2"
    content_slots = "seiri.content-slots.v2"
    route_target = "seiri.route-target.v3"
    github_semantics = "seiri.github-semantics.v2"
    document_consistency = "seiri.document-consistency.v2"
    profiles = "seiri.profiles.v2"
    claim_projection = "seiri.claim-semantics.v2"
    calibration = "seiri.calibration-semantics.v4"
    delta = "seiri.audit-delta-semantics.v3"
    patch_planner = "seiri.patch-planner.v4"
    completion = "seiri.completion-semantics.v4"
}
$ExpectedHostCommandSet = @(
    "native_contract",
    "native_codex_summary",
    "schema_integrity",
    "launcher_codex_summary"
)

function Write-RepoSeiriError {
    param([string] $Code, [string] $Message, [int] $ExitCode)
    [ordered]@{
        schema_version = "seiri.error.v1"
        class = "contract"
        code = $Code
        message = $Message
    } | ConvertTo-Json -Compress | Write-Error -ErrorAction Continue
    exit $ExitCode
}

function Get-RepoSeiriSha256 {
    param([Parameter(Mandatory = $true)][string] $LiteralPath)
    $Stream = $null
    $Hasher = $null
    try {
        $Stream = [System.IO.File]::Open(
            $LiteralPath,
            [System.IO.FileMode]::Open,
            [System.IO.FileAccess]::Read,
            [System.IO.FileShare]::Read
        )
        $Hasher = [System.Security.Cryptography.SHA256]::Create()
        $Digest = $Hasher.ComputeHash($Stream)
        return ([System.BitConverter]::ToString($Digest)).Replace("-", "").ToLowerInvariant()
    } catch {
        Write-RepoSeiriError "digest_read_failed" "RepoSeiri could not hash a bundle file" 5
    } finally {
        if ($null -ne $Hasher) {
            $Hasher.Dispose()
        }
        if ($null -ne $Stream) {
            $Stream.Dispose()
        }
    }
}

function Test-RevisionSet {
    param($Actual)
    if ($null -eq $Actual) {
        return $false
    }
    foreach ($Entry in $ExpectedRevisions.GetEnumerator()) {
        if ($Actual.($Entry.Key) -ne $Entry.Value) {
            return $false
        }
    }
    return @($Actual.PSObject.Properties).Count -eq $ExpectedRevisions.Count
}

if ($env:REPOSEIRI_BIN) {
    if (-not (Test-Path -LiteralPath $env:REPOSEIRI_BIN -PathType Leaf)) {
        Write-RepoSeiriError "configured_binary_missing" "REPOSEIRI_BIN does not name a file" 5
    }
    $Binary = (Resolve-Path -LiteralPath $env:REPOSEIRI_BIN).Path
} else {
    $PluginRoot = Split-Path -Parent $PSScriptRoot
    $BundleBinary = Join-Path $PluginRoot "bin/seiri.exe"
    if (Test-Path -LiteralPath $BundleBinary -PathType Leaf) {
        $Binary = $BundleBinary
    } else {
        $Command = Get-Command seiri -CommandType Application -ErrorAction SilentlyContinue
        if ($null -eq $Command) {
            Write-RepoSeiriError "binary_missing" "RepoSeiri binary was not found in the plugin bundle or PATH" 5
        }
        $Binary = $Command.Source
    }
}

$ContractText = & $Binary contract --format json
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
try {
    $Contract = $ContractText | ConvertFrom-Json
} catch {
    Write-RepoSeiriError "contract_invalid" "RepoSeiri binary returned an invalid contract document" 5
}
if ($Contract.schema_version -ne $ExpectedContractSchema -or
    $Contract.analysis_schema -ne "seiri.analysis.v2" -or
    $Contract.patch_plan_schema -ne "seiri.patch-plan.v2" -or
    $Contract.codex_schema -ne $ExpectedSchema -or
    $Contract.error_schema -ne "seiri.error.v1" -or
    $Contract.completion_schema -ne "seiri.completion.v3" -or
    $Contract.portable_audit_schema -ne "seiri.portable-audit.v2" -or
    $Contract.audit_delta_schema -ne "seiri.audit-delta.v2" -or
    $Contract.wording_lint_schema -ne "seiri.wording-lint.v1" -or
    -not (Test-RevisionSet $Contract.semantic_revisions)) {
    Write-RepoSeiriError "schema_mismatch" "RepoSeiri binary contract or semantic revisions do not match this plugin" 5
}

$RuntimeManifestPath = Join-Path (Split-Path -Parent $PSScriptRoot) "runtime-manifest.json"
if (Test-Path -LiteralPath $RuntimeManifestPath -PathType Leaf) {
    try {
        $RuntimeManifest = Get-Content -LiteralPath $RuntimeManifestPath -Raw | ConvertFrom-Json
    } catch {
        Write-RepoSeiriError "bundle_manifest_invalid" "RepoSeiri bundle manifest is invalid" 5
    }
    if ($RuntimeManifest.schema_version -ne "reposeiri.runtime-manifest.v3" -or
        $RuntimeManifest.bundle_metadata_version -ne "reposeiri.bundle-metadata.v1" -or
        $RuntimeManifest.tool_version -ne $Contract.tool_version -or
        $RuntimeManifest.binary -ne "bin/seiri.exe" -or
        $RuntimeManifest.standalone_smoke -ne "passed" -or
        $RuntimeManifest.source_digest -notmatch '^sha256:[0-9a-f]{64}$' -or
        $RuntimeManifest.cargo_lock_digest -notmatch '^sha256:[0-9a-f]{64}$' -or
        $RuntimeManifest.contract_schema -ne $ExpectedContractSchema -or
        $RuntimeManifest.analysis_schema -ne $Contract.analysis_schema -or
        $RuntimeManifest.patch_plan_schema -ne $Contract.patch_plan_schema -or
        $RuntimeManifest.codex_schema -ne $Contract.codex_schema -or
        $RuntimeManifest.error_schema -ne $Contract.error_schema -or
        $RuntimeManifest.completion_schema -ne $Contract.completion_schema -or
        $RuntimeManifest.portable_audit_schema -ne $Contract.portable_audit_schema -or
        $RuntimeManifest.audit_delta_schema -ne $Contract.audit_delta_schema -or
        (@($RuntimeManifest.command_set) -join "|") -ne ($ExpectedHostCommandSet -join "|") -or
        -not (Test-RevisionSet $RuntimeManifest.semantic_revisions)) {
        Write-RepoSeiriError "bundle_contract_mismatch" "RepoSeiri bundle metadata does not match the binary contract" 5
    }
    $BinaryHash = Get-RepoSeiriSha256 -LiteralPath $Binary
    if ($BinaryHash -ne $RuntimeManifest.sha256) {
        Write-RepoSeiriError "binary_digest_mismatch" "RepoSeiri bundle binary digest does not match its manifest" 5
    }
    $SchemaRoot = Join-Path (Split-Path -Parent $PSScriptRoot) "schemas"
    $ManifestSchemas = @($RuntimeManifest.schema_sha256.PSObject.Properties)
    $BundledSchemas = @(Get-ChildItem -LiteralPath $SchemaRoot -File -Filter "*.json")
    if ($ManifestSchemas.Count -ne $BundledSchemas.Count -or $ManifestSchemas.Count -eq 0) {
        Write-RepoSeiriError "schema_set_mismatch" "RepoSeiri bundle schema set does not match its manifest" 5
    }
    foreach ($Schema in $ManifestSchemas) {
        if ($Schema.Name -notmatch '^seiri\.[a-z0-9.-]+\.json$') {
            Write-RepoSeiriError "schema_path_invalid" "RepoSeiri bundle manifest contains a non-portable schema name" 5
        }
        $SchemaPath = Join-Path $SchemaRoot $Schema.Name
        if (-not (Test-Path -LiteralPath $SchemaPath -PathType Leaf)) {
            Write-RepoSeiriError "schema_missing" "RepoSeiri bundle schema file is missing" 5
        }
        $SchemaHash = Get-RepoSeiriSha256 -LiteralPath $SchemaPath
        if ($SchemaHash -ne $Schema.Value) {
            Write-RepoSeiriError "schema_digest_mismatch" "RepoSeiri bundle schema digest does not match its manifest" 5
        }
    }
}

& $Binary codex --path $Path --profile $Profile --scope $Scope --query $Query --format $Format
exit $LASTEXITCODE
