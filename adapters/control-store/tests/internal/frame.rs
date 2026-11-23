//! Candidate authority-frame law tests.

use super::*;

#[test]
fn frame_round_trip_uses_the_pale_initial_v1_prefix() {
    let payload = b"opaque canonical application bytes";
    let encoded = encode(payload, AuthorityByteClass::JournalEntry).unwrap();

    assert_eq!(&encoded[..FRAME_MAGIC.len()], b"PALEAF01");
    assert_eq!(encoded[FRAME_MAGIC.len()], 1);

    let length_start = FRAME_MAGIC.len() + 1;
    let length_end = length_start + size_of::<u64>();
    assert_eq!(
        &encoded[length_start..length_end],
        &(payload.len() as u64).to_be_bytes()
    );

    let checksum_start = FRAME_MAGIC.len() + 1 + size_of::<u64>();
    assert_eq!(
        &encoded[checksum_start..FRAME_HEADER_BYTES],
        sha256_hex(payload).as_bytes()
    );
    assert_eq!(&encoded[FRAME_HEADER_BYTES..], payload);
    assert_eq!(encoded.len(), FRAME_HEADER_BYTES + payload.len());
    assert_eq!(
        decode(&encoded, payload.len(), AuthorityByteClass::JournalEntry).unwrap(),
        payload
    );
    assert_eq!(FRAME_VERSION, 1);
}

#[test]
fn frame_rejects_the_removed_legacy_prefix() {
    const REMOVED_PREFIX: [u8; 8] = [0x4a, 0x52, 0x56, 0x53, 0x41, 0x46, 0x30, 0x31];

    let payload = b"payload";
    let mut encoded = encode(payload, AuthorityByteClass::JournalEntry).unwrap();
    encoded[..FRAME_MAGIC.len()].copy_from_slice(&REMOVED_PREFIX);

    assert!(matches!(
        decode(&encoded, payload.len(), AuthorityByteClass::JournalEntry)
            .map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::Corrupt(
            AuthorityByteClass::JournalEntry
        ))
    ));
}

#[test]
fn frame_rejects_wrong_magic_version_length_checksum_and_trailing_bytes() {
    let payload = b"payload";
    let encoded = encode(payload, AuthorityByteClass::JournalEntry).unwrap();
    for mutation in [0, FRAME_MAGIC.len(), FRAME_HEADER_BYTES - 1] {
        let mut changed = encoded.clone();
        changed[mutation] ^= 1;
        assert!(matches!(
            decode(&changed, payload.len(), AuthorityByteClass::JournalEntry)
                .map_err(|error| error.kind()),
            Err(DurableAuthorityStoreErrorKind::Corrupt(
                AuthorityByteClass::JournalEntry
            ))
        ));
    }

    let mut wrong_length = encoded.clone();
    let length_last_byte = FRAME_MAGIC.len() + size_of::<u64>();
    wrong_length[length_last_byte] = 0;
    assert!(matches!(
        decode(
            &wrong_length,
            payload.len(),
            AuthorityByteClass::JournalEntry
        )
        .map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::Corrupt(
            AuthorityByteClass::JournalEntry
        ))
    ));

    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(matches!(
        decode(&trailing, payload.len(), AuthorityByteClass::JournalEntry)
            .map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::Corrupt(
            AuthorityByteClass::JournalEntry
        ))
    ));
}

#[test]
fn frame_checks_the_payload_bound_before_returning_bytes() {
    let encoded = encode(b"four", AuthorityByteClass::RecoveryCheckpoint).unwrap();
    assert_eq!(
        decode(&encoded, 3, AuthorityByteClass::RecoveryCheckpoint).map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::RecoveryCheckpoint
        })
    );
}
