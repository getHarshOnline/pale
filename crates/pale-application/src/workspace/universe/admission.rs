//! Publisher-gated proof admission and principal-universe sealing.

use super::derivation::{build_derivation_receipt, WorkspaceDerivationFrontier};
use super::ir::{
    AdmittedWorkspaceSemanticIr, VerifiedWorkspaceSemanticIr, WorkspaceSemanticResourceOccurrence,
};
use super::*;

/// Exact publisher frontier proved outside the workspace compiler.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct PublisherAdmissionCoordinate {
    module_id: WorkspaceModuleId,
    release_id: WorkspaceReleaseId,
    module_digest: WorkspaceModuleDigest,
    publisher_id: WorkspacePublisherId,
    publisher_root_key_id: WorkspacePublisherKeyId,
    recovery_authority_id: WorkspaceRecoveryAuthorityId,
    delegated_key_id: WorkspacePublisherKeyId,
    release_state: WorkspaceReleaseStateV1,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
/// Exact protected-delivery path bound into admission evidence.
struct AdmissionPathCoordinate {
    importer_module: WorkspaceModuleId,
    upstream_module: WorkspaceModuleId,
    upstream_release: WorkspaceReleaseId,
    upstream_module_digest: WorkspaceModuleDigest,
    publisher_id: WorkspacePublisherId,
    publisher_root_key_id: WorkspacePublisherKeyId,
    recovery_authority_id: WorkspaceRecoveryAuthorityId,
    delegated_key_id: WorkspacePublisherKeyId,
    export_id: pale_core::records::WorkspaceExportId,
    source_namespace: pale_core::records::WorkspaceNamespaceId,
    local_namespace: pale_core::records::WorkspaceNamespaceId,
    protected_delivery_authority: Option<pale_core::records::WorkspaceResourceId>,
    effective_surface_digest: WorkspaceImportSurfaceDigest,
}

impl AdmissionPathCoordinate {
    fn from_path(path: &ResolvedImportPath) -> Result<Self, WorkspaceError> {
        Ok(Self {
            importer_module: path.importer_module.clone(),
            upstream_module: path.upstream_module.clone(),
            upstream_release: path.upstream_release.clone(),
            upstream_module_digest: path.upstream_module_digest.clone(),
            publisher_id: path.upstream_publisher.publisher_id.clone(),
            publisher_root_key_id: path.upstream_publisher.root_key_id.clone(),
            recovery_authority_id: path.upstream_publisher.recovery_authority_id.clone(),
            delegated_key_id: path.upstream_publisher.delegated_release_key_id.clone(),
            export_id: path.export_id.clone(),
            source_namespace: path.source_namespace.clone(),
            local_namespace: path.local_namespace.clone(),
            protected_delivery_authority: path.protected_delivery_authority.clone(),
            effective_surface_digest: WorkspaceImportSurfaceDigest::hash_canonical(&(
                "pale.workspace-import-surface.v1",
                path,
            ))?,
        })
    }
}

