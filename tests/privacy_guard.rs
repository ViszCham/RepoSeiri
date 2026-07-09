use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn q0_public_boundary_guard_scans_committable_text() {
    let root = repo_root();
    let tokens = forbidden_public_boundary_tokens(&root);
    assert!(
        !tokens.is_empty(),
        "Q0 privacy guard must have at least one token to scan"
    );

    let files = committable_files(&root);
    assert!(
        !files.is_empty(),
        "Q0 privacy guard could not discover committable files"
    );

    let mut leaks = Vec::new();
    for relative_path in files {
        if should_skip_path(&relative_path) {
            continue;
        }

        let path = root.join(&relative_path);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        for token in &tokens {
            if text.contains(token) {
                leaks.push(format!(
                    "{} contains a local-only public-boundary token",
                    relative_path.display()
                ));
            }
        }
    }

    assert!(
        leaks.is_empty(),
        "Q0 public boundary guard found local-only material in committable text:\n{}",
        leaks.join("\n")
    );
}

#[test]
fn q11_privacy_guard_covers_tracked_public_text_surfaces() {
    let root = repo_root();
    let files = committable_files(&root)
        .into_iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect::<BTreeSet<_>>();

    for expected in [
        "README.md",
        "docs/design/low-level-claim-boundary-roadmap.md",
        "fixtures/verified-security-route-repo/README.md",
        "fixtures/verified-security-route-repo/SECURITY.md",
        "fixtures/wording-safe-repo/README.md",
        "fixtures/wording-lint-repo/README.md",
        "tests/privacy_guard.rs",
        "tests/q11_public_surface_regression.rs",
        "tests/q11_regression_suite.rs",
    ] {
        assert!(
            files.contains(expected),
            "privacy guard did not cover {expected}"
        );
    }
}

fn committable_files(root: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("ls-files")
        .arg("--cached")
        .arg("--others")
        .arg("--exclude-standard")
        .output()
        .expect("run git ls-files");
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect()
}

fn forbidden_public_boundary_tokens(root: &Path) -> Vec<String> {
    let mut tokens = BTreeSet::new();
    add_token(
        &mut tokens,
        &format!(
            "{}{}",
            "REPOSEIRI_PRIVATE_ANALYSIS_", "CANARY_DO_NOT_COMMIT"
        ),
    );
    add_token(
        &mut tokens,
        &format!("{}{}", "LOCAL_PRIVATE_CALIBRATION_", "CANARY_DO_NOT_COMMIT"),
    );
    add_token(
        &mut tokens,
        &format!("{}{}", "UNPUBLISHED_ANALYSIS_", "CANARY_DO_NOT_COMMIT"),
    );
    add_token(&mut tokens, &format!("{}{}", ".codex/", "reposeiri"));
    add_token(&mut tokens, &format!("{}{}", ".codex\\", "reposeiri"));

    for var in ["USERPROFILE", "HOME"] {
        if let Ok(value) = env::var(var) {
            add_normalized_path_tokens(&mut tokens, &value);
        }
    }

    add_normalized_path_tokens(&mut tokens, &root.display().to_string());

    if let Ok(extra_tokens) = env::var("REPOSEIRI_PUBLIC_BOUNDARY_TOKENS") {
        for token in extra_tokens.split(['\n', '\r', ';']) {
            add_token(&mut tokens, token);
        }
    }

    tokens.into_iter().collect()
}

fn add_normalized_path_tokens(tokens: &mut BTreeSet<String>, value: &str) {
    add_token(tokens, value);
    add_token(tokens, &value.replace('\\', "/"));
}

fn add_token(tokens: &mut BTreeSet<String>, value: &str) {
    let token = value.trim();
    if token.len() >= 8 && !token.contains('<') && !token.contains('>') {
        tokens.insert(token.to_string());
    }
}

fn should_skip_path(relative_path: &Path) -> bool {
    let path = relative_path.to_string_lossy().replace('\\', "/");
    path == "Cargo.lock"
        || path.starts_with(".git/")
        || path.starts_with("target/")
        || path.starts_with("reposeiri-audit/")
}
