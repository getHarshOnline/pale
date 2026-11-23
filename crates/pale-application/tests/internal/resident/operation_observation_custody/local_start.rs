//! Local start assembly over source-backed definitions and contextual session fixtures.
//! External authority/signature and native preparation ports are explicit test doubles;
//! these tests do not establish a production signer, accumulator or provider.
//! The inherited capability id `read` is an opaque fixture coordinate. Its selected
//! registered path-inventory action has Compute effect and never reads a filesystem.

use super::*;
use crate::local_operation_start_fixture::StartAdmission;
use crate::resident::local_operation_coordinator::{
    ResidentLocalOperationCheckpointInput, ResidentLocalOperationCoordinator,
    ResidentLocalOperationStartInput,
};
use crate::resident::operation_custody::operation_request_key;
use crate::resident::operation_service::{ResidentOperationService, ResidentOperationTaskState};
use crate::resident::task_owner::ResidentOperationTaskLimits;
use crate::resident::task_reservation::ResidentAggregateTaskLimits;
use crate::resident::{
    LocalOperationCheckpointFailure, LocalOperationStartError, PendingLocalOperationStart,
    PreparedLocalOperationCheckpoint, PreparedLocalOperationStart, PreparedOperationStart,
};
use crate::semantic::{CapabilityObligationDisposition, ProviderUnavailability};
use pale_ports::{
    AuthorityStoreCoordinate, OperationCustodyIndexObservation, OperationCustodyRecord,
};
use std::num::NonZeroUsize;

use crate::effect_case_lineage_fixture as request_fixture;
use crate::local_operation_start_fixture::{
    host_source_generation_input_copies, operation_id, source_generation_input_copies_with,
    source_generation_inputs_with,
};
pub(super) use crate::local_operation_start_fixture::{
    with_consult_start, with_invoke_start_input, with_start, RecoveryParents,
};

pub(in crate::resident) fn source_generation() -> (
    crate::resident::AdmittedCompiledGeneration,
    EffectiveProjectProfileV1,
) {
    source_generation_with(|_| {}, |_| {})
}

pub(in crate::resident) fn source_generation_copies(
    copies: usize,
) -> (
    Vec<crate::resident::AdmittedCompiledGeneration>,
    EffectiveProjectProfileV1,
) {
    let inputs = source_generation_input_copies_with(|_| {}, |_| {}, copies);
    let profile = inputs
        .first()
        .expect("positive source-generation copy count")
        .2
        .clone();
    let generations = inputs
        .into_iter()
        .map(|(compilation, materializations, _)| {
            crate::resident::AdmittedCompiledGeneration::admit(
                compilation,
                materializations,
                crate::resident_fixture::synthetic_set_policy(),
            )
            .expect("actual resident generation copy")
        })
        .collect();
    (generations, profile)
}

/// Compile admitted copies and one byte-identical cold-reconstruction input.
///
/// The compiler fixture carries workspace identity in its output. Keeping the
/// reconstruction input in this same compilation batch proves substitution
/// against the exact generation instead of compiling a look-alike workspace
/// later.
pub(in crate::resident) fn source_generation_copies_with_reconstruction(
    copies: usize,
) -> (
    Vec<crate::resident::AdmittedCompiledGeneration>,
    EffectiveProjectProfileV1,
    crate::resident::ResidentGenerationReconstructionInput,
) {
    let input_count = copies
        .checked_add(1)
        .expect("source-generation reconstruction copy count");
    let mut inputs = source_generation_input_copies_with(|_| {}, |_| {}, input_count).into_iter();
    let (reconstruction_compilation, reconstruction_materializations, profile) = inputs
        .next()
        .expect("one cold-reconstruction generation input");
    let generations = inputs
        .map(|(compilation, materializations, _)| {
            crate::resident::AdmittedCompiledGeneration::admit(
                compilation,
                materializations,
                crate::resident_fixture::synthetic_set_policy(),
            )
            .expect("actual resident generation copy")
        })
        .collect::<Vec<_>>();
    assert_eq!(generations.len(), copies);
    let expected_digest = generations
        .first()
        .expect("positive admitted generation copy count")
        .record()
        .compiled_generation_digest()
        .clone();
    let reconstruction = crate::resident::ResidentGenerationReconstructionInput::new(
        expected_digest,
        reconstruction_compilation,
        reconstruction_materializations,
    );
    (generations, profile, reconstruction)
}

pub(in crate::resident) fn host_source_generation_copies(
    copies: usize,
) -> (
    Vec<crate::resident::AdmittedCompiledGeneration>,
    EffectiveProjectProfileV1,
) {
    let inputs = host_source_generation_input_copies("bounded_compute", copies);
    let profile = inputs
        .first()
        .expect("positive host source-generation copy count")
        .2
        .clone();
    let generations = inputs
        .into_iter()
        .map(|(compilation, materializations, _)| {
            crate::resident::AdmittedCompiledGeneration::admit(
                compilation,
                materializations,
                crate::resident_fixture::synthetic_set_policy(),
            )
            .expect("actual host resident generation copy")
        })
        .collect();
    (generations, profile)
}

