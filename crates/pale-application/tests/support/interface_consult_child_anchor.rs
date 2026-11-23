//! Interrupt an initial collection's anchor update without repairing it during inspection.

use super::*;

pub(super) type InterruptedTransition = Rc<RefCell<Option<PreparedAntiRollbackAnchorTransition>>>;

pub(super) struct CollectionAnchor<'a> {
    pub(super) base: &'a mut ConformanceAntiRollbackAnchors,
    pub(super) interrupt: bool,
    pub(super) collection: SavedCollection,
    pub(super) parent: InterruptedParent,
    pub(super) transition: InterruptedTransition,
}

impl AntiRollbackAnchorPort for CollectionAnchor<'_> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.base.read(key)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.base.observe(transition)
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        if self.interrupt {
            if let Some(collection) = self.collection.borrow().as_ref() {
                assert_eq!(
                    self.base.observe(transition).unwrap(),
                    AntiRollbackAnchorObservation::ExactPredecessor,
                    "the collection checkpoint has not reached the anchor"
                );
                if let Some(expected) = self.transition.borrow().as_ref() {
                    assert_eq!(transition, expected, "retry only the same prepared update");
                }
                *self.transition.borrow_mut() = Some(transition.clone());
                *self.parent.borrow_mut() = Some(coordinate(collection.parent()));
                return Err(AntiRollbackAnchorError::new(
                    AntiRollbackAnchorErrorKind::Unavailable,
                ));
            }
        }
        self.base.compare_and_advance(transition)
    }
}

fn coordinate(reference: &ImmutableRecordReferenceV1) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        reference.record_ref.clone(),
        ControlRecordDigest::parse(reference.record_sha256.as_str().to_owned()).unwrap(),
    )
}

/// Capture physical bytes only; recovery and anchor repair belong to the next service.
pub(super) fn snapshot_committed_collection(
    store: &AuthorityStoreCoordinate,
    request: &CommonRequestEnvelopeV1,
    collection: &ConsultationCollection,
    custody: &mut dyn OperationCustodyPort,
) -> (ResidentOperationFrameRecords, ControlRecord, ControlRecord) {
    let parent = coordinate(collection.parent());
    let handle = custody
        .read_operation_record(store, &parent, MAXIMUM_RECORD_BYTES)
        .unwrap()
        .unwrap();
    let request_key = crate::resident::operation_custody::operation_request_key(request).unwrap();
    let observation = custody
        .read_index(
            store,
            &OperationCustodyIndexQuery::StartObservation(request_key),
            MAXIMUM_RECORD_BYTES,
        )
        .unwrap();
    let observation = observation.record().unwrap();
    let records = ResidentOperationFrameRecords::new(
        handle.coordinate().record_ref().as_str().to_owned(),
        String::from_utf8(handle.canonical_record_jcs().as_slice().to_vec()).unwrap(),
        observation.coordinate().record_ref().as_str().to_owned(),
        String::from_utf8(observation.canonical_record_jcs().as_slice().to_vec()).unwrap(),
    );
    let [slot] = collection.slots() else {
        panic!("one actual child")
    };
    let ConsultationContributionSource::FreshChild { child } = slot.roster().source() else {
        panic!("an issued child")
    };
    let child = custody
        .read_operation_record(store, &coordinate(child), MAXIMUM_RECORD_BYTES)
        .unwrap()
        .unwrap();
    let collection = custody
        .read_index(
            store,
            &OperationCustodyIndexQuery::CollectionHead(
                OperationCustodyIndexKey::for_collection_head(&parent),
            ),
            MAXIMUM_RECORD_BYTES,
        )
        .unwrap();
    (
        records,
        child,
        collection
            .record()
            .expect("collection committed before the anchor call")
            .clone(),
    )
}
