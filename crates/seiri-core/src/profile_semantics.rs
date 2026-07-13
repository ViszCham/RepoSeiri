use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationPriorState {
    #[default]
    NotRequested,
    AppliedRedacted,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileFit(u8);

impl ProfileFit {
    #[must_use]
    pub const fn from_bounded(value: u8) -> Self {
        Self(if value > 100 { 100 } else { value })
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfilePurposeAffinity(u8);

impl ProfilePurposeAffinity {
    #[must_use]
    pub const fn from_bounded(value: u8) -> Self {
        Self(if value > 100 { 100 } else { value })
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRankScore(u8);

impl ProfileRankScore {
    #[must_use]
    pub const fn from_bounded(value: u8) -> Self {
        Self(if value > 100 { 100 } else { value })
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileBranchSemantics {
    pub fit: ProfileFit,
    pub purpose_affinity: ProfilePurposeAffinity,
    pub rank_score: ProfileRankScore,
    pub calibration_prior: CalibrationPriorState,
}
