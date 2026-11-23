//! Contextual snapshot construction and bounded query laws.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Unbounded};
use std::sync::{Arc, Weak};

use pale_core::records::DiagnosticEffectCertaintyV1;

use crate::compile::{
    EffectiveProfileCompilation, EffectiveProfileOperationExposure,
    EffectiveProfileRequirementDisposition,
};
use crate::resident::{DesiredProcessState, ResidentApplication};

use super::types::{
    ObservationBackpressureRecovery, ObservationCancellation, ObservationCausalPath,
    ObservationCausalSelector, ObservationCausalStep, ObservationChange, ObservationCursor,
    ObservationDelta, ObservationDeltaOutcome, ObservationDeltaRequest, ObservationFact,
    ObservationFactKey, ObservationFreshness, ObservationGeneration, ObservationHealthState,
    ObservationLens, ObservationLensCursor, ObservationLensDisposition, ObservationLensNode,
    ObservationLensRequest, ObservationNativeState, ObservationPage, ObservationPageCursor,
    ObservationPageDisposition, ObservationPageRequest, ObservationProcessIntent,
    ObservationReadinessState, ObservationResumeCoordinate, ObservationResynchronization,
    ObservationResynchronizationReason, ObservationRevision, ObservationStepCompletion,
    ObservationStepLimit, ObservationSubject, DEFAULT_OBSERVATION_CAUSAL_PATH_LOOKUPS,
    DEFAULT_OBSERVATION_CAUSAL_STEPS, DEFAULT_OBSERVATION_COPIED_STRING_BYTES,
    DEFAULT_OBSERVATION_INSPECTION_LIMIT, DEFAULT_OBSERVATION_PAGE_LIMIT,
    MAX_OBSERVATION_CAUSAL_PATHS, MAX_OBSERVATION_CAUSAL_PATH_LOOKUPS,
    MAX_OBSERVATION_CAUSAL_STEPS, MAX_OBSERVATION_COPIED_STRING_BYTES, MAX_OBSERVATION_DELTA_ITEMS,
    MAX_OBSERVATION_FACTS, MAX_OBSERVATION_INSPECTION_LIMIT, MAX_OBSERVATION_LENS_NODES,
    MAX_OBSERVATION_PAGE_LIMIT,
};

/// Immutable bounded observation projection over one admitted owner coordinate.
///
/// The snapshot owns no source, store, graph, provider payload, or native body.
/// It can therefore outlive the compiler/resident borrow while remaining bound
/// to the exact generation and revision from which it was projected.
/// A profile snapshot also remembers a weak source origin for later association.
/// That reference does not keep the admitted source alive; existing facts remain
/// readable after it expires, but source association is then unavailable.
#[derive(Clone, Debug)]
pub struct ObservationSnapshot {
    pub(super) profile_origin: Option<Weak<crate::workspace::PrincipalAdmittedUniverse>>,
    cursor: ObservationCursor,
    facts: BTreeMap<ObservationFactKey, ObservationFact>,
    subjects: BTreeMap<ObservationSubject, Vec<ObservationFactKey>>,
    causal_paths: BTreeMap<ObservationSubject, ObservationCausalPath>,
    omitted_fact_count: u32,
}

