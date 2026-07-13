#![no_main]
use libfuzzer_sys::fuzz_target;
use std::fs;

fuzz_target!(|data: &[u8]| {
    let Ok(root) = tempfile::tempdir() else { return };
    let path = root.path().join(".github/ISSUE_TEMPLATE/fuzz.yml");
    let Some(parent) = path.parent() else { return };
    if fs::create_dir_all(parent).is_err() || fs::write(path, data).is_err() { return }
    let _ = seiri_report::audit_repository(root.path());
});
