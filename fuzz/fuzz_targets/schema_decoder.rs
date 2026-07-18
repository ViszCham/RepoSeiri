#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<seiri_core::ContractManifest>(data);
    let _ = serde_json::from_slice::<seiri_core::PatchPlan>(data);
    let _ = serde_json::from_slice::<seiri_core::PortableAuditSnapshot>(data);
    let _ = serde_json::from_slice::<seiri_core::AuditDeltaReport>(data);
    let _ = serde_json::from_slice::<seiri_core::CalibrationRun>(data);
});
