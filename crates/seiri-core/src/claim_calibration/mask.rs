use crate::{ClaimBoundaryKind, MeaningAtom};

const ALL_MEANINGS: [MeaningAtom; 13] = [
    MeaningAtom::RouteObserved,
    MeaningAtom::RouteMissing,
    MeaningAtom::RepositoryLocalTargetPresent,
    MeaningAtom::RepositoryLocalTargetMissing,
    MeaningAtom::ReadmeMentionsRoute,
    MeaningAtom::StructuredFilePresent,
    MeaningAtom::AutomationConfigured,
    MeaningAtom::HumanReviewRequired,
    MeaningAtom::PatchPreviewOnly,
    MeaningAtom::CalibrationCandidate,
    MeaningAtom::ContentSlotObserved,
    MeaningAtom::ExpectedOutputDocumented,
    MeaningAtom::StructuralParallelCandidate,
];

const ALL_BOUNDARIES: [ClaimBoundaryKind; 13] = [
    ClaimBoundaryKind::NotPopularityGuarantee,
    ClaimBoundaryKind::NotTrustGuarantee,
    ClaimBoundaryKind::NotSecurityGuarantee,
    ClaimBoundaryKind::NotQualityGuarantee,
    ClaimBoundaryKind::NotLegalFitnessGuarantee,
    ClaimBoundaryKind::NotLegalAdvice,
    ClaimBoundaryKind::NotMaintenanceGuarantee,
    ClaimBoundaryKind::NotRuntimeVerification,
    ClaimBoundaryKind::NotPublicationReadiness,
    ClaimBoundaryKind::NotOwnerApproval,
    ClaimBoundaryKind::NotProductionReadiness,
    ClaimBoundaryKind::NotAutomaticPolicyAdoption,
    ClaimBoundaryKind::NotAutomaticWeightAdoption,
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct MeaningMask(u16);

impl MeaningMask {
    #[must_use]
    pub fn from_atoms(atoms: &[MeaningAtom]) -> Self {
        atoms
            .iter()
            .fold(Self::default(), |mask, atom| mask.with(*atom))
    }

    #[must_use]
    pub const fn with(self, atom: MeaningAtom) -> Self {
        Self(self.0 | meaning_bit(atom))
    }

    #[must_use]
    pub const fn contains(self, atom: MeaningAtom) -> bool {
        self.0 & meaning_bit(atom) != 0
    }

    #[must_use]
    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = MeaningAtom> {
        ALL_MEANINGS
            .into_iter()
            .filter(move |atom| self.contains(*atom))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct ClaimBoundaryMask(u16);

impl ClaimBoundaryMask {
    #[must_use]
    pub const fn with(self, boundary: ClaimBoundaryKind) -> Self {
        Self(self.0 | boundary_bit(boundary))
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub const fn contains(self, boundary: ClaimBoundaryKind) -> bool {
        self.0 & boundary_bit(boundary) != 0
    }

    #[must_use]
    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = ClaimBoundaryKind> {
        ALL_BOUNDARIES
            .into_iter()
            .filter(move |boundary| self.contains(*boundary))
    }

    #[must_use]
    pub fn to_vec(self) -> Vec<ClaimBoundaryKind> {
        self.iter().collect()
    }
}

const fn meaning_bit(atom: MeaningAtom) -> u16 {
    match atom {
        MeaningAtom::RouteObserved => 1 << 0,
        MeaningAtom::RouteMissing => 1 << 1,
        MeaningAtom::RepositoryLocalTargetPresent => 1 << 2,
        MeaningAtom::RepositoryLocalTargetMissing => 1 << 3,
        MeaningAtom::ReadmeMentionsRoute => 1 << 4,
        MeaningAtom::StructuredFilePresent => 1 << 5,
        MeaningAtom::AutomationConfigured => 1 << 6,
        MeaningAtom::HumanReviewRequired => 1 << 7,
        MeaningAtom::PatchPreviewOnly => 1 << 8,
        MeaningAtom::CalibrationCandidate => 1 << 9,
        MeaningAtom::ContentSlotObserved => 1 << 10,
        MeaningAtom::ExpectedOutputDocumented => 1 << 11,
        MeaningAtom::StructuralParallelCandidate => 1 << 12,
    }
}

const fn boundary_bit(boundary: ClaimBoundaryKind) -> u16 {
    match boundary {
        ClaimBoundaryKind::NotPopularityGuarantee => 1 << 0,
        ClaimBoundaryKind::NotTrustGuarantee => 1 << 1,
        ClaimBoundaryKind::NotSecurityGuarantee => 1 << 2,
        ClaimBoundaryKind::NotQualityGuarantee => 1 << 3,
        ClaimBoundaryKind::NotLegalFitnessGuarantee => 1 << 4,
        ClaimBoundaryKind::NotLegalAdvice => 1 << 5,
        ClaimBoundaryKind::NotMaintenanceGuarantee => 1 << 6,
        ClaimBoundaryKind::NotRuntimeVerification => 1 << 7,
        ClaimBoundaryKind::NotPublicationReadiness => 1 << 8,
        ClaimBoundaryKind::NotOwnerApproval => 1 << 9,
        ClaimBoundaryKind::NotProductionReadiness => 1 << 10,
        ClaimBoundaryKind::NotAutomaticPolicyAdoption => 1 << 11,
        ClaimBoundaryKind::NotAutomaticWeightAdoption => 1 << 12,
    }
}
