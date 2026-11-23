//! Private session-basis decoding and protected membership over the existing
//! contextual start fixture. Native BLOB bounds and transactions are adapter-owned.

use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) enum Fault {
    Missing,
    Malformed,
    Noncanonical,
    UnknownField,
    Format,
    Store,
    Incarnation,
    Domain,
    Epoch,
    HandleRef,
    HandleDigest,
    RequestRef,
    RequestDigest,
    SessionDigest,
    OuterHandle,
    OuterRequest,
    OuterSession,
    AmbientGrant,
    PolicyTargetScope,
    PolicyClassification,
    PolicyVisibility,
    PolicyDisclosureCeiling,
    PolicyPayloadResolution,
    PolicyActorProvenance,
}

#[test]
fn original_session_basis_requires_closed_exact_association_and_anchored_membership() {
    for fault in [
        Fault::Missing,
        Fault::Malformed,
        Fault::Noncanonical,
        Fault::UnknownField,
        Fault::Format,
        Fault::Store,
        Fault::Incarnation,
        Fault::Domain,
        Fault::Epoch,
        Fault::HandleRef,
        Fault::HandleDigest,
        Fault::RequestRef,
        Fault::RequestDigest,
        Fault::SessionDigest,
        Fault::OuterHandle,
        Fault::OuterRequest,
        Fault::OuterSession,
        Fault::AmbientGrant,
        Fault::PolicyTargetScope,
        Fault::PolicyClassification,
        Fault::PolicyVisibility,
        Fault::PolicyDisclosureCeiling,
        Fault::PolicyPayloadResolution,
        Fault::PolicyActorProvenance,
    ] {
        locator_recovery_cases(&["session-basis"], false, Some(fault));
    }
}

impl Fault {
    pub(super) fn apply(self, store: &mut RecoveryStore) {
        let Some(original) = store.session_basis.take() else {
            panic!("real start must produce a basis")
        };
        if matches!(self, Self::Missing) {
            return;
        }
        let mut body = pale_core::canon::parse_strict(original.canonical_basis_jcs()).unwrap();
        let mut handle = original.handle().clone();
        let mut request = original.request().clone();
        let mut session = original.session_lease_digest().clone();
        match self {
            Self::Format => body["format"] = serde_json::json!("unknown-private-format"),
            Self::Store => body["control_store_id"] = serde_json::json!("other-store"),
            Self::Incarnation => {
                body["control_incarnation_id"] = serde_json::json!("other-incarnation")
            }
            Self::Domain => {
                body["checkpoint_domain_id"] = serde_json::json!("operation:other-domain")
            }
            Self::Epoch => body["domain_epoch"] = serde_json::json!(99),
            Self::HandleRef => body["handle_ref"] = serde_json::json!("other-handle"),
            Self::HandleDigest => body["handle_sha256"] = serde_json::json!(sha('f')),
            Self::RequestRef => body["request_ref"] = serde_json::json!("other-request"),
            Self::RequestDigest => body["request_sha256"] = serde_json::json!(sha('f')),
            Self::SessionDigest => body["session_lease_digest"] = serde_json::json!(sha('f')),
            Self::UnknownField => body["unowned_extension"] = serde_json::json!(true),
            Self::AmbientGrant => {
                body["ambient_basis"]["grant_ref"] = serde_json::json!("grant/substituted")
            }
            Self::PolicyTargetScope => {
                body["publication_policy"]["target_scope"]["environment"] =
                    serde_json::json!("other-environment")
            }
            Self::PolicyClassification => {
                body["publication_policy"]["classification"] = serde_json::json!("restricted")
            }
            Self::PolicyVisibility => {
                body["publication_policy"]["visibility"] = serde_json::json!("public")
            }
            Self::PolicyDisclosureCeiling => {
                body["publication_policy"]["disclosure_ceiling"] = serde_json::json!("protected")
            }
            Self::PolicyPayloadResolution => {
                body["publication_policy"]["payload_resolution_policy"] =
                    serde_json::json!("inline-only")
            }
            Self::PolicyActorProvenance => {
                body["publication_policy"]["actor_provenance"]["record_ref"] =
                    serde_json::json!("actor/substituted")
            }
            Self::OuterHandle => {
                handle = ControlRecordCoordinate::new(
                    record_ref("other-handle"),
                    handle.record_sha256().clone(),
                )
            }
            Self::OuterRequest => {
                request = ControlRecordCoordinate::new(
                    record_ref("other-request"),
                    request.record_sha256().clone(),
                )
            }
            Self::OuterSession => session = SessionAdmissionLeaseDigest::parse(sha('f')).unwrap(),
            Self::Malformed | Self::Noncanonical => {}
            Self::Missing => unreachable!("handled before decoding"),
        }
        let bytes = match self {
            Self::Malformed => b"{".to_vec(),
            Self::Noncanonical => serde_json::to_string_pretty(&body).unwrap().into_bytes(),
            _ => pale_core::canon::to_canonical_string(&body)
                .unwrap()
                .into_bytes(),
        };
        // Recompute the physical digest deliberately. A syntactically valid,
        // differently admitted basis must still fail original-root membership.
        let changed = OperationSessionBasis::new(
            handle,
            request,
            session,
            bytes,
            MAX_OPERATION_SESSION_BASIS_BYTES,
        )
        .unwrap();
        assert_ne!(
            &changed, &original,
            "{self:?} must change selected evidence"
        );
        store.session_basis = Some(changed);
    }

    pub(super) fn expected_stages(self, error: &LocalOperationRecoveryError) -> [usize; 8] {
        let correct = match self {
            Self::Missing => matches!(error, LocalOperationRecoveryError::Missing),
            Self::Malformed => matches!(error, LocalOperationRecoveryError::Canonical(_)),
            Self::UnknownField => {
                matches!(error, LocalOperationRecoveryError::Decode(value) if value.is_data())
            }
            Self::AmbientGrant
            | Self::PolicyTargetScope
            | Self::PolicyClassification
            | Self::PolicyVisibility
            | Self::PolicyDisclosureCeiling
            | Self::PolicyPayloadResolution
            | Self::PolicyActorProvenance => {
                matches!(error, LocalOperationRecoveryError::Membership(
                OperationAccumulatorPreparationError::Store(value)) if value.kind() == ControlStoreErrorKind::RecoveryRequired)
            }
            _ => matches!(error, LocalOperationRecoveryError::Association),
        };
        assert!(correct, "{self:?}: wrong rejection boundary: {error:?}");
        if matches!(
            self,
            Self::AmbientGrant
                | Self::PolicyTargetScope
                | Self::PolicyClassification
                | Self::PolicyVisibility
                | Self::PolicyDisclosureCeiling
                | Self::PolicyPayloadResolution
                | Self::PolicyActorProvenance
        ) {
            [4, 1, 1, 1, 1, 2, 1, 1]
        } else {
            [4, 1, 1, 1, 0, 1, 0, 0]
        }
    }
}
