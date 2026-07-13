#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let limits = seiri_calibration::StreamingCalibrationLimits::new(64 * 1024, 1024, 4096, 128)
        .expect("fixed limits");
    let metadata = seiri_calibration::StreamingCalibrationMetadata::new("fuzz", "fuzz", "unknown");
    let _ = seiri_calibration::calibrate_jsonl_reader_with_limits(Cursor::new(data), metadata, limits);
});
