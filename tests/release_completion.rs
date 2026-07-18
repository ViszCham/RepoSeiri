use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(root().join(path)).expect("public contract file")
}

#[test]
fn source_plugin_and_contract_versions_are_synchronized() {
    let workspace = read("Cargo.toml");
    assert!(workspace.contains("version = \"1.0.0\""));

    let plugin: serde_json::Value =
        serde_json::from_str(&read("plugins/reposeiri/.codex-plugin/plugin.json"))
            .expect("plugin manifest");
    assert_eq!(plugin["version"], "1.0.0");

    let contract = seiri_core::ContractManifest::current(env!("CARGO_PKG_VERSION"));
    assert_eq!(contract.tool_version, "1.0.0");
    assert_eq!(contract.analysis_schema, "seiri.analysis.v2");
    assert_eq!(contract.patch_plan_schema, "seiri.patch-plan.v2");
    assert_eq!(contract.codex_schema, "seiri.codex.v2");
}

#[test]
fn plugin_surface_has_no_workspace_or_cargo_runtime_fallback() {
    let skill = read("plugins/reposeiri/skills/reposeiri/SKILL.md");
    let powershell = read("plugins/reposeiri/scripts/reposeiri-codex.ps1");
    let shell = read("plugins/reposeiri/scripts/reposeiri-codex.sh");
    assert!(skill.contains("seiri.codex.v2"));
    assert!(!skill.contains("cargo run"));
    assert!(!powershell.contains("cargo run"));
    assert!(!shell.contains("cargo run"));

    for launcher in [powershell, shell] {
        let configured = launcher.find("REPOSEIRI_BIN").expect("configured binary");
        let bundled = launcher.find("bin/seiri").expect("bundle binary");
        let path = launcher.rfind("PATH").expect("PATH fallback");
        assert!(configured < bundled && bundled < path);
        assert!(launcher.contains("schema_mismatch"));
        assert!(launcher.contains("seiri.error.v1"));
    }
}

#[test]
fn completion_ci_and_fuzz_surfaces_cover_required_hosts_and_boundaries() {
    let ci = read(".github/workflows/ci.yml");
    assert!(ci.contains("cargo run --locked --quiet -p xtask -- completion --format json"));
    assert!(ci.contains("cargo run --locked --quiet -p xtask -- calibration-holdout --format json"));
    assert!(ci.contains("cargo audit"));
    assert!(ci.contains("- dependency-audit"));
    assert!(ci.contains("x86_64-unknown-linux-gnu"));
    assert!(ci.contains("x86_64-pc-windows-msvc"));
    assert!(
        ci.contains("runtime-manifest.json")
            || read("xtask/src/bundle.rs").contains("runtime-manifest.json")
    );

    for target in [
        "markdown",
        "github_yaml",
        "codeowners",
        "predicate",
        "patch_span",
        "calibration_jsonl",
        "gitfile",
        "executable_pack",
        "schema_decoder",
        "audit_delta",
    ] {
        assert!(root()
            .join("fuzz/fuzz_targets")
            .join(format!("{target}.rs"))
            .is_file());
    }
    assert!(ci.contains("fuzz-smoke:"));
    assert!(ci.contains("nightly-2026-07-01"));
    assert!(ci.contains("cargo-fuzz --locked --version"));

    assert!(!ci.contains("ubuntu-latest"));
    assert!(!ci.contains("windows-latest"));
    assert!(!ci.contains("toolchain install stable"));
    for line in ci
        .lines()
        .filter(|line| line.trim_start().starts_with("uses:"))
    {
        let revision = line
            .split('@')
            .nth(1)
            .and_then(|value| value.split_whitespace().next())
            .expect("action revision");
        assert_eq!(revision.len(), 40, "action is not commit-pinned: {line}");
        assert!(
            revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "action revision is not hexadecimal: {line}"
        );
    }

    let bundle = read("xtask/src/bundle.rs");
    for binding in [
        "source_digest",
        "cargo_lock_digest",
        "binary_digest",
        "command_set",
        "schema_sha256",
    ] {
        assert!(bundle.contains(binding), "bundle receipt omitted {binding}");
    }
    let completion = read("xtask/src/completion.rs");
    for claim in [
        "Implemented",
        "LocallyVerified",
        "HostVerified",
        "Calibrated",
        "ManualPolicy",
    ] {
        assert!(
            completion.contains(claim),
            "completion matrix omitted {claim}"
        );
    }
    assert!(completion.contains("ImplementedWithBlockedEvidence"));
}

#[test]
fn security_dependency_floor_and_msrv_are_explicit() {
    let workspace = read("Cargo.toml");
    assert!(workspace.contains("rust-version = \"1.88\""));
    assert!(workspace.contains("gix = { version = \"=0.83.0\""));
    assert!(workspace.contains("time = \"=0.3.47\""));

    let ci = read(".github/workflows/ci.yml");
    assert!(ci.contains("cargo +1.88.0 check --workspace --all-targets --locked"));
}

#[test]
fn every_workspace_crate_root_forbids_unsafe_code() {
    let mut roots = vec![root().join("src/lib.rs"), root().join("xtask/src/main.rs")];
    let crates = fs::read_dir(root().join("crates")).expect("crates directory");
    for entry in crates {
        let path = entry.expect("crate entry").path();
        let lib = path.join("src/lib.rs");
        let main = path.join("src/main.rs");
        if lib.is_file() {
            roots.push(lib);
        } else if main.is_file() {
            roots.push(main);
        }
    }
    for crate_root in roots {
        let body = fs::read_to_string(&crate_root).expect("crate root");
        assert!(
            body.contains("#![forbid(unsafe_code)]"),
            "{} does not forbid unsafe code",
            crate_root.display()
        );
    }
}