impl std::fmt::Display for AdmissionPathCoordinate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}:{}:{}:{}->{}",
            self.upstream_module,
            self.upstream_release,
            self.export_id,
            self.source_namespace,
            self.local_namespace
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Opaque publisher proof bound to its exact release coordinate.
struct PublisherAdmissionEvidence {
    coordinate: PublisherAdmissionCoordinate,
    proof_digest: WorkspacePublisherProofDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Opaque protected-delivery proof bound to its exact import path.
struct ProtectedDeliveryEvidence {
    coordinate: AdmissionPathCoordinate,
    proof_digest: WorkspaceProtectedDeliveryProofDigest,
}

/// Two disjoint opaque evidence sets. Only test code can currently construct them.
#[derive(Debug, Default)]
pub(in crate::workspace) struct WorkspaceAdmissionProofs {
    admitted: BTreeMap<PublisherAdmissionCoordinate, PublisherAdmissionEvidence>,
    protected_delivery: BTreeMap<AdmissionPathCoordinate, ProtectedDeliveryEvidence>,
}

#[derive(Debug)]
/// Exact proof coordinates retained while the loaded source graph is dropped.
pub(super) struct WorkspaceAdmissionRequirements {
    publishers: Vec<PublisherAdmissionCoordinate>,
    protected_delivery: Vec<AdmissionPathCoordinate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Canonical evidence set consumed by one successful admission.
struct WorkspaceAdmissionEvidencePreimage {
    publisher: Vec<PublisherAdmissionEvidence>,
    protected_delivery: Vec<ProtectedDeliveryEvidence>,
}

/// Exact proof frontier consumed by one successful admission.
#[derive(Debug)]
pub(crate) struct WorkspaceAdmissionEvidence {
    preimage: WorkspaceAdmissionEvidencePreimage,
    digest: WorkspaceAdmissionEvidenceDigest,
}

/// Principal-relative admitted universe. This is desired-state admission only.
#[derive(Debug)]
pub(crate) struct PrincipalAdmittedUniverse {
    digest: PrincipalAdmittedUniverseDigest,
    workspace_semantic_ir: AdmittedWorkspaceSemanticIr,
    admission_evidence: WorkspaceAdmissionEvidence,
}

/// Exact immutable causal-proof evidence retained by one resident generation.
///
/// [`WorkspaceDerivationNodeV1::inputs`] remains part of the admitted public
/// proof DAG so the next transition receipt can reproduce prior/current
/// semantics. It is not a second operational or alignment adjacency: resident
/// lookup and invalidation traverse only the packed definition graph, while
/// receipt comparison builds and discards a bounded proof index.
#[derive(Debug)]
pub(crate) struct ResidentWorkspaceDerivation {
    frontier: WorkspaceDerivationFrontier,
}

/// Workspace definitions consumed by the resident definition store.
pub(crate) struct ResidentWorkspaceParts {
    pub(crate) principal_digest: PrincipalAdmittedUniverseDigest,
    pub(crate) admission_evidence: WorkspaceAdmissionEvidence,
    pub(crate) derivation: ResidentWorkspaceDerivation,
    pub(crate) declarations: Vec<super::ir::WorkspaceSemanticDeclarationEntry>,
    pub(crate) composition: ExperienceCompositionDeclarationsV1,
    pub(crate) definition: ResidentWorkspaceDefinition,
}

/// Admitted workspace fields not lowered into declaration or composition nodes.
///
/// This residual owns the remaining semantic bodies exactly once. It carries no
/// live dependency adjacency; operational causal queries use the resident
/// packed graph. Retained compiler proofs have their own query owner.
pub(crate) struct ResidentWorkspaceDefinition {
    domain: &'static str,
    digest: WorkspaceSemanticIrDigest,
    workspace_id: WorkspaceId,
    adopted_releases: Vec<WorkspaceAdoptedReleaseV1>,
    responsibility_derivations: ResponsibilityDerivationIrV1,
    organizational_translation: Option<OrganizationalTranslationIrV1>,
    extensions: Vec<LinkedExtension>,
    resolved_imports: Vec<ResolvedImportPath>,
    access_entries: Vec<LinkedAccessEntry>,
    resources: Vec<WorkspaceSemanticResourceOccurrence>,
    package_releases: Vec<PackageReleaseV1>,
    normative_pack_releases: Vec<NormativePackReleaseV1>,
    exclusive_authority: Vec<pale_core::records::WorkspaceExclusiveAuthorityClaimV1>,
}

impl PrincipalAdmittedUniverse {
    /// Select one existing record without exposing the private semantic preimage.
    pub(in crate::workspace) fn organization_record_at(
        &self,
        position: super::super::organization_inspection::OrganizationPosition,
    ) -> Result<
        Option<super::super::OrganizationInspectionRecord<'_>>,
        super::super::OrganizationInspectionError,
    > {
        super::super::organization_inspection::record_at(
            self.workspace_semantic_ir
                .preimage
                .organizational_translation
                .as_ref(),
            &self
                .workspace_semantic_ir
                .preimage
                .responsibility_derivations,
            position,
        )
    }

    /// Optional source selection is distinct from a selected empty translation.
    pub(in crate::workspace) fn organization_selected(&self) -> bool {
        self.workspace_semantic_ir
            .preimage
            .organizational_translation
            .is_some()
    }

    /// Borrow the principal workspace identity admitted into this universe.
    pub(crate) fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_semantic_ir.preimage.workspace_id
    }

    /// Borrow the admitted universe identity.
    pub(crate) fn digest(&self) -> &PrincipalAdmittedUniverseDigest {
        &self.digest
    }

