use crate::{ProfileKind, RouteKind};
use std::fmt::{Display, Formatter};
use std::num::NonZeroU64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalibrationKey {
    RouteGap(RouteKind),
    CoOccurrence(Box<str>),
    ProfileBranch(ProfileKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorBasis {
    AggregateAnalysis,
    PublicSynthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorVisibility {
    PublicSynthetic,
    LocalOnly,
    Redacted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregatePriorError {
    ObservedExceedsSample,
    RankWeightOutOfRange,
}

impl Display for AggregatePriorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ObservedExceedsSample => {
                formatter.write_str("aggregate prior observation exceeds its sample")
            }
            Self::RankWeightOutOfRange => {
                formatter.write_str("aggregate prior rank weight exceeds 100")
            }
        }
    }
}

impl std::error::Error for AggregatePriorError {}

// Values intentionally have no Debug, Serialize, or Deserialize implementation.
#[derive(Clone, PartialEq, Eq)]
pub struct AggregatePrior {
    observed: u64,
    sample_size: NonZeroU64,
    rank_weight_x100: u8,
    basis: PriorBasis,
}

impl AggregatePrior {
    pub fn try_new(
        observed: u64,
        sample_size: NonZeroU64,
        rank_weight_x100: u8,
        basis: PriorBasis,
    ) -> Result<Self, AggregatePriorError> {
        if observed > sample_size.get() {
            return Err(AggregatePriorError::ObservedExceedsSample);
        }
        if rank_weight_x100 > 100 {
            return Err(AggregatePriorError::RankWeightOutOfRange);
        }
        Ok(Self {
            observed,
            sample_size,
            rank_weight_x100,
            basis,
        })
    }

    #[must_use]
    pub const fn observed(&self) -> u64 {
        self.observed
    }

    #[must_use]
    pub const fn sample_size(&self) -> NonZeroU64 {
        self.sample_size
    }

    #[must_use]
    pub const fn rank_weight_x100(&self) -> u8 {
        self.rank_weight_x100
    }

    #[must_use]
    pub const fn basis(&self) -> PriorBasis {
        self.basis
    }

    #[must_use]
    pub fn rate_x1000(&self) -> u16 {
        ((u128::from(self.observed) * 1000) / u128::from(self.sample_size.get())) as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationUnavailableReason {
    KeyMissing,
    ProviderRejected,
}

#[derive(Clone, PartialEq, Eq)]
pub enum CalibrationLookup {
    NotRequested,
    Available(AggregatePrior),
    Unavailable(CalibrationUnavailableReason),
}

pub trait CalibrationProvider {
    fn prior(&self, key: &CalibrationKey) -> CalibrationLookup;

    fn visibility(&self) -> Option<PriorVisibility>;

    fn redacted_fingerprint(&self) -> Option<&str> {
        None
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoCalibrationProvider;

impl CalibrationProvider for NoCalibrationProvider {
    fn prior(&self, _key: &CalibrationKey) -> CalibrationLookup {
        CalibrationLookup::NotRequested
    }

    fn visibility(&self) -> Option<PriorVisibility> {
        None
    }
}
