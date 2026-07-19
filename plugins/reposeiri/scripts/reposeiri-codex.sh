#!/bin/sh
set -eu

expected_schema='seiri.codex.v2'
expected_contract_schema='seiri.contract.v4'
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
plugin_root=$(dirname -- "$script_dir")

fail_contract() {
    code=$1
    message=$2
    printf '%s\n' "{\"schema_version\":\"seiri.error.v1\",\"class\":\"contract\",\"code\":\"$code\",\"message\":\"$message\"}" >&2
    exit 5
}

sha256_path() {
    path=$1
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$path" | awk '{print $1}'
    else
        fail_contract digest_unavailable 'No SHA-256 utility is available for bundle validation'
    fi
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
require_contract_value() {
    key=$1
    value=$2
    printf '%s' "$contract" | grep -Fq "\"$key\": \"$value\"" || \
        fail_contract schema_mismatch 'RepoSeiri binary contract or semantic revisions do not match this plugin'
}

require_contract_value schema_version "$expected_contract_schema"
require_contract_value analysis_schema seiri.analysis.v2
require_contract_value patch_plan_schema seiri.patch-plan.v2
require_contract_value codex_schema "$expected_schema"
require_contract_value error_schema seiri.error.v1
require_contract_value completion_schema seiri.completion.v3
require_contract_value portable_audit_schema seiri.portable-audit.v2
require_contract_value audit_delta_schema seiri.audit-delta.v2
require_contract_value wording_lint_schema seiri.wording-lint.v1
require_contract_value repository_identity seiri.repository-identity.v3
require_contract_value source_session seiri.source-session.v2
require_contract_value stable_digest seiri.stable-digest.v3
require_contract_value markdown_parser seiri.markdown-parser.v3
require_contract_value semantic_index seiri.semantic-index.v1
require_contract_value language_topology seiri.language-topology.v1
require_contract_value path_classification seiri.path-classification.v2
require_contract_value document_selection seiri.document-selection.v2
require_contract_value coverage seiri.coverage.v2
require_contract_value content_slots seiri.content-slots.v3
require_contract_value route_target seiri.route-target.v3
require_contract_value route_assessment seiri.route-assessment.v3
require_contract_value github_semantics seiri.github-semantics.v2
require_contract_value document_consistency seiri.document-consistency.v2
require_contract_value profiles seiri.profiles.v2
require_contract_value rule_registry seiri.rule-registry.v1
require_contract_value claim_projection seiri.claim-semantics.v2
require_contract_value review_projection seiri.review-projection.v1
require_contract_value calibration seiri.calibration-semantics.v4
require_contract_value delta seiri.audit-delta-semantics.v4
require_contract_value patch_planner seiri.patch-planner.v5
require_contract_value completion seiri.completion-semantics.v5

runtime_manifest="$plugin_root/runtime-manifest.json"
if [ -f "$runtime_manifest" ]; then
    manifest=$(cat "$runtime_manifest") || fail_contract bundle_manifest_invalid 'RepoSeiri bundle manifest is invalid'
    require_manifest_value() {
        key=$1
        value=$2
        printf '%s' "$manifest" | grep -Fq "\"$key\": \"$value\"" || \
            fail_contract bundle_contract_mismatch 'RepoSeiri bundle metadata does not match the binary contract'
    }
    require_manifest_value schema_version reposeiri.runtime-manifest.v3
    require_manifest_value bundle_metadata_version reposeiri.bundle-metadata.v1
    require_manifest_value binary bin/seiri
    require_manifest_value standalone_smoke passed
    require_manifest_value contract_schema "$expected_contract_schema"
    require_manifest_value codex_schema "$expected_schema"
    require_manifest_value repository_identity seiri.repository-identity.v3
    require_manifest_value source_session seiri.source-session.v2
    require_manifest_value stable_digest seiri.stable-digest.v3
    require_manifest_value markdown_parser seiri.markdown-parser.v3
    require_manifest_value semantic_index seiri.semantic-index.v1
    require_manifest_value language_topology seiri.language-topology.v1
    require_manifest_value path_classification seiri.path-classification.v2
    require_manifest_value document_selection seiri.document-selection.v2
    require_manifest_value coverage seiri.coverage.v2
    require_manifest_value content_slots seiri.content-slots.v3
    require_manifest_value route_target seiri.route-target.v3
    require_manifest_value route_assessment seiri.route-assessment.v3
    require_manifest_value github_semantics seiri.github-semantics.v2
    require_manifest_value document_consistency seiri.document-consistency.v2
    require_manifest_value profiles seiri.profiles.v2
    require_manifest_value rule_registry seiri.rule-registry.v1
    require_manifest_value claim_projection seiri.claim-semantics.v2
    require_manifest_value review_projection seiri.review-projection.v1
    require_manifest_value calibration seiri.calibration-semantics.v4
    require_manifest_value delta seiri.audit-delta-semantics.v4
    require_manifest_value patch_planner seiri.patch-planner.v5
    require_manifest_value completion seiri.completion-semantics.v5
    source_digest=$(printf '%s' "$manifest" | sed -n 's/.*"source_digest":[[:space:]]*"\(sha256:[0-9a-f]\{64\}\)".*/\1/p')
    cargo_lock_digest=$(printf '%s' "$manifest" | sed -n 's/.*"cargo_lock_digest":[[:space:]]*"\(sha256:[0-9a-f]\{64\}\)".*/\1/p')
    [ "${#source_digest}" -eq 71 ] || fail_contract bundle_source_binding_invalid 'RepoSeiri bundle source digest is invalid'
    [ "${#cargo_lock_digest}" -eq 71 ] || fail_contract bundle_source_binding_invalid 'RepoSeiri bundle Cargo.lock digest is invalid'
    for command_name in native_contract native_codex_summary schema_integrity launcher_codex_summary; do
        printf '%s' "$manifest" | grep -Fq "\"$command_name\"" || \
            fail_contract bundle_command_set_mismatch 'RepoSeiri bundle command set is incomplete'
    done
    expected_hash=$(printf '%s' "$manifest" | sed -n 's/.*"sha256":[[:space:]]*"\([0-9a-f]\{64\}\)".*/\1/p')
    [ "${#expected_hash}" -eq 64 ] || fail_contract bundle_manifest_invalid 'RepoSeiri bundle manifest binary digest is invalid'
    actual_hash=$(sha256_path "$binary")
    [ "$actual_hash" = "$expected_hash" ] || \
        fail_contract binary_digest_mismatch 'RepoSeiri bundle binary digest does not match its manifest'
    schema_count=0
    for schema_name in \
        seiri.analysis.v2.json \
        seiri.audit-delta.v2.json \
        seiri.calibration-corpus.v1.json \
        seiri.calibration-holdout.v1.json \
        seiri.calibration.v2.json \
        seiri.codex.v2.json \
        seiri.completion.v3.json \
        seiri.error.v1.json \
        seiri.executable-pattern-pack.v2.json \
        seiri.local-calibration-priors.v2.json \
        seiri.patch-plan.v2.json \
        seiri.portable-audit.v2.json
    do
        schema_path="$plugin_root/schemas/$schema_name"
        [ -f "$schema_path" ] || fail_contract schema_missing 'RepoSeiri bundle schema file is missing'
        expected_schema_hash=$(printf '%s' "$manifest" | sed -n "s/.*\"$schema_name\":[[:space:]]*\"\([0-9a-f]\{64\}\)\".*/\1/p")
        [ "${#expected_schema_hash}" -eq 64 ] || fail_contract schema_set_mismatch 'RepoSeiri bundle schema set does not match its manifest'
        actual_schema_hash=$(sha256_path "$schema_path")
        [ "$actual_schema_hash" = "$expected_schema_hash" ] || \
            fail_contract schema_digest_mismatch 'RepoSeiri bundle schema digest does not match its manifest'
        schema_count=$((schema_count + 1))
    done
    manifest_schema_count=$(printf '%s\n' "$manifest" | grep -Ec '"seiri\.[a-z0-9.-]+\.json":[[:space:]]*"[0-9a-f]{64}"' || true)
    [ "$manifest_schema_count" -eq "$schema_count" ] || \
        fail_contract schema_set_mismatch 'RepoSeiri bundle schema set contains unexpected entries'
fi

exec "$binary" codex "$@"
