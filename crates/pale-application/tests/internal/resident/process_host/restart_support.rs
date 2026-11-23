//! Test-only constructors for recovered operation lineages.

use super::*;

impl ResidentProcessOperationLineage {
    pub(in crate::resident::process_host) fn pairwise_delegate_fixture(
        child_ref: &ResidentRecordRefV1,
        child: &OperationHandleV1,
    ) -> Self {
        let parent = child
            .parent_operation()
            .as_ref()
            .expect("pairwise Delegate fixture has one parent");
        Self::PairwiseDelegate(Box::new(ResidentProcessPairwiseDelegateLineage {
            parent: ImmutableRecordReferenceV1 {
                record_ref: parent.operation_handle_ref.clone(),
                record_sha256: ArtifactDigest::parse(
                    parent.operation_handle_sha256.as_str().to_owned(),
                )
                .expect("pairwise Delegate parent digest"),
            },
            child: ImmutableRecordReferenceV1 {
                record_ref: child_ref.clone(),
                record_sha256: ArtifactDigest::parse(
                    child
                        .canonical_operation_handle_sha256()
                        .as_str()
                        .to_owned(),
                )
                .expect("pairwise Delegate child digest"),
            },
        }))
    }
}

impl ResidentProcessRecoveredOperationResolution {
    pub(in crate::resident::process_host) fn pairwise_fixture(
        selector: ResidentDeploymentAuthoritySelector,
        control: RecoveredOperationControl,
        generation_digest: pale_core::records::CompiledGenerationDigest,
        child_ref: &ResidentRecordRefV1,
        child: &OperationHandleV1,
        phase: ResidentProcessOperationResolutionPhase,
    ) -> Self {
        Self {
            start: ResidentProcessRecoveredOperationStart {
                selector,
                control,
                generation_digest,
                lineage: ResidentProcessOperationLineage::pairwise_delegate_fixture(
                    child_ref, child,
                ),
            },
            phase,
        }
    }
}
