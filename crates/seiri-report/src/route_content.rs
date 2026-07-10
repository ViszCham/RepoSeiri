use seiri_core::{
    ContentObservation, CoverageIndex, CoverageScope, EvidenceFact, EvidenceKernel, EvidenceKind,
    Observation, RouteContentAssessment, RouteContentAtom, RouteContentAtomAssessment, RouteKind,
};

pub(crate) fn build_route_content(
    kernel: &EvidenceKernel,
    coverage: &CoverageIndex,
) -> Vec<RouteContentAssessment> {
    route_content_routes()
        .iter()
        .map(|route| RouteContentAssessment {
            route: *route,
            atoms: RouteContentAtom::ALL
                .into_iter()
                .filter(|atom| atom.route() == *route)
                .map(|atom| RouteContentAtomAssessment {
                    atom,
                    observation: observe_atom(atom, kernel.facts(), coverage),
                })
                .collect(),
        })
        .collect()
}

fn observe_atom(
    atom: RouteContentAtom,
    facts: &[EvidenceFact],
    coverage: &CoverageIndex,
) -> ContentObservation {
    let evidence = facts
        .iter()
        .filter(|fact| fact_matches_atom(fact, atom))
        .map(|fact| fact.id)
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        return ContentObservation::from(
            coverage.observe_absence::<()>(CoverageScope::MarkdownDocuments),
        );
    }
    ContentObservation::from(
        Observation::present((), evidence)
            .expect("matched route content atoms always retain evidence identifiers"),
    )
}

fn fact_matches_atom(fact: &EvidenceFact, atom: RouteContentAtom) -> bool {
    match atom {
        RouteContentAtom::LicenseReference => {
            fact.kind == EvidenceKind::ImportantFile && fact.value == "License"
        }
        RouteContentAtom::AutomationWorkflowReference => {
            fact.kind == EvidenceKind::ImportantFile
                && matches!(fact.value.as_str(), "Workflow" | "SecurityAutomation")
        }
        RouteContentAtom::AutomationStatusSignal => fact.kind == EvidenceKind::MarkdownBadge,
        RouteContentAtom::OwnershipReference => {
            fact.kind == EvidenceKind::ImportantFile && fact.value == "Codeowners"
        }
        RouteContentAtom::HygieneGeneratedArtifactPolicy => {
            fact.kind == EvidenceKind::ImportantFile
                && matches!(
                    fact.value.as_str(),
                    "Gitignore" | "Gitattributes" | "EditorConfig"
                )
        }
        _ if !is_markdown_content_fact(fact.kind) => false,
        _ => contains_any_normalized(&fact.value, atom_markers(atom)),
    }
}

fn is_markdown_content_fact(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::MarkdownHeading | EvidenceKind::MarkdownLink | EvidenceKind::RouteCandidate
    )
}

fn contains_any_normalized(value: &str, markers: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    markers.iter().any(|marker| lower.contains(marker))
}

fn atom_markers(atom: RouteContentAtom) -> &'static [&'static str] {
    match atom {
        RouteContentAtom::IdentityPurpose => &["purpose", "overview", "about", "what is"],
        RouteContentAtom::IdentityAudienceOrScope => &["audience", "scope", "for users"],
        RouteContentAtom::DocsNavigation => &["docs", "documentation", "guide", "manual"],
        RouteContentAtom::DocsConceptGuide => &["concept", "architecture", "design"],
        RouteContentAtom::QuickstartInstallation => &["quickstart", "quick start", "install"],
        RouteContentAtom::QuickstartFirstRun => {
            &["getting started", "first run", "usage", "example"]
        }
        RouteContentAtom::SupportQuestionChannel => &["support", "discussion", "question", "help"],
        RouteContentAtom::SupportResponseExpectation => &["response", "contact", "reply"],
        RouteContentAtom::IntakeReproductionContext => {
            &["reproduce", "reproduction", "environment", "version"]
        }
        RouteContentAtom::IntakeSecurityRedirect => &["security", "vulnerability", "disclosure"],
        RouteContentAtom::ContributingDevelopmentSetup => {
            &["contributing", "development", "develop"]
        }
        RouteContentAtom::ContributingValidationCommand => &["test", "check", "lint", "clippy"],
        RouteContentAtom::SecurityDisclosureChannel => &["report", "disclosure", "vulnerability"],
        RouteContentAtom::SecurityPolicyScope => &["security policy", "security scope"],
        RouteContentAtom::ReleaseChangeHistory => &["changelog", "release notes", "changes"],
        RouteContentAtom::ReleaseCompatibilityNotes => {
            &["compatibility", "breaking change", "migration"]
        }
        RouteContentAtom::LifecycleMaintenanceStatus => {
            &["maintenance", "maintained", "support status"]
        }
        RouteContentAtom::LifecycleDeprecationPlan => &["deprecation", "deprecated", "end of life"],
        RouteContentAtom::GovernanceDecisionProcess => {
            &["governance", "decision", "rfc", "proposal"]
        }
        RouteContentAtom::GovernanceMaintainerRole => &["maintainer", "steward", "owner"],
        RouteContentAtom::LicenseReference => &[],
        RouteContentAtom::LicenseUsageTerms => &["license", "copyright", "permission"],
        RouteContentAtom::AutomationWorkflowReference => &[],
        RouteContentAtom::AutomationStatusSignal => &[],
        RouteContentAtom::OwnershipReference => &[],
        RouteContentAtom::OwnershipCriticalPath => &["codeowners", "critical path", "ownership"],
        RouteContentAtom::HygieneGeneratedArtifactPolicy => &[],
        RouteContentAtom::HygieneFormattingPolicy => {
            &["format", "formatting", "style", "editorconfig"]
        }
    }
}

fn route_content_routes() -> &'static [RouteKind] {
    &[
        RouteKind::Identity,
        RouteKind::Docs,
        RouteKind::Quickstart,
        RouteKind::Support,
        RouteKind::Intake,
        RouteKind::Contributing,
        RouteKind::Security,
        RouteKind::Release,
        RouteKind::Lifecycle,
        RouteKind::Governance,
        RouteKind::License,
        RouteKind::Automation,
        RouteKind::Ownership,
        RouteKind::Hygiene,
    ]
}
