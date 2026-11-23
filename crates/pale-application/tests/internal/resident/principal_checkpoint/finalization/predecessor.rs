//! Explicit read models exercise freshness checks, not independent anchor durability.

use super::*;
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorErrorKind, AntiRollbackAnchorKey, AntiRollbackAnchorObservation,
    AntiRollbackAnchorPort, ControlDomainAntiRollbackAnchor, ControlRecord, ControlRecordBytes,
    ControlRecordCoordinate, ControlRecordDigest, PreparedAntiRollbackAnchorTransition,
    PrincipalTrustCheckpointHead, PrincipalTrustMinimumAnchor, PrincipalTrustSnapshot,
    PrincipalTrustStoreBinding,
};
use std::collections::VecDeque;

pub(super) fn snapshot(records: &fixture::BootstrapRecords) -> PrincipalTrustSnapshot {
    let trust = &records.trust;
    let coordinate = ControlRecordCoordinate::new(
        records.trust_ref.clone(),
        ControlRecordDigest::parse(trust.canonical_trust_checkpoint_sha256().as_str()).unwrap(),
    );
    PrincipalTrustSnapshot::new(
        PrincipalTrustStoreBinding::new(
            trust.trust_store_id().clone(),
            trust.trust_store_incarnation_id().clone(),
            trust.principal_id().clone(),
        ),
        PrincipalTrustCheckpointHead::new(
            trust.principal_id().clone(),
            *trust.checkpoint_epoch(),
            coordinate.clone(),
        ),
        ControlRecord::new(
            coordinate,
            trust.schema_id().clone(),
            trust.schema_sha256().clone(),
            ControlRecordBytes::new(canon::to_canonical_string(trust).unwrap().into_bytes())
                .unwrap(),
        ),
    )
    .unwrap()
}

pub(super) struct Anchors {
    reads: VecDeque<Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError>>,
    keys: Vec<AntiRollbackAnchorKey>,
}

pub(super) fn anchors(
    records: &fixture::BootstrapRecords,
    prior_ref: &ResidentRecordRefV1,
    prior: &ControlStateCheckpointV1,
) -> Anchors {
    let trust = snapshot(records);
    let minimum: AntiRollbackAnchor = PrincipalTrustMinimumAnchor::new(
        records.trust.principal_id().clone(),
        *records.trust.checkpoint_epoch(),
        trust.through().coordinate().clone(),
    )
    .into();
    let domain: AntiRollbackAnchor = ControlDomainAntiRollbackAnchor::new(
        prior.control_store_id().clone(),
        prior.control_incarnation_id().clone(),
        prior.checkpoint_domain_id().clone(),
        *prior.domain_epoch(),
        *prior.domain_sequence(),
        ControlRecordCoordinate::new(
            prior_ref.clone(),
            ControlRecordDigest::parse(prior.canonical_control_checkpoint_sha256().as_str())
                .unwrap(),
        ),
    )
    .into();
    Anchors {
        reads: [
            Ok(Some(minimum.clone())),
            Ok(Some(domain.clone())),
            Ok(Some(minimum)),
            Ok(Some(domain)),
        ]
        .into(),
        keys: Vec::new(),
    }
}

impl AntiRollbackAnchorPort for Anchors {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.keys.push(key.clone());
        self.reads.pop_front().expect("no unbounded anchor reads")
    }
    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("preparation must not advance anchors")
    }
    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("preparation reads exact keys without a proposed transition")
    }
}

#[test]
fn exact_reads_bracket_preparation_and_keep_distinct_store_coordinates() {
    let (mut store, control, records, finalization) = setup();
    let prior = records.checkpoint(&control);
    let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
    let mut anchor = anchors(&records, &prior_ref, &prior);
    let bytes = snapshot(&records)
        .checkpoint_record()
        .canonical_record_jcs()
        .as_slice()
        .len();
    let _prepared = profile()
        .prepare_finalization(
            AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
            (&prior_ref, &prior),
            bytes,
            limits(),
            &mut store,
            &mut anchor,
        )
        .unwrap();
    assert_eq!(store.trust_requests.len(), 2);
    assert_eq!(
        store.trust_requests,
        vec![(snapshot(&records).binding().clone(), bytes); 2]
    );
    assert_eq!(anchor.keys.len(), 4);
    assert_eq!(anchor.keys[0], anchor.keys[2]);
    assert_eq!(anchor.keys[1], anchor.keys[3]);
    assert!(anchor.reads.is_empty());
    assert!(store.trust_reads.is_empty());
    assert_ne!(
        snapshot(&records).binding().trust_store_id(),
        prior.control_store_id()
    );
}

