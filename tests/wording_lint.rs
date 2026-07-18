use seiri_core::{ClaimBoundaryKind, ProfileKind, WordingRuleKind};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn wording_linter_emits_byte_spans_for_overclaims() {
    let report = seiri_report::lint_wording_repository(fixture("wording-lint-repo"))
        .expect("wording lint report");

    assert_eq!(report.summary.files_scanned, 2);
    assert_eq!(report.summary.generated_surfaces, 3);
    assert_eq!(report.summary.findings, 4);
    assert!(report.summary.suppressed_boundary_exceptions >= 3);

    let finding = report
        .findings
        .iter()
        .find(|finding| finding.matched == "guarantees security")
        .expect("security guarantee finding");
    assert_eq!(finding.path, "README.md");
    assert_eq!(finding.line, 3);
    assert_eq!(finding.column, 17);
    assert_eq!(finding.rule, WordingRuleKind::SecurityGuarantee);
    assert_eq!(finding.boundary, ClaimBoundaryKind::NotSecurityGuarantee);

    let source = std::fs::read_to_string(fixture("wording-lint-repo").join("README.md"))
        .expect("read fixture README");
    assert_eq!(
        &source[finding.byte_start..finding.byte_end],
        finding.matched
    );
}

#[test]
fn wording_linter_uses_typed_boundary_exceptions() {
    let report = seiri_report::lint_wording_repository(fixture("wording-lint-repo"))
        .expect("wording lint report");

    assert!(report
        .findings
        .iter()
        .all(|finding| finding.matched != "guarantee quality"));
    assert!(report
        .findings
        .iter()
        .all(|finding| finding.matched != "legal advice"));
    assert!(report
        .findings
        .iter()
        .all(|finding| !finding.matched.contains("NotSecurityGuarantee")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.matched == "production-ready"));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.matched == "legally compliant"));
}

#[test]
fn wording_linter_renders_json_and_markdown() {
    let report = seiri_report::lint_wording_repository_with_profile(
        fixture("wording-lint-repo"),
        ProfileKind::Common,
    )
    .expect("wording lint report");

    let json = seiri_report::wording_lint_to_json(&report).expect("wording lint JSON");
    let parsed = serde_json::from_str::<serde_json::Value>(&json).expect("valid JSON");
    assert_eq!(parsed["schema_version"], "seiri.wording-lint.v1");
    assert_eq!(parsed["summary"]["findings"], 4);
    assert!(parsed["findings"][0]["byte_start"].is_number());
    assert!(json.contains("\"replacement_hint\""));

    let markdown = seiri_report::wording_lint_to_markdown(&report);
    assert!(markdown.contains("# RepoSeiri Wording Lint Report"));
    assert!(markdown.contains("## Findings"));
    assert!(markdown.contains("- Byte range: `"));
    assert!(markdown.contains("- Boundary: `Not"));
    assert!(markdown.contains("Replacement hint:"));
}

#[test]
fn wording_linter_ignores_markdown_dead_zones() {
    let root = tempfile::tempdir().expect("temporary repository");
    std::fs::write(
        root.path().join("README.md"),
        concat!(
            "# Claims\n\n",
            "Visible prose guarantees security.\n\n",
            "```text\nHidden block guarantees trust.\n```\n\n",
            "    Hidden indent guarantees quality.\n\n",
            "`inline guarantees popularity`\n\n",
            "<!-- comment guarantees maintenance -->\n\n",
            "<pre>raw HTML guarantees security</pre>\n",
        ),
    )
    .expect("write README");

    let report = seiri_report::lint_wording_repository(root.path()).expect("wording lint report");
    let repository_findings = report
        .findings
        .iter()
        .filter(|finding| finding.source == seiri_core::WordingLintSourceKind::RepositoryFile)
        .collect::<Vec<_>>();
    assert_eq!(repository_findings.len(), 1, "{repository_findings:#?}");
    assert_eq!(repository_findings[0].matched, "guarantees security");
    assert_eq!(repository_findings[0].line, 3);
    assert_eq!(
        repository_findings[0].byte_start,
        std::fs::read_to_string(root.path().join("README.md"))
            .expect("read README")
            .find("guarantees security")
            .expect("visible phrase")
    );
}
