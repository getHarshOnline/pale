//! Append-only invalidation over typed Package causal dependencies.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use pale_core::records::ImmutableRecordReferenceV1;

use super::{
    AdmittedNormativePackReference, AdmittedNormativePackUse, AdmittedPackageActivation,
    AdmittedPackageMaterialization, AdmittedPackageNativeEffect, AdmittedPackageOccurrence,
    AdmittedPackageSettlement, AdmittedPackageUse,
};

/// Maximum exact nodes or dependency edges retained by one lineage graph.
pub const MAX_PACKAGE_LINEAGE_MEMBERS: usize = 4_096;

/// One cause that can invalidate an occurrence and its admitted descendants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageInvalidationKind {
    /// Package source changed or was withdrawn.
    Source,
    /// Parent occurrence or inherited authority changed.
    Ancestry,
    /// Package dependency changed or was revoked.
    Dependency,
    /// Selected provider coordinate changed.
    Provider,
    /// Evidence or freshness frontier changed.
    Evidence,
}

/// Append-only invalidation evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageInvalidation {
    /// Invalidation class.
    pub kind: PackageInvalidationKind,
    /// Exact cause evidence.
    pub evidence: ImmutableRecordReferenceV1,
}

/// Typed node in Package occurrence/use/settlement lineage.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PackageLineageNode {
    /// Package occurrence record.
    Occurrence(ImmutableRecordReferenceV1),
    /// Materialization record.
    Materialization(ImmutableRecordReferenceV1),
    /// Activation record.
    Activation(ImmutableRecordReferenceV1),
    /// Structurally admitted Package-use claim.
    Use(ImmutableRecordReferenceV1),
    /// Structurally admitted owner-native effect claim.
    NativeEffect(ImmutableRecordReferenceV1),
    /// Evidence-settlement record.
    Settlement(ImmutableRecordReferenceV1),
    /// Admitted body-free normative-Pack reference.
    NormativePackAdmission(ImmutableRecordReferenceV1),
    /// Structurally admitted normative-Pack use claim.
    NormativePackUse(ImmutableRecordReferenceV1),
    /// Generated dependent that must be invalidated with its exact input.
    GeneratedDependent(ImmutableRecordReferenceV1),
}

/// Body-free causal edge between two Package lineage records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageLineageEdge {
    /// Exact causal input.
    pub cause: PackageLineageNode,
    /// Exact generated or downstream dependent.
    pub dependent: PackageLineageNode,
    /// Immutable evidence for the dependency.
    pub evidence: ImmutableRecordReferenceV1,
}

/// Bounded acyclic Package dependency graph used only for invalidation closure.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageLineageGraph {
    nodes: BTreeSet<PackageLineageNode>,
    edges: BTreeMap<(PackageLineageNode, PackageLineageNode), ImmutableRecordReferenceV1>,
}

