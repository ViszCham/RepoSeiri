#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<seiri_core::TextEditSpan>(data);
    let _ = serde_json::from_slice::<seiri_core::SourceSpan>(data);
});
