#![no_main]
use libfuzzer_sys::fuzz_target;
use std::fs;

fuzz_target!(|data: &[u8]| {
    let Ok(root) = tempfile::tempdir() else { return };
    if fs::write(root.path().join(".git"), data).is_err() { return }
    let _ = seiri_git_local::discover_repository(root.path(), seiri_core::AnalysisScope::Repository);
});
