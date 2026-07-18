#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok((before, after)) =
        serde_json::from_slice::<(
            seiri_core::PortableAuditSnapshot,
            seiri_core::PortableAuditSnapshot,
        )>(data)
    else {
        return;
    };
    let _ = seiri_delta::compare(&before, &after);
});
