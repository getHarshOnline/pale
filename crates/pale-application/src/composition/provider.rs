//! Finite monotone provider-declaration SCC compilation.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    CompositionGraphKindV1, ParticipantProviderManifestV1, ParticipantRefV1,
    ProviderCompatibilityDeclarationV1,
};

use super::graph::{participant_id, CompositionGraphs};
use super::CompositionError;

/// Validate provider declarations and add only their authored compatibility edges.
///
/// Component closure proves that the finite reciprocal declarations converge;
/// it does not manufacture transitive edges in [`CompositionGraphs`].
///
/// # Errors
///
/// Returns [`CompositionError`] for duplicate identities, missing participants
/// or providers, asymmetric declarations, or a non-canonical derived partition.
pub(super) fn validate_providers(
    manifests: &[ParticipantProviderManifestV1],
    graphs: &mut CompositionGraphs,
) -> Result<BTreeSet<ParticipantRefV1>, CompositionError> {
    let mut participants = BTreeSet::new();
    for manifest in manifests {
        let mut local = BTreeSet::new();
        for participant in &manifest.participants {
            if !local.insert(participant.clone()) || !participants.insert(participant.clone()) {
                return Err(CompositionError::DuplicateId {
                    kind: "participant",
                    id: participant_id(participant).to_string(),
                });
            }
        }
    }
    let providers = manifests
        .iter()
        .flat_map(|manifest| &manifest.providers)
        .collect::<Vec<_>>();
    let mut by_id = BTreeMap::new();
    for provider in &providers {
        if !participants.contains(&provider.participant) {
            return Err(CompositionError::MissingReference {
                kind: "provider",
                owner: provider.id.clone(),
                target_kind: "participant",
                target: participant_id(&provider.participant).to_string(),
            });
        }
        if by_id.insert(provider.id.as_str(), *provider).is_some() {
            return Err(CompositionError::DuplicateId {
                kind: "provider",
                id: provider.id.clone(),
            });
        }
    }
    compile_provider_components(&providers, &by_id)?.verify()?;
    for provider in providers {
        for compatible_id in &provider.compatible_provider_ids {
            let compatible = by_id
                .get(compatible_id.as_str())
                .expect("component compilation resolved every compatibility reference");
            graphs.insert(
                CompositionGraphKindV1::ProviderCompatibility,
                format!("{}=>{compatible_id}", provider.id),
                provider.participant.clone(),
                compatible.participant.clone(),
            );
        }
    }
    Ok(participants)
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ProviderComponents<'a> {
    /// Provider identities in canonical order.
    pub(super) providers: Vec<&'a str>,
    /// Canonical component representative for each provider position.
    pub(super) component_of: Vec<usize>,
}

impl ProviderComponents<'_> {
    fn verify(self) -> Result<Self, CompositionError> {
        let canonical = self.providers.len() == self.component_of.len()
            && self
                .component_of
                .iter()
                .enumerate()
                .all(|(index, representative)| {
                    *representative <= index
                        && self.component_of.get(*representative) == Some(representative)
                });
        if !canonical {
            return Err(CompositionError::ProviderCompatibility {
                provider: "provider declaration set".into(),
                reason: "derived provider components do not have canonical representatives".into(),
            });
        }
        Ok(self)
    }
}

/// Mutable union-find used only while deriving the canonical partition.
///
/// Union-by-size and path compression bound the mechanical forest depth. The
/// smallest provider position is tracked independently from the forest root,
/// so canonical output does not compromise the union-find complexity bound.
#[derive(Debug)]
struct DisjointProviders {
    /// Parent position for every canonical provider position.
    parent: Vec<usize>,
    /// Current component cardinality, meaningful only at a forest root.
    size: Vec<usize>,
    /// Smallest canonical provider position in each rooted component.
    minimum: Vec<usize>,
}