impl ObservationSnapshot {
    /// Project already-admitted effective-profile meaning and honest unknowns.
    ///
    /// The selected private machine and providers remain unobserved. A bound
    /// capability requirement is therefore `Unproven`, not `Ready`; an absent
    /// provider is `Unavailable`; and policy exclusion stays `Blocked` or
    /// `Narrowed`. No native status is fabricated.
    #[must_use]
    pub fn from_effective_profile(compilation: &EffectiveProfileCompilation) -> Self {
        let inspection = compilation.inspection();
        let generation = ObservationGeneration::EffectiveProfile {
            profile_digest: inspection.profile_digest().clone(),
            instance_digest: compilation.instance().instance_digest.clone(),
            composition_plan_digest: compilation
                .experience_composition_plan()
                .plan_digest
                .clone(),
        };
        let cursor = ObservationCursor {
            generation: generation.clone(),
            revision: ObservationRevision::Static,
        };
        let mut facts = BTreeMap::new();
        let mut causal_paths = BTreeMap::new();

        insert_fact(
            &mut facts,
            ObservationFact::Readiness {
                subject: ObservationSubject::EffectiveProfile(inspection.profile_digest().clone()),
                state: ObservationReadinessState::Ready,
                freshness: ObservationFreshness::Fresh,
            },
        );

        let machine_subject = inspection.native_realization().observation_subject();
        insert_fact(
            &mut facts,
            ObservationFact::Health {
                subject: machine_subject.clone(),
                state: ObservationHealthState::Unknown,
                freshness: ObservationFreshness::Unknown,
            },
        );
        insert_fact(
            &mut facts,
            ObservationFact::Readiness {
                subject: machine_subject.clone(),
                state: ObservationReadinessState::Unproven,
                freshness: ObservationFreshness::Unknown,
            },
        );
        insert_fact(
            &mut facts,
            ObservationFact::NativeOwner {
                subject: machine_subject,
                state: ObservationNativeState::Unknown,
                freshness: ObservationFreshness::Unknown,
            },
        );

        for repository in inspection.repositories() {
            insert_fact(
                &mut facts,
                ObservationFact::Readiness {
                    subject: repository.observation_subject(),
                    state: ObservationReadinessState::Ready,
                    freshness: ObservationFreshness::Fresh,
                },
            );
            for operation in repository.operations() {
                let operation_subject = operation.observation_subject();
                let requirements = operation.requirements().collect::<Vec<_>>();
                let readiness = operation_readiness(operation.exposure(), &requirements);
                insert_fact(
                    &mut facts,
                    ObservationFact::OperationExposure {
                        subject: operation_subject.clone(),
                        state: operation.exposure(),
                        freshness: ObservationFreshness::Fresh,
                    },
                );
                insert_fact(
                    &mut facts,
                    ObservationFact::Readiness {
                        subject: operation_subject.clone(),
                        state: readiness,
                        freshness: ObservationFreshness::Fresh,
                    },
                );
                insert_fact(
                    &mut facts,
                    ObservationFact::Effect {
                        subject: operation_subject,
                        certainty: match operation.exposure() {
                            EffectiveProfileOperationExposure::SemanticOnly => {
                                DiagnosticEffectCertaintyV1::NotApplicable
                            }
                            EffectiveProfileOperationExposure::Unbound
                            | EffectiveProfileOperationExposure::CapabilityBearing => {
                                DiagnosticEffectCertaintyV1::NotStarted
                            }
                        },
                        freshness: ObservationFreshness::Fresh,
                    },
                );

                for requirement in requirements {
                    let subject = requirement.observation_subject();
                    let readiness = requirement_readiness(requirement.disposition());
                    insert_fact(
                        &mut facts,
                        ObservationFact::CapabilityRequirementDisposition {
                            subject: subject.clone(),
                            state: requirement.disposition(),
                            freshness: ObservationFreshness::Fresh,
                        },
                    );
                    insert_fact(
                        &mut facts,
                        ObservationFact::Readiness {
                            subject: subject.clone(),
                            state: readiness,
                            freshness: ObservationFreshness::Fresh,
                        },
                    );
                    insert_fact(
                        &mut facts,
                        ObservationFact::NativeOwner {
                            subject,
                            state: match readiness {
                                ObservationReadinessState::Blocked
                                | ObservationReadinessState::Narrowed
                                | ObservationReadinessState::Unavailable => {
                                    ObservationNativeState::Unavailable
                                }
                                ObservationReadinessState::Ready
                                | ObservationReadinessState::Unproven => {
                                    ObservationNativeState::Unknown
                                }
                            },
                            freshness: ObservationFreshness::Unknown,
                        },
                    );
                }
            }
        }

        for path in inspection.execution_paths() {
            let subject = path.observation_subject();
            insert_fact(
                &mut facts,
                ObservationFact::Readiness {
                    subject: subject.clone(),
                    state: ObservationReadinessState::Unproven,
                    freshness: ObservationFreshness::Unknown,
                },
            );
            if causal_paths.len() < MAX_OBSERVATION_CAUSAL_PATHS {
                causal_paths.insert(
                    subject.clone(),
                    ObservationCausalPath {
                        selector: ObservationCausalSelector {
                            generation: generation.clone(),
                            subject,
                        },
                        steps: path
                            .reason_steps()
                            .iter()
                            .cloned()
                            .map(ObservationCausalStep::Composition)
                            .collect(),
                    },
                );
            }
        }

        let omitted_fact_count = truncate_facts(&mut facts);
        let subjects = index_subjects(&facts);
        Self {
            profile_origin: Some(Arc::downgrade(compilation.admitted_source_owner())),
            cursor,
            facts,
            subjects,
            causal_paths,
            omitted_fact_count,
        }
    }

