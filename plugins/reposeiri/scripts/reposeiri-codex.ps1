param(
    [string] $Path = ".",
    [string] $Profile = "common",
    [ValidateSet("json", "markdown")]
    [string] $Format = "markdown",
    [ValidateSet("context", "pr-body")]
    [string] $View = "context"
)

$ErrorActionPreference = "Stop"

cargo run --quiet -p seiri-cli -- codex --path $Path --profile $Profile --format $Format --view $View
