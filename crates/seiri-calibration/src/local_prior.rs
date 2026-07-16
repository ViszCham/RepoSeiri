use seiri_core::{
    AggregatePrior, CalibrationKey, CalibrationLookup, CalibrationProvider,
    CalibrationUnavailableReason, PriorBasis, PriorVisibility, ProfileKind, RouteKind,
};
use seiri_digest::{Digest32, StableHasher};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Take};
use std::num::NonZeroU64;
use std::path::Path;

pub const LOCAL_PRIOR_SCHEMA_VERSION: &str = "seiri.local-calibration-priors.v2";
const MAX_LOCAL_PRIOR_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LOCAL_PRIORS: usize = 4096;
const MAX_RULE_ID_BYTES: usize = 128;

pub struct LocalCalibrationProvider {
    priors: BTreeMap<CalibrationKey, AggregatePrior>,
    registry_fingerprint: Box<str>,
    _private_digest: PrivateCalibrationDigest,
    comparison_binding: Option<Box<str>>,
    source_bytes: u64,
}

// Deliberately has no Debug, Display, Serialize, or public accessor.
struct PrivateCalibrationDigest {
    _value: Digest32,
}

impl LocalCalibrationProvider {
    #[must_use]
    pub fn registry_fingerprint(&self) -> &str {
        &self.registry_fingerprint
    }

    #[must_use]
    pub const fn source_bytes(&self) -> u64 {
        self.source_bytes
    }

    #[must_use]
    pub fn prior_count(&self) -> usize {
        self.priors.len()
    }
}

impl CalibrationProvider for LocalCalibrationProvider {
    fn prior(&self, key: &CalibrationKey) -> CalibrationLookup {
        self.priors.get(key).cloned().map_or(
            CalibrationLookup::Unavailable(CalibrationUnavailableReason::KeyMissing),
            CalibrationLookup::Available,
        )
    }

    fn visibility(&self) -> Option<PriorVisibility> {
        Some(PriorVisibility::LocalOnly)
    }

    fn comparison_binding(&self) -> Option<&str> {
        self.comparison_binding.as_deref()
    }
}

#[derive(Debug)]
pub enum LocalPriorLoadError {
    Io(std::io::ErrorKind),
    Json { line: usize, column: usize },
    SourceTooLarge,
    TooManyPriors,
    UnsupportedSchema,
    RegistryFingerprintMismatch,
    InvalidPrior,
    InvalidRuleId,
    InvalidOpaqueRevision,
    DuplicateKey,
}

impl Display for LocalPriorLoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(kind) => write!(formatter, "failed to read local calibration pack: {kind:?}"),
            Self::Json { line, column } => write!(
                formatter,
                "failed to parse local calibration pack at line {line}, column {column}"
            ),
            Self::SourceTooLarge => {
                formatter.write_str("local calibration pack exceeds the byte limit")
            }
            Self::TooManyPriors => {
                formatter.write_str("local calibration pack exceeds the prior-count limit")
            }
            Self::UnsupportedSchema => {
                formatter.write_str("local calibration pack schema is unsupported")
            }
            Self::RegistryFingerprintMismatch => {
                formatter.write_str("local calibration pack registry fingerprint does not match")
            }
            Self::InvalidPrior => {
                formatter.write_str("local calibration pack contains an invalid prior")
            }
            Self::InvalidRuleId => {
                formatter.write_str("local calibration pack contains an invalid rule id")
            }
            Self::InvalidOpaqueRevision => {
                formatter.write_str("local calibration pack contains an invalid opaque revision")
            }
            Self::DuplicateKey => {
                formatter.write_str("local calibration pack contains a duplicate key")
            }
        }
    }
}

impl std::error::Error for LocalPriorLoadError {}

#[derive(Deserialize)]
struct WirePack {
    schema_version: String,
    registry_fingerprint: String,
    #[serde(default, alias = "private_note")]
    _private_note: Option<String>,
    #[serde(default)]
    opaque_revision: Option<String>,
    priors: Vec<WirePrior>,
}

