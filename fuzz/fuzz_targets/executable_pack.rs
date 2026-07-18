#![no_main]
use libfuzzer_sys::fuzz_target;
use std::fs;

fuzz_target!(|data: &[u8]| {
    if data.len() > 2 * 1024 * 1024 {
        return;
    }
    let root = tempfile::tempdir().expect("fuzz tempdir");
    let path = root.path().join("pack.json");
    fs::write(&path, data).expect("fuzz pack");
    let _ = seiri_patterns::load_executable_pattern_pack(path);
});
