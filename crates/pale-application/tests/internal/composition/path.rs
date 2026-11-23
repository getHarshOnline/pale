//! Checks the finite, non-widening meet laws for path effect ceilings.

use super::*;

fn ceiling(values: &[EffectClass]) -> Vec<EffectClass> {
    values.to_vec()
}

#[test]
fn effect_ceiling_meet_obeys_the_finite_non_widening_laws() {
    let a = ceiling(&[EffectClass::Read, EffectClass::Write, EffectClass::Publish]);
    let b = ceiling(&[EffectClass::Compute, EffectClass::Read, EffectClass::Write]);
    let c = ceiling(&[EffectClass::Execute, EffectClass::Read]);
    let expected = BTreeSet::from([EffectClass::Read]);

    assert_eq!(
        effect_ceiling_meet([a.as_slice(), b.as_slice(), c.as_slice()]),
        expected
    );
    assert_eq!(
        effect_ceiling_meet([c.as_slice(), a.as_slice(), b.as_slice()]),
        expected
    );
    assert_eq!(
        effect_ceiling_meet([a.as_slice(), b.as_slice(), b.as_slice(), c.as_slice()]),
        expected
    );
    assert_eq!(
        effect_ceiling_meet([a.as_slice(), a.as_slice()]),
        a.iter().cloned().collect()
    );
    assert!(expected.is_subset(&a.iter().cloned().collect()));
    assert!(expected.is_subset(&b.iter().cloned().collect()));
    assert!(expected.is_subset(&c.iter().cloned().collect()));
}
