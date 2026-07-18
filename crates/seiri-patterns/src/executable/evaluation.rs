use super::ExecutablePatternPack;
use crate::{common_registry, PredicateContext};
use seiri_core::{
    stable_id, Finding, GateKind, Observation, PatternExtensionEvaluation, PatternExtensionReport,
    PatternExtensionState, PatternExtensionStatus, PatternMatch, PatternOutcome,
    PatternPackProvenance, Recommendation, RepositoryAnalysis, RouteKind, Severity,
};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableOverlayEvaluation {
    pub report: PatternExtensionReport,
    pub pattern_matches: Vec<PatternMatch>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternExtensionError {
    CommonPatternConflict(String),
}

impl Display for PatternExtensionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommonPatternConflict(id) => {
                write!(
                    formatter,
                    "executable pattern `{id}` conflicts with the common registry"
                )
            }
        }
    }
}

impl std::error::Error for PatternExtensionError {}

pub fn evaluate_executable_overlay(
    snapshot: &RepositoryAnalysis,
    pack: &ExecutablePatternPack,
) -> Result<ExecutableOverlayEvaluation, PatternExtensionError> {
    let common_ids = common_registry()
        .definitions()
        .iter()
        .map(|definition| definition.id)
        .collect::<BTreeSet<_>>();
    if let Some(conflict) = pack
        .definitions()
        .iter()
        .find(|definition| common_ids.contains(definition.id.as_str()))
    {
        return Err(PatternExtensionError::CommonPatternConflict(
            conflict.id.clone(),
        ));
    }

    let mut evaluations = Vec::with_capacity(pack.definitions().len());
    let mut pattern_matches = Vec::new();
    let mut findings = Vec::new();
    for definition in pack.definitions() {
        let route = route_for_group(definition.group);
        if !definition.enabled {
            evaluations.push(PatternExtensionEvaluation {
                pattern_id: definition.id.clone(),
                group: definition.group,
                route,
                state: PatternExtensionState::Disabled,
                evidence_ids: Vec::new(),
                boundaries: definition.boundaries.clone(),
            });
            continue;
        }

        let observation = definition
            .predicate
            .evaluate(PredicateContext::from_snapshot(snapshot));
        let (state, evidence_ids, outcome) = match observation {
            Observation::Present { evidence, .. } => (
                PatternExtensionState::Present,
                evidence.as_slice().to_vec(),
                Some(PatternOutcome::Present),
            ),
            Observation::Absent { .. } => (
                PatternExtensionState::Absent,
                Vec::new(),
                Some(PatternOutcome::Missing),
            ),
            Observation::Unknown(reason) => {
                (PatternExtensionState::Unknown(reason), Vec::new(), None)
            }
            Observation::Conflict { alternatives } => (
                PatternExtensionState::Conflict,
                alternatives.as_slice().to_vec(),
                None,
            ),
        };
        if let Some(outcome) = outcome {
            pattern_matches.push(PatternMatch {
                id: stable_id("extension-pattern-match", pattern_matches.len() + 1),
                pattern_id: definition.id.clone(),
                title: definition.id.clone(),
                route: Some(route),
                outcome,
                evidence_ids: evidence_ids.clone(),
                basis: format!(
                    "validated data-only predicate from executable pack {}@{}",
                    pack.id(),
                    pack.version()
                ),
            });
            if outcome == PatternOutcome::Missing {
                let finding_number = findings.len() + 1;
                findings.push(Finding {
                    id: stable_id("extension-finding", finding_number),
                    severity: Severity::Info,
                    title: "Executable pattern candidate was not observed".to_string(),
                    message: format!(
                        "The explicitly selected data-only predicate `{}` was absent under complete predicate coverage.",
                        definition.id
                    ),
                    evidence_ids: Vec::new(),
                    recommendation: Some(Recommendation {
                        id: stable_id("extension-rec", finding_number),
                        gate: GateKind::Manual,
                        title: "Review candidate pattern gap".to_string(),
                        message: "Review the candidate against repository intent; RepoSeiri does not automatically adopt pack policy.".to_string(),
                    }),
                });
            }
        }
        evaluations.push(PatternExtensionEvaluation {
            pattern_id: definition.id.clone(),
            group: definition.group,
            route,
            state,
            evidence_ids,
            boundaries: definition.boundaries.clone(),
        });
    }

    Ok(ExecutableOverlayEvaluation {
        report: PatternExtensionReport {
            status: PatternExtensionStatus::Applied,
            pack: Some(PatternPackProvenance {
                id: pack.id().to_string(),
                version: pack.version().to_string(),
                fingerprint: pack.fingerprint().to_string(),
            }),
            evaluations,
            ..PatternExtensionReport::default()
        },
        pattern_matches,
        findings,
    })
}

const fn route_for_group(group: seiri_core::PatternGroup) -> RouteKind {
    match group {
        seiri_core::PatternGroup::Idn => RouteKind::Identity,
        seiri_core::PatternGroup::Doc => RouteKind::Docs,
        seiri_core::PatternGroup::Qst => RouteKind::Quickstart,
        seiri_core::PatternGroup::Sup => RouteKind::Support,
        seiri_core::PatternGroup::Sec => RouteKind::Security,
        seiri_core::PatternGroup::Ctr => RouteKind::Contributing,
        seiri_core::PatternGroup::Int => RouteKind::Intake,
        seiri_core::PatternGroup::Aut => RouteKind::Automation,
        seiri_core::PatternGroup::Rel => RouteKind::Release,
        seiri_core::PatternGroup::Own => RouteKind::Ownership,
        seiri_core::PatternGroup::Gov => RouteKind::Governance,
        seiri_core::PatternGroup::Hyg => RouteKind::Hygiene,
        seiri_core::PatternGroup::Lif => RouteKind::Lifecycle,
    }
}