    /// Project the exact current resident authority without native telemetry.
    ///
    /// Only committed lifecycle retention and desired-process intent supply
    /// facts under the journal cursor. Admitting an unpublished candidate into
    /// memory does not change this snapshot; publication makes it observable
    /// at the new journal revision. Active or retained generation status does
    /// not prove provider availability, process liveness, or effect success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::resident::ResidentApplicationError`] when durable
    /// authority is stale or outcome-unknown, or when a committed authority
    /// frontier requires cold projection recovery.
    pub fn from_resident(
        resident: &ResidentApplication<'_>,
    ) -> Result<Self, crate::resident::ResidentApplicationError> {
        resident.ensure_current_projection()?;
        let active_generation = resident
            .active_generation()
            .map(|generation| generation.record().compiled_generation_digest().clone());
        let generation = ObservationGeneration::Resident {
            authority_store: resident.store().clone(),
            active_generation: active_generation.clone(),
        };
        let cursor = ObservationCursor {
            generation,
            revision: ObservationRevision::ResidentJournal(resident.journal_head()),
        };
        let mut facts = BTreeMap::new();
        for digest in resident.retained_generation_digests() {
            let subject = ObservationSubject::CompiledGeneration(digest.clone());
            insert_fact(
                &mut facts,
                ObservationFact::Readiness {
                    subject: subject.clone(),
                    state: ObservationReadinessState::Unproven,
                    freshness: ObservationFreshness::Fresh,
                },
            );
            insert_fact(
                &mut facts,
                ObservationFact::NativeOwner {
                    subject,
                    state: ObservationNativeState::Unknown,
                    freshness: ObservationFreshness::Unknown,
                },
            );
        }
        if let Some(state) = resident.desired_process_state() {
            insert_fact(
                &mut facts,
                ObservationFact::ProcessIntent {
                    state: match state {
                        DesiredProcessState::Running => ObservationProcessIntent::Running,
                        DesiredProcessState::Stopped => ObservationProcessIntent::Stopped,
                    },
                    freshness: ObservationFreshness::Fresh,
                },
            );
            insert_fact(
                &mut facts,
                ObservationFact::Health {
                    subject: ObservationSubject::ResidentProcess,
                    state: ObservationHealthState::Unknown,
                    freshness: ObservationFreshness::Unknown,
                },
            );
        }
        let omitted_fact_count = truncate_facts(&mut facts);
        let subjects = index_subjects(&facts);
        Ok(Self {
            profile_origin: None,
            cursor,
            facts,
            subjects,
            causal_paths: BTreeMap::new(),
            omitted_fact_count,
        })
    }

    /// Borrow the exact generation/revision cursor of this snapshot.
    #[must_use]
    pub const fn cursor(&self) -> &ObservationCursor {
        &self.cursor
    }

    /// Return a caller-held resume coordinate at this complete snapshot.
    #[must_use]
    pub fn resume_coordinate(&self) -> ObservationResumeCoordinate {
        ObservationResumeCoordinate(self.cursor.clone())
    }

    /// Return the number of retained facts.
    #[must_use]
    pub fn fact_count(&self) -> usize {
        self.facts.len()
    }

    /// Return how many matching facts the snapshot bound omitted.
    ///
    /// Omitted facts are never implied to be absent from the owner state. A
    /// nonzero value tells a caller that this bounded snapshot is incomplete.
    #[must_use]
    pub const fn omitted_fact_count(&self) -> u32 {
        self.omitted_fact_count
    }

