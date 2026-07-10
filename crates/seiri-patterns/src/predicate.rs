use seiri_core::{
    ContentObservation, CoverageIndex, CoverageScope, DocumentIndex, DocumentRole, EvidenceAtom,
    EvidenceFactV2, EvidenceId, Observation, RepoSnapshot, RouteContentAssessment,
    RouteContentAtom, UnknownReason,
};
use std::fmt::{Display, Formatter};

pub const MAX_PREDICATE_ATOMS: usize = 128;
pub const MAX_PREDICATE_OPERATIONS: usize = 256;
pub const MAX_PREDICATE_STACK: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateAtom {
    Evidence(EvidenceAtom),
    DocumentRole(DocumentRole),
    RouteContent(RouteContentAtom),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateInstruction {
    PushAtom(u8),
    All(u8),
    Any(u8),
    AtLeast { arity: u8, minimum: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateProgram {
    atoms: Box<[PredicateAtom]>,
    instructions: Box<[PredicateInstruction]>,
}

impl PredicateProgram {
    pub fn try_new(
        atoms: Vec<PredicateAtom>,
        instructions: Vec<PredicateInstruction>,
    ) -> Result<Self, PredicateProgramError> {
        let program = Self {
            atoms: atoms.into_boxed_slice(),
            instructions: instructions.into_boxed_slice(),
        };
        program.validate()?;
        Ok(program)
    }

    #[must_use]
    pub fn atoms(&self) -> &[PredicateAtom] {
        &self.atoms
    }

    #[must_use]
    pub fn instructions(&self) -> &[PredicateInstruction] {
        &self.instructions
    }

    pub fn validate(&self) -> Result<(), PredicateProgramError> {
        if self.atoms.len() > MAX_PREDICATE_ATOMS {
            return Err(PredicateProgramError::TooManyAtoms {
                actual: self.atoms.len(),
                limit: MAX_PREDICATE_ATOMS,
            });
        }
        if self.instructions.len() > MAX_PREDICATE_OPERATIONS {
            return Err(PredicateProgramError::TooManyInstructions {
                actual: self.instructions.len(),
                limit: MAX_PREDICATE_OPERATIONS,
            });
        }

        let mut depth = 0usize;
        for (index, instruction) in self.instructions.iter().enumerate() {
            match *instruction {
                PredicateInstruction::PushAtom(atom) => {
                    if usize::from(atom) >= self.atoms.len() {
                        return Err(PredicateProgramError::UnknownAtom { index, atom });
                    }
                    depth += 1;
                    if depth > MAX_PREDICATE_STACK {
                        return Err(PredicateProgramError::StackLimitExceeded {
                            index,
                            limit: MAX_PREDICATE_STACK,
                        });
                    }
                }
                PredicateInstruction::All(arity) | PredicateInstruction::Any(arity) => {
                    validate_arity(index, arity)?;
                    if depth < usize::from(arity) {
                        return Err(PredicateProgramError::StackUnderflow { index });
                    }
                    depth = depth - usize::from(arity) + 1;
                }
                PredicateInstruction::AtLeast { arity, minimum } => {
                    validate_arity(index, arity)?;
                    if minimum == 0 || minimum > arity {
                        return Err(PredicateProgramError::InvalidThreshold {
                            index,
                            arity,
                            minimum,
                        });
                    }
                    if depth < usize::from(arity) {
                        return Err(PredicateProgramError::StackUnderflow { index });
                    }
                    depth = depth - usize::from(arity) + 1;
                }
            }
        }
        if depth != 1 {
            return Err(PredicateProgramError::FinalStackDepth { depth });
        }
        Ok(())
    }

    #[must_use]
    pub fn evaluate(&self, context: PredicateContext<'_>) -> Observation<()> {
        self.validate()
            .expect("predicate programs are validated before evaluation");
        let mut stack: [Option<Observation<()>>; MAX_PREDICATE_STACK] =
            std::array::from_fn(|_| None);
        let mut depth = 0usize;
        for instruction in self.instructions.iter().copied() {
            match instruction {
                PredicateInstruction::PushAtom(index) => {
                    stack[depth] = Some(context.observe(self.atoms[usize::from(index)]));
                    depth += 1;
                }
                PredicateInstruction::All(arity) => {
                    depth = reduce_stack(&mut stack, depth, usize::from(arity), combine_all);
                }
                PredicateInstruction::Any(arity) => {
                    depth = reduce_stack(&mut stack, depth, usize::from(arity), combine_any);
                }
                PredicateInstruction::AtLeast { arity, minimum } => {
                    depth = reduce_stack(&mut stack, depth, usize::from(arity), |values| {
                        combine_at_least(values, usize::from(minimum))
                    });
                }
            }
        }
        stack[0]
            .take()
            .expect("validated predicate program leaves one result")
    }
}

fn validate_arity(index: usize, arity: u8) -> Result<(), PredicateProgramError> {
    if arity < 2 {
        return Err(PredicateProgramError::InvalidArity { index, arity });
    }
    Ok(())
}

fn reduce_stack(
    stack: &mut [Option<Observation<()>>; MAX_PREDICATE_STACK],
    depth: usize,
    arity: usize,
    reducer: impl FnOnce(&[Observation<()>]) -> Observation<()>,
) -> usize {
    let start = depth - arity;
    let values = stack[start..depth]
        .iter_mut()
        .map(|slot| slot.take().expect("validated stack has values"))
        .collect::<Vec<_>>();
    stack[start] = Some(reducer(&values));
    start + 1
}

#[derive(Debug, Clone, Copy)]
pub struct PredicateContext<'a> {
    evidence: &'a [EvidenceFactV2],
    coverage: &'a CoverageIndex,
    document_index: &'a DocumentIndex,
    route_content: &'a [RouteContentAssessment],
}

impl<'a> PredicateContext<'a> {
    #[must_use]
    pub fn from_snapshot(snapshot: &'a RepoSnapshot) -> Self {
        Self {
            evidence: snapshot.evidence_kernel_v2.facts(),
            coverage: &snapshot.coverage,
            document_index: &snapshot.document_index,
            route_content: &snapshot.route_content,
        }
    }

    #[must_use]
    pub const fn new(
        evidence: &'a [EvidenceFactV2],
        coverage: &'a CoverageIndex,
        document_index: &'a DocumentIndex,
        route_content: &'a [RouteContentAssessment],
    ) -> Self {
        Self {
            evidence,
            coverage,
            document_index,
            route_content,
        }
    }

    fn observe(self, atom: PredicateAtom) -> Observation<()> {
        match atom {
            PredicateAtom::Evidence(expected) => {
                observation_for_evidence(self.evidence, |fact| fact.atom == expected, self.coverage)
            }
            PredicateAtom::DocumentRole(role) => observation_for_document_role(
                self.evidence,
                self.document_index,
                self.coverage,
                role,
            ),
            PredicateAtom::RouteContent(atom) => self
                .route_content
                .iter()
                .find(|assessment| assessment.route == atom.route())
                .and_then(|assessment| assessment.observation(atom))
                .map(content_observation_as_observation)
                .unwrap_or(Observation::Unknown(UnknownReason::NotRequested)),
        }
    }
}

fn observation_for_evidence(
    evidence: &[EvidenceFactV2],
    predicate: impl Fn(&EvidenceFactV2) -> bool,
    coverage: &CoverageIndex,
) -> Observation<()> {
    let ids = evidence
        .iter()
        .filter(|fact| predicate(fact))
        .map(|fact| fact.id)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        coverage.observe_absence(CoverageScope::RepositoryFiles)
    } else {
        Observation::present((), ids).expect("matched evidence facts have ids")
    }
}

fn observation_for_document_role(
    evidence: &[EvidenceFactV2],
    document_index: &DocumentIndex,
    coverage: &CoverageIndex,
    role: DocumentRole,
) -> Observation<()> {
    let ids = document_index
        .entries()
        .iter()
        .filter(|entry| entry.role == role)
        .filter_map(|entry| entry.document_id)
        .flat_map(|document_id| {
            evidence
                .iter()
                .filter(move |fact| fact.provenance.document == Some(document_id))
                .map(|fact| fact.id)
        })
        .collect::<Vec<_>>();
    if ids.is_empty() {
        coverage.observe_absence(CoverageScope::DocumentRole(role))
    } else {
        Observation::present((), ids).expect("indexed documents retain evidence ids")
    }
}

fn content_observation_as_observation(observation: &ContentObservation) -> Observation<()> {
    match observation {
        ContentObservation::Present { evidence } => Observation::Present {
            value: (),
            evidence: evidence.clone(),
        },
        ContentObservation::Absent { coverage } => Observation::Absent {
            coverage: *coverage,
        },
        ContentObservation::Unknown(reason) => Observation::Unknown(*reason),
        ContentObservation::Conflict { alternatives } => Observation::Conflict {
            alternatives: alternatives.clone(),
        },
    }
}

fn combine_all(values: &[Observation<()>]) -> Observation<()> {
    if let Some(conflict) = first_conflict(values) {
        return conflict;
    }
    if let Some(unknown) = first_unknown(values) {
        return Observation::Unknown(unknown);
    }
    if let Some(absent) = first_absent(values) {
        return Observation::Absent { coverage: absent };
    }
    present_from(values)
}

fn combine_any(values: &[Observation<()>]) -> Observation<()> {
    let present = present_ids(values);
    if !present.is_empty() {
        return present_observation(present);
    }
    if let Some(conflict) = first_conflict(values) {
        return conflict;
    }
    if let Some(unknown) = first_unknown(values) {
        return Observation::Unknown(unknown);
    }
    Observation::Absent {
        coverage: first_absent(values).expect("all-false Any has absence coverage"),
    }
}

fn combine_at_least(values: &[Observation<()>], minimum: usize) -> Observation<()> {
    let present = present_ids(values);
    let present_count = values
        .iter()
        .filter(|value| matches!(value, Observation::Present { .. }))
        .count();
    if present_count >= minimum {
        return present_observation(present);
    }
    let unknown_count = values
        .iter()
        .filter(|value| {
            matches!(
                value,
                Observation::Unknown(_) | Observation::Conflict { .. }
            )
        })
        .count();
    if present_count + unknown_count >= minimum {
        return first_conflict(values).unwrap_or_else(|| {
            Observation::Unknown(first_unknown(values).expect("unknown count is non-zero"))
        });
    }
    Observation::Absent {
        coverage: first_absent(values).expect("insufficient true values require absence coverage"),
    }
}

fn present_from(values: &[Observation<()>]) -> Observation<()> {
    present_observation(present_ids(values))
}

fn present_observation(ids: Vec<EvidenceId>) -> Observation<()> {
    Observation::present((), ids).expect("all-present predicate values retain evidence")
}

fn present_ids(values: &[Observation<()>]) -> Vec<EvidenceId> {
    values
        .iter()
        .filter_map(|value| match value {
            Observation::Present { evidence, .. } => Some(evidence.as_slice()),
            Observation::Absent { .. } | Observation::Unknown(_) | Observation::Conflict { .. } => {
                None
            }
        })
        .flatten()
        .copied()
        .collect()
}

fn first_absent(values: &[Observation<()>]) -> Option<seiri_core::CoverageId> {
    values.iter().find_map(|value| match value {
        Observation::Absent { coverage } => Some(*coverage),
        _ => None,
    })
}

fn first_unknown(values: &[Observation<()>]) -> Option<UnknownReason> {
    values.iter().find_map(|value| match value {
        Observation::Unknown(reason) => Some(*reason),
        _ => None,
    })
}

fn first_conflict(values: &[Observation<()>]) -> Option<Observation<()>> {
    values.iter().find_map(|value| match value {
        Observation::Conflict { alternatives } => Some(Observation::Conflict {
            alternatives: alternatives.clone(),
        }),
        _ => None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateProgramError {
    TooManyAtoms {
        actual: usize,
        limit: usize,
    },
    TooManyInstructions {
        actual: usize,
        limit: usize,
    },
    UnknownAtom {
        index: usize,
        atom: u8,
    },
    StackLimitExceeded {
        index: usize,
        limit: usize,
    },
    InvalidArity {
        index: usize,
        arity: u8,
    },
    InvalidThreshold {
        index: usize,
        arity: u8,
        minimum: u8,
    },
    StackUnderflow {
        index: usize,
    },
    FinalStackDepth {
        depth: usize,
    },
}

impl Display for PredicateProgramError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyAtoms { actual, limit } => {
                write!(formatter, "predicate has {actual} atoms; limit is {limit}")
            }
            Self::TooManyInstructions { actual, limit } => {
                write!(
                    formatter,
                    "predicate has {actual} operations; limit is {limit}"
                )
            }
            Self::UnknownAtom { index, atom } => {
                write!(
                    formatter,
                    "predicate operation {index} references atom {atom}"
                )
            }
            Self::StackLimitExceeded { index, limit } => {
                write!(
                    formatter,
                    "predicate operation {index} exceeds stack limit {limit}"
                )
            }
            Self::InvalidArity { index, arity } => {
                write!(
                    formatter,
                    "predicate operation {index} has invalid arity {arity}"
                )
            }
            Self::InvalidThreshold {
                index,
                arity,
                minimum,
            } => write!(
                formatter,
                "predicate operation {index} has threshold {minimum} outside 1..={arity}"
            ),
            Self::StackUnderflow { index } => {
                write!(
                    formatter,
                    "predicate operation {index} underflows its stack"
                )
            }
            Self::FinalStackDepth { depth } => {
                write!(
                    formatter,
                    "predicate program finishes with stack depth {depth}"
                )
            }
        }
    }
}

impl std::error::Error for PredicateProgramError {}

#[cfg(test)]
mod tests {
    use super::*;
    use seiri_core::{
        stable_evidence_id, CoverageStatus, EvidenceConfidence, EvidenceProducer,
        EvidenceProvenance, SourceDomain,
    };

    #[test]
    fn bounded_evaluator_matches_reference_for_simple_program() {
        let evidence = [EvidenceFactV2 {
            id: stable_evidence_id(1),
            atom: EvidenceAtom::FilePresent,
            provenance: EvidenceProvenance {
                domain: SourceDomain::RepositoryLocal,
                producer: EvidenceProducer::FileWalker,
                document: None,
                span: None,
            },
            confidence: EvidenceConfidence::High,
        }];
        let coverage =
            CoverageIndex::try_new([(CoverageScope::RepositoryFiles, CoverageStatus::Complete)])
                .expect("coverage");
        let document_index = DocumentIndex::default();
        let context = PredicateContext::new(&evidence, &coverage, &document_index, &[]);
        for instruction in [
            PredicateInstruction::All(2),
            PredicateInstruction::Any(2),
            PredicateInstruction::AtLeast {
                arity: 2,
                minimum: 2,
            },
        ] {
            let program = PredicateProgram::try_new(
                vec![
                    PredicateAtom::Evidence(EvidenceAtom::FilePresent),
                    PredicateAtom::Evidence(EvidenceAtom::Readme(
                        seiri_core::ReadmePresence::Present,
                    )),
                ],
                vec![
                    PredicateInstruction::PushAtom(0),
                    PredicateInstruction::PushAtom(1),
                    instruction,
                ],
            )
            .expect("program");
            assert_eq!(
                program.evaluate(context),
                evaluate_reference(&program, context),
                "reference mismatch for {instruction:?}"
            );
        }
    }

    #[test]
    fn validator_rejects_bad_stack_arity_and_atom_references() {
        assert!(matches!(
            PredicateProgram::try_new(
                vec![PredicateAtom::Evidence(EvidenceAtom::FilePresent)],
                vec![PredicateInstruction::Any(1)],
            ),
            Err(PredicateProgramError::InvalidArity { .. })
        ));
        assert!(matches!(
            PredicateProgram::try_new(
                vec![PredicateAtom::Evidence(EvidenceAtom::FilePresent)],
                vec![PredicateInstruction::PushAtom(1)],
            ),
            Err(PredicateProgramError::UnknownAtom { .. })
        ));
        assert!(matches!(
            PredicateProgram::try_new(
                vec![PredicateAtom::Evidence(EvidenceAtom::FilePresent)],
                vec![
                    PredicateInstruction::PushAtom(0),
                    PredicateInstruction::PushAtom(0)
                ],
            ),
            Err(PredicateProgramError::FinalStackDepth { depth: 2 })
        ));
    }

    fn evaluate_reference(
        program: &PredicateProgram,
        context: PredicateContext<'_>,
    ) -> Observation<()> {
        let mut stack = Vec::new();
        for instruction in program.instructions() {
            match *instruction {
                PredicateInstruction::PushAtom(index) => {
                    stack.push(context.observe(program.atoms()[usize::from(index)]))
                }
                PredicateInstruction::All(arity) => {
                    let values = stack.split_off(stack.len() - usize::from(arity));
                    stack.push(reference_all(&values));
                }
                PredicateInstruction::Any(arity) => {
                    let values = stack.split_off(stack.len() - usize::from(arity));
                    stack.push(reference_any(&values));
                }
                PredicateInstruction::AtLeast { arity, minimum } => {
                    let values = stack.split_off(stack.len() - usize::from(arity));
                    stack.push(reference_at_least(&values, usize::from(minimum)));
                }
            }
        }
        stack.pop().expect("validated program")
    }

    fn reference_all(values: &[Observation<()>]) -> Observation<()> {
        if let Some(value) = reference_conflict(values) {
            return value;
        }
        if let Some(reason) = reference_unknown(values) {
            return Observation::Unknown(reason);
        }
        if let Some(coverage) = reference_absent(values) {
            return Observation::Absent { coverage };
        }
        reference_present(values)
    }

    fn reference_any(values: &[Observation<()>]) -> Observation<()> {
        let ids = reference_present_ids(values);
        if !ids.is_empty() {
            return Observation::present((), ids).expect("reference present ids");
        }
        if let Some(value) = reference_conflict(values) {
            return value;
        }
        if let Some(reason) = reference_unknown(values) {
            return Observation::Unknown(reason);
        }
        Observation::Absent {
            coverage: reference_absent(values).expect("reference Any has absence"),
        }
    }

    fn reference_at_least(values: &[Observation<()>], minimum: usize) -> Observation<()> {
        let present_count = values
            .iter()
            .filter(|value| matches!(value, Observation::Present { .. }))
            .count();
        if present_count >= minimum {
            return reference_present(values);
        }
        let undecided_count = values
            .iter()
            .filter(|value| {
                matches!(
                    value,
                    Observation::Unknown(_) | Observation::Conflict { .. }
                )
            })
            .count();
        if present_count + undecided_count >= minimum {
            return reference_conflict(values).unwrap_or_else(|| {
                Observation::Unknown(reference_unknown(values).expect("reference unknown"))
            });
        }
        Observation::Absent {
            coverage: reference_absent(values).expect("reference threshold has absence"),
        }
    }

    fn reference_present(values: &[Observation<()>]) -> Observation<()> {
        Observation::present((), reference_present_ids(values)).expect("reference all present")
    }

    fn reference_present_ids(values: &[Observation<()>]) -> Vec<EvidenceId> {
        let mut ids = Vec::new();
        for value in values {
            if let Observation::Present { evidence, .. } = value {
                ids.extend_from_slice(evidence.as_slice());
            }
        }
        ids
    }

    fn reference_absent(values: &[Observation<()>]) -> Option<seiri_core::CoverageId> {
        for value in values {
            if let Observation::Absent { coverage } = value {
                return Some(*coverage);
            }
        }
        None
    }

    fn reference_unknown(values: &[Observation<()>]) -> Option<UnknownReason> {
        for value in values {
            if let Observation::Unknown(reason) = value {
                return Some(*reason);
            }
        }
        None
    }

    fn reference_conflict(values: &[Observation<()>]) -> Option<Observation<()>> {
        for value in values {
            if let Observation::Conflict { alternatives } = value {
                return Some(Observation::Conflict {
                    alternatives: alternatives.clone(),
                });
            }
        }
        None
    }
}
