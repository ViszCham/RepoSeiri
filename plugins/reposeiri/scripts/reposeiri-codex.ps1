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
if ($Contract.codex_schema -ne $ExpectedSchema) {
    Write-RepoSeiriError "schema_mismatch" "RepoSeiri binary does not implement seiri.codex.v2" 5
}

& $Binary codex --path $Path --profile $Profile --scope $Scope --query $Query --format $Format
exit $LASTEXITCODE
