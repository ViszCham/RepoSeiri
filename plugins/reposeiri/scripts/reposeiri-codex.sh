#!/bin/sh
set -eu

expected_schema='seiri.codex.v2'
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
plugin_root=$(dirname -- "$script_dir")

fail_contract() {
    code=$1
    message=$2
    printf '%s\n' "{\"schema_version\":\"seiri.error.v1\",\"class\":\"contract\",\"code\":\"$code\",\"message\":\"$message\"}" >&2
    exit 5
}

if [ "${REPOSEIRI_BIN:-}" != "" ]; then
    [ -f "$REPOSEIRI_BIN" ] || fail_contract configured_binary_missing 'REPOSEIRI_BIN does not name a file'
    binary=$REPOSEIRI_BIN
elif [ -x "$plugin_root/bin/seiri" ]; then
    binary=$plugin_root/bin/seiri
elif command -v seiri >/dev/null 2>&1; then
    binary=$(command -v seiri)
else
    fail_contract binary_missing 'RepoSeiri binary was not found in the plugin bundle or PATH'
fi

contract=$("$binary" contract --format json) || exit $?
printf '%s' "$contract" | grep -Fq "\"codex_schema\": \"$expected_schema\"" || \
    fail_contract schema_mismatch 'RepoSeiri binary does not implement seiri.codex.v2'

exec "$binary" codex "$@"