    /// Borrow the verified workspace semantic identity.
    pub(crate) fn workspace_semantic_ir_digest(&self) -> &WorkspaceSemanticIrDigest {
        &self.workspace_semantic_ir.digest
    }

    /// Borrow the exact causal evidence-frontier identity consumed at admission.
    pub(crate) fn admission_evidence_digest(&self) -> &WorkspaceAdmissionEvidenceDigest {
        &self.admission_evidence.digest
    }

    /// Return the number of exact proof records retained for causal custody.
    pub(crate) fn admission_evidence_count(&self) -> usize {
        self.admission_evidence.preimage.publisher.len()
            + self.admission_evidence.preimage.protected_delivery.len()
    }

    /// Borrow the exact release-resolution lock admitted into the universe.
    pub(crate) fn resolution_lock_digest(&self) -> &WorkspaceResolutionLockDigest {
        &self
            .workspace_semantic_ir
            .derivation_frontier
            .resolution_digest
    }

    /// Borrow the exact principal-relative adoption lock admitted into the universe.
    pub(crate) fn universe_lock_digest(&self) -> &WorkspaceUniverseLockDigest {
        &self
            .workspace_semantic_ir
            .derivation_frontier
            .universe_digest
    }

    /// Borrow the canonically lowered responsibility-set identity.
    pub(crate) fn responsibility_derivation_closure_digest(
        &self,
    ) -> &ResponsibilityDerivationClosureDigest {
        &self
            .workspace_semantic_ir
            .derivation_frontier
            .responsibility_digest
    }

    /// Borrow the canonical optional organizational-translation identity.
    pub(crate) fn organizational_translation_closure_digest(
        &self,
    ) -> &OrganizationalTranslationClosureDigest {
        &self
            .workspace_semantic_ir
            .derivation_frontier
            .organizational_digest
    }

    /// Borrow the explicit neutral declarations admitted from the workspace root.
    pub(crate) fn experience_composition(&self) -> &ExperienceCompositionDeclarationsV1 {
        &self.workspace_semantic_ir.preimage.experience_composition
    }

    /// Borrow the identity of the explicit neutral composition source closure.
    pub(crate) fn experience_composition_source_digest(
        &self,
    ) -> &ExperienceCompositionSourceDigest {
        &self
            .workspace_semantic_ir
            .derivation_frontier
            .composition_digest
    }

    /// Borrow the exact source coordinates retained by the derivation frontier.
    pub(crate) fn derivation_sources(&self) -> &[WorkspaceDerivationSourceV1] {
        &self.workspace_semantic_ir.derivation_frontier.sources
    }

    /// Borrow the canonical causal nodes retained by the derivation frontier.
    pub(crate) fn derivation_nodes(&self) -> &[WorkspaceDerivationNodeV1] {
        &self.workspace_semantic_ir.derivation_frontier.nodes
    }

    /// Borrow the exact root/module release closure selected by the universe.
    pub(crate) fn adopted_releases(&self) -> &[WorkspaceAdoptedReleaseV1] {
        &self.workspace_semantic_ir.preimage.adopted_releases
    }

    /// Borrow Package releases selected through workspace logical resources.
    pub(crate) fn package_releases(&self) -> &[PackageReleaseV1] {
        &self.workspace_semantic_ir.preimage.package_releases
    }

    /// Borrow attestor-scoped Pack releases selected through workspace resources.
    pub(crate) fn normative_pack_releases(&self) -> &[NormativePackReleaseV1] {
        &self.workspace_semantic_ir.preimage.normative_pack_releases
    }

    /// Derive and semantically admit a causal receipt from a prior universe.
    ///
    /// Unlike [`WorkspaceDerivationReceiptV1::verify`], successful return proves
    /// equality with the exact predecessor and current admitted frontiers,
    /// including removed-root reachability.
    pub(crate) fn derivation_receipt(
        &self,
        prior: &ResidentWorkspaceDerivation,
    ) -> Result<WorkspaceDerivationReceiptV1, WorkspaceError> {
        self.derivation_receipt_from_frontier(&prior.frontier)
    }

    /// Derive a causal receipt between two admitted portable universes.
    pub(crate) fn portable_derivation_receipt(
        &self,
        prior: &Self,
    ) -> Result<WorkspaceDerivationReceiptV1, WorkspaceError> {
        self.derivation_receipt_from_frontier(&prior.workspace_semantic_ir.derivation_frontier)
    }