/// Structurally admitted source of one generated dependent.
pub enum PackageGeneratedDependencySource<'a> {
    /// Package occurrence.
    Occurrence(&'a AdmittedPackageOccurrence),
    /// Package materialization.
    Materialization(&'a AdmittedPackageMaterialization),
    /// Package activation.
    Activation(&'a AdmittedPackageActivation),
    /// Package-use claim.
    Use(&'a AdmittedPackageUse),
    /// Owner-native effect claim.
    NativeEffect(&'a AdmittedPackageNativeEffect),
    /// Evidence settlement.
    Settlement(&'a AdmittedPackageSettlement),
    /// Admitted normative-Pack reference.
    NormativePackAdmission(&'a AdmittedNormativePackReference),
    /// Normative-Pack use claim.
    NormativePackUse(&'a AdmittedNormativePackUse),
}

impl PackageGeneratedDependencySource<'_> {
    fn node(&self) -> PackageLineageNode {
        match self {
            Self::Occurrence(value) => {
                PackageLineageNode::Occurrence(value.record().occurrence.clone())
            }
            Self::Materialization(value) => {
                PackageLineageNode::Materialization(value.record().materialization.clone())
            }
            Self::Activation(value) => {
                PackageLineageNode::Activation(value.record().activation.clone())
            }
            Self::Use(value) => PackageLineageNode::Use(value.record().use_record.clone()),
            Self::NativeEffect(value) => {
                PackageLineageNode::NativeEffect(value.record().effect_record.clone())
            }
            Self::Settlement(value) => {
                PackageLineageNode::Settlement(value.record().settlement.clone())
            }
            Self::NormativePackAdmission(value) => {
                PackageLineageNode::NormativePackAdmission(value.record().admission.clone())
            }
            Self::NormativePackUse(value) => {
                PackageLineageNode::NormativePackUse(value.record().use_record.clone())
            }
        }
    }
}

impl PackageLineageGraph {
    /// Admit an exact occurrence as an invalidation root.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError::CapacityExceeded`] at the finite bound.
    pub fn admit_occurrence(
        &mut self,
        occurrence: &AdmittedPackageOccurrence,
    ) -> Result<(), PackageLineageError> {
        self.admit_node(PackageLineageNode::Occurrence(
            occurrence.record().occurrence.clone(),
        ))
    }

    /// Join an admitted child to its exact parent occurrence.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when the child does not name `parent` or
    /// the edge violates graph law.
    pub fn admit_child_occurrence(
        &mut self,
        parent: &AdmittedPackageOccurrence,
        child: &AdmittedPackageOccurrence,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        if child.record().parent.as_ref() != Some(&parent.record().occurrence) {
            return Err(PackageLineageError::RecordJoinMismatch);
        }
        self.admit_edge(PackageLineageEdge {
            cause: PackageLineageNode::Occurrence(parent.record().occurrence.clone()),
            dependent: PackageLineageNode::Occurrence(child.record().occurrence.clone()),
            evidence,
        })
    }

    /// Join admitted materialization to its exact occurrence.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when exact references disagree or graph
    /// law rejects the edge.
    pub fn admit_materialization(
        &mut self,
        occurrence: &AdmittedPackageOccurrence,
        materialization: &AdmittedPackageMaterialization,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        if materialization.record().occurrence != occurrence.record().occurrence {
            return Err(PackageLineageError::RecordJoinMismatch);
        }
        self.admit_edge(PackageLineageEdge {
            cause: PackageLineageNode::Occurrence(occurrence.record().occurrence.clone()),
            dependent: PackageLineageNode::Materialization(
                materialization.record().materialization.clone(),
            ),
            evidence,
        })
    }

    /// Join admitted activation to its exact materialization.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when exact references disagree or graph
    /// law rejects the edge.
    pub fn admit_activation(
        &mut self,
        materialization: &AdmittedPackageMaterialization,
        activation: &AdmittedPackageActivation,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        if activation.record().materialization != materialization.record().materialization {
            return Err(PackageLineageError::RecordJoinMismatch);
        }
        self.admit_edge(PackageLineageEdge {
            cause: PackageLineageNode::Materialization(
                materialization.record().materialization.clone(),
            ),
            dependent: PackageLineageNode::Activation(activation.record().activation.clone()),
            evidence,
        })
    }

    /// Join admitted actual use to its exact occurrence.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when exact references disagree or graph
    /// law rejects the edge.
    pub fn admit_use(
        &mut self,
        occurrence: &AdmittedPackageOccurrence,
        activation: Option<&AdmittedPackageActivation>,
        package_use: &AdmittedPackageUse,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        if package_use.record().occurrence != occurrence.record().occurrence {
            return Err(PackageLineageError::RecordJoinMismatch);
        }
        let cause = match (activation, package_use.record().activation.as_ref()) {
            (None, None) => PackageLineageNode::Occurrence(occurrence.record().occurrence.clone()),
            (Some(activation), Some(activation_record))
                if activation.record().activation == *activation_record
                    && activation.occurrence() == &occurrence.record().occurrence =>
            {
                PackageLineageNode::Activation(activation_record.clone())
            }
            _ => return Err(PackageLineageError::RecordJoinMismatch),
        };
        self.admit_edge(PackageLineageEdge {
            cause,
            dependent: PackageLineageNode::Use(package_use.record().use_record.clone()),
            evidence,
        })
    }

    /// Join admitted native effect to its exact actual use.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when exact references disagree or graph
    /// law rejects the edge.
    pub fn admit_native_effect(
        &mut self,
        package_use: &AdmittedPackageUse,
        native_effect: &AdmittedPackageNativeEffect,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        if native_effect.record().use_record != package_use.record().use_record {
            return Err(PackageLineageError::RecordJoinMismatch);
        }
        self.admit_edge(PackageLineageEdge {
            cause: PackageLineageNode::Use(package_use.record().use_record.clone()),
            dependent: PackageLineageNode::NativeEffect(
                native_effect.record().effect_record.clone(),
            ),
            evidence,
        })
    }

    /// Join admitted settlement to its exact occurrence.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when exact references disagree or graph
    /// law rejects the edge.
    pub fn admit_settlement(
        &mut self,
        occurrence: &AdmittedPackageOccurrence,
        package_use: Option<&AdmittedPackageUse>,
        settlement: &AdmittedPackageSettlement,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        if settlement.record().occurrence != occurrence.record().occurrence {
            return Err(PackageLineageError::RecordJoinMismatch);
        }
        let cause = match (package_use, settlement.record().use_record.as_ref()) {
            (None, None) => PackageLineageNode::Occurrence(occurrence.record().occurrence.clone()),
            (Some(package_use), Some(use_record))
                if package_use.record().use_record == *use_record =>
            {
                PackageLineageNode::Use(use_record.clone())
            }
            _ => return Err(PackageLineageError::RecordJoinMismatch),
        };
        self.admit_edge(PackageLineageEdge {
            cause,
            dependent: PackageLineageNode::Settlement(settlement.record().settlement.clone()),
            evidence,
        })
    }

    /// Add the exact native-effect dependency of an admitted settlement.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when the settlement and native effect do
    /// not name the same use, or graph law rejects the edge.
    pub fn admit_native_effect_settlement(
        &mut self,
        native_effect: &AdmittedPackageNativeEffect,
        settlement: &AdmittedPackageSettlement,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        if settlement.record().use_record.as_ref() != Some(&native_effect.record().use_record)
            || settlement.native_effect() != Some(&native_effect.record().effect_record)
        {
            return Err(PackageLineageError::RecordJoinMismatch);
        }
        self.admit_edge(PackageLineageEdge {
            cause: PackageLineageNode::NativeEffect(native_effect.record().effect_record.clone()),
            dependent: PackageLineageNode::Settlement(settlement.record().settlement.clone()),
            evidence,
        })
    }

    /// Join an occurrence to each exact normative-Pack admission it selected.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when `occurrence` did not select
    /// `admission`, or graph law rejects the edge.
    pub fn admit_normative_pack_dependency(
        &mut self,
        admission: &AdmittedNormativePackReference,
        occurrence: &AdmittedPackageOccurrence,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        if !occurrence
            .record()
            .normative_packs
            .iter()
            .any(|selected| selected == admission)
        {
            return Err(PackageLineageError::RecordJoinMismatch);
        }
        self.admit_edge(PackageLineageEdge {
            cause: PackageLineageNode::NormativePackAdmission(admission.record().admission.clone()),
            dependent: PackageLineageNode::Occurrence(occurrence.record().occurrence.clone()),
            evidence,
        })
    }

    /// Join actual normative-Pack use to both its admission and occurrence.
    ///
    /// Admission is atomic: either both causal parents are recorded or neither
    /// is. This preserves exact revocation and occurrence invalidation paths.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when exact references disagree or graph
    /// law rejects either edge.
    pub fn admit_normative_pack_use(
        &mut self,
        admission: &AdmittedNormativePackReference,
        occurrence: &AdmittedPackageOccurrence,
        pack_use: &AdmittedNormativePackUse,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        if pack_use.record().admission != admission.record().admission
            || pack_use.record().occurrence != occurrence.record().occurrence
            || !occurrence
                .record()
                .normative_packs
                .iter()
                .any(|selected| selected == admission)
        {
            return Err(PackageLineageError::RecordJoinMismatch);
        }
        let dependent = PackageLineageNode::NormativePackUse(pack_use.record().use_record.clone());
        let mut candidate = self.clone();
        candidate.admit_edge(PackageLineageEdge {
            cause: PackageLineageNode::NormativePackAdmission(admission.record().admission.clone()),
            dependent: dependent.clone(),
            evidence: evidence.clone(),
        })?;
        candidate.admit_edge(PackageLineageEdge {
            cause: PackageLineageNode::Occurrence(occurrence.record().occurrence.clone()),
            dependent,
            evidence,
        })?;
        *self = candidate;
        Ok(())
    }

    /// Join one generated dependent to an exact admitted lineage record.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] when graph law rejects the edge.
    pub fn admit_generated_dependent(
        &mut self,
        source: PackageGeneratedDependencySource<'_>,
        dependent: ImmutableRecordReferenceV1,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), PackageLineageError> {
        self.admit_edge(PackageLineageEdge {
            cause: source.node(),
            dependent: PackageLineageNode::GeneratedDependent(dependent),
            evidence,
        })
    }

    /// Register one exact record even when it has no admitted dependent yet.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError::CapacityExceeded`] at the finite graph
    /// bound.
    pub fn admit_node(&mut self, node: PackageLineageNode) -> Result<(), PackageLineageError> {
        if !self.nodes.contains(&node) && self.nodes.len() == MAX_PACKAGE_LINEAGE_MEMBERS {
            return Err(PackageLineageError::CapacityExceeded);
        }
        self.nodes.insert(node);
        Ok(())
    }

    /// Admit a typed dependency edge after checking direction and acyclicity.
    ///
    /// # Errors
    ///
    /// Returns [`PackageLineageError`] for a disallowed relation, duplicate
    /// pair, or cycle.
    pub(super) fn admit_edge(
        &mut self,
        edge: PackageLineageEdge,
    ) -> Result<(), PackageLineageError> {
        if !lineage_edge_allowed(&edge.cause, &edge.dependent) {
            return Err(PackageLineageError::DisallowedRelation);
        }
        let pair = (edge.cause.clone(), edge.dependent.clone());
        if self.edges.contains_key(&pair) {
            return Err(PackageLineageError::DuplicateEdge);
        }
        let new_nodes = usize::from(!self.nodes.contains(&edge.cause))
            + usize::from(!self.nodes.contains(&edge.dependent));
        if self.edges.len() == MAX_PACKAGE_LINEAGE_MEMBERS
            || self.nodes.len() + new_nodes > MAX_PACKAGE_LINEAGE_MEMBERS
        {
            return Err(PackageLineageError::CapacityExceeded);
        }
        if self.reaches(&edge.dependent, &edge.cause) {
            return Err(PackageLineageError::Cycle);
        }
        self.nodes.insert(edge.cause);
        self.nodes.insert(edge.dependent);
        self.edges.insert(pair, edge.evidence);
        Ok(())
    }

    fn children<'a>(
        &'a self,
        parent: &'a PackageLineageNode,
    ) -> impl Iterator<Item = &'a PackageLineageNode> + 'a {
        self.edges
            .keys()
            .filter_map(move |(cause, dependent)| (cause == parent).then_some(dependent))
    }

    fn reaches(&self, from: &PackageLineageNode, target: &PackageLineageNode) -> bool {
        let mut visited = BTreeSet::new();
        let mut pending = VecDeque::from([from]);
        while let Some(node) = pending.pop_front() {
            if node == target {
                return true;
            }
            if visited.insert(node) {
                pending.extend(self.children(node));
            }
        }
        false
    }
}

fn lineage_edge_allowed(cause: &PackageLineageNode, dependent: &PackageLineageNode) -> bool {
    matches!(
        (cause, dependent),
        (
            PackageLineageNode::Occurrence(_),
            PackageLineageNode::Occurrence(_)
                | PackageLineageNode::Materialization(_)
                | PackageLineageNode::NormativePackUse(_)
                | PackageLineageNode::Use(_)
                | PackageLineageNode::Settlement(_)
                | PackageLineageNode::GeneratedDependent(_)
        ) | (
            PackageLineageNode::NormativePackAdmission(_),
            PackageLineageNode::Occurrence(_)
                | PackageLineageNode::NormativePackUse(_)
                | PackageLineageNode::GeneratedDependent(_)
        ) | (
            PackageLineageNode::NormativePackUse(_),
            PackageLineageNode::Settlement(_) | PackageLineageNode::GeneratedDependent(_)
        ) | (
            PackageLineageNode::Materialization(_),
            PackageLineageNode::Activation(_) | PackageLineageNode::GeneratedDependent(_)
        ) | (
            PackageLineageNode::Activation(_),
            PackageLineageNode::Use(_) | PackageLineageNode::GeneratedDependent(_)
        ) | (
            PackageLineageNode::Use(_),
            PackageLineageNode::NativeEffect(_)
                | PackageLineageNode::Settlement(_)
                | PackageLineageNode::GeneratedDependent(_)
        ) | (
            PackageLineageNode::NativeEffect(_),
            PackageLineageNode::Settlement(_) | PackageLineageNode::GeneratedDependent(_)
        ) | (
            PackageLineageNode::Settlement(_),
            PackageLineageNode::GeneratedDependent(_)
        )
    )
}

/// A Package dependency edge is not an admissible acyclic causal relation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageLineageError {
    /// Node kinds do not form a permitted causal direction.
    #[error("Package lineage edge has a disallowed causal direction")]
    DisallowedRelation,
    /// The same exact dependency pair was already admitted.
    #[error("Package lineage edge duplicates an admitted dependency")]
    DuplicateEdge,
    /// The dependency would make invalidation lineage cyclic.
    #[error("Package lineage edge would create a cycle")]
    Cycle,
    /// Node or edge count reached the finite graph bound.
    #[error("Package lineage exceeds its 4096-member bound")]
    CapacityExceeded,
    /// Contextually admitted records do not name one another.
    #[error("Package lineage records do not form the claimed exact join")]
    RecordJoinMismatch,
}

/// Invalidation cause paired with its exact transitive affected closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageInvalidationClosure {
    /// Exact append-only invalidation evidence.
    pub invalidation: PackageInvalidation,
    /// Known affected records; historical records remain unchanged.
    pub affected: BTreeSet<PackageLineageNode>,
}

/// Propagate invalidation through admitted exact dependencies.
///
/// Unknown roots affect nothing. Occurrences, actual uses, settlements, native
/// effects, activations, and generated dependents remain separately typed.
#[must_use]
pub fn propagate_package_invalidation(
    graph: &PackageLineageGraph,
    roots: impl IntoIterator<Item = PackageLineageNode>,
    invalidation: PackageInvalidation,
) -> PackageInvalidationClosure {
    let mut affected = BTreeSet::new();
    let mut pending = roots
        .into_iter()
        .filter(|root| graph.nodes.contains(root))
        .collect::<VecDeque<_>>();
    while let Some(node) = pending.pop_front() {
        if !affected.insert(node.clone()) {
            continue;
        }
        pending.extend(
            graph
                .children(&node)
                .filter(|child| !affected.contains(*child))
                .cloned(),
        );
    }
    PackageInvalidationClosure {
        invalidation,
        affected,
    }
}