#[test]
fn missing_unreadable_or_substituted_anchor_fails_at_each_read() {
    for position in 0..4 {
        for fault in 0..3 {
            let (mut store, control, records, finalization) = setup();
            let prior = records.checkpoint(&control);
            let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
            let mut anchor = anchors(&records, &prior_ref, &prior);
            anchor.reads[position] = match fault {
                0 => Ok(None),
                1 => Err(AntiRollbackAnchorError::new(
                    AntiRollbackAnchorErrorKind::Unavailable,
                )),
                _ => Ok(anchor.reads[(position + 1) % 4].as_ref().unwrap().clone()),
            };
            let result = profile().prepare_finalization(
                AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
                (&prior_ref, &prior),
                65_536,
                limits(),
                &mut store,
                &mut anchor,
            );
            match fault {
                1 => assert!(matches!(
                    result,
                    Err(PrincipalCheckpointError::AnchorRead(_))
                )),
                _ if position % 2 == 0 => assert!(matches!(
                    result,
                    Err(PrincipalCheckpointError::TrustMinimum)
                )),
                _ => assert!(matches!(
                    result,
                    Err(PrincipalCheckpointError::ControlAnchor)
                )),
            }
            assert_eq!(anchor.keys.len(), position + 1);
            assert_eq!(store.request.is_some(), position >= 2);
            assert_eq!(store.transaction_reads, usize::from(position >= 2));
        }
    }
}

#[test]
fn changed_trust_body_binding_or_absence_fails_before_and_after_tree_preparation() {
    for position in 0..2 {
        for fault in 0..4 {
            let (mut store, control, records, finalization) = setup();
            let prior = records.checkpoint(&control);
            let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
            let expected = snapshot(&records);
            store.trust_reads[position] = match fault {
                0 => Ok(None),
                1 => Ok(Some(
                    PrincipalTrustSnapshot::new(
                        PrincipalTrustStoreBinding::new(
                            ControlStoreIdV1::parse("foreign-trust-store").unwrap(),
                            expected.binding().trust_store_incarnation_id().clone(),
                            expected.binding().principal_id().clone(),
                        ),
                        expected.through().clone(),
                        expected.checkpoint_record().clone(),
                    )
                    .unwrap(),
                )),
                2 => Ok(Some(
                    PrincipalTrustSnapshot::new(
                        expected.binding().clone(),
                        expected.through().clone(),
                        ControlRecord::new(
                            expected.checkpoint_record().coordinate().clone(),
                            expected.checkpoint_record().schema_id().clone(),
                            expected.checkpoint_record().schema_sha256().clone(),
                            ControlRecordBytes::new(b"{}".to_vec()).unwrap(),
                        ),
                    )
                    .unwrap(),
                )),
                _ => Err(ControlStoreError::new(
                    pale_ports::ControlStoreErrorKind::Storage,
                )),
            };
            let mut anchor = anchors(&records, &prior_ref, &prior);
            let result = profile().prepare_finalization(
                AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
                (&prior_ref, &prior),
                65_536,
                limits(),
                &mut store,
                &mut anchor,
            );
            if fault == 3 {
                assert!(matches!(
                    result,
                    Err(PrincipalCheckpointError::TrustRead(_))
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(PrincipalCheckpointError::CurrentTrust)
                ));
            }
            assert_eq!(store.trust_requests.len(), position + 1);
            assert_eq!(anchor.keys.len(), position * 2);
            assert_eq!(store.request.is_some(), position == 1);
        }
    }
}

