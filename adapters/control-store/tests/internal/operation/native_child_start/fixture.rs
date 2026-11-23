//! Child fixture independently admitting the same lease under its own grant reference.
//! Signatures and external authority remain the parent fixture's declared models.

use crate::resident_fixture::effect_case_lineage_fixture as request_fixture;
use crate::resident_fixture::local_operation_start_fixture::{
    checkpoint_authority, checkpoint_preimage, operation_id, profile, record_ref, schema_digest,
    signature, RecoveryParents,
};
use crate::resident_fixture::session_admission_fixture::ProvingAuthority;
use pale_application::resident::local_operation_start_test_support::prepare_local_start;
use pale_application::resident::{
    admit_ambient_session, AdmittedAmbientSession, AmbientSessionAdmissionContext,
    CapabilityGrantAuthority, LocalOperationPublicationPolicyInput, LocalOperationStartError,
    LocalOperationStartInput, PreparedLocalChildParent, PreparedLocalOperationStart,
    SupportedOperationCheckpointProfile,
};
use pale_core::records::*;
use std::cell::Cell;

#[derive(Clone, Copy)]
pub(super) enum Admission {
    Generic,
    Native,
    Delegate,
}

/// Reconstructed current authority is a declared fixture model, not recovered
/// permission. The distinct reference is intentionally absent from the lease.
pub(super) fn with_child_session<R>(
    parents: &RecoveryParents<'_>,
    run: impl FnOnce(&AdmittedAmbientSession, &AmbientSessionAdmissionContext<'_>) -> R,
) -> R {
    let original = parents.current;
    let child_grant_ref = record_ref("grant/native-child-independent-basis");
    let current = AmbientSessionAdmissionContext {
        now: UnixTimeV1::new(65).unwrap(),
        subject: original.subject,
        grant: Some(CapabilityGrantAuthority {
            grant_ref: &child_grant_ref,
            ..original.grant.unwrap()
        }),
        current_principal_trust: original.current_principal_trust,
        capability_revocations: original.capability_revocations,
        deployment_binding_ref: original.deployment_binding_ref,
        deployment_binding: original.deployment_binding,
        deployment_revocations: original.deployment_revocations,
        runtime_authority: original.runtime_authority,
        session_revocations: original.session_revocations,
        route_use: original.route_use,
        generation: SessionAdmissionLeaseContextV1 {
            active_head: original.generation.active_head,
            compiled_generation: original.generation.compiled_generation,
            effective_profile: original.generation.effective_profile,
            activation_fence: original.generation.activation_fence,
            signed: original.generation.signed,
        },
    };
    let session = admit_ambient_session(
        SessionAdmissionLeaseWireV1::from(parents.session.lease()),
        &current,
        &mut ProvingAuthority,
    )
    .expect("child independently admits its own non-lease basis");
    assert_eq!(session.lease(), parents.session.lease());
    assert_ne!(session.grant_ref(), parents.session.grant_ref());
    run(&session, &current)
}

pub(super) fn with_child_start<R>(
    fixture: &super::Fixture<'_>,
    admission: Admission,
    prepare_parent: impl FnOnce(&ResidentRecordRefV1, &OperationLeaseV1) -> PreparedLocalChildParent,
    suffix: &str,
    adjust_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        &OperationLeaseV1,
        usize,
        usize,
    ) -> R,
) -> R {
    let generation = fixture.generation;
    let parent = fixture.operation;
    let parent_request = fixture.request;
    let parents = fixture.parents;
    with_child_session(parents, |session, current| {
        let lease = session
            .admit_operation(
                schema_digest(OPERATION_LEASE_V1_SCHEMA_ID),
                OperationLeaseRequestV1 {
                    operation_lease_id: OperationLeaseIdV1::parse(format!("child-lease-{suffix}"))
                        .unwrap(),
                    operation: parents.lease.operation().clone(),
                    capabilities: parents.lease.capabilities().to_vec(),
                    effects: parents.lease.effects().to_vec(),
                    read_set: parents.lease.read_set().to_vec(),
                    evidence_obligations: parents.lease.evidence_obligations().to_vec(),
                    budget_units: *parents.lease.budget_units(),
                    timeout_millis: *parents.lease.timeout_millis(),
                    expires_at: *parent.handle().1.expires_at(),
                    invocation_correlation: parents.lease.invocation_correlation().clone(),
                },
                current,
                &mut ProvingAuthority,
            )
            .expect("actual child lease admits against current exact session");
        lease
            .verify_against_parent_operation(parents.lease)
            .expect("child narrows parent operation");
        let mut request = parent_request.to_preimage();
        request.issued_at = current.now;
        request.session_admission.checked_at = current.now;
        request.request_id = ResidentRequestIdV1::parse(format!("child-request-{suffix}")).unwrap();
        request.correlation_id =
            ResidentCorrelationIdV1::parse(format!("child-correlation-{suffix}")).unwrap();
        adjust_request(&mut request);
        request.body.payload_sha256 = ArtifactDigest::hash_bytes(
            pale_core::canon::to_canonical_string(&request.body.payload)
                .unwrap()
                .as_bytes(),
        );
        let request = request_fixture::request(&request);
        let binding = request.local_request_binding().unwrap();
        let (parent_ref, handle) = parent.handle();
        let lease_ref = record_ref(&format!("child-lease/{suffix}"));
        // Retain the child lease before capturing the parent's mutation fence.
        // A lease write after that capture must invalidate the start, not refresh it.
        let protected = prepare_parent(&lease_ref, &lease);
        let schema = schema_digest(OPERATION_HANDLE_V1_SCHEMA_ID);
        let context = OperationHandleAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: parents.signing,
            },
            request_binding: &binding,
            operation_lease_ref: &lease_ref,
            operation_lease: &lease,
            parent_operation: Some(ParentOperationAdmissionContextV1 {
                operation_handle_ref: parent_ref,
                operation_handle: handle,
                operation_lease: parents.lease,
            }),
            compiled_generation_sha256: generation.record().compiled_generation_digest(),
            activation_fence_sha256: parents.session.lease().activation_fence_digest(),
            instance_sha256: generation.record().instance_digest(),
        };
        let preimage = OperationHandlePreimageV1 {
            schema_id: handle.schema_id().clone(),
            schema_sha256: schema.clone(),
            operation_handle_id: OperationHandleIdV1::parse(format!("child-operation-{suffix}"))
                .unwrap(),
            operation_class: match admission {
                Admission::Delegate => OperationClassV1::Delegate,
                Admission::Generic | Admission::Native => OperationClassV1::Invoke,
            },
            parent_operation: Some(ParentOperationReferenceV1 {
                operation_handle_id: handle.operation_handle_id().clone(),
                operation_handle_ref: parent_ref.clone(),
                operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
            }),
            request_binding: binding.clone(),
            operation_lease_ref: lease_ref.clone(),
            operation_lease_sha256: lease.operation_lease_digest().clone(),
            compiled_generation_sha256: handle.compiled_generation_sha256().clone(),
            activation_fence_sha256: handle.activation_fence_sha256().clone(),
            instance_sha256: handle.instance_sha256().clone(),
            created_at: current.now,
            expires_at: *handle.expires_at(),
            retain_until: *handle.retain_until(),
            deployment_runtime_authority_ref: parents.session.runtime_authority_ref().clone(),
            deployment_runtime_authority_sha256: ArtifactDigest::parse(
                parents.session.runtime_authority_sha256().as_str(),
            )
            .unwrap(),
            issuer_principal_id: parents.signing.principal_id().clone(),
            issuer_key_id: ResidentKeyIdV1::parse(parents.signing.signer_key_id()).unwrap(),
            issuer_key_epoch: *parents.signing.signer_key_epoch(),
        };
        let supported = profile();
        let mut checkpoint = checkpoint_preimage(1, None);
        checkpoint.checkpoint_domain_id = fixture.checkpoint.checkpoint_domain_id.clone();
        checkpoint.checkpoint_profile_sha256 =
            SupportedOperationCheckpointProfile::content_digest().unwrap();
        let owners = Cell::new(0);
        let signs = Cell::new(0);
        let publication_scope = EventScopeV1 {
            principal_scope: request.scope().principal_scope.clone(),
            controller_scope: request.scope().controller_scope.clone(),
            relationship_scope: request.scope().relationship_scope.clone(),
            environment: DeploymentEnvironmentV1::parse(request.scope().environment.as_str())
                .expect("publication environment"),
        };
        let publication_classification =
            DataClassificationV1::parse(request.scope().data_classification.as_str())
                .expect("publication classification");
        let publication_visibility =
            InteractionVisibilityV1::parse("private").expect("publication visibility");
        let publication_disclosure =
            InteractionDisclosureCeilingV1::parse("private").expect("publication disclosure");
        let publication_payload_policy =
            PayloadResolutionPolicyV1::parse("reference-only").expect("payload policy");
        let publication_actor = OwnerRecordCoordinateV1 {
            owner_system_id: OwnerSystemIdV1::parse("pale-test-owner").expect("owner system"),
            record_ref: record_ref(&format!("actor/child-{suffix}")),
            record_schema_id: SchemaId::parse("test/actor.v1").expect("actor schema"),
            record_schema_sha256: schema_digest(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID),
            record_content_sha256: ArtifactDigest::parse("9".repeat(64)).expect("actor digest"),
            native_revision: NativeRevisionV1 {
                native_revision_owner: NativeRevisionOwnerV1::parse("pale-test-owner")
                    .expect("revision owner"),
                native_revision_token: NativeRevisionTokenV1::parse(format!(
                    "actor-revision-child-{suffix}"
                ))
                .expect("revision token"),
            },
        };
        let input = LocalOperationStartInput {
            checkpoint_profile: &supported,
            request_ref: record_ref(&format!("child-request/{suffix}")),
            request: &request,
            handle_ref: record_ref(&format!("child-handle/{suffix}")),
            handle: preimage,
            context: &context,
            session,
            current,
            generation,
            publication_policy: LocalOperationPublicationPolicyInput {
                target_scope: &publication_scope,
                classification: &publication_classification,
                visibility: &publication_visibility,
                disclosure_ceiling: &publication_disclosure,
                payload_resolution_policy: &publication_payload_policy,
                actor_provenance: &publication_actor,
            },
            checkpoint_authority: &checkpoint_authority(&checkpoint),
            observation_ref: record_ref(&format!("child-observation/{suffix}")),
            observation_schema: schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        };
        let admit_owner =
            |actual: &CommonRequestEnvelopeV1,
             _: &AdmittedAmbientSession,
             _: &AmbientSessionAdmissionContext<'_>,
             _: &OperationHandlePreimageV1,
             _: &OperationLeaseV1,
             selected: &pale_application::semantic::ResolvedOperation,
             policy: &LocalOperationPublicationPolicyInput<'_>,
             _: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>| {
                owners.set(owners.get() + 1);
                assert_eq!(actual.body(), request.body());
                assert_eq!(selected.record.id, operation_id());
                assert_eq!(policy.target_scope, &publication_scope);
                assert_eq!(policy.classification, &publication_classification);
                assert_eq!(policy.visibility, &publication_visibility);
                assert_eq!(policy.disclosure_ceiling, &publication_disclosure);
                assert_eq!(
                    policy.payload_resolution_policy,
                    &publication_payload_policy
                );
                assert_eq!(policy.actor_provenance, &publication_actor);
                Ok(())
            };
        let sign_handle = |_: &[u8], _: &OperationHandlePreimageV1, _: &OperationHandleDigest| {
            signs.set(signs.get() + 1);
            Ok(signature())
        };
        let sign_observation =
            |_: &[u8], _: &OperationObservationPreimageV1, _: &OperationObservationDigest| {
                signs.set(signs.get() + 1);
                Ok(signature())
            };
        let (operation_class, registry) = match admission {
            Admission::Generic => (OperationClassV1::Invoke, None),
            Admission::Native => (OperationClassV1::Invoke, Some(fixture.registry)),
            Admission::Delegate => (OperationClassV1::Delegate, None),
        };
        let result = prepare_local_start(
            input,
            Some(protected),
            operation_class,
            registry,
            &mut ProvingAuthority,
            admit_owner,
            sign_handle,
            sign_observation,
        );
        run(result, &request, &lease, owners.get(), signs.get())
    })
}