    /// Test membership in the retained selected-proof map without deriving a receipt.
    pub(in crate::workspace) fn contains_derivation_subject(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> bool {
        self.workspace_semantic_ir
            .derivation_frontier
            .facts
            .contains_key(subject)
    }

    /// Compare one subject using the two retained compiler fact indexes.
    pub(in crate::workspace) fn compare_portable_subject<'a>(
        &'a self,
        prior: &'a Self,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> super::super::portable::PortableWorkspaceSubjectComparison<'a> {
        super::derivation::compare_derivation_subject(
            &prior.workspace_semantic_ir.derivation_frontier,
            &self.workspace_semantic_ir.derivation_frontier,
            subject,
        )
    }

    /// Merge selected subject keys from both retained fact maps.
    pub(in crate::workspace) fn comparison_subjects<'a>(
        &'a self,
        prior: &'a Self,
        after: Option<&WorkspaceDerivationSubjectV1>,
        requested_limit: u16,
    ) -> super::super::PortableWorkspaceComparisonSubjectPage<'a> {
        super::derivation::comparison_subject_page(
            &prior.workspace_semantic_ir.derivation_frontier,
            &self.workspace_semantic_ir.derivation_frontier,
            after,
            requested_limit,
        )
    }

    /// Borrow one selected root without reading its direct inputs.
    pub(in crate::workspace) fn subject_node(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> Option<&WorkspaceDerivationNodeV1> {
        super::derivation::subject_node(&self.workspace_semantic_ir.derivation_frontier, subject)
    }

    /// Read direct proof inputs from this admitted compiler frontier.
    pub(in crate::workspace) fn direct_dependency_page(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        offset: usize,
        limit: u16,
    ) -> Result<
        Option<super::super::PortableWorkspaceDependencyPage<'_>>,
        super::super::InvalidDependencyOffset,
    > {
        super::derivation::direct_dependency_page(
            &self.workspace_semantic_ir.derivation_frontier,
            subject,
            offset,
            limit,
        )
    }

    /// Scan a bounded candidate slice for direct consumers of one selected fact.
    pub(in crate::workspace) fn direct_dependent_page(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        offset: usize,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<
        Option<super::super::PortableWorkspaceDependentPage<'_>>,
        super::super::InvalidDependentOffset,
    > {
        super::derivation::direct_dependent_page(
            &self.workspace_semantic_ir.derivation_frontier,
            subject,
            offset,
            maximum_inspected,
            maximum_returned,
        )
    }

    /// Explain one receipt subject against the exact admitted portable owners.
    pub(in crate::workspace) fn explain_portable_derivation(
        &self,
        prior: &Self,
        receipt: &WorkspaceDerivationReceiptV1,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> Result<super::derivation::WorkspaceDerivationExplanation, WorkspaceError> {
        super::derivation::explain_derivation_subject(
            &prior.workspace_semantic_ir.derivation_frontier,
            &self.workspace_semantic_ir.derivation_frontier,
            receipt,
            subject,
        )
    }

    fn derivation_receipt_from_frontier(
        &self,
        prior: &WorkspaceDerivationFrontier,
    ) -> Result<WorkspaceDerivationReceiptV1, WorkspaceError> {
        build_derivation_receipt(
            prior,
            &self.workspace_semantic_ir.derivation_frontier,
            &self.workspace_semantic_ir.digest,
        )
    }

    /// Consume admitted workspace bodies into the resident generation boundary.
    pub(crate) fn into_resident_parts(self) -> ResidentWorkspaceParts {
        let AdmittedWorkspaceSemanticIr {
            mut preimage,
            digest,
            derivation_frontier,
        } = self.workspace_semantic_ir;
        let declarations = std::mem::take(&mut preimage.declarations);
        let composition = std::mem::replace(
            &mut preimage.experience_composition,
            ExperienceCompositionDeclarationsV1::empty(),
        );
        let super::ir::WorkspaceSemanticIrPreimage {
            domain,
            workspace_id,
            adopted_releases,
            responsibility_derivations,
            organizational_translation,
            experience_composition: _,
            declarations: _,
            extensions,
            resolved_imports,
            access_entries,
            resources,
            package_releases,
            normative_pack_releases,
            exclusive_authority,
        } = preimage;
        ResidentWorkspaceParts {
            principal_digest: self.digest,
            admission_evidence: self.admission_evidence,
            derivation: ResidentWorkspaceDerivation {
                frontier: derivation_frontier,
            },
            declarations,
            composition,
            definition: ResidentWorkspaceDefinition {
                domain,
                digest,
                workspace_id,
                adopted_releases,
                responsibility_derivations,
                organizational_translation,
                extensions,
                resolved_imports,
                access_entries,
                resources,
                package_releases,
                normative_pack_releases,
                exclusive_authority,
            },
        }
    }
}

impl WorkspaceAdmissionEvidence {
    /// Borrow the exact canonical identity of the retained proof frontier.
    pub(crate) fn digest(&self) -> &WorkspaceAdmissionEvidenceDigest {
        &self.digest
    }

    /// Verify that the bounded retained proof preimage reproduces its admitted identity.
    ///
    /// The collection bounds defend resident memory, while canonical reproduction
    /// proves the retained publisher and protected-delivery evidence is the exact
    /// preimage admitted under [`Self::digest`]. A bounded collection alone is not
    /// evidence integrity.
    pub(crate) fn custody_is_complete(&self) -> Result<bool, pale_core::canon::CanonError> {
        if self.preimage.publisher.len() > 1_024 || self.preimage.protected_delivery.len() > 4_096 {
            return Ok(false);
        }
        let reproduced = WorkspaceAdmissionEvidenceDigest::hash_canonical(&(
            "pale.workspace-admission-evidence.v1",
            &self.preimage,
        ))?;
        Ok(reproduced == self.digest)
    }
}

impl ResidentWorkspaceDefinition {
    /// Borrow the admitted identity of this complete residual body.
    pub(crate) fn digest(&self) -> &WorkspaceSemanticIrDigest {
        &self.digest
    }

    /// Verify that all fixed and collection-bearing residual fields remain owned.
    pub(crate) fn custody_is_complete(&self) -> bool {
        self.domain == "pale.workspace-semantic-ir.v1"
            && !self.workspace_id.as_str().is_empty()
            && self.adopted_releases.len() <= 1_024
            && self.extensions.len() <= 1_024
            && self.resolved_imports.len() <= 1_024
            && self.access_entries.len() <= 4_096
            && self.resources.len() <= 4_096
            && self.package_releases.len() <= 1_024
            && self.normative_pack_releases.len() <= 1_024
            && self.exclusive_authority.len() <= 1_024
            && self.responsibility_derivations.derivations.len() <= 1_024
            && self
                .organizational_translation
                .as_ref()
                .is_none_or(|translation| translation.derived_relations.len() <= 4_096)
    }
}

impl ResidentWorkspaceDerivation {
    /// Borrow exact admitted source coordinates retained for receipt proof.
    pub(crate) fn sources(&self) -> &[WorkspaceDerivationSourceV1] {
        &self.frontier.sources
    }

    /// Borrow the exact admitted causal facts used to map successor impacts.
    pub(crate) fn causal_nodes(&self) -> &[WorkspaceDerivationNodeV1] {
        &self.frontier.nodes
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/workspace/universe/admission.rs"]
mod test_support;

pub(super) fn admission_requirements_from(
    normalized: &NormalizedWorkspace,
) -> Result<WorkspaceAdmissionRequirements, WorkspaceError> {
    let mut publishers = normalized
        .linked
        .adopted_modules
        .iter()
        .map(|module_id| {
            let module = &normalized.linked.source.modules[module_id].value;
            let release = normalized
                .linked
                .source
                .resolution_lock
                .value
                .releases
                .iter()
                .find(|release| &release.module_id == module_id)
                .expect("linked adopted release is resolved");
            PublisherAdmissionCoordinate {
                module_id: module.module.module_id.clone(),
                release_id: module.module.release_id.clone(),
                module_digest: module.module_digest.clone(),
                publisher_id: module.module.publisher.publisher_id.clone(),
                publisher_root_key_id: module.module.publisher.root_key_id.clone(),
                recovery_authority_id: module.module.publisher.recovery_authority_id.clone(),
                delegated_key_id: module.module.publisher.delegated_release_key_id.clone(),
                release_state: release.release_state,
            }
        })
        .collect::<Vec<_>>();
    publishers.sort();
    publishers.dedup();
    let mut protected_delivery = normalized
        .linked
        .resolved_imports
        .iter()
        .filter(|path| {
            path.export_visibility == pale_core::records::WorkspaceVisibilityV1::Protected
        })
        .map(AdmissionPathCoordinate::from_path)
        .collect::<Result<Vec<_>, _>>()?;
    protected_delivery.sort();
    protected_delivery.dedup();
    Ok(WorkspaceAdmissionRequirements {
        publishers,
        protected_delivery,
    })
}

impl VerifiedWorkspaceSemanticIr {
    /// Consume verified semantics using exact publisher-release and protected-delivery evidence.
    ///
    /// Evidence must be present in [`WorkspaceAdmissionProofs`] at the exact
    /// coordinates retained while sealing the semantic IR.
    pub(in crate::workspace) fn admit(
        self,
        proof: &WorkspaceAdmissionProofs,
    ) -> Result<PrincipalAdmittedUniverse, WorkspaceError> {
        let mut publisher_evidence = Vec::new();
        for coordinate in &self.admission_requirements.publishers {
            if coordinate.release_state != WorkspaceReleaseStateV1::Published {
                return Err(WorkspaceError::ReleaseStateNotAdmissible {
                    module: coordinate.module_id.to_string(),
                    release: coordinate.release_id.to_string(),
                });
            }
            let Some(evidence) = proof.admitted.get(coordinate) else {
                return Err(WorkspaceError::PublisherAdmissionUnavailable {
                    publisher: coordinate.publisher_id.to_string(),
                    release: coordinate.release_id.to_string(),
                });
            };
            if evidence.coordinate != *coordinate {
                return Err(WorkspaceError::PublisherAdmissionUnavailable {
                    publisher: coordinate.publisher_id.to_string(),
                    release: coordinate.release_id.to_string(),
                });
            }
            publisher_evidence.push(evidence.clone());
        }
        let mut protected_evidence = Vec::new();
        for coordinate in &self.admission_requirements.protected_delivery {
            let Some(evidence) = proof.protected_delivery.get(coordinate) else {
                return Err(WorkspaceError::ProtectedDeliveryAdmissionUnavailable {
                    importer: coordinate.importer_module.to_string(),
                    path: coordinate.to_string(),
                });
            };
            if evidence.coordinate != *coordinate {
                return Err(WorkspaceError::ProtectedDeliveryAdmissionUnavailable {
                    importer: coordinate.importer_module.to_string(),
                    path: coordinate.to_string(),
                });
            }
            protected_evidence.push(evidence.clone());
        }
        let evidence_preimage = WorkspaceAdmissionEvidencePreimage {
            publisher: publisher_evidence,
            protected_delivery: protected_evidence,
        };
        let evidence_digest = WorkspaceAdmissionEvidenceDigest::hash_canonical(&(
            "pale.workspace-admission-evidence.v1",
            &evidence_preimage,
        ))?;
        let digest = PrincipalAdmittedUniverseDigest::hash_canonical(&(
            "pale.principal-admitted-universe.v1",
            &self.preimage.workspace_id,
            &self.derivation_frontier.universe_digest,
            &self.digest,
            &evidence_digest,
        ))?;
        let VerifiedWorkspaceSemanticIr {
            preimage,
            digest: semantic_digest,
            derivation_frontier,
            admission_requirements: _,
            source_custody,
        } = self;
        drop(source_custody);
        Ok(PrincipalAdmittedUniverse {
            digest,
            workspace_semantic_ir: AdmittedWorkspaceSemanticIr {
                preimage,
                digest: semantic_digest,
                derivation_frontier,
            },
            admission_evidence: WorkspaceAdmissionEvidence {
                preimage: evidence_preimage,
                digest: evidence_digest,
            },
        })
    }

    /// Admit only a universe that requires no external publisher or protected
    /// delivery proof.
    ///
    /// This is the exact production behavior until an owner-published proof
    /// contract and adapter are selected. It does not synthesize evidence: any
    /// non-empty owner-proof frontier is rejected by [`Self::admit`].
    pub(in crate::workspace) fn admit_without_external_evidence(
        self,
    ) -> Result<PrincipalAdmittedUniverse, WorkspaceError> {
        self.admit(&WorkspaceAdmissionProofs::default())
    }
}