    /// Query one bounded step in deterministic category/subject order.
    ///
    /// A continuation binds the exact snapshot, filter and raw work limits.
    /// Reaching a limit returns an explicit stop and a continuation without an
    /// uncharged exhaustion scan. Cancellation is observed between steps.
    ///
    /// # Errors
    ///
    /// Returns [`ObservationResynchronization`] when the page continuation was
    /// issued for another snapshot coordinate or filter.
    pub fn page(
        &self,
        request: ObservationPageRequest,
    ) -> Result<ObservationPageDisposition, ObservationResynchronization> {
        let valid_after = request.after.as_ref().is_none_or(|after| {
            after.snapshot == self.cursor
                && after.filter == request.filter
                && after.maximum_inspected == request.maximum_inspected
                && after.maximum_returned == request.maximum_returned
                && after.maximum_copied_string_bytes == request.maximum_copied_string_bytes
        });
        if !valid_after {
            return Err(ObservationResynchronization {
                reason: ObservationResynchronizationReason::ContinuationMismatch,
                current: Box::new(self.cursor.clone()),
            });
        }
        if request.cancellation == ObservationCancellation::Cancelled {
            return Ok(ObservationPageDisposition::Cancelled {
                continuation: request.after,
            });
        }
        let maximum_inspected = normalized_u16_limit(
            request.maximum_inspected,
            DEFAULT_OBSERVATION_INSPECTION_LIMIT,
            MAX_OBSERVATION_INSPECTION_LIMIT,
        );
        let maximum_returned = normalized_u16_limit(
            request.maximum_returned,
            DEFAULT_OBSERVATION_PAGE_LIMIT,
            MAX_OBSERVATION_PAGE_LIMIT,
        );
        let maximum_copied_string_bytes = normalized_u32_limit(
            request.maximum_copied_string_bytes,
            DEFAULT_OBSERVATION_COPIED_STRING_BYTES,
            MAX_OBSERVATION_COPIED_STRING_BYTES,
        );
        let after_key = request
            .after
            .as_ref()
            .and_then(|cursor| cursor.after_key.clone());
        let base_position = request
            .after
            .as_ref()
            .map_or(0, |cursor| cursor.inspected_position);
        let mut range = self
            .facts
            .range((after_key.as_ref().map_or(Unbounded, Excluded), Unbounded));
        let mut facts = Vec::with_capacity(usize::from(maximum_returned));
        let mut inspected = 0_u16;
        let mut advanced = 0_u32;
        let mut copied_string_bytes = 0_u32;
        let mut last_key = after_key;
        let completion = loop {
            if inspected == maximum_inspected {
                break ObservationStepCompletion::Stopped(ObservationStepLimit::Inspected);
            }
            if facts.len() == usize::from(maximum_returned) {
                break ObservationStepCompletion::Stopped(ObservationStepLimit::Returned);
            }
            let Some((key, fact)) = range.next() else {
                break ObservationStepCompletion::Complete;
            };
            inspected = inspected.saturating_add(1);
            if !request.filter.matches(fact) {
                advanced = advanced.saturating_add(1);
                last_key = Some(key.clone());
                continue;
            }
            let copied = fact.copied_string_bytes();
            if copied > maximum_copied_string_bytes.saturating_sub(copied_string_bytes) {
                let limit = ObservationStepLimit::CopiedStringBytes {
                    minimum_required: copied,
                };
                break if copied > maximum_copied_string_bytes {
                    ObservationStepCompletion::Incomplete(limit)
                } else {
                    ObservationStepCompletion::Stopped(limit)
                };
            }
            copied_string_bytes = copied_string_bytes.saturating_add(copied);
            facts.push(fact.clone());
            advanced = advanced.saturating_add(1);
            last_key = Some(key.clone());
        };
        let continuation = matches!(completion, ObservationStepCompletion::Stopped(_)).then(|| {
            ObservationPageCursor {
                snapshot: self.cursor.clone(),
                filter: request.filter,
                after_key: last_key,
                inspected_position: base_position.saturating_add(advanced),
                maximum_inspected: request.maximum_inspected,
                maximum_returned: request.maximum_returned,
                maximum_copied_string_bytes: request.maximum_copied_string_bytes,
            }
        });
        Ok(ObservationPageDisposition::Ready(ObservationPage {
            cursor: self.cursor.clone(),
            facts,
            continuation,
            completion,
            inspected,
            copied_string_bytes,
            snapshot_omitted_count: self.omitted_fact_count,
        }))
    }

