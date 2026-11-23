//! Physical operation-scoped binding candidates, never an authenticated task set.

use super::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordCoordinate, ControlStoreError,
    OperationCustodyIndexKey,
};
use crate::ControlStoreErrorKind;
use pale_core::records::{
    ControlTransactionSequenceV1, ResidentRecordRefV1, A2A_TASK_BINDING_V1_SCHEMA,
};

/// Maximum candidates returned by one indexed seek (plus one look-ahead).
pub const MAX_OPERATION_A2A_BINDINGS: usize = 128;
/// Maximum combined binding-body bytes returned by one page.
pub const MAX_OPERATION_A2A_BINDING_BYTES: usize = 8 * 1024 * 1024;

/// Existing opaque handle key and exact immutable handle coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationA2aBindingScope {
    operation_key: OperationCustodyIndexKey,
    handle: ControlRecordCoordinate,
}
impl OperationA2aBindingScope {
    /// Pair owner-derived coordinates; the adapter must check their index join.
    #[must_use]
    pub const fn new(
        operation_key: OperationCustodyIndexKey,
        handle: ControlRecordCoordinate,
    ) -> Self {
        Self {
            operation_key,
            handle,
        }
    }
    /// Existing operation identity key; this port does not derive it.
    #[must_use]
    pub const fn operation_key(&self) -> &OperationCustodyIndexKey {
        &self.operation_key
    }
    /// Exact immutable handle, including digest.
    #[must_use]
    pub const fn handle(&self) -> &ControlRecordCoordinate {
        &self.handle
    }
}

/// Private physical continuation. This is not an authenticated recipient token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationA2aBindingCursor {
    store: AuthorityStoreCoordinate,
    scope: OperationA2aBindingScope,
    through: ControlTransactionSequenceV1,
    after_transaction: ControlTransactionSequenceV1,
    after_reference: ResidentRecordRefV1,
}
impl OperationA2aBindingCursor {
    /// Retain one scope and inclusive prefix without asserting its existence.
    /// # Errors
    /// Rejects a position beyond the retained transaction prefix.
    pub fn new(
        store: AuthorityStoreCoordinate,
        scope: OperationA2aBindingScope,
        through: ControlTransactionSequenceV1,
        after_transaction: ControlTransactionSequenceV1,
        after_reference: ResidentRecordRefV1,
    ) -> Result<Self, ControlStoreError> {
        if after_transaction.get() > through.get() {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        Ok(Self {
            store,
            scope,
            through,
            after_transaction,
            after_reference,
        })
    }
    /// Exact store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }
    /// Exact operation scope.
    #[must_use]
    pub const fn scope(&self) -> &OperationA2aBindingScope {
        &self.scope
    }
    /// Inclusive retained transaction frontier.
    #[must_use]
    pub const fn through(&self) -> ControlTransactionSequenceV1 {
        self.through
    }
    /// Last returned insertion transaction.
    #[must_use]
    pub const fn after_transaction(&self) -> ControlTransactionSequenceV1 {
        self.after_transaction
    }
    /// Last returned immutable reference, breaking transaction ties.
    #[must_use]
    pub const fn after_reference(&self) -> &ResidentRecordRefV1 {
        &self.after_reference
    }
}

/// One physically joined binding; no semantic or signature admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationA2aBindingSource {
    binding_key: OperationCustodyIndexKey,
    transaction: ControlTransactionSequenceV1,
    binding: ControlRecord,
}
impl OperationA2aBindingSource {
    /// Retain the immutable binding after its physical index/stamp checks.
    /// # Errors
    /// Rejects a record from another family.
    pub fn new(
        binding_key: OperationCustodyIndexKey,
        transaction: ControlTransactionSequenceV1,
        binding: ControlRecord,
    ) -> Result<Self, ControlStoreError> {
        if binding.schema_id().as_str() != A2A_TASK_BINDING_V1_SCHEMA {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        Ok(Self {
            binding_key,
            transaction,
            binding,
        })
    }
    /// Existing operation-and-binding identity key.
    #[must_use]
    pub const fn binding_key(&self) -> &OperationCustodyIndexKey {
        &self.binding_key
    }
    /// Original binding insertion transaction.
    #[must_use]
    pub const fn transaction(&self) -> ControlTransactionSequenceV1 {
        self.transaction
    }
    /// Exact bounded signed-record bytes, still unadmitted.
    #[must_use]
    pub const fn binding(&self) -> &ControlRecord {
        &self.binding
    }
}

/// Bounded physical page. Exhaustion proves no authenticated absence.
#[derive(Debug)]
pub struct OperationA2aBindings {
    /// Candidates in insertion-transaction/reference order.
    pub bindings: Vec<OperationA2aBindingSource>,
    /// Position in the same immutable prefix, if more indexed rows remain.
    pub continuation: Option<OperationA2aBindingCursor>,
}
