#![forbid(unsafe_code)]

use seiri_core::{
    AddExistingRouteLink, ClaimStrength, ExistingTargetId, GateKind, PatchAnalysisRun,
    PatchBaseDigest, PatchDecisionBasis, PatchHold, PatchHoldReason, PatchPlan, PatchProposal,
    PatchProposalBinding, PatchProposalDecision, PatchTextEdit, RepositoryAnalysis, RouteKind,
    RouteTargetRole, TextDocumentBase, TextEditSpan, TextEncoding,
};
use std::fs;
use std::path::{Component, Path};

const PATCH_ROUTES: &[RouteKind] = &[
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
];

const PLANNER_SEMANTIC_REVISION: &str = "seiri.patch-planner.v3";

/// Produces bound, dry-run README links to targets that already exist locally.
#[must_use]
pub fn plan_patches(analysis: &RepositoryAnalysis) -> PatchPlan {
    let mut report = PatchPlan::default();
    let Some(readme) = analysis.readme_document.as_ref() else {
        hold_all(analysis, &mut report, PatchHoldReason::MissingReadme);
        return report;
    };
    let Some(document_id) = analysis
        .document_index
        .entries()
        .iter()
        .find(|entry| entry.path == readme.path())
        .and_then(|entry| entry.document_id)
    else {
        hold_all(analysis, &mut report, PatchHoldReason::MissingReadme);
        return report;
    };
    let current = match read_current_document_bytes(analysis, readme.path()) {
        Ok(bytes) => bytes,
        Err(_) => {
            hold_all(analysis, &mut report, PatchHoldReason::StaleBase);
            return report;
        }
    };
    let base = TextDocumentBase::from_bytes(&current);
    if base != *readme.base() || base.encoding() == TextEncoding::Unknown {
        hold_all(
            analysis,
            &mut report,
            if base.encoding() == TextEncoding::Unknown {
                PatchHoldReason::UnsupportedEncoding
            } else {
                PatchHoldReason::StaleBase
            },
        );
        return report;
    }

    let run_digest = seiri_delta::portable_snapshot(analysis)
        .map(|portable| PatchBaseDigest::from_bytes(portable.digest.routes.to_string().as_bytes()))
        .unwrap_or_else(|_| PatchBaseDigest::from_bytes(analysis.schema_version.as_bytes()));
    let analysis_run = PatchAnalysisRun::new(format!("patch-plan-{run_digest}"), run_digest);
    let pair = analysis
        .route_content
        .structural_pairs
        .iter()
        .find(|pair| pair.document_path == readme.path());

    for (ordinal, route) in PATCH_ROUTES.iter().copied().enumerate() {
        if readme_has_route(analysis, route) {
            continue;
        }
        let Some(target_path) = existing_target(analysis, route) else {
            report.held.push(PatchHold {
                route,
                target_path: None,
                reason: PatchHoldReason::NoExistingTarget,
                decision_basis: decision_basis(analysis, route, GateKind::Guarded),
            });
            continue;
        };
        if analysis
            .document_consistency
            .conflicts
            .iter()
            .any(|conflict| conflict.route == route)
        {
            report.held.push(PatchHold {
                route,
                target_path: Some(target_path.to_string()),
                reason: PatchHoldReason::CanonicalConflict,
                decision_basis: decision_basis(analysis, route, GateKind::Manual),
            });
            continue;
        }
        if analysis
            .document_consistency
            .relations
            .iter()
            .any(|relation| {
                relation.route == route && relation.relation == seiri_core::TargetRelation::Unknown
            })
        {
            report.held.push(PatchHold {
                route,
                target_path: Some(target_path.to_string()),
                reason: PatchHoldReason::UnknownTargetRelation,
                decision_basis: decision_basis(analysis, route, GateKind::Manual),
            });
            continue;
        }

        let paired_language = pair.is_some();
        let spans = match insertion_spans(pair, &current) {
            Some(spans) => spans,
            None => {
                report.held.push(PatchHold {
                    route,
                    target_path: Some(target_path.to_string()),
                    reason: PatchHoldReason::PairedLanguageIncomplete,
                    decision_basis: decision_basis(analysis, route, GateKind::Manual),
                });
                continue;
            }
        };
        let eol = base.line_ending().sequence().unwrap_or("\n");
        let label = route_label(route);
        let edits = spans
            .iter()
            .enumerate()
            .map(|(index, offset)| {
                PatchTextEdit::literal(
                    format!("patch-edit-{}-{}", ordinal + 1, index + 1),
                    TextEditSpan::insertion(*offset),
                    format!("{eol}- [{label}]({target_path}){eol}"),
                )
            })
            .collect::<Vec<_>>();
        let proposal = PatchProposal::new(
            format!("patch-proposal-{}", ordinal + 1),
            readme.path(),
            base.clone(),
            edits,
        );
        if proposal.preflight_against(&current).decision != PatchProposalDecision::Ready {
            report.held.push(PatchHold {
                route,
                target_path: Some(target_path.to_string()),
                reason: PatchHoldReason::StaleAnchor,
                decision_basis: decision_basis(analysis, route, GateKind::Guarded),
            });
            continue;
        }
        let Ok(binding) = PatchProposalBinding::bind(analysis_run.clone(), &proposal, &current)
        else {
            report.held.push(PatchHold {
                route,
                target_path: Some(target_path.to_string()),
                reason: PatchHoldReason::StaleAnchor,
                decision_basis: decision_basis(analysis, route, GateKind::Guarded),
            });
            continue;
        };
        let Some(insertion_anchor) = binding.anchors.first().cloned() else {
            report.held.push(PatchHold {
                route,
                target_path: Some(target_path.to_string()),
                reason: PatchHoldReason::StaleAnchor,
                decision_basis: decision_basis(analysis, route, GateKind::Guarded),
            });
            continue;
        };
        report.operations.push(AddExistingRouteLink {
            route,
            target: ExistingTargetId((ordinal + 1) as u32),
            target_path: target_path.to_string(),
            target_role: RouteTargetRole::Canonical,
            document: document_id,
            insertion_anchor,
            analysis_run: analysis_run.clone(),
            proposal,
            binding,
            paired_language,
            decision_basis: decision_basis(analysis, route, GateKind::Safe),
        });
    }
    report.operations.sort_by_key(|operation| operation.route);
    report.held.sort_by_key(|item| item.route);
    report
}

