use seiri_codex::{CodexQuery, CodexQueryKind, CodexView};
use seiri_core::{ProfileKind, CODEX_SCHEMA_VERSION};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn every_codex_query_uses_one_borrowed_schema() {
    let root = temp_root("codex");
    fs::create_dir(root.join("docs")).expect("docs");
    fs::write(root.join("README.md"), "# Tool\n").expect("README");
    fs::write(root.join("docs/README.md"), "# Docs\n").expect("docs");
    let analysis =
        seiri_report::audit_repository_with_profile(&root, ProfileKind::Cli).expect("audit");
    let plan = seiri_planner::plan_patches(&analysis);
    let adapter = CodexView::new(&analysis, &plan, None);

    for kind in CodexQueryKind::ALL {
        let view = adapter.query(kind);
        assert_eq!(view.schema_version, CODEX_SCHEMA_VERSION);
        let json = serde_json::to_string(&view).expect("query JSON");
        assert!(json.contains("seiri.codex.v2"));
        for removed in ["compatibility-v1", "native-v2", "native-v3"] {
            assert!(!json.contains(removed));
        }
        let markdown = seiri_codex::render_query_markdown(&view);
        assert!(markdown.contains("# RepoSeiri Codex Query"));
    }

    assert!(matches!(
        adapter.query(CodexQueryKind::Patches).query,
        CodexQuery::Patches(_)
    ));
    assert!(matches!(
        adapter.query(CodexQueryKind::PrBody).query,
        CodexQuery::PrBody(_)
    ));
    fs::remove_dir_all(root).expect("remove temp repository");
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-{label}-{nonce}"));
    fs::create_dir_all(&root).expect("temp root");
    root
}
