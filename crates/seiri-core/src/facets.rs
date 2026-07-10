use crate::{CoverageIndex, CoverageScope, EvidenceFact, EvidenceId, EvidenceKind, Observation};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// A repository can expose several facets at once; this is not a type classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryFacet {
    Package,
    Binary,
    Infrastructure,
    Documentation,
    Research,
    Template,
    Product,
}

impl RepositoryFacet {
    pub const ALL: [Self; 7] = [
        Self::Package,
        Self::Binary,
        Self::Infrastructure,
        Self::Documentation,
        Self::Research,
        Self::Template,
        Self::Product,
    ];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Binary => "binary",
            Self::Infrastructure => "infrastructure",
            Self::Documentation => "documentation",
            Self::Research => "research",
            Self::Template => "template",
            Self::Product => "product",
        }
    }
}

impl Display for RepositoryFacet {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.slug())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetAssessment {
    pub facet: RepositoryFacet,
    pub observation: Observation<()>,
}

impl FacetAssessment {
    #[must_use]
    pub fn evidence_ids(&self) -> Option<&[EvidenceId]> {
        match &self.observation {
            Observation::Present { evidence, .. }
            | Observation::Conflict {
                alternatives: evidence,
            } => Some(evidence.as_slice()),
            Observation::Absent { .. } | Observation::Unknown(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetReport {
    pub facets: Vec<FacetAssessment>,
    pub boundary: String,
}

impl FacetReport {
    pub fn try_new(facets: Vec<FacetAssessment>) -> Result<Self, FacetReportError> {
        if facets.len() != RepositoryFacet::ALL.len() {
            return Err(FacetReportError::FacetCountMismatch {
                expected: RepositoryFacet::ALL.len(),
                actual: facets.len(),
            });
        }
        for (expected, assessment) in RepositoryFacet::ALL.iter().zip(&facets) {
            if assessment.facet != *expected {
                return Err(FacetReportError::NonCanonicalFacetOrder);
            }
        }
        Ok(Self {
            facets,
            boundary: "Facets are coexisting, evidence-backed repository observations. They do not select a repository type, assert exclusivity, or establish suitability, quality, security, or policy compliance.".to_string(),
        })
    }

    #[must_use]
    pub fn assessment(&self, facet: RepositoryFacet) -> Option<&FacetAssessment> {
        self.facets
            .iter()
            .find(|assessment| assessment.facet == facet)
    }

    #[must_use]
    pub fn observed_evidence(&self, facet: RepositoryFacet) -> Option<&[EvidenceId]> {
        self.assessment(facet)
            .and_then(FacetAssessment::evidence_ids)
    }
}

impl Default for FacetReport {
    fn default() -> Self {
        let coverage = CoverageIndex::default();
        let facets = RepositoryFacet::ALL
            .into_iter()
            .map(|facet| FacetAssessment {
                facet,
                observation: coverage.observe_absence(CoverageScope::RepositoryFiles),
            })
            .collect();
        Self::try_new(facets).expect("complete facet registry is canonical")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetReportError {
    FacetCountMismatch { expected: usize, actual: usize },
    NonCanonicalFacetOrder,
}

impl Display for FacetReportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FacetCountMismatch { expected, actual } => write!(
                formatter,
                "facet report must contain {expected} facets in canonical order, found {actual}"
            ),
            Self::NonCanonicalFacetOrder => {
                formatter.write_str("facet report facets must use canonical complete order")
            }
        }
    }
}

impl std::error::Error for FacetReportError {}

#[must_use]
pub fn facet_evidence_ids(facet: RepositoryFacet, facts: &[EvidenceFact]) -> Vec<EvidenceId> {
    let mut ids = facts
        .iter()
        .filter(|fact| fact_supports_facet(facet, fact))
        .map(|fact| fact.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn fact_supports_facet(facet: RepositoryFacet, fact: &EvidenceFact) -> bool {
    let path = fact
        .path
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let value = fact.value.to_ascii_lowercase();
    match facet {
        RepositoryFacet::Package => {
            (fact.kind == EvidenceKind::ImportantFile && fact.value == "CargoToml")
                || is_package_manifest(&path)
        }
        RepositoryFacet::Binary => {
            is_binary_entrypoint(&path)
                || is_markdown_signal(fact.kind)
                    && contains_any(
                        &value,
                        &["cli", "command line", "command-line", "command-line tool"],
                    )
        }
        RepositoryFacet::Infrastructure => {
            (fact.kind == EvidenceKind::ImportantFile
                && matches!(
                    fact.value.as_str(),
                    "Workflow" | "DependencyBot" | "SecurityAutomation"
                ))
                || has_path_segment(
                    &path,
                    &[
                        "infra",
                        "infrastructure",
                        "terraform",
                        "k8s",
                        "helm",
                        "deploy",
                        "deployments",
                        "ops",
                    ],
                )
        }
        RepositoryFacet::Documentation => {
            (fact.kind == EvidenceKind::ImportantFile && fact.value == "DocsDirectory")
                || path == "docs"
                || path.starts_with("docs/")
        }
        RepositoryFacet::Research => {
            has_path_segment(
                &path,
                &[
                    "research",
                    "paper",
                    "papers",
                    "dataset",
                    "datasets",
                    "notebook",
                    "notebooks",
                    "experiment",
                    "experiments",
                ],
            ) || is_markdown_signal(fact.kind)
                && contains_any(
                    &value,
                    &[
                        "research",
                        "dataset",
                        "experiment",
                        "paper",
                        "reproducibility",
                    ],
                )
        }
        RepositoryFacet::Template => {
            has_path_segment(
                &path,
                &["template", "templates", "cookiecutter", "scaffold"],
            ) || is_markdown_signal(fact.kind)
                && contains_any(
                    &value,
                    &["template", "scaffold", "starter project", "boilerplate"],
                )
        }
        RepositoryFacet::Product => {
            has_path_segment(
                &path,
                &["app", "apps", "web", "frontend", "backend", "product"],
            ) || is_markdown_signal(fact.kind)
                && contains_any(
                    &value,
                    &["product", "application", "end user", "user-facing"],
                )
        }
    }
}

fn is_package_manifest(path: &str) -> bool {
    matches!(
        path,
        "cargo.toml" | "package.json" | "pyproject.toml" | "go.mod"
    )
}

fn is_binary_entrypoint(path: &str) -> bool {
    path == "src/main.rs"
        || path == "main.go"
        || path == "main.py"
        || path.starts_with("src/bin/")
        || path.starts_with("cmd/")
}

fn is_markdown_signal(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::MarkdownHeading | EvidenceKind::MarkdownLink | EvidenceKind::RouteCandidate
    )
}

fn has_path_segment(path: &str, candidates: &[&str]) -> bool {
    path.split('/').any(|segment| candidates.contains(&segment))
}

fn contains_any(value: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| value.contains(marker))
}
