#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = seiri_markdown::scan_document_with_options(
            "README.md",
            text,
            &seiri_markdown::DocumentScanOptions {
                max_source_bytes: 64 * 1024,
                max_events: 1024,
                max_diagnostics: 128,
            },
        );
    }
});
