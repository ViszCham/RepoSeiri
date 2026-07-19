use crate::{
    ContentClaim, EvidenceId, EvidenceKernel, Finding, RepositoryAnalysis, ReviewPriorityReport,
    RouteAssessment, RouteContentReport,
};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy)]
pub struct AnalysisCoreView<'a> {
    evidence: &'a EvidenceKernel,
    routes: &'a [RouteAssessment],
    content: &'a RouteContentReport,
    reviews: &'a ReviewPriorityReport,
}

impl<'a> AnalysisCoreView<'a> {
    #[must_use]
    pub const fn evidence(self) -> &'a EvidenceKernel {
        self.evidence
    }

    #[must_use]
    pub const fn routes(self) -> &'a [RouteAssessment] {
        self.routes
    }

    #[must_use]
    pub const fn content(self) -> &'a RouteContentReport {
        self.content
    }

    #[must_use]
    pub const fn reviews(self) -> &'a ReviewPriorityReport {
        self.reviews
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisIntegrityError {
    pub owner: &'static str,
    pub owner_id: String,
    pub evidence_id: EvidenceId,
}

impl Display for AnalysisIntegrityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} '{}' references unknown evidence {}",
            self.owner, self.owner_id, self.evidence_id
        )
    }
}

impl std::error::Error for AnalysisIntegrityError {}

impl RepositoryAnalysis {
    #[must_use]
    pub fn core(&self) -> AnalysisCoreView<'_> {
        AnalysisCoreView {
            evidence: &self.evidence_kernel,
            routes: &self.route_assessments,
            content: &self.route_content,
            reviews: &self.review_priority,
        }
    }

    pub fn validate_derived_views(&self) -> Result<(), AnalysisIntegrityError> {
        for assessment in &self.route_assessments {
            validate_ids(
                &self.evidence_kernel,
                "route assessment",
                assessment.route().slug(),
                &assessment.summary_evidence_ids(),
            )?;
        }
        for pattern in &self.pattern_matches {
            validate_ids(
                &self.evidence_kernel,
                "pattern match",
                &pattern.id,
                &pattern.evidence_ids,
            )?;
        }
        for priority in &self.missing_route_priority.priorities {
            validate_ids(
                &self.evidence_kernel,
                "missing route priority",
                priority.route.slug(),
                &priority.evidence_ids,
            )?;
        }
        for priority in &self.review_priority.priorities {
            validate_ids(
                &self.evidence_kernel,
                "review priority",
                &priority.rank.to_string(),
                &priority.evidence_ids,
            )?;
        }
        validate_claims(&self.evidence_kernel, &self.claims)?;
        validate_findings(&self.evidence_kernel, &self.findings)?;
        if let Some(baseline) = &self.baseline {
            for rule in &baseline.rules {
                validate_ids(
                    &self.evidence_kernel,
                    "baseline rule",
                    &rule.rule_id,
                    &rule.evidence_ids,
                )?;
            }
        }
        if let Some(profile) = &self.profile {
            for rule in &profile.rules {
                validate_ids(
                    &self.evidence_kernel,
                    "profile rule",
                    &rule.rule_id,
                    &rule.evidence_ids,
                )?;
            }
        }
        Ok(())
    }
}

fn validate_claims(
    kernel: &EvidenceKernel,
    claims: &[ContentClaim],
) -> Result<(), AnalysisIntegrityError> {
    for claim in claims {
        validate_ids(kernel, "content claim", claim.id(), claim.evidence_ids())?;
    }
    Ok(())
}

fn validate_findings(
    kernel: &EvidenceKernel,
    findings: &[Finding],
) -> Result<(), AnalysisIntegrityError> {
    for finding in findings {
        validate_ids(kernel, "finding", &finding.id, &finding.evidence_ids)?;
    }
    Ok(())
}

fn validate_ids(
    kernel: &EvidenceKernel,
    owner: &'static str,
    owner_id: &str,
    evidence_ids: &[EvidenceId],
) -> Result<(), AnalysisIntegrityError> {
    if let Some(evidence_id) = evidence_ids
        .iter()
        .copied()
        .find(|id| !kernel.contains(*id))
    {
        return Err(AnalysisIntegrityError {
            owner,
            owner_id: owner_id.to_string(),
            evidence_id,
        });
    }
    Ok(())
}
