use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn diff_and_plan_are_dry_run_cli_surfaces() {
    let root = temp_root();
    let before = root.join("before");
    let after = root.join("after");
    for path in [&before, &after] {
        fs::create_dir_all(path.join("docs")).unwrap();
        fs::write(path.join("docs/index.md"), "# Docs\n").unwrap();
    }
    fs::write(
        before.join("README.md"),
        "# Demo\n\n[Documentation](docs/)\n",
    )
    .unwrap();
    fs::write(after.join("README.md"), "# Demo\n").unwrap();

    let diff = Command::new(env!("CARGO_BIN_EXE_seiri"))
        .args(["diff", "--before"])
        .arg(&before)
        .arg("--after")
        .arg(&after)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(
        diff.status.success(),
        "{}",
        String::from_utf8_lossy(&diff.stderr)
    );
    let delta: serde_json::Value = serde_json::from_slice(&diff.stdout).unwrap();
    assert_eq!(delta["compatibility"]["state"], "comparable");
    assert!(delta["regressions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["key"] == "Docs"));

    let plan = Command::new(env!("CARGO_BIN_EXE_seiri"))
        .args(["plan", "--path"])
        .arg(&after)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert_eq!(plan["writes_files"], false);
    assert!(plan["operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["route"] == "docs"));
    assert_eq!(fs::read(after.join("README.md")).unwrap(), b"# Demo\n");
    fs::remove_dir_all(root).unwrap();
}

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-ad-cli-{nonce}"));
    fs::create_dir_all(&root).unwrap();
    root
}
