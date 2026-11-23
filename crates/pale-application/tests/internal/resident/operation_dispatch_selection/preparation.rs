//! Preparation preserves the signer boundary; fixture authority is not native dispatch proof.

use super::*;
use crate::local_operation_start_fixture::{
    issue_observation, signature, store_coordinate, RecoveryParents,
};
use crate::resident::operation_dispatch::provider::{
    prepare_provider_resolution, ProviderOperationInput,
};
use crate::resident::operation_dispatch::OperationAttemptContext;
use crate::resident::{
    AmbientSessionAdmissionContext, OperationObservationAuthorityStoreRejection,
    OperationObservationAuthorityStoreResolver,
};
use crate::session_admission_fixture::ProvingAuthority;
use pale_ports::AuthorityStoreCoordinate;
use std::cell::Cell;

struct Store;

impl OperationObservationAuthorityStoreResolver for Store {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(store_coordinate())
    }
}

struct Fixture<'a> {
    selection: &'a AttemptSelection,
    selected: AttemptSelectionContext<'a>,
    admission: ProviderInvocationAdmissionContext<'a>,
    authority: RecoveryParents<'a>,
}

impl Fixture<'_> {
    fn check(&self, case: &str, request: &CommonRequestEnvelopeV1, lease: &OperationLeaseV1) {
        let (handle_ref, handle) = self.selected.operation.handle();
        let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let initial_ref = record_ref("attempt-initial");
        let initial = issue_observation(
            self.authority.signing,
            handle_ref,
            handle,
            schema.clone(),
            OperationObservationRevisionV1::new(1).unwrap(),
            None,
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving,
            },
            60,
        );
        let state = OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        };
        let predecessor_ref = record_ref("attempt-provider-resolved");
        let foreign_handle_ref = record_ref("other-handle");
        let predecessor_handle_ref = if case == "foreign-predecessor" {
            &foreign_handle_ref
        } else {
            handle_ref
        };
        let predecessor_initial = issue_observation(
            self.authority.signing,
            predecessor_handle_ref,
            handle,
            schema.clone(),
            OperationObservationRevisionV1::new(1).unwrap(),
            None,
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving,
            },
            60,
        );
        let predecessor = issue_observation(
            self.authority.signing,
            predecessor_handle_ref,
            handle,
            schema.clone(),
            OperationObservationRevisionV1::new(2).unwrap(),
            Some((
                &initial_ref,
                if case == "foreign-predecessor" {
                    &predecessor_initial
                } else {
                    &initial
                },
            )),
            state,
            60,
        );
        let observation = OperationObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: self.authority.signing,
            },
            handle_ref,
            handle,
            predecessor: Some((&predecessor_ref, &predecessor)),
        };
        let original = self.authority.current;
        let mut current = AmbientSessionAdmissionContext {
            now: original.now,
            subject: original.subject,
            grant: original.grant,
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
        match case {
            "expired-session" => current.now = UnixTimeV1::new(151).unwrap(),
            "expired-handle" => current.now = *handle.expires_at(),
            "expired-lease" => current.now = lease.expires_at(),
            "expired-request" => current.now = request.expires_at(),
            "missing-grant" => current.grant = None,
            "missing-runtime" => current.runtime_authority = None,
            _ => {}
        }
        let selection_ref = if case == "request-alias" {
            self.selected
                .operation
                .request_coordinate()
                .record_ref()
                .clone()
        } else {
            record_ref("attempt-selection")
        };
        let signatures = Cell::new(0);
        let invocation = self.selection.provider_request(self.selected).unwrap();
        let prepared = prepare_provider_resolution(
            ProviderOperationInput {
                context: OperationAttemptContext {
                    operation: self.selected.operation,
                    request,
                    session: self.authority.session,
                    current: &current,
                    lease,
                    context_binding: None,
                    generation: self.selected.generation,
                    selection_ref,
                    maximum_selection_bytes: if case == "bound" { 1 } else { 65_536 },
                    observation_ref: record_ref("attempt-prepared"),
                    observation,
                },
                invocation,
                result_admission: self.admission,
                resolution_ref: record_ref("attempt/provider-resolution"),
            },
            &mut ProvingAuthority,
            |_, _, _, _| panic!("unchanged runtime does not request rotation"),
            |domain, preimage, digest| {
                signatures.set(signatures.get() + 1);
                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                assert_eq!(
                    digest,
                    &OperationObservationDigest::hash_canonical(preimage).unwrap()
                );
                Ok(signature())
            },
        );
        if case == "exact" {
            let prepared = prepared.unwrap();
            assert_eq!(signatures.get(), 1);
            let (reference, observation) = prepared.observation();
            assert_eq!(reference, &record_ref("attempt/provider-resolution"));
            assert_eq!(observation.revision().get(), 3);
            assert_eq!(
                observation.predecessor_ref().as_ref(),
                Some(&predecessor_ref)
            );
            assert_eq!(
                observation.state(),
                &OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ProviderResolved {
                        provider_binding: self.selection.provider().unwrap().clone(),
                    },
                }
            );
            let append = prepared.prepare_append(&Store).unwrap();
            assert_eq!(append.successor().1, observation);
            assert_eq!(append.associated_record(), None);
        } else {
            assert!(prepared.is_err(), "{case} must reject before signing");
            assert_eq!(signatures.get(), 0, "{case}");
        }
    }
}

#[test]
fn resolution_signs_only_an_exact_current_bounded_provider_selection() {
    with_selection_custody_context(
        crate::session_admission_fixture::LocalStartContext::SelfContained,
        |selection, selected, admission, authority, _, _| {
            let fixture = Fixture {
                selection: &selection,
                selected,
                admission,
                authority,
            };
            for case in [
                "exact",
                "expired-session",
                "expired-handle",
                "expired-lease",
                "expired-request",
                "missing-grant",
                "missing-runtime",
                "request-alias",
                "bound",
                "foreign-predecessor",
            ] {
                fixture.check(case, selected.request, selected.lease);
            }
        },
    );
}

#[test]
fn preparation_rejects_a_different_admitted_request_or_child_lease_before_signing() {
    with_selection_custody_context(
        crate::session_admission_fixture::LocalStartContext::SelfContained,
        |selection, selected, admission, authority, _, _| {
            let fixture = Fixture {
                selection: &selection,
                selected,
                admission,
                authority,
            };
            let (generation, profile) = host_generation("other_compute");
            local_start::with_issued(
                &generation,
                &profile,
                "foreign-attempt",
                |_, request, parents| {
                    assert_ne!(
                        request.canonical_request_sha256(),
                        selected.request.canonical_request_sha256()
                    );
                    assert_ne!(
                        parents.lease.operation_lease_digest(),
                        selected.lease.operation_lease_digest()
                    );
                    fixture.check("foreign-request", request, selected.lease);
                    fixture.check("foreign-lease", selected.request, parents.lease);
                },
            );
        },
    );
}