    /// Derive the next complete delta from an exact caller resume coordinate.
    ///
    /// Only equal or adjacent revisions in the same generation are eligible.
    /// A generation mismatch or revision gap always requests a full snapshot.
    /// If a complete adjacent delta exceeds the consumer bound, no partial
    /// change set is returned.
    #[must_use]
    pub fn delta_from(
        &self,
        prior: &Self,
        request: ObservationDeltaRequest,
    ) -> ObservationDeltaOutcome {
        if request.cancellation == ObservationCancellation::Cancelled {
            return ObservationDeltaOutcome::Cancelled(request.resume);
        }
        if request.resume.cursor() == self.cursor() {
            return ObservationDeltaOutcome::CaughtUp(self.cursor.clone());
        }
        if request.resume.cursor() != prior.cursor() {
            return resynchronize_for_requested(self, request.resume.cursor());
        }
        if prior.cursor.generation != self.cursor.generation {
            return ObservationDeltaOutcome::Resynchronize(ObservationResynchronization {
                reason: ObservationResynchronizationReason::GenerationMismatch {
                    requested: Box::new(prior.cursor.generation.clone()),
                    current: Box::new(self.cursor.generation.clone()),
                },
                current: Box::new(self.cursor.clone()),
            });
        }
        if prior.cursor.revision == self.cursor.revision {
            return ObservationDeltaOutcome::CaughtUp(self.cursor.clone());
        }
        if !adjacent_revisions(&prior.cursor.revision, &self.cursor.revision) {
            return ObservationDeltaOutcome::Resynchronize(ObservationResynchronization {
                reason: ObservationResynchronizationReason::RevisionGap {
                    requested: prior.cursor.revision.clone(),
                    current: self.cursor.revision.clone(),
                },
                current: Box::new(self.cursor.clone()),
            });
        }

        let changes = changes_between(&prior.facts, &self.facts);
        let maximum_changes = normalized_u16_limit(
            request.maximum_changes,
            DEFAULT_OBSERVATION_PAGE_LIMIT,
            MAX_OBSERVATION_DELTA_ITEMS,
        );
        if changes.len() > usize::from(maximum_changes) {
            return ObservationDeltaOutcome::Backpressured {
                pending_changes: saturated_u32(changes.len()),
                maximum_changes,
                recovery: ObservationBackpressureRecovery::RequestSnapshot,
                current: self.cursor.clone(),
            };
        }
        ObservationDeltaOutcome::Delta(ObservationDelta {
            from: prior.cursor.clone(),
            to: self.cursor.clone(),
            changes,
            successor_omitted_fact_count: self.omitted_fact_count,
        })
    }

    /// Return one retained compiler-owned causal path.
    ///
    /// A selector from another generation returns resynchronization instead of
    /// being interpreted against current facts. Unknown current subjects return
    /// `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns [`ObservationResynchronization`] when the selector belongs to a
    /// different admitted generation.
    pub fn causal_path(
        &self,
        selector: &ObservationCausalSelector,
    ) -> Result<Option<ObservationCausalPath>, ObservationResynchronization> {
        if selector.generation != self.cursor.generation {
            return Err(ObservationResynchronization {
                reason: ObservationResynchronizationReason::GenerationMismatch {
                    requested: Box::new(selector.generation.clone()),
                    current: Box::new(self.cursor.generation.clone()),
                },
                current: Box::new(self.cursor.clone()),
            });
        }
        Ok(self.causal_paths.get(&selector.subject).cloned())
    }

