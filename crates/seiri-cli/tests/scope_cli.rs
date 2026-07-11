use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn audit_scope_flag_requires_explicit_subtree_mode() {
    let root = temp_root();
    create_git(&root.join(".git"));
    let nested = root.join("fixtures/case");
    fs::create_dir_all(&nested).expect("nested");
    fs::write(nested.join("README.md"), "# Fixture\n").expect("README");

    let default = run_audit(&nested, None);
    let subtree = run_audit(&nested, Some("subtree"));
    let root_text = normalized(&fs::canonicalize(&root).unwrap());
    let nested_text = normalized(&fs::canonicalize(&nested).unwrap());
    assert!(default.contains(&format!("- Repository: `{root_text}`")));
    assert!(default.contains("- Analysis scope: `Repository`"));
    assert!(subtree.contains(&format!("- Repository: `{nested_text}`")));
    assert!(subtree.contains("- Analysis scope: `Subtree`"));
    fs::remove_dir_all(root).expect("cleanup");
}

fn run_audit(path: &Path, scope: Option<&str>) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_seiri"));
    command
        .arg("audit")
        .arg("--path")
        .arg(path)
        .arg("--format")
        .arg("markdown");
    if let Some(scope) = scope {
        command.arg("--scope").arg(scope);
    }
    let output = command.output().expect("run seiri");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 output")
}

fn create_git(path: &Path) {
    fs::create_dir_all(path.join("objects")).expect("objects");
    fs::create_dir_all(path.join("refs/heads")).expect("refs");
    fs::write(path.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
}

fn normalized(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    path.strip_prefix("//?/").unwrap_or(&path).to_string()
}

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-ab-cli-{nonce}"));
    fs::create_dir_all(&root).expect("temp root");
    root
}
