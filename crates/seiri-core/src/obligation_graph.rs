use crate::{
    classify_target_relation, CoverageStatus, DocumentId, EvidenceId, EvidenceSet, Observation,
    RepositoryFacet, RouteKind, RouteTargetRef, RouteTargetRole, SourceSpan, TargetRelation,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalObligation {
    pub id: String,
    pub facet: RepositoryFacet,
    pub route: RouteKind,
    pub reason: EvidenceSet,
    pub observation: Observation<()>,
}

impl ConditionalObligation {
    #[must_use]
    pub fn new(
        facet: RepositoryFacet,
        route: RouteKind,
        reason: EvidenceSet,
        observation: Observation<()>,
    ) -> Self {
        Self {
            id: format!("facet.{}.{}", facet.slug(), route_slug(route)),
            facet,
            route,
            reason,
            observation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentConflictSide {
    pub document: DocumentId,
    pub evidence: EvidenceId,
    pub target: String,
    #[serde(default)]
    pub role: RouteTargetRole,
    #[serde(default)]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentConflict {
    pub id: String,
    pub route: RouteKind,
    pub left: DocumentConflictSide,
    pub right: DocumentConflictSide,
    #[serde(default = "competes_relation")]
    pub relation: TargetRelation,
}

const fn competes_relation() -> TargetRelation {
    TargetRelation::Competes
}

impl DocumentConflict {
    pub fn try_new(
        route: RouteKind,
        mut left: DocumentConflictSide,
        mut right: DocumentConflictSide,
    ) -> Result<Self, DocumentConsistencyError> {
        if left.document == right.document {
            return Err(DocumentConsistencyError::SameDocumentConflict);
        }
        if left.evidence == right.evidence {
            return Err(DocumentConsistencyError::SameEvidenceConflict);
        }
        if left.target.is_empty() || right.target.is_empty() {
            return Err(DocumentConsistencyError::EmptyConflictTarget);
        }
        if left.target == right.target {
            return Err(DocumentConsistencyError::EqualConflictTargets);
        }
        let relation = relation_for_sides(route, &left, &right);
        if relation != TargetRelation::Competes {
            return Err(DocumentConsistencyError::NonCompetingTargets);
        }
        if (right.document, right.evidence) < (left.document, left.evidence) {
            std::mem::swap(&mut left, &mut right);
        }
        let id = format!(
            "document-conflict.{}.{}.{}",
            route_slug(route),
            left.evidence.ordinal(),
            right.evidence.ordinal()
        );
        Ok(Self {
            id,
            route,
            left,
            right,
            relation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentTargetRelation {
    pub id: String,
    pub route: RouteKind,
    pub left: DocumentConflictSide,
    pub right: DocumentConflictSide,
    pub relation: TargetRelation,
}

impl DocumentTargetRelation {
    #[must_use]
    pub fn new(
        route: RouteKind,
        mut left: DocumentConflictSide,
        mut right: DocumentConflictSide,
        relation: TargetRelation,
    ) -> Self {
        if (right.document, right.evidence) < (left.document, left.evidence) {
            std::mem::swap(&mut left, &mut right);
        }
        Self {
            id: format!(
                "document-relation.{}.{}.{}",
                route_slug(route),
                left.evidence.ordinal(),
                right.evidence.ordinal()
            ),
            route,
            left,
            right,
            relation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentConsistencyReport {
    pub obligations: Vec<ConditionalObligation>,
    #[serde(default)]
    pub relations: Vec<DocumentTargetRelation>,
    pub conflicts: Vec<DocumentConflict>,
    pub conflict_coverage: CoverageStatus,
    pub boundary: String,
}

impl DocumentConsistencyReport {
    pub fn try_new(
        obligations: Vec<ConditionalObligation>,
        relations: Vec<DocumentTargetRelation>,
        conflicts: Vec<DocumentConflict>,
        conflict_coverage: CoverageStatus,
    ) -> Result<Self, DocumentConsistencyError> {
        validate_unique_ordered(
            obligations.iter().map(|obligation| obligation.id.as_str()),
            "conditional obligations",
        )?;
        validate_unique_ordered(
            relations.iter().map(|relation| relation.id.as_str()),
            "document target relations",
        )?;
        validate_unique_ordered(
            conflicts.iter().map(|conflict| conflict.id.as_str()),
            "document conflicts",
        )?;
        Ok(Self {
            obligations,
            relations,
            conflicts,
            conflict_coverage,
            boundary: "Each conditional obligation is enabled only by observed facet evidence. A missing obligation observation requires complete repository coverage. Target relations distinguish equivalent, refining, shared-hub, competing, and unknown links. Only Competes becomes a document conflict; Unknown does not mean consistent. Conflict coverage becomes partial when the bounded graph reaches its candidate or pair limit.".to_string(),
        })
    }
}

impl Default for DocumentConsistencyReport {
    fn default() -> Self {
        Self::try_new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            CoverageStatus::NotRequested,
        )
        .expect("empty document consistency report is valid")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentConsistencyError {
    SameDocumentConflict,
    SameEvidenceConflict,
    EmptyConflictTarget,
    EqualConflictTargets,
    NonCompetingTargets,
    NonCanonicalOrder(&'static str),
}

impl Display for DocumentConsistencyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SameDocumentConflict => formatter
                .write_str("cross-document conflict sides must refer to distinct documents"),
            Self::SameEvidenceConflict => formatter
                .write_str("cross-document conflict sides must refer to distinct evidence ids"),
            Self::EmptyConflictTarget => {
                formatter.write_str("document conflict targets must not be empty")
            }
            Self::EqualConflictTargets => {
                formatter.write_str("document conflict targets must differ")
            }
            Self::NonCompetingTargets => formatter
                .write_str("document conflicts require two competing canonical route targets"),
            Self::NonCanonicalOrder(kind) => write!(
                formatter,
                "{kind} must be strictly ordered by deterministic identifier"
            ),
        }
    }
}

fn relation_for_sides(
    route: RouteKind,
    left: &DocumentConflictSide,
    right: &DocumentConflictSide,
) -> TargetRelation {
    let Some(left_span) = left.span else {
        return TargetRelation::Unknown;
    };
    let Some(right_span) = right.span else {
        return TargetRelation::Unknown;
    };
    classify_target_relation(
        &RouteTargetRef {
            route,
            document: left.document,
            evidence: left.evidence,
            span: left_span,
            role: left.role,
            normalized_target: left.target.clone(),
        },
        &RouteTargetRef {
            route,
            document: right.document,
            evidence: right.evidence,
            span: right_span,
            role: right.role,
            normalized_target: right.target.clone(),
        },
    )
}

impl std::error::Error for DocumentConsistencyError {}

fn validate_unique_ordered<'a>(
    mut ids: impl Iterator<Item = &'a str>,
    kind: &'static str,
) -> Result<(), DocumentConsistencyError> {
    let Some(mut previous) = ids.next() else {
        return Ok(());
    };
    for current in ids {
        if previous >= current {
            return Err(DocumentConsistencyError::NonCanonicalOrder(kind));
        }
        previous = current;
    }
    Ok(())
}

fn route_slug(route: RouteKind) -> String {
    format!("{route:?}").to_ascii_lowercase()
}