impl DisjointProviders {
    /// Construct the discrete partition over `len` provider positions.
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            size: vec![1; len],
            minimum: (0..len).collect(),
        }
    }

    /// Find and path-compress the mechanical forest root of `index`.
    fn root(&mut self, index: usize) -> usize {
        let mut root = index;
        while self.parent[root] != root {
            root = self.parent[root];
        }

        let mut cursor = index;
        while self.parent[cursor] != cursor {
            let next = self.parent[cursor];
            self.parent[cursor] = root;
            cursor = next;
        }
        root
    }

    /// Merge two components without allowing source order to affect output.
    fn unite(&mut self, left: usize, right: usize) {
        let left = self.root(left);
        let right = self.root(right);
        if left == right {
            return;
        }

        let (root, child) = if self.size[left] > self.size[right]
            || (self.size[left] == self.size[right] && left < right)
        {
            (left, right)
        } else {
            (right, left)
        };
        self.parent[child] = root;
        self.size[root] += self.size[child];
        self.minimum[root] = self.minimum[root].min(self.minimum[child]);
    }

    /// Consume the forest and return one canonical minimum per position.
    fn canonicalize(mut self) -> Vec<usize> {
        (0..self.parent.len())
            .map(|index| {
                let root = self.root(index);
                self.minimum[root]
            })
            .collect()
    }
}

/// Compile reciprocal provider declarations into deterministic components.
///
/// Reciprocal positive compatibility makes the declaration graph undirected,
/// so its least fixed point is exactly its connected-component partition. If
/// `G` is the declaration graph with provider set `V` and reciprocal edge set
/// `E`, this function computes the equivalence relation generated by `E`
/// without cloning a complete reachability map on every pass. The rendered law
/// states both ordered-index and union-find bounds. Canonical provider order
/// makes the smallest provider position the stable representative of each
/// component; `alpha` is the inverse Ackermann function.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/provider-component-closure.display.mathml"
))]
///
/// Only explicit edges enter the composition plan. Component membership is a
/// convergence witness, not a new compatibility declaration, provider
/// selection, reverse call, or authority.
///
/// # Errors
///
/// Returns [`CompositionError`] when the canonical index differs from the
/// declaration set, a referenced provider is absent, or reciprocity is not
/// proved by the opposite declaration.
pub(super) fn compile_provider_components<'a>(
    providers: &[&'a ProviderCompatibilityDeclarationV1],
    by_id: &BTreeMap<&'a str, &'a ProviderCompatibilityDeclarationV1>,
) -> Result<ProviderComponents<'a>, CompositionError> {
    let canonical = by_id.keys().copied().collect::<Vec<_>>();
    if canonical.len() != providers.len() {
        return Err(CompositionError::ProviderCompatibility {
            provider: "provider declaration set".into(),
            reason: "provider index does not match the declaration set".into(),
        });
    }
    let positions = canonical
        .iter()
        .enumerate()
        .map(|(index, provider)| (*provider, index))
        .collect::<BTreeMap<_, _>>();
    let declared_edges = providers
        .iter()
        .flat_map(|provider| {
            provider
                .compatible_provider_ids
                .iter()
                .map(move |compatible| (provider.id.as_str(), compatible.as_str()))
        })
        .collect::<BTreeSet<_>>();
    let mut components = DisjointProviders::new(canonical.len());
    for provider in providers {
        let provider_index = positions.get(provider.id.as_str()).ok_or_else(|| {
            CompositionError::ProviderCompatibility {
                provider: provider.id.clone(),
                reason: "provider declaration is absent from its canonical index".into(),
            }
        })?;
        for compatible_id in &provider.compatible_provider_ids {
            if !by_id.contains_key(compatible_id.as_str()) {
                return Err(CompositionError::MissingReference {
                    kind: "provider",
                    owner: provider.id.clone(),
                    target_kind: "provider",
                    target: compatible_id.clone(),
                });
            }
            if !declared_edges.contains(&(compatible_id.as_str(), provider.id.as_str())) {
                return Err(CompositionError::ProviderCompatibility {
                    provider: provider.id.clone(),
                    reason: format!("reverse compatibility `{compatible_id}` is not declared"),
                });
            }
            let compatible_index = positions
                .get(compatible_id.as_str())
                .expect("the canonical index contains every resolved provider");
            components.unite(*provider_index, *compatible_index);
        }
    }
    Ok(ProviderComponents {
        providers: canonical,
        component_of: components.canonicalize(),
    })
}

#[cfg(test)]
#[path = "../../tests/internal/composition/provider.rs"]
mod tests;
