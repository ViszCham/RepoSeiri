use crate::{
    load_local_calibration_provider_for_registry,
    load_local_calibration_provider_for_registry_and_revision, LocalCalibrationProvider,
    LocalPriorLoadError, PrivateCalibrationFreshness,
};
use seiri_core::{CalibrationKey, CalibrationLookup, CalibrationProvider, PriorVisibility};
use seiri_patterns::{load_executable_pattern_pack, ExecutablePatternPack, PatternPackLoadError};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::path::Path;

pub struct PrivateCalibrationOverlay {
    pattern_pack: ExecutablePatternPack,
    calibration: LocalCalibrationProvider,
}

impl PrivateCalibrationOverlay {
    #[must_use]
    pub fn pattern_pack(&self) -> &ExecutablePatternPack {
        &self.pattern_pack
    }

    #[must_use]
    pub fn registry_fingerprint(&self) -> &str {
        self.calibration.registry_fingerprint()
    }

    #[must_use]
    pub fn metadata(&self) -> PrivateOverlayMetadata {
        PrivateOverlayMetadata {
            schema_version: PRIVATE_OVERLAY_METADATA_SCHEMA_VERSION,
            visibility: "local_only",
            registry_fingerprint: self.calibration.registry_fingerprint().to_string(),
            pattern_pack_fingerprint: self.pattern_pack.fingerprint().to_string(),
            resource_trace: PrivateOverlayResourceTrace {
                source_bytes: self.calibration.source_bytes(),
                prior_count: self.calibration.prior_count(),
            },
            source_path_redacted: true,
            source_body_redacted: true,
            exact_priors_redacted: true,
            freshness: self.calibration.freshness(),
        }
    }
}

pub const PRIVATE_OVERLAY_METADATA_SCHEMA_VERSION: &str = "seiri.private-overlay-metadata.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrivateOverlayMetadata {
    pub schema_version: &'static str,
    pub visibility: &'static str,
    pub registry_fingerprint: String,
    pub pattern_pack_fingerprint: String,
    pub resource_trace: PrivateOverlayResourceTrace,
    pub source_path_redacted: bool,
    pub source_body_redacted: bool,
    pub exact_priors_redacted: bool,
    pub freshness: PrivateCalibrationFreshness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PrivateOverlayResourceTrace {
    pub source_bytes: u64,
    pub prior_count: usize,
}

impl CalibrationProvider for PrivateCalibrationOverlay {
    fn prior(&self, key: &CalibrationKey) -> CalibrationLookup {
        self.calibration.prior(key)
    }

    fn visibility(&self) -> Option<PriorVisibility> {
        Some(PriorVisibility::LocalOnly)
    }

    fn comparison_binding(&self) -> Option<&str> {
        self.calibration.comparison_binding()
    }
}

pub fn load_private_calibration_overlay(
    pattern_pack_path: impl AsRef<Path>,
    calibration_path: impl AsRef<Path>,
) -> Result<PrivateCalibrationOverlay, PrivateOverlayLoadError> {
    let pattern_pack = load_executable_pattern_pack(pattern_pack_path)
        .map_err(PrivateOverlayLoadError::Pattern)?;
    let calibration =
        load_local_calibration_provider_for_registry(calibration_path, pattern_pack.fingerprint())
            .map_err(PrivateOverlayLoadError::Calibration)?;
    Ok(PrivateCalibrationOverlay {
        pattern_pack,
        calibration,
    })
}

pub fn load_private_calibration_overlay_for_revision(
    pattern_pack_path: impl AsRef<Path>,
    calibration_path: impl AsRef<Path>,
    expected_opaque_revision: &str,
) -> Result<PrivateCalibrationOverlay, PrivateOverlayLoadError> {
    let pattern_pack = load_executable_pattern_pack(pattern_pack_path)
        .map_err(PrivateOverlayLoadError::Pattern)?;
    let calibration = load_local_calibration_provider_for_registry_and_revision(
        calibration_path,
        pattern_pack.fingerprint(),
        Some(expected_opaque_revision),
    )
    .map_err(PrivateOverlayLoadError::Calibration)?;
    Ok(PrivateCalibrationOverlay {
        pattern_pack,
        calibration,
    })
}

#[derive(Debug)]
pub enum PrivateOverlayLoadError {
    Pattern(PatternPackLoadError),
    Calibration(LocalPriorLoadError),
}

impl Display for PrivateOverlayLoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern(error) => write!(formatter, "private pattern pack is invalid: {error}"),
            Self::Calibration(error) => {
                write!(formatter, "private calibration overlay is invalid: {error}")
            }
        }
    }
}

impl std::error::Error for PrivateOverlayLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pattern(error) => Some(error),
            Self::Calibration(error) => Some(error),
        }
    }
}
