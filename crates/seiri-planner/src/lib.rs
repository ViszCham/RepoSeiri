#![forbid(unsafe_code)]

use seiri_core::{
    AddExistingRouteLink, ClaimStrength, ExistingTargetId, GateKind, PatchAnalysisRun,
    PatchBaseDigest, PatchDecisionBasis, PatchHold, PatchHoldReason, PatchPlan, PatchProposal,
    PatchProposalBinding, PatchProposalDecision, PatchTextEdit, RepositoryAnalysis, RouteKind,
    RouteTargetRole, TextDocumentBase, TextEditSpan, TextEncoding,
};
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

const PLANNER_SEMANTIC_REVISION: &str = "seiri.patch-planner.v5";

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
    let current = match analysis.source_store().get(readme.path()) {
        Some(source) => source.bytes(),
        None => {
            hold_all(analysis, &mut report, PatchHoldReason::StaleBase);
            return report;
        }
    };
    let base = TextDocumentBase::from_bytes(current);
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
    let topology = analysis.language_topology().for_path(readme.path());

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

        let insertion_points = match insertion_points(topology, current) {
            Some(points) => points,
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
        let paired_language = insertion_points.len() == 2;
        let eol = base.line_ending().sequence().unwrap_or("\n");
        let edits = insertion_points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                let label = route_label(route, point.language);
                PatchTextEdit::literal(
                    format!("patch-edit-{}-{}", ordinal + 1, index + 1),
                    TextEditSpan::insertion(point.offset),
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
        if proposal.preflight_against(current).decision != PatchProposalDecision::Ready {
            report.held.push(PatchHold {
                route,
                target_path: Some(target_path.to_string()),
                reason: PatchHoldReason::StaleAnchor,
                decision_basis: decision_basis(analysis, route, GateKind::Guarded),
            });
            continue;
        }
        let Ok(binding) = PatchProposalBinding::bind(analysis_run.clone(), &proposal, current)
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
        seiri_delta::evidence_fingerprints_for_ids(analysis, &evidence_ids).unwrap_or_default();
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
        source_session_digest: analysis.analysis_configuration.source_session_digest,
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
    route.target_candidates().iter().copied().find(|candidate| {
        let canonical_candidate = candidate.trim_end_matches('/');
        is_safe_relative(candidate)
            && analysis.files.iter().any(|record| {
                record.path == canonical_candidate
                    || (candidate.ends_with('/') && record.path.starts_with(candidate))
            })
    })
}

fn is_safe_relative(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

#[derive(Debug, Clone, Copy)]
struct InsertionPoint {
    offset: usize,
    language: seiri_core::DocumentLanguage,
}

fn insertion_points(
    topology: Option<seiri_core::LanguageTopology>,
    source: &[u8],
) -> Option<Vec<InsertionPoint>> {
    let points = match topology? {
        seiri_core::LanguageTopology::Monolingual(language) => vec![InsertionPoint {
            offset: source.len(),
            language,
        }],
        seiri_core::LanguageTopology::Parallel {
            japanese_insertion,
            english_insertion,
        } => vec![
            InsertionPoint {
                offset: japanese_insertion,
                language: seiri_core::DocumentLanguage::Japanese,
            },
            InsertionPoint {
                offset: english_insertion,
                language: seiri_core::DocumentLanguage::English,
            },
        ],
        seiri_core::LanguageTopology::Ambiguous => return None,
    };
    let text = std::str::from_utf8(source).ok()?;
    if points.len() == 2 && points[0].offset == points[1].offset {
        return None;
    }
    if points
        .iter()
        .any(|point| point.offset > source.len() || !text.is_char_boundary(point.offset))
    {
        None
    } else {
        Some(points)
    }
}

fn route_label(route: RouteKind, language: seiri_core::DocumentLanguage) -> &'static str {
    route.label(language)
}
