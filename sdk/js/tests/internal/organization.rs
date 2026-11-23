//! Numeric projection boundaries; these construct wire values, not admission.

use super::*;

#[test]
fn unsigned_interval_values_preserve_every_bit_without_number_coercion() {
    for value in [0, 9_007_199_254_740_993, u64::MAX] {
        let projected = OrganizationInterval::from_api(&pale_sdk::OrganizationInterval {
            start: value,
            end_exclusive: Some(value),
        });
        assert_eq!(projected.start.get_u64(), (false, value, true));
        assert_eq!(
            projected.end_exclusive.unwrap().get_u64(),
            (false, value, true)
        );
    }
    assert!(
        OrganizationInterval::from_api(&pale_sdk::OrganizationInterval {
            start: 0,
            end_exclusive: None,
        })
        .end_exclusive
        .is_none()
    );
}

#[test]
fn copy_limit_representation_preserves_raw_sdk_input_and_rejects_coercion() {
    for value in [0, 1, pale_sdk::MAX_ORGANIZATION_COPY_BYTES, u32::MAX] {
        assert_eq!(organization_copy_limit(f64::from(value)).unwrap(), value);
    }
    for value in [-1.0, 0.5, f64::NAN, f64::INFINITY, 4_294_967_296.0] {
        assert_eq!(
            organization_copy_limit(value).unwrap_err().status,
            napi::Status::InvalidArg
        );
    }
}
