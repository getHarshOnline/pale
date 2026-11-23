//! Compares provider-equivalence compilation with an independent fixed-point closure.

use super::*;

impl ProviderComponents<'_> {
    fn materialized_closure(&self) -> BTreeMap<String, BTreeSet<String>> {
        self.providers
            .iter()
            .enumerate()
            .map(|(index, provider)| {
                let representative = self.component_of[index];
                let members = self
                    .providers
                    .iter()
                    .enumerate()
                    .filter(|(candidate, _)| self.component_of[*candidate] == representative)
                    .map(|(_, candidate)| (*candidate).to_owned())
                    .collect();
                ((*provider).to_owned(), members)
            })
            .collect()
    }
}

fn reference_closure(
    providers: &[ProviderCompatibilityDeclarationV1],
) -> BTreeMap<String, BTreeSet<String>> {
    let declared = providers
        .iter()
        .map(|provider| {
            (
                provider.id.clone(),
                provider
                    .compatible_provider_ids
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut closure = declared
        .keys()
        .map(|provider| (provider.clone(), BTreeSet::from([provider.clone()])))
        .collect::<BTreeMap<_, _>>();
    loop {
        let previous = closure.clone();
        let mut changed = false;
        for (provider, neighbours) in &declared {
            let reachable = closure
                .get_mut(provider)
                .expect("each declared provider owns one reference row");
            for neighbour in neighbours {
                changed |= reachable.insert(neighbour.clone());
                for transitive in &previous[neighbour] {
                    changed |= reachable.insert(transitive.clone());
                }
            }
        }
        if !changed {
            return closure;
        }
    }
}

fn provider(id: &str, neighbours: &[&str]) -> ProviderCompatibilityDeclarationV1 {
    ProviderCompatibilityDeclarationV1 {
        id: id.into(),
        participant: ParticipantRefV1::Engine {
            engine_id: format!("{id}-engine"),
        },
        capability_ids: vec!["capability".into()],
        compatible_provider_ids: neighbours.iter().map(|id| (*id).into()).collect(),
    }
}

fn compile(
    providers: &[ProviderCompatibilityDeclarationV1],
) -> Result<ProviderComponents<'_>, CompositionError> {
    let providers = providers.iter().collect::<Vec<_>>();
    let by_id = providers
        .iter()
        .map(|provider| (provider.id.as_str(), *provider))
        .collect::<BTreeMap<_, _>>();
    compile_provider_components(&providers, &by_id)
}

#[test]
fn empty_declarations_are_the_identity_and_recompilation_is_idempotent() {
    let empty = compile(&[]).unwrap();
    assert!(empty.providers.is_empty());
    assert!(empty.component_of.is_empty());

    let declarations = [provider("a", &["b"]), provider("b", &["a"])];
    let first = compile(&declarations).unwrap();
    let second = compile(&declarations).unwrap();
    assert_eq!(first, second);
}

#[test]
fn closure_is_permutation_invariant_monotone_and_disjointly_non_interfering() {
    let base = [provider("a", &["b"]), provider("b", &["a"])];
    let base_result = compile(&base).unwrap();
    let base_closure = base_result.materialized_closure();

    let extended = [
        provider("d", &["c"]),
        provider("b", &["a"]),
        provider("c", &["d"]),
        provider("a", &["b"]),
    ];
    let extended_result = compile(&extended).unwrap();
    let extended_closure = extended_result.materialized_closure();
    assert_eq!(extended_closure["a"], base_closure["a"]);
    assert_eq!(extended_closure["b"], base_closure["b"]);
    assert!(extended_closure["a"].is_disjoint(&extended_closure["c"]));

    let permuted = [
        provider("c", &["d"]),
        provider("a", &["b"]),
        provider("d", &["c"]),
        provider("b", &["a"]),
    ];
    let permuted_result = compile(&permuted).unwrap();
    assert_eq!(extended_result, permuted_result);
}

#[test]
fn malformed_joins_fail_before_any_closure_is_exposed() {
    let missing = [provider("a", &["missing"])];
    assert!(matches!(
        compile(&missing),
        Err(CompositionError::MissingReference { .. })
    ));

    let asymmetric = [provider("a", &["b"]), provider("b", &[])];
    assert!(matches!(
        compile(&asymmetric),
        Err(CompositionError::ProviderCompatibility { .. })
    ));
}

#[test]
fn component_partition_matches_the_iterative_reference_for_all_small_graphs() {
    const PROVIDER_COUNT: usize = 5;
    let ids = (0..PROVIDER_COUNT)
        .map(|index| format!("p{index}"))
        .collect::<Vec<_>>();
    let possible_edges = (0..PROVIDER_COUNT)
        .flat_map(|left| ((left + 1)..PROVIDER_COUNT).map(move |right| (left, right)))
        .collect::<Vec<_>>();

    for mask in 0..(1_usize << possible_edges.len()) {
        let mut neighbours = vec![Vec::new(); PROVIDER_COUNT];
        for (bit, (left, right)) in possible_edges.iter().enumerate() {
            if mask & (1 << bit) != 0 {
                neighbours[*left].push(ids[*right].as_str());
                neighbours[*right].push(ids[*left].as_str());
            }
        }
        let declarations = ids
            .iter()
            .zip(&neighbours)
            .map(|(id, neighbours)| provider(id, neighbours))
            .collect::<Vec<_>>();
        let expected = reference_closure(&declarations);
        let selected = compile(&declarations)
            .expect("generated reciprocal graph must compile")
            .materialized_closure();
        assert_eq!(selected, expected, "edge mask {mask:#014b}");

        let reversed = declarations.iter().cloned().rev().collect::<Vec<_>>();
        assert_eq!(
            compile(&reversed)
                .expect("source permutation must compile")
                .materialized_closure(),
            expected,
            "reversed edge mask {mask:#014b}"
        );
    }
}
