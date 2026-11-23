//! Property-style proofs for immutable Package-text mechanics.

use super::*;
use pale_core::records::{PackageIdV1, PackageResourceIdV1};

fn owner(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("text-fixture").unwrap(),
        resource_id: PackageResourceIdV1::parse(id).unwrap(),
    }
}

fn referenced_bytes(recipe: &TextRecipe) -> usize {
    recipe
        .0
        .parts
        .iter()
        .map(|part| match part {
            TextPart::Slice(reference)
            | TextPart::ResourceLink {
                source: reference, ..
            } => reference.len(),
            TextPart::Owned(_) | TextPart::Static(_) | TextPart::Hashes(_) => 0,
        })
        .sum()
}

#[test]
fn store_generation_is_order_independent_and_owner_qualified() {
    let left = seal_texts([
        (owner("beta"), Arc::<str>::from("βeta")),
        (owner("alpha"), Arc::<str>::from("alpha")),
    ]);
    let right = seal_texts([
        (owner("alpha"), Arc::<str>::from("alpha")),
        (owner("beta"), Arc::<str>::from("βeta")),
    ]);

    assert_eq!(left, right);
    assert_eq!(left[0].0, owner("alpha"));
    assert_eq!(
        &left[0].1.store.objects[left[0].1.object].owner,
        &owner("alpha")
    );
    assert_eq!(left[0].1.object_id.0, ArtifactDigest::hash_bytes(b"alpha"));
    assert_eq!(left[1].1.text(), "βeta");

    let alpha_owner = seal_texts([(owner("alpha"), Arc::<str>::from("same"))]);
    let beta_owner = seal_texts([(owner("beta"), Arc::<str>::from("same"))]);
    assert_ne!(alpha_owner[0].1.generation, beta_owner[0].1.generation);
}

#[test]
fn stale_generation_and_non_utf8_spans_fail_closed() {
    let mut stores = seal_texts([(owner("unicode"), Arc::<str>::from("aλz"))]);
    let (_, reference) = stores.pop().unwrap();
    assert!(reference.slice(1..2).is_none());
    assert_eq!(reference.slice(1..3).unwrap().text(), "λ");

    let mut stale = reference.clone();
    stale.generation = ArtifactDigest::hash_bytes(b"another generation");
    assert_eq!(
        reference.store.resolve(&stale),
        Err(TextStoreError::StaleGeneration)
    );
}

#[test]
fn recipe_identity_and_regrouping_preserve_exact_bytes() {
    let mut refs = seal_texts([(owner("body"), Arc::<str>::from("alphaβeta"))]);
    let (_, body) = refs.pop().unwrap();
    let alpha = {
        let mut recipe = TextRecipeBuilder::default();
        recipe.push_ref(body.slice(0..5).unwrap());
        recipe.finish()
    };
    let beta = TextRecipe::owned(" + ");
    let gamma = {
        let mut recipe = TextRecipeBuilder::default();
        recipe.push_ref(body.slice(5..body.len()).unwrap());
        recipe.finish()
    };

    let mut left_group = TextRecipeBuilder::default();
    left_group.append(&alpha);
    left_group.append(&beta);
    let left_group = left_group.finish();
    let mut left = TextRecipeBuilder::default();
    left.append(&left_group);
    left.append(&gamma);

    let mut right_group = TextRecipeBuilder::default();
    right_group.append(&beta);
    right_group.append(&gamma);
    let right_group = right_group.finish();
    let mut right = TextRecipeBuilder::default();
    right.append(&TextRecipeBuilder::default().finish());
    right.append(&alpha);
    right.append(&right_group);

    let left = left.finish();
    let right = right.finish();
    assert_eq!(left, right);
    assert_eq!(left.materialize().as_str(), "alpha + βeta");
    assert!(Arc::ptr_eq(&left.materialize(), &left.materialize()));
    assert_eq!(left.0.byte_len, "alpha + βeta".len());
    assert_eq!(referenced_bytes(&left), "alphaβeta".len());
}

#[test]
fn every_utf8_three_way_partition_obeys_recipe_associativity() {
    let source = "aβ\n# γ😀";
    let (_, body) = seal_texts([(owner("partitioned"), Arc::<str>::from(source))])
        .pop()
        .unwrap();
    let boundaries = source
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once(source.len()))
        .collect::<Vec<_>>();

    for &first in &boundaries {
        for &second in boundaries.iter().filter(|&&offset| offset >= first) {
            let parts = [
                body.slice(0..first).unwrap(),
                body.slice(first..second).unwrap(),
                body.slice(second..source.len()).unwrap(),
            ]
            .map(|reference| {
                let mut builder = TextRecipeBuilder::default();
                builder.push_ref(reference);
                builder.finish()
            });

            let mut left_pair = TextRecipeBuilder::default();
            left_pair.append(&parts[0]);
            left_pair.append(&parts[1]);
            let left_pair = left_pair.finish();
            let mut left = TextRecipeBuilder::default();
            left.append(&left_pair);
            left.append(&parts[2]);

            let mut right_pair = TextRecipeBuilder::default();
            right_pair.append(&parts[1]);
            right_pair.append(&parts[2]);
            let right_pair = right_pair.finish();
            let mut right = TextRecipeBuilder::default();
            right.append(&parts[0]);
            right.append(&right_pair);

            let left = left.finish();
            let right = right.finish();
            assert_eq!(left, right);
            assert_eq!(left.materialize().as_str(), source);
        }
    }
}
