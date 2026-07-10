use seiri_core::{stable_id, CodexCommand, CodexNativeAction, ProfileKind, RepoSnapshot};

pub(crate) fn build_native_actions(
    snapshot: &RepoSnapshot,
    profile: Option<ProfileKind>,
) -> Vec<CodexNativeAction> {
    let selected_profile = profile.unwrap_or(ProfileKind::Common).to_string();
    [
        (
            "Render audit report",
            "audit",
            "Re-run the Rust core audit and inspect evidence, baseline, profile, and findings.",
        ),
        (
            "Render dry-run patch plan",
            "plan",
            "Show safe operations and guarded/manual blocked items without writing files.",
        ),
        (
            "Render Codex PR draft context",
            "codex",
            "Generate the Codex-facing review context and draft PR body from Rust core outputs.",
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (label, subcommand, detail))| CodexNativeAction {
        id: stable_id("codex-action", index + 1),
        label: label.to_string(),
        command: CodexCommand::new(
            "cargo",
            [
                "run".to_string(),
                "--quiet".to_string(),
                "-p".to_string(),
                "seiri-cli".to_string(),
                "--".to_string(),
                subcommand.to_string(),
                "--path".to_string(),
                snapshot.repo_root.clone(),
                "--profile".to_string(),
                selected_profile.clone(),
                "--format".to_string(),
                "markdown".to_string(),
            ],
        )
        .expect("built-in Codex action argv is valid"),
        mutates_files: false,
        requires_confirmation: false,
        detail: detail.to_string(),
    })
    .collect()
}