fn source_generation_with(
    change_operation: impl FnOnce(&mut OnDemandOperation),
    change_private: impl FnOnce(&mut CapabilitiesFile),
) -> (
    crate::resident::AdmittedCompiledGeneration,
    EffectiveProjectProfileV1,
) {
    let (compilation, materializations, profile) =
        source_generation_inputs_with(change_operation, change_private);
    let generation = crate::resident::AdmittedCompiledGeneration::admit(
        compilation,
        materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .expect("actual resident generation");
    (generation, profile)
}

pub(super) fn assert_publication_policy(
    operation: &crate::resident::IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    suffix: &str,
) {
    let policy = operation.publication_policy();
    assert_eq!(
        policy.target_scope(),
        &EventScopeV1 {
            principal_scope: request.scope().principal_scope.clone(),
            controller_scope: request.scope().controller_scope.clone(),
            relationship_scope: request.scope().relationship_scope.clone(),
            environment: DeploymentEnvironmentV1::parse(request.scope().environment.as_str())
                .expect("publication environment"),
        }
    );
    assert_eq!(
        policy.classification(),
        &DataClassificationV1::parse(request.scope().data_classification.as_str())
            .expect("publication classification")
    );
    assert_eq!(
        policy.visibility(),
        &InteractionVisibilityV1::parse("private").expect("publication visibility")
    );
    assert_eq!(
        policy.disclosure_ceiling(),
        &InteractionDisclosureCeilingV1::parse("private").expect("publication disclosure")
    );
    assert_eq!(
        policy.payload_resolution_policy(),
        &PayloadResolutionPolicyV1::parse("reference-only").expect("payload resolution policy")
    );
    assert_eq!(
        policy.actor_provenance(),
        &OwnerRecordCoordinateV1 {
            owner_system_id: OwnerSystemIdV1::parse("pale-test-owner").expect("owner system"),
            record_ref: record_ref(&format!("actor/{suffix}")),
            record_schema_id: SchemaId::parse("test/actor.v1").expect("actor schema"),
            record_schema_sha256: schema_digest(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID),
            record_content_sha256: artifact_digest('9'),
            native_revision: NativeRevisionV1 {
                native_revision_owner: NativeRevisionOwnerV1::parse("pale-test-owner")
                    .expect("revision owner"),
                native_revision_token: NativeRevisionTokenV1::parse(format!(
                    "actor-revision-{suffix}"
                ))
                .expect("revision token"),
            },
        }
    );
}

pub(in crate::resident) struct StartStore<'a>(
    pub(in crate::resident) &'a crate::resident::AdmittedAmbientSession,
);

impl OperationObservationAuthorityStoreResolver for StartStore<'_> {
    fn resolve(
        &self,
        observation: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        assert_eq!(
            observation.deployment_runtime_authority_ref(),
            self.0.runtime_authority_ref()
        );
        assert_eq!(
            observation.deployment_runtime_authority_sha256().as_str(),
            self.0.runtime_authority_sha256().as_str()
        );
        Ok(store_coordinate())
    }
}

// Application association fixture only: actual Merkle computation is exercised
// by the adapter tests. This returns finite physical bytes, never a production
// construction callback or proof that these synthetic roots commit the leaves.
pub(in crate::resident) fn initial_response(
    request: &pale_ports::OperationAccumulatorPreparation,
) -> Result<
    pale_ports::PreparedOperationAccumulator,
    pale_ports::OperationAccumulatorPreparationError,