#[test]
fn zero_and_undersized_trust_bounds_fail_without_port_access() {
    for bound in [
        0,
        snapshot(&setup().2)
            .checkpoint_record()
            .canonical_record_jcs()
            .as_slice()
            .len()
            - 1,
    ] {
        let (mut store, control, records, finalization) = setup();
        let prior = records.checkpoint(&control);
        let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
        let mut anchor = anchors(&records, &prior_ref, &prior);
        let result = profile().prepare_finalization(
            AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
            (&prior_ref, &prior),
            bound,
            limits(),
            &mut store,
            &mut anchor,
        );
        assert!(matches!(
            result,
            Err(PrincipalCheckpointError::TrustReadBound)
        ));
        assert!(store.trust_requests.is_empty());
        assert!(anchor.keys.is_empty());
        assert!(store.request.is_none());
    }
}

#[test]
fn same_kind_anchor_identity_position_and_record_substitutions_are_not_freshness() {
    for phase in 0..2 {
        for fault in 0..11 {
            let (mut store, control, records, finalization) = setup();
            let prior = records.checkpoint(&control);
            let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
            let mut anchor = anchors(&records, &prior_ref, &prior);
            let position = phase * 2 + usize::from(fault >= 4);
            let original = anchor.reads[position].as_ref().unwrap().as_ref().unwrap();
            let changed: AntiRollbackAnchor = match original {
                AntiRollbackAnchor::PrincipalTrustMinimum(value) => {
                    let mut principal = value.principal_id().clone();
                    let mut epoch = value.checkpoint_epoch();
                    let mut coordinate = value.coordinate().clone();
                    match fault {
                        0 => principal = ResidentPrincipalIdV1::parse("foreign-principal").unwrap(),
                        1 => epoch = epoch.checked_successor().unwrap(),
                        2 => {
                            coordinate = ControlRecordCoordinate::new(
                                ResidentRecordRefV1::parse("foreign-trust-record").unwrap(),
                                coordinate.record_sha256().clone(),
                            )
                        }
                        3 => {
                            coordinate = ControlRecordCoordinate::new(
                                coordinate.record_ref().clone(),
                                ControlRecordDigest::parse("f".repeat(64)).unwrap(),
                            )
                        }
                        _ => unreachable!(),
                    }
                    PrincipalTrustMinimumAnchor::new(principal, epoch, coordinate).into()
                }
                AntiRollbackAnchor::ControlDomain(value) => {
                    let mut store = value.control_store_id().clone();
                    let mut incarnation = value.control_incarnation_id().clone();
                    let mut domain = value.checkpoint_domain_id().clone();
                    let mut epoch = value.domain_epoch();
                    let mut sequence = value.domain_sequence();
                    let mut coordinate = value.coordinate().clone();
                    match fault {
                        4 => store = records.trust.trust_store_id().clone(),
                        5 => {
                            incarnation =
                                ControlIncarnationIdV1::parse("foreign-incarnation").unwrap()
                        }
                        6 => domain = ControlCheckpointDomainIdV1::parse("foreign-domain").unwrap(),
                        7 => epoch = ControlDomainEpochV1::new(epoch.get() + 1).unwrap(),
                        8 => sequence = sequence.checked_successor().unwrap(),
                        9 => {
                            coordinate = ControlRecordCoordinate::new(
                                ResidentRecordRefV1::parse("foreign-control-record").unwrap(),
                                coordinate.record_sha256().clone(),
                            )
                        }
                        10 => {
                            coordinate = ControlRecordCoordinate::new(
                                coordinate.record_ref().clone(),
                                ControlRecordDigest::parse("f".repeat(64)).unwrap(),
                            )
                        }
                        _ => unreachable!(),
                    }
                    ControlDomainAntiRollbackAnchor::new(
                        store,
                        incarnation,
                        domain,
                        epoch,
                        sequence,
                        coordinate,
                    )
                    .into()
                }
            };
            assert_ne!(&changed, original);
            anchor.reads[position] = Ok(Some(changed));
            let result = profile().prepare_finalization(
                AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
                (&prior_ref, &prior),
                65_536,
                limits(),
                &mut store,
                &mut anchor,
            );
            if fault < 4 {
                assert!(matches!(
                    result,
                    Err(PrincipalCheckpointError::TrustMinimum)
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(PrincipalCheckpointError::ControlAnchor)
                ));
            }
            assert_eq!(store.request.is_some(), phase == 1);
            assert_eq!(anchor.keys.len(), position + 1);
        }
    }
}
