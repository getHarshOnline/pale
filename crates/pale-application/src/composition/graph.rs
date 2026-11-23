//! Single-owner graph storage and deterministic composition-plan projections.
//!
//! [`CompositionGraphs`] is the only adjacency owner used by the reference
//! composition compiler. Each inserted edge retains its declared identity, so
//! parallel edges and graph-kind distinctions survive derived views.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{CompositionGraphKindV1, CompositionGraphViewV1, ParticipantRefV1};

/// Detect a cycle in a derived directed projection without recursive traversal.
///
/// Kahn's algorithm consumes only an ephemeral ordered index. The owning
/// [`Adjacency`] values remain the single source of edge identity and topology;
/// the projection neither persists nor invents another graph representation.
fn has_directed_cycle<'a>(
    edges: impl IntoIterator<Item = (&'a ParticipantRefV1, &'a ParticipantRefV1)>,
) -> bool {
    let mut outgoing = BTreeMap::<&ParticipantRefV1, BTreeSet<&ParticipantRefV1>>::new();
    let mut incoming = BTreeMap::<&ParticipantRefV1, usize>::new();
    for (from, to) in edges {
        incoming.entry(from).or_default();
        incoming.entry(to).or_default();
        if outgoing.entry(from).or_default().insert(to) {
            *incoming.get_mut(to).expect("destination was indexed") += 1;
        }
    }

    let mut ready = incoming
        .iter()
        .filter_map(|(participant, count)| (*count == 0).then_some(*participant))
        .collect::<BTreeSet<_>>();
    let mut visited = 0;
    while let Some(participant) = ready.pop_first() {
        visited += 1;
        if let Some(neighbors) = outgoing.get(participant) {
            for neighbor in neighbors {
                let count = incoming.get_mut(neighbor).expect("destination was indexed");
                *count -= 1;
                if *count == 0 {
                    ready.insert(neighbor);
                }
            }
        }
    }
    visited != incoming.len()
}

/// Directional adjacency for one exact composition graph kind.
///
/// The nested sets retain declared edge IDs instead of synthesizing identity
/// from endpoints. This matters when two semantically independent contracts
/// connect the same participants.
#[derive(Debug, Default)]
struct Adjacency {
    edges: BTreeMap<ParticipantRefV1, BTreeMap<ParticipantRefV1, BTreeSet<String>>>,
}

impl Adjacency {
    fn insert(&mut self, edge_id: String, from: ParticipantRefV1, to: ParticipantRefV1) -> bool {
        self.edges
            .entry(from)
            .or_default()
            .entry(to)
            .or_default()
            .insert(edge_id)
    }

    fn nodes(&self) -> Vec<ParticipantRefV1> {
        self.edges
            .iter()
            .flat_map(|(from, tos)| std::iter::once(from).chain(tos.keys()))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn edge_ids(&self) -> Vec<String> {
        self.edges
            .values()
            .flat_map(|tos| tos.values())
            .flat_map(BTreeSet::iter)
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn has_cycle(&self) -> bool {
        has_directed_cycle(
            self.edges
                .iter()
                .flat_map(|(from, tos)| tos.keys().map(move |to| (from, to))),
        )
    }
}

/// Sole adjacency authority for one linked composition plan.
///
/// Graph kinds remain separate map entries: sharing storage does not merge UI
/// embedding, work delegation, native calls, or settlement semantics. Provider
/// compatibility may contain reciprocal cycles by construction. The remaining
/// six kinds participate in [`Self::reject_operational_cycles`]: four as one
/// runtime/effect projection and two as independent structural projections.
#[derive(Debug, Default)]
pub(super) struct CompositionGraphs {
    by_kind: BTreeMap<CompositionGraphKindV1, Adjacency>,
}

impl CompositionGraphs {
    /// Insert one already-validated edge under its intrinsic graph kind.
    pub(super) fn insert(
        &mut self,
        kind: CompositionGraphKindV1,
        edge_id: String,
        from: ParticipantRefV1,
        to: ParticipantRefV1,
    ) -> bool {
        self.by_kind
            .entry(kind)
            .or_default()
            .insert(edge_id, from, to)
    }

    /// Derive one deterministic graph view without transferring ownership.
    pub(super) fn view(&self, kind: CompositionGraphKindV1) -> CompositionGraphViewV1 {
        let adjacency = self.by_kind.get(&kind);
        CompositionGraphViewV1 {
            kind,
            nodes: adjacency.map_or_else(Vec::new, Adjacency::nodes),
            edges: adjacency.map_or_else(Vec::new, Adjacency::edge_ids),
        }
    }

    /// Reject cycles in operational composition domains.
    ///
    /// Runtime calls and effects are checked as one aggregate projection so a
    /// cycle cannot hide by crossing edge kinds. UI containment and plugin
    /// dependency remain non-authorizing structural domains and are checked
    /// independently. Reciprocal provider compatibility is excluded because
    /// its finite positive monotone relation belongs to the provider compiler.
    pub(super) fn reject_operational_cycles(&self) -> Result<(), super::CompositionError> {
        let runtime_effect_kinds = [
            CompositionGraphKindV1::WorkDelegation,
            CompositionGraphKindV1::NativeApplicationCall,
            CompositionGraphKindV1::TerminalLeaf,
            CompositionGraphKindV1::ObservationSettlement,
        ];
        if self.has_cycle_across(&runtime_effect_kinds) {
            return Err(super::CompositionError::Path {
                path: "runtime_call_and_effect".into(),
                reason: "selected runtime/effect graph is cyclic".into(),
            });
        }
        for kind in [
            CompositionGraphKindV1::UiEmbedding,
            CompositionGraphKindV1::PluginDependency,
        ] {
            if self.by_kind.get(&kind).is_some_and(Adjacency::has_cycle) {
                return Err(super::CompositionError::Path {
                    path: kind.to_string(),
                    reason: "selected structural graph is cyclic".into(),
                });
            }
        }
        Ok(())
    }

    /// Detect a directed cycle across an allowlisted union of stored graph kinds.
    fn has_cycle_across(&self, kinds: &[CompositionGraphKindV1]) -> bool {
        has_directed_cycle(
            kinds
                .iter()
                .filter_map(|kind| self.by_kind.get(kind))
                .flat_map(|adjacency| adjacency.edges.iter())
                .flat_map(|(from, tos)| tos.keys().map(move |to| (from, to))),
        )
    }
}

/// Return the local identity while the caller retains the participant's tag.
///
/// The local spelling is not globally unique: callers constructing keys or
/// editor identities must retain the [`ParticipantRefV1`] variant rather than
/// collapsing every participant into this string.
pub(crate) fn participant_id(participant: &ParticipantRefV1) -> &str {
    match participant {
        ParticipantRefV1::Interface { interface_id } => interface_id,
        ParticipantRefV1::Application { application_id } => application_id,
        ParticipantRefV1::Engine { engine_id } => engine_id,
        ParticipantRefV1::Capability { capability_id } => capability_id,
        ParticipantRefV1::TerminalLeaf { terminal_leaf_id } => terminal_leaf_id,
        ParticipantRefV1::Plugin { plugin_id } => plugin_id,
        ParticipantRefV1::Skill { skill_id } => skill_id,
    }
}

#[cfg(test)]
#[path = "../../tests/internal/composition/graph.rs"]
mod tests;