> {
    assert!(matches!(
        request.basis(),
        pale_ports::AccumulatorPreparationBasis::Initial(_)
    ));
    assert_eq!(request.basis().epoch().get(), 1);
    assert_eq!(request.state().changes().len(), 4);
    assert!(request
        .state()
        .changes()
        .iter()
        .all(|change| change.predecessor().is_none()));
    assert!(request.unresolved().changes().is_empty());
    Ok(pale_ports::PreparedOperationAccumulator::new(
        request.clone(),
        pale_ports::StateAccumulatorBatch::new(
            artifact_digest('0'),
            artifact_digest('d'),
            vec![pale_ports::AccumulatorMutation::new(
                pale_ports::AccumulatorPosition::Leaf([1; 32]),
                None,
                Some(vec![2; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        pale_ports::UnresolvedCustodyAccumulatorBatch::new(
            artifact_digest('0'),
            artifact_digest('0'),
            vec![],
            1,
        )
        .unwrap(),
    )
    .unwrap())
}

pub(in crate::resident) fn prepared_initial(
    prepared: PreparedLocalOperationStart,
) -> PreparedLocalOperationCheckpoint {
    prepared_initial_at_store(prepared, &store_coordinate())
}

fn prepared_initial_at_store(
    prepared: PreparedLocalOperationStart,
    store: &AuthorityStoreCoordinate,
) -> PreparedLocalOperationCheckpoint {
    let mut port = super::checkpoint_preparation::PreparationOnly::at_store(
        initial_response,
        ControlTransactionSequenceV1::new(1).unwrap(),
        store.clone(),
    );
    prepared
        .prepare_checkpoint(
            pale_ports::AccumulatorPreparationLimits::new(4, 1024, 1024, 512, 65_536).unwrap(),
            &mut port,
        )
        .expect("initial association and finite native response")
}

pub(in crate::resident) fn issue_initial(
    prepared: PreparedLocalOperationCheckpoint,
    request: &CommonRequestEnvelopeV1,
    change: impl FnOnce(&mut ControlStateCheckpointPreimageV1),
    signatures: &Cell<usize>,
) -> Result<
    (
        PreparedOperationStart,
        PendingLocalOperationStart,
        ControlStateCheckpointV1,
    ),
    LocalOperationCheckpointFailure,
> {
    let preimage = initial_checkpoint_preimage(&prepared, request, change);
    let (physical, pending) = prepared.issue_checkpoint(
        &record_ref("operation/checkpoint-1"),
        preimage.clone(),
        &preimage.schema_sha256,
        &checkpoint_authority(&preimage),
        |_, _, _| {
            signatures.set(signatures.get() + 1);
            Ok(signature())
        },
    )?;
    // Re-admit the exact signed preimage through the existing test owner. The
    // positive custody test compares it to the actual committed record bytes.
    let checkpoint = admit_initial_checkpoint(&preimage);
    Ok((physical, pending, checkpoint))
}

fn initial_checkpoint_preimage(
    prepared: &PreparedLocalOperationCheckpoint,
    request: &CommonRequestEnvelopeV1,
    change: impl FnOnce(&mut ControlStateCheckpointPreimageV1),
) -> ControlStateCheckpointPreimageV1 {
    let mut preimage = checkpoint_preimage(1, None);
    preimage.committed_transaction_sequence = prepared.transaction_sequence();
    preimage.issued_at = UnixTimeV1::new(61).unwrap();
    preimage.checkpoint_profile_sha256 =
        crate::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
    preimage.checkpoint_domain_id =
        crate::resident::SupportedOperationCheckpointProfile::conflict_domain(request).unwrap();
    preimage.state_root_sha256 = prepared.state_root().clone();
    preimage.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    preimage.changed_leaf_keys = prepared.changed_keys().to_vec();
    preimage.leaf_count = ControlLeafCountV1::new(4).unwrap();
    change(&mut preimage);
    preimage
}

pub(in crate::resident) fn initial_anchor() -> RecordingAnchor {
    let mut anchor = RecordingAnchor::advancing();
    anchor.observation_result = Some(Ok(AntiRollbackAnchorObservation::Missing));
    anchor
}

/// Model an independently retained predecessor for one successor append.
pub(in crate::resident) fn successor_anchor() -> RecordingAnchor {
    RecordingAnchor::advancing()
}

pub(in crate::resident) fn absent_start_store(request: &CommonRequestEnvelopeV1) -> RecordingStore {
    let key = operation_request_key(request).expect("actual owner request key");
    RecordingStore::recovering(vec![
        OperationCustodyIndexObservation::absent(OperationCustodyIndexQuery::RequestOperation(
            key.clone(),
        )),
        OperationCustodyIndexObservation::absent(OperationCustodyIndexQuery::RequestEvidence(key)),
    ])
}

/// Reuse the admitted-start ceremony for tests that inspect later private data.
/// The recording ports remain test models, not native durability evidence.
pub(in crate::resident) fn with_issued<R>(
    generation: &crate::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    run: impl FnOnce(
        &crate::resident::IssuedLocalOperation,
        &CommonRequestEnvelopeV1,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_issued_request(generation, profile, suffix, |_| {}, run)
}

/// Select the exact signed request body before the same admitted-start ceremony.
pub(in crate::resident) fn with_issued_request<R>(
    generation: &crate::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    adjust_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    run: impl FnOnce(
        &crate::resident::IssuedLocalOperation,
        &CommonRequestEnvelopeV1,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_issued_context(
        generation,
        profile,
        suffix,
        adjust_request,
        crate::session_admission_fixture::LocalStartContext::Inherited,
        |issued, request, parents, _, _, _| run(issued, request, parents),
    )
}

/// Carry the actual initial checkpoint and retained observation into a later phase test.
pub(in crate::resident) fn with_issued_context<R>(
    generation: &crate::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    adjust_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    context_selection: crate::session_admission_fixture::LocalStartContext,
    run: impl FnOnce(
        &crate::resident::IssuedLocalOperation,
        &CommonRequestEnvelopeV1,
        RecoveryParents<'_>,
        &ControlStateCheckpointV1,
        &ControlRecord,
        &OperationCustodyCandidate,
    ) -> R,
) -> R {
    with_issued_context_at_store(
        generation,
        profile,
        suffix,
        adjust_request,
        context_selection,
        &store_coordinate(),
        run,
    )
}

pub(in crate::resident) fn with_issued_context_at_store<R>(
    generation: &crate::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    adjust_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    context_selection: crate::session_admission_fixture::LocalStartContext,
    store_coordinate: &AuthorityStoreCoordinate,
    run: impl FnOnce(
        &crate::resident::IssuedLocalOperation,
        &CommonRequestEnvelopeV1,
        RecoveryParents<'_>,
        &ControlStateCheckpointV1,
        &ControlRecord,
        &OperationCustodyCandidate,
    ) -> R,
) -> R {
    crate::local_operation_start_fixture::with_start_context_at_store(
        generation,
        profile,
        suffix,
        adjust_request,
        StartAdmission::AcceptedOwnerModel,
        context_selection,
        store_coordinate,
        |prepared, request, _, _, parents| {
            let (physical, pending, checkpoint) = issue_initial(
                prepared_initial_at_store(prepared.expect("admitted start"), store_coordinate),
                request,
                |preimage| {
                    preimage.control_store_id = store_coordinate.store_id().clone();
                    preimage.control_incarnation_id = store_coordinate.incarnation_id().clone();
                },
                &Cell::new(0),
            )
            .expect("prepared checkpoint");
            let mut store = absent_start_store(request).at_store(store_coordinate);
            let mut anchor = initial_anchor();
            let completed = physical
                .commit(&mut store, &mut anchor, 1024 * 1024)
                .expect("modeled initial transaction")
                .advance_anchor(&mut anchor)
                .expect("modeled initial anchor");
            let initial = completed
                .candidate()
                .records()
                .iter()
                .find_map(|record| {
                    if let OperationCustodyRecord::OperationObservation(record) = record {
                        Some(record.clone())
                    } else {
                        None
                    }
                })
                .expect("actual retained initial observation");
            let candidate = completed.candidate().clone();
            let issued = pending
                .finish(completed)
                .unwrap_or_else(|_| panic!("issued fixture handle"));
            run(&issued, request, parents, &checkpoint, &initial, &candidate)
        },
    )
}

/// Carry an issued operation whose authority records are the cold-open trust set.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn with_issued_context_at_store_and_authorities<R>(
    generation: &crate::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    store: &AuthorityStoreCoordinate,
    trust_ref: &ResidentRecordRefV1,
    trust: &PrincipalTrustCheckpointV1,
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    runtime_ref: &ResidentRecordRefV1,
    runtime: &DeploymentRuntimeAuthorityV1,
    run: impl FnOnce(
        &crate::resident::IssuedLocalOperation,
        &CommonRequestEnvelopeV1,
        RecoveryParents<'_>,
        &ControlStateCheckpointV1,
        &ControlRecord,
        &OperationCustodyCandidate,
    ) -> R,
) -> R {
    crate::local_operation_start_fixture::with_start_context_at_store_and_authorities(
        generation,
        profile,
        suffix,
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        store,
        trust_ref,
        trust,
        binding_ref,
        binding,
        runtime_ref,
        runtime,
        |prepared, request, _, _, parents| {
            let (physical, pending, checkpoint) = issue_initial(
                prepared_initial_at_store(prepared.expect("admitted start"), store),
                request,
                |preimage| {
                    preimage.control_store_id = store.store_id().clone();
                    preimage.control_incarnation_id = store.incarnation_id().clone();
                },
                &Cell::new(0),
            )
            .expect("prepared checkpoint");
            let mut custody = absent_start_store(request).at_store(store);
            let mut anchor = initial_anchor();
            let completed = physical
                .commit(&mut custody, &mut anchor, 1024 * 1024)
                .expect("modeled initial transaction")
                .advance_anchor(&mut anchor)
                .expect("modeled initial anchor");
            let initial = completed
                .candidate()
                .records()
                .iter()
                .find_map(|record| {
                    if let OperationCustodyRecord::OperationObservation(record) = record {
                        Some(record.clone())
                    } else {
                        None
                    }
                })
                .expect("actual retained initial observation");
            let candidate = completed.candidate().clone();
            let issued = pending
                .finish(completed)
                .unwrap_or_else(|_| panic!("issued fixture handle"));
            run(&issued, request, parents, &checkpoint, &initial, &candidate)
        },
    )
}

#[test]
fn local_start_issues_only_the_exact_signed_bytes_after_database_and_anchor() {
    let (generation, profile) = source_generation();
    with_start(
        &generation,
        &profile,
        "exact",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, owners, signatures, _| {
            let prepared = prepared.expect("contextual local start");
            assert_eq!((owners, signatures), (1, 2));
            let original = prepared.records().to_vec();
            let OperationCustodyRecord::CommonRequest(stored_request) = &original[0] else {
                panic!("request role")
            };
            assert_eq!(
                stored_request.coordinate().record_ref(),
                &record_ref("request/exact")
            );
            assert_eq!(
                stored_request.canonical_record_jcs().as_slice(),
                pale_core::canon::to_canonical_string(request)
                    .expect("actual request bytes")
                    .as_bytes()
            );
            let checkpoint_signs = Cell::new(0);
            let (physical, pending, checkpoint) = issue_initial(
                prepared_initial(prepared),
                request,
                |_| {},
                &checkpoint_signs,
            )
            .expect("native association and existing core signer");
            assert_eq!(checkpoint_signs.get(), 1);
            let mut store = absent_start_store(request);
            let mut anchor = initial_anchor();
            let database = physical
                .commit(&mut store, &mut anchor, 1024 * 1024)
                .expect("database custody");
            assert_eq!((store.commits, anchor.advances), (1, 0));
            assert_eq!(database.candidate().records(), original.as_slice());
            assert_eq!(
                database.candidate().kind(),
                OperationCustodyTransitionKind::StartRoot
            );
            assert_eq!(
                database
                    .candidate()
                    .checkpoint()
                    .successor_record()
                    .canonical_record_jcs()
                    .as_slice(),
                pale_core::canon::to_canonical_string(&checkpoint)
                    .unwrap()
                    .as_bytes()
            );
            let completed = database.advance_anchor(&mut anchor).expect("exact anchor");
            let issued = pending
                .finish(completed)
                .unwrap_or_else(|_| panic!("exact custody joins"));
            assert_publication_policy(&issued, request, "exact");
            assert_eq!((store.commits, anchor.advances), (1, 1));
            assert_eq!(issued.handle().0, &record_ref("handle/exact"));
            assert_eq!(
                original[1].record().canonical_record_jcs().as_slice(),
                pale_core::canon::to_canonical_string(issued.handle().1)
                    .expect("issued signed bytes")
                    .as_bytes()
            );
            issued
                .checkpoint_selection()
                .verify(&store_coordinate(), issued.handle().1, &checkpoint)
                .expect("start retains exact profile and original request association");
            // Reuse the original records that this test actually committed. The
            // recovery owner supplies evidence, not a manually minted selection.
            let request_key = operation_request_key(request).unwrap();
            let checkpoint_record =
                project_checkpoint_record(&record_ref("operation/checkpoint-1"), &checkpoint)
                    .unwrap();
            let mut recovery = RecordingStore::recovering(
                [
                    (
                        OperationCustodyIndexQuery::RequestOperation(request_key.clone()),
                        original[1].record().clone(),
                    ),
                    (
                        OperationCustodyIndexQuery::RequestEvidence(request_key.clone()),
                        original[0].record().clone(),
                    ),
                    (
                        OperationCustodyIndexQuery::StartObservation(request_key),
                        original[2].record().clone(),
                    ),
                    (
                        OperationCustodyIndexQuery::ObservationCheckpoint(
                            OperationCustodyIndexKey::for_observation(
                                original[2].record().coordinate(),
                            ),
                        ),
                        checkpoint_record,
                    ),
                ]
                .into_iter()
                .map(|(query, record)| {
                    OperationCustodyIndexObservation::exact(query, record).unwrap()
                })
                .collect(),
            );
            let evidence = crate::resident::operation_custody::read_operation_start_for_request(
                &mut recovery,
                &store_coordinate(),
                request,
                1024 * 1024,
            )
            .unwrap()
            .expect("original start evidence");
            assert!(recovery.index_reads.is_empty());
            let recovered = super::checkpoint_preparation::profile()
                .recover_selection(&evidence, request, issued.handle().1, &checkpoint)
                .expect("same contextually admitted original records");
            recovered
                .verify(&store_coordinate(), issued.handle().1, &checkpoint)
                .unwrap();
            assert_eq!(
                format!("{recovered:?}"),
                "OperationCheckpointSelection { .. }"
            );
            let mut unlike = request.to_preimage();
            unlike.scope.purpose = BoundedResidentTextV1::parse("replacement request").unwrap();
            let unlike = request_fixture::request(&unlike);
            assert_eq!(
                operation_request_key(&unlike).unwrap(),
                operation_request_key(request).unwrap()
            );
            assert!(matches!(
                super::checkpoint_preparation::profile().recover_selection(
                    &evidence,
                    &unlike,
                    issued.handle().1,
                    &checkpoint,
                ),
                Err(crate::resident::OperationCheckpointProfileError::Association)
            ));
        },
    );
}

#[test]
fn resident_operation_service_exposes_start_only_after_database_and_anchor() {
    let (generation, profile) = source_generation();
    with_invoke_start_input(
        &generation,
        &profile,
        "invoke-coordinator",
        |_| {},
        |semantic, request, checkpoint_template, _| {
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let mut store = absent_start_store(request).with_preparation(
                initial_response,
                ControlTransactionSequenceV1::new(1).unwrap(),
            );
            let mut anchor = initial_anchor();
            let mut authority = crate::session_admission_fixture::ProvingAuthority;
            let mut tasks = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).expect("consultation batch bound"),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .unwrap_or_else(|_| panic!("one bounded provider task slot must allocate"));

            let issued = {
                let mut service = ResidentOperationService::new(
                    &mut authority,
                    &mut store,
                    &mut anchor,
                    &mut tasks,
                );
                service
                    .start_invoke(
                        ResidentLocalOperationStartInput {
                            semantic,
                            accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
                                4, 1024, 1024, 512, 65_536,
                            )
                            .unwrap(),
                            checkpoint: ResidentLocalOperationCheckpointInput {
                                id: checkpoint_template.control_checkpoint_id.clone(),
                                reference: record_ref("operation/checkpoint-1"),
                                schema: &checkpoint_schema,
                                authority: checkpoint_authority,
                            },
                            maximum_record_bytes: 1024 * 1024,
                        },
                        |_, _, _, _, _, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                        |_, _, _| Ok(signature()),
                        move |prepared| {
                            let mut preimage = checkpoint_template;
                            preimage.committed_transaction_sequence =
                                prepared.transaction_sequence();
                            preimage.issued_at = UnixTimeV1::new(61).unwrap();
                            preimage.checkpoint_profile_sha256 =
                            crate::resident::SupportedOperationCheckpointProfile::content_digest()
                                .unwrap();
                            preimage.checkpoint_domain_id =
                            crate::resident::SupportedOperationCheckpointProfile::conflict_domain(
                                request,
                            )
                            .unwrap();
                            preimage.state_root_sha256 = prepared.state_root().clone();
                            preimage.unresolved_custody_root_sha256 =
                                prepared.unresolved_root().clone();
                            preimage.changed_leaf_keys = prepared.changed_keys().to_vec();
                            preimage.leaf_count = prepared.leaf_count();
                            Ok(preimage)
                        },
                        |_, _, _| Ok(signature()),
                    )
                    .expect("database and independent anchor must precede the handle")
            };

            assert_eq!((store.commits, anchor.advances), (1, 1));
            assert_eq!(issued.handle().0, &record_ref("handle/invoke-coordinator"));
            assert_eq!(
                issued.handle().1.operation_class(),
                &OperationClassV1::Invoke
            );
        },
    );
}

#[test]
fn resident_local_operation_coordinator_completes_an_already_issued_checkpoint() {
    let (generation, profile) = source_generation();
    with_start(
        &generation,
        &profile,
        "invoke-completion",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, _| {
            let (physical, pending, _) = issue_initial(
                prepared_initial(prepared.expect("contextual local start")),
                request,
                |_| {},
                &Cell::new(0),
            )
            .expect("prepared checkpoint");
            let mut store = absent_start_store(request);
            let mut anchor = initial_anchor();
            let mut authority = crate::session_admission_fixture::ProvingAuthority;

            let issued =
                ResidentLocalOperationCoordinator::new(&mut authority, &mut store, &mut anchor)
                    .complete_start(physical, pending, 1024 * 1024)
                    .expect("physical completion must expose the exact pending handle");

            assert_eq!((store.commits, anchor.advances), (1, 1));
            assert_eq!(issued.handle().0, &record_ref("handle/invoke-completion"));
        },
    );
}

#[test]
fn consult_start_uses_the_same_atomic_locator_custody_without_claiming_a_roster() {
    let (generation, profile) = source_generation();
    with_consult_start(
        &generation,
        &profile,
        "consult-root",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, owners, signatures, _| {
            let prepared = prepared.expect("contextual Consult start");
            assert_eq!((owners, signatures), (1, 2));
            let (physical, pending, _) =
                issue_initial(prepared_initial(prepared), request, |_| {}, &Cell::new(0))
                    .expect("Consult checkpoint");
            let mut store = absent_start_store(request);
            let mut anchor = initial_anchor();
            let completed = physical
                .commit(&mut store, &mut anchor, 1024 * 1024)
                .expect("Consult database custody")
                .advance_anchor(&mut anchor)
                .expect("Consult independent anchor");
            let issued = pending
                .finish(completed)
                .unwrap_or_else(|_| panic!("Consult locator"));
            assert_eq!(
                issued.handle().1.operation_class(),
                &OperationClassV1::Consult
            );
        },
    );
}

#[test]
fn local_start_rejects_independently_valid_foreign_request_axes_before_owner_or_signer() {
    let (generation, profile) = source_generation();
    let changes: &[fn(&mut CommonRequestEnvelopePreimageV1)] = &[
        |request| {
            request.caller.identity = RequestCallerIdV1::parse("other-caller").expect("caller")
        },
        |request| {
            request.workspace_id = ResidentWorkspaceIdV1::parse("default")
                .expect("execution spelling is not workspace")
        },
        |request| {
            request.session_admission.session_grant_id =
                SessionGrantIdV1::parse("other-grant").expect("grant")
        },
        |request| {
            request.session_admission.deployment_binding_ref = record_ref("other-deployment");
            let signed = &request.signing_authority_binding;
            let mut authority = signed
                .integrated_runtime_authority()
                .expect("runtime regime")
                .clone();
            authority.deployment_binding_ref =
                request.session_admission.deployment_binding_ref.clone();
            request.signing_authority_binding = SigningAuthorityBindingV1::integrated_runtime(
                signed.principal_id().clone(),
                ResidentKeyIdV1::parse(signed.signer_key_id()).expect("same signer"),
                *signed.signer_key_epoch(),
                authority,
            )
            .expect("consistent other-deployment authority");
        },
        |request| {
            request.session_admission.session_floor_epoch =
                PrincipalSessionFloorEpochV1::new(2).expect("floor")
        },
        |request| {
            request.session_admission.grant_issuer_key_id =
                ResidentKeyIdV1::parse("other-issuer").expect("issuer")
        },
        |request| {
            request.session_admission.checked_revocation_epoch =
                PrincipalRevocationEpochV1::new(2).expect("revocation epoch")
        },
        |request| {
            request.session_admission.checked_revocation_ref = Some(record_ref("other-revocation"));
            request.session_admission.checked_revocation_sha256 = Some(artifact_digest('e'));
        },
        |request| request.route.route_binding_ref = record_ref("other-route"),
        |request| {
            request.scope.controller_scope =
                ResidentControllerIdV1::parse("other-controller").expect("controller")
        },
        |request| request.capability_id = CapabilityIdV1::parse("write").expect("capability"),
    ];
    for change in changes {
        with_start(
            &generation,
            &profile,
            "foreign",
            change,
            StartAdmission::AcceptedOwnerModel,
            |result, _, owners, signatures, _| {
                assert!(matches!(result, Err(LocalOperationStartError::Coordinate)));
                assert_eq!((owners, signatures), (0, 0));
            },
        );
    }
    with_start(
        &generation,
        &profile,
        "owner-rejects",
        |_| {},
        StartAdmission::RejectedOwnerModel,
        |result, _, owners, signatures, _| {
            assert!(matches!(
                result,
                Err(LocalOperationStartError::Admission(_))
            ));
            assert_eq!((owners, signatures), (1, 0));
        },
    );
}

#[test]
fn local_start_freezes_all_prepared_checkpoint_coordinates_before_signing() {
    let (generation, profile) = source_generation();
    let changes: &[fn(&mut ControlStateCheckpointPreimageV1)] = &[
        // issue_initial derives the witness from this changed preimage too.
        |value| {
            value.issuer_principal_id = ResidentPrincipalIdV1::parse("other-principal").unwrap()
        },
        |value| value.control_store_id = ControlStoreIdV1::parse("other-store").unwrap(),
        |value| {
            value.control_incarnation_id =
                ControlIncarnationIdV1::parse("other-incarnation").unwrap()
        },
        |value| {
            value.checkpoint_domain_id =
                ControlCheckpointDomainIdV1::parse("other:principal-1").unwrap()
        },
        |value| value.checkpoint_profile_ref = record_ref("other-profile"),
        |value| value.checkpoint_profile_sha256 = CheckpointProfileDigest::parse(sha('a')).unwrap(),
        |value| value.domain_epoch = ControlDomainEpochV1::new(2).unwrap(),
        |value| value.domain_sequence = ControlDomainSequenceV1::new(2).unwrap(),
        |value| {
            value.committed_transaction_sequence = ControlTransactionSequenceV1::new(1).unwrap()
        },
        |value| value.state_root_sha256 = artifact_digest('e'),
        |value| value.unresolved_custody_root_sha256 = artifact_digest('e'),
        |value| value.changed_leaf_keys.clear(),
        |value| {
            value.changed_leaf_keys[0].logical_key =
                ControlLogicalKeyV1::parse("other-key").unwrap()
        },
        |value| value.leaf_count = ControlLeafCountV1::new(5).unwrap(),
        |value| value.control_schema_version = 2,
    ];
    for (case, change) in changes.iter().enumerate() {
        with_start(
            &generation,
            &profile,
            "scope",
            |_| {},
            StartAdmission::AcceptedOwnerModel,
            |prepared, request, _, _, _| {
                let called = Cell::new(0);
                let result = issue_initial(
                    prepared_initial(prepared.expect("prepared before checkpoint")),
                    request,
                    change,
                    &called,
                );
                assert!(
                    matches!(
                        result.as_ref().map_err(|failure| failure.reason()),
                        Err(LocalOperationStartError::Coordinate)
                    ),
                    "coordinate case {case}"
                );
                assert_eq!(
                    called.get(),
                    0,
                    "unlike prepared coordinate must not be signed: {case}"
                );
            },
        );
    }
}

#[test]
fn local_start_rejects_same_basis_foreign_changes_and_limits_before_checkpoint_signing() {
    use pale_ports::{
        AccumulatorLeafChange, AccumulatorPreparationLimits, OperationAccumulatorPreparation,
        StateLeafChanges,
    };
    let (generation, profile) = source_generation();
    for different_leaf in [true, false] {
        with_start(
            &generation,
            &profile,
            "response-swap",
            |_| {},
            StartAdmission::AcceptedOwnerModel,
            |prepared, _request, _, _, _| {
                let mut port = super::checkpoint_preparation::PreparationOnly::new(
                    |original: &OperationAccumulatorPreparation| {
                        let mut changes = original.state().changes().to_vec();
                        let mut limits = original.limits();
                        if different_leaf {
                            let old = &changes[0];
                            let other =
                                artifact_digest(if old.successor().unwrap().as_str() == sha('a') {
                                    'b'
                                } else {
                                    'a'
                                });
                            changes[0] =
                                AccumulatorLeafChange::new(old.key().clone(), None, Some(other))
                                    .unwrap();
                        } else {
                            limits = AccumulatorPreparationLimits::new(4, 1025, 1024, 512, 65_536)
                                .unwrap();
                        }
                        let swapped = OperationAccumulatorPreparation::new(
                            original.basis().clone(),
                            StateLeafChanges::new(changes, 4).unwrap(),
                            original.unresolved().clone(),
                            limits,
                            original.implementation(),
                            original.transaction().clone(),
                        )
                        .unwrap();
                        assert_eq!(swapped.basis(), original.basis());
                        assert_ne!(&swapped, original);
                        initial_response(&swapped)
                    },
                    ControlTransactionSequenceV1::new(1).unwrap(),
                );
                let signs = Cell::new(0);
                let result = prepared.unwrap().prepare_checkpoint(
                    AccumulatorPreparationLimits::new(4, 1024, 1024, 512, 65_536).unwrap(),
                    &mut port,
                );
                assert!(matches!(
                    result.as_ref().map_err(|failure| failure.reason()),
                    Err(LocalOperationStartError::Coordinate)
                ));
                assert_eq!(
                    signs.get(),
                    0,
                    "foreign result cannot reach checkpoint signing"
                );
            },
        );
    }
}

#[test]
fn local_start_uncertain_commit_recovers_original_bytes_without_resigning_or_recommit() {
    let (generation, profile) = source_generation();
    with_start(
        &generation,
        &profile,
        "uncertain",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, owners, signatures, _| {
            let prepared = prepared.expect("prepare");
            let original = prepared.records().to_vec();
            let (physical, pending, _) =
                issue_initial(prepared_initial(prepared), request, |_| {}, &Cell::new(0))
                    .expect("initial construction");
            let mut store = absent_start_store(request);
            store.commit_result = Some(Err(ControlStoreError::new(
                ControlStoreErrorKind::OutcomeUnknown,
            )));
            store.read_back_results =
                VecDeque::from([Ok(OperationCustodyReadBackOutcome::ExactSuccessor)]);
            let mut anchor = initial_anchor();
            let database = physical
                .commit(&mut store, &mut anchor, 1024 * 1024)
                .expect("exact readback");
            assert_eq!(
                database.database_evidence(),
                OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit
            );
            assert_eq!(database.candidate().records(), original.as_slice());
            assert_eq!(
                (store.commits, store.read_backs, anchor.advances),
                (1, 1, 0)
            );
            let issued = pending
                .finish(database.advance_anchor(&mut anchor).expect("anchor"))
                .unwrap_or_else(|_| panic!("original custody"));
            assert_eq!(issued.handle().0, &record_ref("handle/uncertain"));
            assert_eq!(
                (owners, signatures, store.commits, anchor.advances),
                (1, 2, 1, 1)
            );
        },
    );
}

#[test]
fn local_start_rejects_other_completed_custody_without_consuming_either_join() {
    let (generation, profile) = source_generation();
    let first = with_start(
        &generation,
        &profile,
        "first",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, _| {
            let (physical, pending, _) = issue_initial(
                prepared_initial(prepared.expect("prepare first")),
                request,
                |_| {},
                &Cell::new(0),
            )
            .expect("initial first");
            let mut store = absent_start_store(request);
            let mut anchor = initial_anchor();
            let completed = physical
                .commit(&mut store, &mut anchor, 1024 * 1024)
                .expect("commit first")
                .advance_anchor(&mut anchor)
                .expect("anchor first");
            (pending, completed)
        },
    );
    with_start(
        &generation,
        &profile,
        "second",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, _| {
            let (physical, second_pending, _) = issue_initial(
                prepared_initial(prepared.expect("prepare second")),
                request,
                |_| {},
                &Cell::new(0),
            )
            .expect("initial second");
            let mut store = absent_start_store(request);
            let mut anchor = initial_anchor();
            let second_completed = physical
                .commit(&mut store, &mut anchor, 1024 * 1024)
                .expect("commit second")
                .advance_anchor(&mut anchor)
                .expect("anchor second");
            let (first_pending, second_completed) = match first.0.finish(second_completed) {
                Ok(_) => panic!("other completed start must not issue first locator"),
                Err(retained) => *retained,
            };
            assert_eq!(
                first_pending
                    .finish(first.1)
                    .unwrap_or_else(|_| panic!("first exact join"))
                    .handle()
                    .0,
                &record_ref("handle/first")
            );
            assert_eq!(
                second_pending
                    .finish(second_completed)
                    .unwrap_or_else(|_| panic!("second exact join"))
                    .handle()
                    .0,
                &record_ref("handle/second")
            );
        },
    );
}

#[test]
fn local_start_requires_native_preparation_before_any_checkpoint_signing() {
    let (generation, profile) = source_generation();
    with_start(
        &generation,
        &profile,
        "no-root-proof",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, _, _, signatures, _| {
            let calls = Cell::new(0);
            let mut port = super::checkpoint_preparation::PreparationOnly::new(
                |_: &pale_ports::OperationAccumulatorPreparation| {
                    calls.set(calls.get() + 1);
                    Err(pale_ports::OperationAccumulatorPreparationError::Store(
                        ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired),
                    ))
                },
                ControlTransactionSequenceV1::new(1).unwrap(),
            );
            let result = prepared.expect("prepare").prepare_checkpoint(
                pale_ports::AccumulatorPreparationLimits::new(4, 1024, 1024, 512, 65_536).unwrap(),
                &mut port,
            );
            assert_eq!(calls.get(), 1);
            assert!(matches!(
                result.as_ref().map_err(|failure| failure.reason()),
                Err(LocalOperationStartError::AccumulatorPreparation(_))
            ));
            let mut retry = super::checkpoint_preparation::PreparationOnly::new(
                |request: &pale_ports::OperationAccumulatorPreparation| {
                    calls.set(calls.get() + 1);
                    initial_response(request)
                },
                ControlTransactionSequenceV1::new(1).unwrap(),
            );
            let failure = match result {
                Err(failure) => failure,
                Ok(_) => panic!("failed native preparation must retain the signed start"),
            };
            let continued = failure
                .continue_checkpoint(
                    pale_ports::AccumulatorPreparationLimits::new(4, 1024, 1024, 512, 65_536)
                        .unwrap(),
                    &mut retry,
                )
                .expect("retry continues the exact retained signed start");
            assert!(continued.predecessor().is_none());
            assert_eq!(
                continued.transaction_sequence(),
                ControlTransactionSequenceV1::new(2).unwrap()
            );
            assert_eq!(calls.get(), 2);
            assert_eq!(signatures, 2, "retry must not reopen either start signer");
        },
    );
}

#[test]
fn checkpoint_signer_failure_retains_the_prepared_checkpoint_for_exact_retry() {
    let (generation, profile) = source_generation();
    with_start(
        &generation,
        &profile,
        "checkpoint-retry",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, signatures, _| {
            let prepared = prepared_initial(prepared.expect("signed start"));
            let preimage = initial_checkpoint_preimage(&prepared, request, |_| {});
            let authority_source = preimage.clone();
            let signs = Cell::new(0);
            let failure = match prepared.issue_checkpoint(
                &record_ref("operation/checkpoint-1"),
                preimage.clone(),
                &authority_source.schema_sha256,
                &checkpoint_authority(&authority_source),
                |_, _, _| {
                    signs.set(signs.get() + 1);
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "test checkpoint signer",
                        invariant: "unavailable",
                    })
                },
            ) {
                Err(failure) => failure,
                Ok(_) => panic!("signer failure must retain the prepared checkpoint"),
            };
            assert!(matches!(
                failure.reason(),
                LocalOperationStartError::Admission(InvalidResidentRecordV1::ContextualAdmission {
                    record: "test checkpoint signer",
                    invariant: "unavailable"
                })
            ));
            let mut unused_custody = super::checkpoint_preparation::PreparationOnly::new(
                |_: &pale_ports::OperationAccumulatorPreparation| {
                    panic!("issued attachment must not repeat native preparation")
                },
                ControlTransactionSequenceV1::new(1).unwrap(),
            );
            let (_prepared_start, _pending) = failure
                .continue_checkpoint(
                    &record_ref("operation/checkpoint-1"),
                    preimage,
                    &authority_source.schema_sha256,
                    &checkpoint_authority(&authority_source),
                    &mut unused_custody,
                    |_, _, _| {
                        signs.set(signs.get() + 1);
                        Ok(signature())
                    },
                )
                .expect("retry signs and attaches the same prepared checkpoint");
            assert_eq!(signs.get(), 2);
            assert_eq!(signatures, 2, "retry must not reopen either start signer");
        },
    );
}

#[test]
fn local_start_keeps_unbound_semantic_only_and_missing_named_provider_distinct() {
    let (generation, profile) =
        source_generation_with(|_| {}, |private| private.operation_bindings.clear());
    with_start(
        &generation,
        &profile,
        "unbound",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |result, _, owners, signs, _| {
            assert!(matches!(
                result,
                Err(LocalOperationStartError::OperationUnbound)
            ));
            assert_eq!((owners, signs), (0, 0));
        },
    );
    let (generation, profile) = source_generation_with(
        |operation| {
            operation.capability_requirements.clear();
            operation.capability_policy = None;
        },
        |_| {},
    );
    with_start(
        &generation,
        &profile,
        "semantic",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |result, _, owners, signs, _| {
            assert!(matches!(
                result,
                Err(LocalOperationStartError::OperationSemanticOnly)
            ));
            assert_eq!((owners, signs), (0, 0));
        },
    );
    let (generation, profile) =
        source_generation_with(|_| {}, |private| private.capability_bindings.clear());
    with_start(
        &generation,
        &profile,
        "no-provider",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |result, _, owners, signs, _| {
            assert_requirement_error(
                result,
                "first",
                "read",
                ProviderUnavailability::MissingBinding,
            );
            assert_eq!((owners, signs), (0, 0));
        },
    );
}

fn assert_requirement_error(
    result: Result<PreparedLocalOperationStart, LocalOperationStartError>,
    requirement: &str,
    capability: &str,
    reason: ProviderUnavailability,
) {
    let Err(LocalOperationStartError::OperationRequirement {
        subject,
        capability: actual,
        disposition,
    }) = result
    else {
        panic!("expected exact requirement failure")
    };
    assert_eq!(subject.operation, operation_id());
    assert_eq!(subject.requirement.as_str(), requirement);
    assert_eq!(actual.as_str(), capability);
    assert_eq!(
        *disposition,
        CapabilityObligationDisposition::Blocked { reason }
    );
}

#[test]
fn local_start_preserves_second_occurrence_and_blocking_sibling_reasons() {
    let (generation, profile) = source_generation_with(
        |operation| {
            operation.capability_requirements[1].effect_class = EffectClass::Write;
        },
        |_| {},
    );
    with_start(
        &generation,
        &profile,
        "second-effect",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |result, _, owners, signs, _| {
            assert_requirement_error(
                result,
                "second",
                "read",
                ProviderUnavailability::EffectMismatch,
            );
            assert_eq!((owners, signs), (0, 0));
        },
    );
    let (generation, profile) = source_generation_with(
        |operation| {
            operation.capability_requirements[1].capability =
                CapabilityIdV1::parse("sibling").expect("capability");
        },
        |_| {},
    );
    with_start(
        &generation,
        &profile,
        "sibling",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |result, _, owners, signs, _| {
            assert_requirement_error(
                result,
                "second",
                "sibling",
                ProviderUnavailability::MissingBinding,
            );
            assert_eq!((owners, signs), (0, 0));
        },
    );
}

#[test]
fn local_start_identical_records_require_the_exact_completed_checkpoint() {
    let (generation, profile) = source_generation();
    let complete = |different_checkpoint: bool| {
        with_start(
            &generation,
            &profile,
            "same",
            |_| {},
            StartAdmission::AcceptedOwnerModel,
            |prepared, request, _, _, _| {
                let prepared = prepared.expect("same prepared records");
                let records = prepared.records().to_vec();
                let (physical, pending, _) = issue_initial(
                    prepared_initial(prepared),
                    request,
                    |checkpoint| {
                        if different_checkpoint {
                            checkpoint.issued_at = UnixTimeV1::new(62).unwrap();
                        }
                    },
                    &Cell::new(0),
                )
                .expect("same prepared roots with a distinct valid signing time");
                let mut store = absent_start_store(request);
                let mut anchor = initial_anchor();
                let completed = physical
                    .commit(&mut store, &mut anchor, 1024 * 1024)
                    .expect("commit")
                    .advance_anchor(&mut anchor)
                    .expect("anchor");
                (records, pending, completed)
            },
        )
    };
    let (first_records, first_pending, first_completed) = complete(false);
    let (second_records, second_pending, second_completed) = complete(true);
    assert_eq!(first_records, second_records, "only checkpoint differs");
    assert_ne!(
        first_completed.candidate().checkpoint(),
        second_completed.candidate().checkpoint()
    );
    let (first_pending, second_completed) = match first_pending.finish(second_completed) {
        Ok(_) => panic!("same locator does not erase unlike completed checkpoint"),
        Err(retained) => *retained,
    };
    let first = first_pending
        .finish(first_completed)
        .unwrap_or_else(|_| panic!("first retained join"));
    let second = second_pending
        .finish(second_completed)
        .unwrap_or_else(|_| panic!("second retained join"));
    assert_eq!(first.handle(), second.handle());
}
