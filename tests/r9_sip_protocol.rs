use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn r9_sip_units_are_strictly_sequential_and_git_authority_is_denied() {
    let body = fs::read_to_string(root().join("docs/design/r9-sip-v1-template.json"))
        .expect("read template");
    let value: Value = serde_json::from_str(&body).expect("valid template");
    assert_eq!(value["schema_version"], "reposeiri.r9-sip.v1");
    let units = value["units"].as_array().expect("units");
    assert_eq!(units.len(), 13);
    for (index, unit) in units.iter().enumerate() {
        assert_eq!(unit["id"], format!("SI{index}"));
        let dependencies = unit["depends_on"].as_array().expect("dependencies");
        if index == 0 {
            assert!(dependencies.is_empty());
        } else {
            assert_eq!(dependencies, &[Value::String(format!("SI{}", index - 1))]);
        }
    }
    for denied in [
        "commit",
        "push",
        "merge",
        "release",
        "publication",
        "plugin_install",
    ] {
        assert_eq!(value["authority_defaults"][denied], false);
    }
    assert_eq!(value["blocked_check_policy"], "never_promote_to_pass");
}

#[test]
fn roadmap_and_protocol_keep_japanese_first_and_english_equivalent() {
    for path in [
        "docs/design/roadmap-v9-semantic-identity-verification-closure.md",
        "docs/design/r9-sip-v1-protocol.md",
    ] {
        let body = fs::read_to_string(root().join(path)).expect("read design document");
        let japanese = body.find("## 日本語").expect("Japanese section");
        let english = body.find("## English").expect("English section");
        assert!(japanese < english);
        let japanese_body = &body[japanese..english];
        let english_body = &body[english..];
        for term in ["SI0", "SI12", "EVIDENCE_COMPLETE"] {
            assert!(japanese_body.contains(term), "{path}: Japanese {term}");
            assert!(english_body.contains(term), "{path}: English {term}");
        }
    }
}