#[derive(Deserialize)]
struct WirePrior {
    key: WireKey,
    observed: u64,
    sample_size: u64,
    rank_weight_x100: u8,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WireKey {
    RouteGap { route: RouteKind },
    CoOccurrence { rule_id: String },
    ProfileBranch { profile: ProfileKind },
}

pub fn load_local_calibration_provider(
    path: impl AsRef<Path>,
) -> Result<LocalCalibrationProvider, LocalPriorLoadError> {
    let expected_fingerprint = seiri_patterns::common_pattern_pack()
        .fingerprint()
        .to_string();
    load_local_calibration_provider_for_registry(path, &expected_fingerprint)
}

pub fn load_local_calibration_provider_for_registry(
    path: impl AsRef<Path>,
    expected_fingerprint: &str,
) -> Result<LocalCalibrationProvider, LocalPriorLoadError> {
    let file = File::open(path.as_ref()).map_err(|error| LocalPriorLoadError::Io(error.kind()))?;
    let mut bytes = Vec::new();
    let mut bounded: Take<File> = file.take(MAX_LOCAL_PRIOR_BYTES + 1);
    bounded
        .read_to_end(&mut bytes)
        .map_err(|error| LocalPriorLoadError::Io(error.kind()))?;
    if bytes.len() as u64 > MAX_LOCAL_PRIOR_BYTES {
        return Err(LocalPriorLoadError::SourceTooLarge);
    }
    let wire: WirePack =
        serde_json::from_slice(&bytes).map_err(|error| LocalPriorLoadError::Json {
            line: error.line(),
            column: error.column(),
        })?;
    if wire.schema_version != LOCAL_PRIOR_SCHEMA_VERSION {
        return Err(LocalPriorLoadError::UnsupportedSchema);
    }
    if wire.priors.len() > MAX_LOCAL_PRIORS {
        return Err(LocalPriorLoadError::TooManyPriors);
    }

    if wire.registry_fingerprint != expected_fingerprint {
        return Err(LocalPriorLoadError::RegistryFingerprintMismatch);
    }

    let mut priors = BTreeMap::new();
    for item in wire.priors {
        let key = match item.key {
            WireKey::RouteGap { route } => CalibrationKey::RouteGap(route),
            WireKey::CoOccurrence { rule_id } => {
                if rule_id.is_empty() || rule_id.len() > MAX_RULE_ID_BYTES {
                    return Err(LocalPriorLoadError::InvalidRuleId);
                }
                CalibrationKey::CoOccurrence(rule_id.into_boxed_str())
            }
            WireKey::ProfileBranch { profile } => CalibrationKey::ProfileBranch(profile),
        };
        let sample_size =
            NonZeroU64::new(item.sample_size).ok_or(LocalPriorLoadError::InvalidPrior)?;
        let prior = AggregatePrior::try_new(
            item.observed,
            sample_size,
            item.rank_weight_x100,
            PriorBasis::AggregateAnalysis,
        )
        .map_err(|_| LocalPriorLoadError::InvalidPrior)?;
        if priors.insert(key, prior).is_some() {
            return Err(LocalPriorLoadError::DuplicateKey);
        }
    }

    let private_digest = private_calibration_digest(&priors, &wire.registry_fingerprint);
    let comparison_binding = wire
        .opaque_revision
        .map(|value| {
            if value.is_empty()
                || value.len() > MAX_RULE_ID_BYTES
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            {
                return Err(LocalPriorLoadError::InvalidOpaqueRevision);
            }
            Ok(value.into_boxed_str())
        })
        .transpose()?;
    Ok(LocalCalibrationProvider {
        priors,
        registry_fingerprint: wire.registry_fingerprint.into_boxed_str(),
        _private_digest: private_digest,
        comparison_binding,
        source_bytes: bytes.len() as u64,
    })
}

fn private_calibration_digest(
    priors: &BTreeMap<CalibrationKey, AggregatePrior>,
    registry_fingerprint: &str,
) -> PrivateCalibrationDigest {
    let mut hasher = StableHasher::new(b"seiri.private-calibration.semantic.v2");
    hasher.str(1, registry_fingerprint);
    hasher.usize(2, priors.len());
    for (key, prior) in priors {
        match key {
            CalibrationKey::RouteGap(route) => {
                hasher.u8(3, 0);
                hasher.u8(4, *route as u8);
            }
            CalibrationKey::CoOccurrence(rule) => {
                hasher.u8(3, 1);
                hasher.str(4, rule);
            }
            CalibrationKey::ProfileBranch(profile) => {
                hasher.u8(3, 2);
                hasher.u8(4, *profile as u8);
            }
        }
        hasher.u64(5, prior.observed());
        hasher.u64(6, prior.sample_size().get());
        hasher.u8(7, prior.rank_weight_x100());
    }
    PrivateCalibrationDigest {
        _value: hasher.finish(),
    }
}
