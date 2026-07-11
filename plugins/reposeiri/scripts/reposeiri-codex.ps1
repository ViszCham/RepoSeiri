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

cargo run --quiet -p seiri-cli -- codex --path $Path --profile $Profile --scope $Scope --query $Query --format $Format