fn hold_all(analysis: &RepositoryAnalysis, report: &mut PatchPlan, reason: PatchHoldReason) {
    report
        .held
        .extend(PATCH_ROUTES.iter().copied().map(|route| PatchHold {
            route,
            target_path: None,
            reason,
            decision_basis: decision_basis(analysis, route, GateKind::Manual),
        }));
}

fn decision_basis(
    analysis: &RepositoryAnalysis,
    route: RouteKind,
    gate: GateKind,
) -> PatchDecisionBasis {
    let mut claims = analysis
        .claims
        .iter()
        .filter(|claim| claim.route() == route)
        .collect::<Vec<_>>();
    claims.sort_by_key(|claim| {
        (
            claim.strength() != ClaimStrength::Observed,
            claim.id().clone(),
        )
    });
    let claim_ids = claims
        .iter()
        .map(|claim| claim.id().clone())
        .collect::<Vec<_>>();
    let mut evidence_ids = claims
        .iter()
        .flat_map(|claim| claim.evidence_ids().iter().copied())
        .collect::<Vec<_>>();
    evidence_ids.sort_unstable();
    evidence_ids.dedup();
    let evidence_fingerprints =
        seiri_delta::evidence_fingerprints_for_ids(&analysis.evidence_kernel, &evidence_ids)
            .unwrap_or_default();
    let priority_rank = analysis
        .missing_route_priority
        .priorities
        .iter()
        .position(|priority| priority.route == route)
        .map(|index| index + 1);
    PatchDecisionBasis {
        gate,
        priority_rank,
        claim_ids,
        evidence_fingerprints,
        claim_semantic_revision: seiri_core::CLAIM_SEMANTIC_REVISION.to_string(),
        planner_semantic_revision: PLANNER_SEMANTIC_REVISION.to_string(),
    }
}