    /// Group one bounded subject step without geometry.
    ///
    /// The continuation binds the exact snapshot, filter and raw limits. Each
    /// call bounds inspected subjects, returned nodes, copied strings, causal
    /// lookups and causal steps independently. Cancellation is observed between
    /// steps, so no partially constructed node or path is published.
    #[must_use]
    pub fn visual_lens(&self, request: ObservationLensRequest) -> ObservationLensDisposition {
        let valid_after = request.after.as_ref().is_none_or(|after| {
            after.snapshot == self.cursor
                && after.filter == request.filter
                && after.maximum_inspected == request.maximum_inspected
                && after.maximum_nodes == request.maximum_nodes
                && after.maximum_copied_string_bytes == request.maximum_copied_string_bytes
                && after.include_causal_paths == request.include_causal_paths
                && after.maximum_causal_path_lookups == request.maximum_causal_path_lookups
                && after.maximum_causal_steps == request.maximum_causal_steps
        });
        if !valid_after {
            return ObservationLensDisposition::Resynchronize(ObservationResynchronization {
                reason: ObservationResynchronizationReason::ContinuationMismatch,
                current: Box::new(self.cursor.clone()),
            });
        }
        if request.cancellation == ObservationCancellation::Cancelled {
            return ObservationLensDisposition::Cancelled {
                continuation: request.after,
            };
        }
        let maximum_inspected = normalized_u16_limit(
            request.maximum_inspected,
            DEFAULT_OBSERVATION_INSPECTION_LIMIT,
            MAX_OBSERVATION_INSPECTION_LIMIT,
        );
        let maximum_nodes = normalized_u16_limit(
            request.maximum_nodes,
            DEFAULT_OBSERVATION_PAGE_LIMIT,
            MAX_OBSERVATION_LENS_NODES,
        );
        let maximum_copied_string_bytes = normalized_u32_limit(
            request.maximum_copied_string_bytes,
            DEFAULT_OBSERVATION_COPIED_STRING_BYTES,
            MAX_OBSERVATION_COPIED_STRING_BYTES,
        );
        let maximum_causal_path_lookups = normalized_u16_limit(
            request.maximum_causal_path_lookups,
            DEFAULT_OBSERVATION_CAUSAL_PATH_LOOKUPS,
            MAX_OBSERVATION_CAUSAL_PATH_LOOKUPS,
        );
        let maximum_causal_steps = normalized_u16_limit(
            request.maximum_causal_steps,
            DEFAULT_OBSERVATION_CAUSAL_STEPS,
            MAX_OBSERVATION_CAUSAL_STEPS,
        );
        let after_subject = request
            .after
            .as_ref()
            .and_then(|cursor| cursor.after_subject.clone());
        let base_position = request
            .after
            .as_ref()
            .map_or(0, |cursor| cursor.inspected_position);
        let mut range = self.subjects.range((
            after_subject.as_ref().map_or(Unbounded, Excluded),
            Unbounded,
        ));
        let mut nodes = Vec::with_capacity(usize::from(maximum_nodes));
        let mut causal_paths = Vec::new();
        let mut inspected_subjects = 0_u16;
        let mut advanced = 0_u32;
        let mut copied_string_bytes = 0_u32;
        let mut causal_path_lookups = 0_u16;
        let mut causal_steps = 0_u16;
        let mut last_subject = after_subject;
        let completion = loop {
            if inspected_subjects == maximum_inspected {
                break ObservationStepCompletion::Stopped(ObservationStepLimit::Inspected);
            }
            if nodes.len() == usize::from(maximum_nodes) {
                break ObservationStepCompletion::Stopped(ObservationStepLimit::Returned);
            }
            let Some((subject, keys)) = range.next() else {
                break ObservationStepCompletion::Complete;
            };
            inspected_subjects = inspected_subjects.saturating_add(1);
            let selected = keys
                .iter()
                .filter_map(|key| self.facts.get(key))
                .filter(|fact| request.filter.matches(fact))
                .collect::<Vec<_>>();
            if selected.is_empty() {
                advanced = advanced.saturating_add(1);
                last_subject = Some(subject.clone());
                continue;
            }
            if request.include_causal_paths && causal_path_lookups == maximum_causal_path_lookups {
                break ObservationStepCompletion::Stopped(ObservationStepLimit::CausalPathLookups);
            }
            let path = request
                .include_causal_paths
                .then(|| self.causal_paths.get(subject))
                .flatten();
            if request.include_causal_paths {
                causal_path_lookups = causal_path_lookups.saturating_add(1);
            }
            let next_steps = path.map_or(0, |value| saturated_u16(value.steps.len()));
            if next_steps > maximum_causal_steps.saturating_sub(causal_steps) {
                let limit = ObservationStepLimit::CausalSteps {
                    minimum_required: next_steps,
                };
                break if next_steps > maximum_causal_steps {
                    ObservationStepCompletion::Incomplete(limit)
                } else {
                    ObservationStepCompletion::Stopped(limit)
                };
            }
            let mut next_bytes = subject.copied_string_bytes();
            for fact in &selected {
                next_bytes = next_bytes.saturating_add(fact.copied_string_bytes());
            }
            if let Some(path) = path {
                next_bytes = next_bytes.saturating_add(path.copied_string_bytes());
            }
            if next_bytes > maximum_copied_string_bytes.saturating_sub(copied_string_bytes) {
                let limit = ObservationStepLimit::CopiedStringBytes {
                    minimum_required: next_bytes,
                };
                break if next_bytes > maximum_copied_string_bytes {
                    ObservationStepCompletion::Incomplete(limit)
                } else {
                    ObservationStepCompletion::Stopped(limit)
                };
            }
            causal_steps = causal_steps.saturating_add(next_steps);
            copied_string_bytes = copied_string_bytes.saturating_add(next_bytes);
            nodes.push(ObservationLensNode {
                subject: subject.clone(),
                facts: selected.into_iter().cloned().collect(),
            });
            if let Some(path) = path {
                causal_paths.push(path.clone());
            }
            advanced = advanced.saturating_add(1);
            last_subject = Some(subject.clone());
        };
        let continuation = matches!(completion, ObservationStepCompletion::Stopped(_)).then(|| {
            ObservationLensCursor {
                snapshot: self.cursor.clone(),
                filter: request.filter,
                after_subject: last_subject,
                inspected_position: base_position.saturating_add(advanced),
                maximum_inspected: request.maximum_inspected,
                maximum_nodes: request.maximum_nodes,
                maximum_copied_string_bytes: request.maximum_copied_string_bytes,
                include_causal_paths: request.include_causal_paths,
                maximum_causal_path_lookups: request.maximum_causal_path_lookups,
                maximum_causal_steps: request.maximum_causal_steps,
            }
        });
        ObservationLensDisposition::Ready(ObservationLens {
            cursor: self.cursor.clone(),
            nodes,
            causal_paths,
            continuation,
            completion,
            inspected_subjects,
            copied_string_bytes,
            causal_path_lookups,
            causal_steps,
            snapshot_omitted_count: self.omitted_fact_count,
        })
    }
}

