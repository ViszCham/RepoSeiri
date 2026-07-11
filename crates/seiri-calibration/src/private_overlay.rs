use crate::{
    load_local_calibration_provider_for_registry, LocalCalibrationProvider, LocalPriorLoadError,
};
use seiri_core::{CalibrationKey, CalibrationLookup, CalibrationProvider, PriorVisibility};
use seiri_patterns::{load_executable_pattern_pack, ExecutablePatternPack, PatternPackLoadError};
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
}

impl CalibrationProvider for PrivateCalibrationOverlay {
    fn prior(&self, key: &CalibrationKey) -> CalibrationLookup {
        self.calibration.prior(key)
    }

    fn visibility(&self) -> Option<PriorVisibility> {
        Some(PriorVisibility::LocalOnly)
    }

    fn redacted_fingerprint(&self) -> Option<&str> {
        Some(self.pattern_pack.fingerprint())
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
