//! Exercises explicit publisher and protected-delivery evidence at workspace-universe admission.

use super::*;

impl WorkspaceAdmissionProofs {
    pub(in crate::workspace) fn synthetic_for(verified: &VerifiedWorkspaceSemanticIr) -> Self {
        let admitted = verified
            .admission_requirements
            .publishers
            .iter()
            .cloned()
            .map(|coordinate| {
                let bytes = pale_core::canon::to_canonical_string(&coordinate)
                    .expect("synthetic publisher coordinate is canonical");
                (
                    coordinate.clone(),
                    PublisherAdmissionEvidence {
                        coordinate,
                        proof_digest: WorkspacePublisherProofDigest::hash_bytes(bytes.as_bytes()),
                    },
                )
            })
            .collect();
        let protected_delivery = verified
            .admission_requirements
            .protected_delivery
            .iter()
            .cloned()
            .map(|coordinate| {
                let bytes = pale_core::canon::to_canonical_string(&coordinate)
                    .expect("synthetic protected coordinate is canonical");
                (
                    coordinate.clone(),
                    ProtectedDeliveryEvidence {
                        coordinate,
                        proof_digest: WorkspaceProtectedDeliveryProofDigest::hash_bytes(
                            bytes.as_bytes(),
                        ),
                    },
                )
            })
            .collect();
        Self {
            admitted,
            protected_delivery,
        }
    }

    pub(in crate::workspace) fn clear_protected_delivery(&mut self) {
        self.protected_delivery.clear();
    }

    pub(in crate::workspace) fn transpose_protected_authority(
        &mut self,
        authority: pale_core::records::WorkspaceResourceId,
    ) {
        self.protected_delivery = self
            .protected_delivery
            .values()
            .cloned()
            .map(|mut evidence| {
                evidence.coordinate.protected_delivery_authority = Some(authority.clone());
                (evidence.coordinate.clone(), evidence)
            })
            .collect();
    }

    pub(in crate::workspace) fn reissue_publisher_proofs(&mut self) {
        for evidence in self.admitted.values_mut() {
            evidence.proof_digest = WorkspacePublisherProofDigest::hash_bytes(
                format!("reissued:{}", evidence.proof_digest).as_bytes(),
            );
        }
    }
}

impl VerifiedWorkspaceSemanticIr {
    /// Borrow the verified semantic identity for focused pre-admission tests.
    pub(in crate::workspace) fn digest(&self) -> &WorkspaceSemanticIrDigest {
        &self.digest
    }
}

impl PrincipalAdmittedUniverse {
    /// Return the number of declarations retained in the path-free semantic IR.
    pub(in crate::workspace) fn semantic_declaration_count(&self) -> usize {
        self.workspace_semantic_ir.preimage.declarations.len()
    }

    /// Verify a synthetic receipt against both admitted semantic owners.
    pub(in crate::workspace) fn verify_derivation_receipt(
        &self,
        prior: &ResidentWorkspaceDerivation,
        receipt: &WorkspaceDerivationReceiptV1,
    ) -> Result<(), WorkspaceError> {
        receipt
            .verify()
            .map_err(|error| WorkspaceError::InvalidDerivationReceipt(Box::new(error)))?;
        let expected = build_derivation_receipt(
            &prior.frontier,
            &self.workspace_semantic_ir.derivation_frontier,
            &self.workspace_semantic_ir.digest,
        )?;
        if receipt == &expected {
            Ok(())
        } else {
            Err(WorkspaceError::DerivationSemanticMismatch)
        }
    }

    pub(in crate::workspace) fn admission_evidence_custody_is_complete(&self) -> bool {
        self.admission_evidence
            .custody_is_complete()
            .expect("admitted proof evidence remains canonical")
    }

    pub(in crate::workspace) fn corrupt_admission_evidence_digest(&mut self) {
        self.admission_evidence.digest = WorkspaceAdmissionEvidenceDigest::hash_canonical(&(
            "synthetic-corrupt-workspace-admission-evidence",
            &self.admission_evidence.preimage,
        ))
        .expect("synthetic corrupted evidence coordinate is canonical");
    }
}
