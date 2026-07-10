param(
    [string] $Path = ".",
    [string] $Profile = "common",
    [ValidateSet("json", "markdown")]
    [string] $Format = "markdown",
    [ValidateSet("context", "pr-body", "query", "linter")]
    [string] $View = "context",
    [ValidateSet("compatibility-v1", "native-v2")]
    [string] $Schema = "compatibility-v1",
    [ValidateSet("summary", "routes", "patches", "linter", "actions")]
    [string] $Query = "summary"
)

$ErrorActionPreference = "Stop"

cargo run --quiet -p seiri-cli -- codex --path $Path --profile $Profile --format $Format --view $View --schema $Schema --query $Query
