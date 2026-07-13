use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn codex_queries_and_plan_use_single_cli_surface() {
    let root = temp_root();
    fs::create_dir(root.join("docs")).expect("docs");
    fs::write(root.join("README.md"), "# Tool\n").expect("README");
    fs::write(root.join("docs/README.md"), "# Docs\n").expect("docs");

    for query in [
        "summary",
        "routes",
        "evidence",
        "documents",
        "governance",
        "patches",
        "linter",
        "actions",
        "remote",
        "pr-body",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_seiri"))
            .args([
                "codex",
                "--path",
                root.to_str().expect("path"),
                "--query",
                query,
                "--format",
                "json",
            ])
            .output()
            .expect("codex");
        assert!(output.status.success(), "query {query}: {:?}", output);
        assert!(String::from_utf8_lossy(&output.stdout).contains("seiri.codex.v2"));
    }

    let plan = Command::new(env!("CARGO_BIN_EXE_seiri"))
        .args(["plan", "--path", root.to_str().expect("path")])
        .output()
        .expect("plan");
    assert!(plan.status.success());

    for removed in [vec!["plan-v5"], vec!["codex", "--schema", "native-v2"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_seiri"))
            .args(removed)
            .output()
            .expect("removed CLI");
        assert!(!output.status.success());
    }

    let contract = Command::new(env!("CARGO_BIN_EXE_seiri"))
        .args(["contract", "--format", "json"])
        .output()
        .expect("contract");
    assert!(contract.status.success());
    let contract_json: serde_json::Value =
        serde_json::from_slice(&contract.stdout).expect("contract JSON");
    assert_eq!(contract_json["codex_schema"], "seiri.codex.v2");
    assert!(!String::from_utf8_lossy(&contract.stderr).contains("seiri.error.v1"));

    let failure = Command::new(env!("CARGO_BIN_EXE_seiri"))
        .args([
            "audit",
            "--path",
            root.join("missing").to_str().expect("path"),
        ])
        .output()
        .expect("typed failure");
    assert!(!failure.status.success());
    assert!(failure.stdout.is_empty());
    let error: serde_json::Value =
        serde_json::from_slice(&failure.stderr).expect("typed error JSON");
    assert_eq!(error["schema_version"], "seiri.error.v1");
    fs::remove_dir_all(root).expect("remove temp repository");
}

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-cli-{nonce}"));
    fs::create_dir_all(&root).expect("temp root");
    root
}