fn readme_has_route(analysis: &RepositoryAnalysis, route: RouteKind) -> bool {
    analysis
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == route)
        .is_some_and(|assessment| assessment.readme().routing().is_present())
}

fn existing_target(analysis: &RepositoryAnalysis, route: RouteKind) -> Option<&str> {
    target_candidates(route).iter().copied().find(|candidate| {
        let canonical_candidate = candidate.trim_end_matches('/');
        is_safe_relative(candidate)
            && analysis.files.iter().any(|record| {
                record.path == canonical_candidate
                    || (candidate.ends_with('/') && record.path.starts_with(candidate))
            })
    })
}

fn target_candidates(route: RouteKind) -> &'static [&'static str] {
    match route {
        RouteKind::Docs => &["docs/", "docs/README.md"],
        RouteKind::Quickstart => &["docs/getting-started.md", "docs/quickstart.md"],
        RouteKind::Support => &["SUPPORT.md"],
        RouteKind::Intake => &[".github/ISSUE_TEMPLATE/", "SUPPORT.md"],
        RouteKind::Contributing => &["CONTRIBUTING.md"],
        RouteKind::Security => &["SECURITY.md"],
        RouteKind::Release => &["CHANGELOG.md", "docs/releases.md"],
        RouteKind::Lifecycle => &["docs/releases.md", "CHANGELOG.md"],
        RouteKind::Governance => &["GOVERNANCE.md"],
        RouteKind::License => &["LICENSE", "LICENSE.md"],
        RouteKind::Automation => &[".github/workflows/"],
        RouteKind::Ownership => &[".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"],
        RouteKind::Hygiene => &[".gitignore", ".gitattributes", ".editorconfig"],
        RouteKind::Identity | RouteKind::Unknown => &[],
    }
}

fn is_safe_relative(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn insertion_spans(
    pair: Option<&seiri_core::BilingualStructuralPair>,
    source: &[u8],
) -> Option<Vec<usize>> {
    match pair {
        None => Some(vec![source.len()]),
        Some(pair) => {
            let mut offsets = vec![pair.left_heading.byte_end, pair.right_heading.byte_end];
            offsets.sort_unstable();
            offsets.dedup();
            if offsets.len() != 2
                || offsets.iter().any(|offset| {
                    *offset > source.len()
                        || std::str::from_utf8(source)
                            .map_or(true, |text| !text.is_char_boundary(*offset))
                })
            {
                None
            } else {
                Some(offsets)
            }
        }
    }
}

fn route_label(route: RouteKind) -> &'static str {
    match route {
        RouteKind::Docs => "Documentation",
        RouteKind::Quickstart => "Quickstart",
        RouteKind::Support => "Support",
        RouteKind::Intake => "Issue intake",
        RouteKind::Contributing => "Contributing",
        RouteKind::Security => "Security policy",
        RouteKind::Release => "Changes and releases",
        RouteKind::Lifecycle => "Lifecycle",
        RouteKind::Governance => "Governance",
        RouteKind::License => "License",
        RouteKind::Automation => "Automation",
        RouteKind::Ownership => "Ownership",
        RouteKind::Hygiene => "Repository hygiene",
        RouteKind::Identity | RouteKind::Unknown => "Repository information",
    }
}

fn read_current_document_bytes(
    analysis: &RepositoryAnalysis,
    relative_path: &str,
) -> Result<Vec<u8>, String> {
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err("Planner refused a non-repository-relative document path.".to_string());
    }

    let root = seiri_fs::RepositoryRoot::resolve(Path::new(&analysis.repo_root))
        .map_err(|error| format!("Repository root could not be resolved: {error}"))?;
    let candidate = root.as_path().join(relative);
    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("Current document could not be resolved: {error}"))?;
    if !canonical.starts_with(root.as_path()) {
        return Err(
            "Planner refused a document whose resolved path escapes the repository root."
                .to_string(),
        );
    }
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Current document metadata could not be read: {error}"))?;
    if !metadata.is_file() {
        return Err(
            "Planner requires the current document target to be a regular file.".to_string(),
        );
    }
    fs::read(&canonical).map_err(|error| format!("Current document could not be read: {error}"))
}