fn insert_fact(facts: &mut BTreeMap<ObservationFactKey, ObservationFact>, fact: ObservationFact) {
    facts.insert(fact.key(), fact);
}

fn index_subjects(
    facts: &BTreeMap<ObservationFactKey, ObservationFact>,
) -> BTreeMap<ObservationSubject, Vec<ObservationFactKey>> {
    let mut subjects = BTreeMap::<ObservationSubject, Vec<ObservationFactKey>>::new();
    for (key, fact) in facts {
        subjects
            .entry(fact.owned_subject())
            .or_default()
            .push(key.clone());
    }
    subjects
}

fn truncate_facts(facts: &mut BTreeMap<ObservationFactKey, ObservationFact>) -> u32 {
    let omitted = facts.len().saturating_sub(MAX_OBSERVATION_FACTS);
    while facts.len() > MAX_OBSERVATION_FACTS {
        facts.pop_last();
    }
    saturated_u32(omitted)
}

fn saturated_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn saturated_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn operation_readiness(
    exposure: EffectiveProfileOperationExposure,
    requirements: &[crate::compile::EffectiveProfileRequirementInspection<'_>],
) -> ObservationReadinessState {
    match exposure {
        EffectiveProfileOperationExposure::Unbound => ObservationReadinessState::Unavailable,
        EffectiveProfileOperationExposure::SemanticOnly => ObservationReadinessState::Ready,
        EffectiveProfileOperationExposure::CapabilityBearing => requirements
            .iter()
            .map(|requirement| requirement_readiness(requirement.disposition()))
            .fold(ObservationReadinessState::Unproven, readiness_meet),
    }
}

/// Combine independently derived readiness evidence by its conservative meet.
///
/// [`ObservationReadinessState::Ready`] is the identity; the operator is
/// commutative, associative, and idempotent. Its precedence is rendered below.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/observation-readiness-meet.display.mathml"
))]
const fn readiness_meet(
    left: ObservationReadinessState,
    right: ObservationReadinessState,
) -> ObservationReadinessState {
    use ObservationReadinessState::{Blocked, Narrowed, Ready, Unavailable, Unproven};
    match (left, right) {
        (Blocked, _) | (_, Blocked) => Blocked,
        (Narrowed, _) | (_, Narrowed) => Narrowed,
        (Unavailable, _) | (_, Unavailable) => Unavailable,
        (Unproven, _) | (_, Unproven) => Unproven,
        (Ready, Ready) => Ready,
    }
}

