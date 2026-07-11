use super::model::{ExecutablePatternPack, FixtureSuiteReport};
use crate::PatternAdoptionStage;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternAdoptionReview {
    pub review_id: String,
    pub reviewer: String,
    pub reviewed_pack_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdoptionBlocker {
    FixtureFailure,
    MissingReview,
    ReviewFingerprintMismatch,
    SchemaMismatch,
    NonCandidateDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptionGateDecision {
    EligibleForMaintainerAdoption,
    Blocked(Vec<AdoptionBlocker>),
}

#[must_use]
pub fn evaluate_adoption_gate(
    pack: &ExecutablePatternPack,
    report: &FixtureSuiteReport,
    review: Option<&PatternAdoptionReview>,
    expected_schema: &str,
) -> AdoptionGateDecision {
    let mut blockers = BTreeSet::new();
    if !report.all_passed() || report.pack_fingerprint != pack.fingerprint() {
        blockers.insert(AdoptionBlocker::FixtureFailure);
    }
    if pack.schema_version() != expected_schema {
        blockers.insert(AdoptionBlocker::SchemaMismatch);
    }
    if pack
        .definitions()
        .iter()
        .any(|definition| definition.adoption_stage != PatternAdoptionStage::Candidate)
    {
        blockers.insert(AdoptionBlocker::NonCandidateDefinition);
    }
    match review {
        Some(review)
            if !review.review_id.trim().is_empty()
                && !review.reviewer.trim().is_empty()
                && review.reviewed_pack_fingerprint == pack.fingerprint() => {}
        Some(_) => {
            blockers.insert(AdoptionBlocker::ReviewFingerprintMismatch);
        }
        None => {
            blockers.insert(AdoptionBlocker::MissingReview);
        }
    }
    if blockers.is_empty() {
        AdoptionGateDecision::EligibleForMaintainerAdoption
    } else {
        AdoptionGateDecision::Blocked(blockers.into_iter().collect())
    }
}
