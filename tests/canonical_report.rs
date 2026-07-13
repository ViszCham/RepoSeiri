use seiri_core::ProfileKind;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn audit_wire_contains_canonical_analysis_without_private_inputs() {
    let root = temp_root();
    fs::write(root.join("README.md"), "# Practice tool\n").expect("README");
    let analysis =
        seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).expect("audit");
    let json = seiri_report::to_json(&analysis).expect("JSON");
    let value: serde_json::Value = serde_json::from_str(&json).expect("value");

    assert_eq!(value["schema_version"], "seiri.analysis.v2");
    assert!(value["evidence_kernel"]["facts"].is_array());
    assert!(value["route_content"]["assessments"].is_array());
    assert!(value["route_assessments"].is_array());
    assert!(value.get("analysis_configuration").is_some());
    for removed in [
        "evidence",
        "evidence_ledger",
        "route_states",
        "evidence_kernel_v2",
    ] {
        assert!(
            value.get(removed).is_none(),
            "removed key leaked: {removed}"
        );
    }
    fs::remove_dir_all(root).expect("remove temp repository");
}

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-report-{nonce}"));
    fs::create_dir_all(&root).expect("temp root");
    root
}