const fn requirement_readiness(
    disposition: EffectiveProfileRequirementDisposition,
) -> ObservationReadinessState {
    match disposition {
        EffectiveProfileRequirementDisposition::Satisfied => ObservationReadinessState::Unproven,
        EffectiveProfileRequirementDisposition::Narrowed(_) => ObservationReadinessState::Narrowed,
        EffectiveProfileRequirementDisposition::ProviderBindingAbsent => {
            ObservationReadinessState::Unavailable
        }
        EffectiveProfileRequirementDisposition::PolicyBlocked(_) => {
            ObservationReadinessState::Blocked
        }
    }
}

const fn normalized_u16_limit(value: u16, default: u16, maximum: u16) -> u16 {
    if value == 0 {
        default
    } else if value > maximum {
        maximum
    } else {
        value
    }
}

const fn normalized_u32_limit(value: u32, default: u32, maximum: u32) -> u32 {
    if value == 0 {
        default
    } else if value > maximum {
        maximum
    } else {
        value
    }
}

fn adjacent_revisions(prior: &ObservationRevision, current: &ObservationRevision) -> bool {
    match (prior, current) {
        (
            ObservationRevision::ResidentJournal(prior),
            ObservationRevision::ResidentJournal(current),
        ) => match (prior, current) {
            (None, Some(current)) => current.get() == 1,
            (Some(prior), Some(current)) => prior.checked_successor() == Some(*current),
            (None, None) | (Some(_), None) => false,
        },
        (ObservationRevision::Static, ObservationRevision::Static) => false,
        _ => false,
    }
}

fn changes_between(
    prior: &BTreeMap<ObservationFactKey, ObservationFact>,
    current: &BTreeMap<ObservationFactKey, ObservationFact>,
) -> Vec<ObservationChange> {
    prior
        .keys()
        .chain(current.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|key| match (prior.get(&key), current.get(&key)) {
            (Some(prior), Some(current)) if prior == current => None,
            (_, Some(current)) => Some(ObservationChange::Upsert(current.clone())),
            (Some(prior), None) => Some(ObservationChange::Remove {
                category: prior.category(),
                subject: prior.owned_subject(),
            }),
            (None, None) => None,
        })
        .collect()
}

fn resynchronize_for_requested(
    current: &ObservationSnapshot,
    requested: &ObservationCursor,
) -> ObservationDeltaOutcome {
    let reason = if requested.generation != current.cursor.generation {
        ObservationResynchronizationReason::GenerationMismatch {
            requested: Box::new(requested.generation.clone()),
            current: Box::new(current.cursor.generation.clone()),
        }
    } else {
        ObservationResynchronizationReason::RevisionGap {
            requested: requested.revision.clone(),
            current: current.cursor.revision.clone(),
        }
    };
    ObservationDeltaOutcome::Resynchronize(ObservationResynchronization {
        reason,
        current: Box::new(current.cursor.clone()),
    })
}

#[cfg(test)]
#[path = "../../tests/internal/observation.rs"]
mod tests;
