use seiri_core::{
    CoverageIncompleteReason, CoverageStatus, Observation, ProfileKind, RepositoryFacet, RouteKind,
    TargetRelation, UnknownReason,
};
use seiri_fs::ScanOptions;
use seiri_markdown::DocumentIndexOptions;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn coexisting_facets_retain_evidence_without_selecting_a_type() {
    let repo = TempRepo::new("coexisting-facets");
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    );
    repo.write("src/main.rs", "fn main() {}\n");
    repo.write(".github/workflows/ci.yml", "name: CI\n");
    repo.write(
        "docs/guide.md",
        "# Documentation\n\n[Documentation](docs/reference.md)\n",
    );
    repo.write("research/paper.txt", "artifact\n");
    repo.write("templates/default.txt", "artifact\n");
    repo.write("app/web.txt", "artifact\n");
    repo.write(
        "README.md",
        concat!(
            "# Fixture\n\n",
            "## Quickstart\n\n",
            "[Documentation](docs/guide.md)\n",
            "[Support](SUPPORT.md)\n",
            "[Security](SECURITY.md)\n",
            "[Automation](.github/workflows/ci.yml)\n",
            "[Release](CHANGELOG.md)\n",
            "[Contributing](CONTRIBUTING.md)\n",
        ),
    );

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit repository");
    for facet in RepositoryFacet::ALL {
        let assessment = snapshot
            .facets
            .assessment(facet)
            .expect("complete facet set");
        assert!(
            matches!(assessment.observation, Observation::Present { .. }),
            "expected observed {facet:?} facet, got {assessment:?}"
        );
        assert!(
            assessment.evidence_ids().is_some_and(|ids| !ids.is_empty()),
            "expected retained evidence for {facet:?}"
        );
    }
    assert_eq!(snapshot.document_consistency.obligations.len(), 14);
    assert!(snapshot
        .document_consistency
        .obligations
        .iter()
        .all(|obligation| !obligation.reason.as_slice().is_empty()));
    assert!(snapshot
        .document_consistency
        .obligations
        .iter()
        .all(|obligation| matches!(obligation.observation, Observation::Present { .. })));

    let docs_relation = snapshot
        .document_consistency
        .relations
        .iter()
        .find(|relation| relation.route == RouteKind::Docs)
        .expect("cross-document docs target relation");
    assert_eq!(docs_relation.relation, TargetRelation::Refines);
    assert_ne!(docs_relation.left.document, docs_relation.right.document);
    assert_ne!(docs_relation.left.evidence, docs_relation.right.evidence);
    assert!(snapshot.document_consistency.conflicts.is_empty());
    assert_eq!(
        snapshot.document_consistency.conflict_coverage,
        CoverageStatus::Complete
    );

    let json = seiri_report::to_json(&snapshot).expect("canonical JSON");
    let wire: serde_json::Value = serde_json::from_str(&json).expect("wire JSON");
    assert!(wire.get("facets").is_some());
    assert!(wire.get("document_consistency").is_some());
}

#[test]
fn partial_filesystem_coverage_keeps_unsatisfied_obligations_unknown() {
    let repo = TempRepo::new("partial-obligation");
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    );
    repo.write("zeta.txt", "forces a bounded walk to truncate\n");

    let snapshot = seiri_report::audit_repository_with_options(
        repo.path(),
        ProfileKind::Common,
        &ScanOptions {
            max_entries: 1,
            ..ScanOptions::default()
        },
        &DocumentIndexOptions::default(),
    )
    .expect("partial audit");

    assert!(matches!(
        snapshot
            .facets
            .assessment(RepositoryFacet::Package)
            .expect("package facet")
            .observation,
        Observation::Present { .. }
    ));
    let package_obligations = snapshot
        .document_consistency
        .obligations
        .iter()
        .filter(|obligation| obligation.facet == RepositoryFacet::Package)
        .collect::<Vec<_>>();
    assert_eq!(package_obligations.len(), 2);
    assert!(package_obligations.iter().all(|obligation| {
        obligation.observation == Observation::Unknown(UnknownReason::LimitExceeded)
    }));
}

#[test]
fn conflict_pair_limit_is_visible_as_partial_coverage() {
    let repo = TempRepo::new("conflict-bound");
    repo.write("README.md", "# Fixture\n");
    for index in 0..12 {
        repo.write(
            &format!("docs/route-{index}.md"),
            &format!("# Documentation\n\n[Documentation](../DOCS-{index}.md)\n"),
        );
    }

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit repository");
    assert_eq!(snapshot.document_consistency.conflicts.len(), 64);
    assert_eq!(
        snapshot.document_consistency.conflict_coverage,
        CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
    );
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "reposeiri-facet-obligation-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp repo");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, content).expect("write fixture");
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
