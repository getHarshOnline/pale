//! Read-through preparation and membership over one borrowed `SQLite` snapshot.
//! Only touched writes are retained; no whole-tree copy or SQL mutation occurs.

use std::{cell::RefCell, collections::BTreeMap};

use pale_core::records::ArtifactDigest;
use pale_ports::{
    AccumulatorLeafChange, AccumulatorMutation, AccumulatorPosition, AccumulatorPreparationBasis,
    ControlStoreError, ControlStoreErrorKind, OperationAccumulatorMembership,
    OperationAccumulatorPreparation, OperationAccumulatorPreparationError,
    PreparedOperationAccumulator, StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
    VerifiedOperationAccumulatorMembership,
};
use rusqlite::{params, types::ValueRef, Connection};
use sha2::{Digest, Sha256};
use sparse_merkle_tree::{
    error::Error as TreeError,
    merge::MergeValue,
    traits::{Hasher, StoreReadOps, StoreWriteOps},
    BranchKey, BranchNode, MerkleProof, SparseMerkleTree, H256,
};

use super::{codec, domain_is_absent, invalid, position_parts, selected_frontier_matches, Store};
use crate::error::{sqlite, sqlite_integer};

mod recovery;
pub(crate) use recovery::recover_committed;

#[derive(Default)]
struct CheckpointHasher(Sha256);

impl Hasher for CheckpointHasher {
    fn write_h256(&mut self, hash: &H256) {
        self.0.update(hash.as_slice());
    }
    fn write_byte(&mut self, byte: u8) {
        self.0.update([byte]);
    }
    fn finish(self) -> H256 {
        let bytes: [u8; 32] = self.0.finalize().into();
        bytes.into()
    }
}

#[derive(Default)]
struct Work {
    reads: usize,
    writes: usize,
    positions: usize,
    proof_bytes: usize,
    // The upstream trait has a String-only storage error. Keep our typed cause
    // privately across that boundary rather than classifying message text.
    failure: Option<OperationAccumulatorPreparationError>,
}

struct Budget {
    maximum_reads: usize,
    maximum_writes: usize,
    maximum_positions: usize,
    maximum_proof_bytes: usize,
    work: RefCell<Work>,
}

impl Budget {
    fn fail(&self, error: OperationAccumulatorPreparationError) -> TreeError {
        self.work.borrow_mut().failure.get_or_insert(error);
        TreeError::Store("checkpoint snapshot preparation failed".into())
    }

    fn charge(
        &self,
        counter: fn(&mut Work) -> &mut usize,
        amount: usize,
        limit: usize,
    ) -> Result<(), TreeError> {
        let accepted = {
            let mut work = self.work.borrow_mut();
            let used = counter(&mut work);
            if let Some(next) = used.checked_add(amount).filter(|next| *next <= limit) {
                *used = next;
                true
            } else {
                false
            }
        };
        if accepted {
            Ok(())
        } else {
            Err(self.fail(OperationAccumulatorPreparationError::Limit))
        }
    }

    fn read(&self) -> Result<(), TreeError> {
        self.charge(|work| &mut work.reads, 1, self.maximum_reads)
    }
    fn write(&self) -> Result<(), TreeError> {
        self.charge(|work| &mut work.writes, 1, self.maximum_writes)
    }
    fn position(&self) -> Result<(), TreeError> {
        self.charge(|work| &mut work.positions, 1, self.maximum_positions)
    }
    fn proof(&self, bytes: usize) -> Result<(), TreeError> {
        self.charge(
            |work| &mut work.proof_bytes,
            bytes,
            self.maximum_proof_bytes,
        )
    }
    fn ensure_proof_fits(&self, bound: usize) -> Result<(), TreeError> {
        let fits = self
            .work
            .borrow()
            .proof_bytes
            .checked_add(bound)
            .is_some_and(|total| total <= self.maximum_proof_bytes);
        if fits {
            Ok(())
        } else {
            Err(self.fail(OperationAccumulatorPreparationError::Limit))
        }
    }
    fn error(&self, error: TreeError) -> OperationAccumulatorPreparationError {
        self.work.borrow_mut().failure.take().unwrap_or_else(|| {
            ControlStoreError::with_source(ControlStoreErrorKind::RecoveryRequired, error).into()
        })
    }
}

struct Touched {
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
}

struct SnapshotOverlay<'a> {
    storage: Store,
    connection: &'a Connection,
    domain: &'a pale_core::records::ControlCheckpointDomainIdV1,
    epoch: pale_core::records::ControlDomainEpochV1,
    profile_ref: &'a pale_core::records::ResidentRecordRefV1,
    profile_sha256: &'a pale_core::records::CheckpointProfileDigest,
    tree: &'static str,
    budget: &'a Budget,
    touched: BTreeMap<AccumulatorPosition, Touched>,
}

impl SnapshotOverlay<'_> {
    // Every call is charged, including misses and overlay hits. SQL constrains
    // length/type before we borrow and decode; only finite valid bytes are copied.
    // This does not bound SQLite's own page cache or I/O latency.
    fn read(&self, position: AccumulatorPosition) -> Result<Option<Vec<u8>>, TreeError> {
        self.budget.read()?;
        if let Some(touched) = self.touched.get(&position) {
            return Ok(touched.after.clone());
        }
        self.read_snapshot(position)
            .map_err(|error| self.budget.fail(error.into()))
    }

    fn read_snapshot(
        &self,
        position: AccumulatorPosition,
    ) -> Result<Option<Vec<u8>>, ControlStoreError> {
        let (kind, key) = position_parts(position);
        let mut statement = self
            .connection
            .prepare(self.storage.statements().position)
            .map_err(|error| sqlite("prepare snapshot accumulator read", error))?;
        let mut rows = statement
            .query(params![
                self.domain.as_str(),
                sqlite_integer(self.epoch.get())?,
                self.profile_ref.as_str(),
                self.profile_sha256.as_str(),
                self.tree,
                kind,
                key.as_slice()
            ])
            .map_err(|error| sqlite("read snapshot accumulator position", error))?;
        let Some(row) = rows
            .next()
            .map_err(|error| sqlite("read snapshot accumulator row", error))?
        else {
            return Ok(None);
        };
        match row
            .get_ref(0)
            .map_err(|error| sqlite("borrow snapshot accumulator bytes", error))?
        {
            ValueRef::Blob(bytes) if codec::valid_bytes(position, bytes) => {
                Ok(Some(bytes.to_vec()))
            }
            _ => Err(invalid()),
        }
    }

    fn write(
        &mut self,
        position: AccumulatorPosition,
        encode: impl FnOnce() -> Option<Vec<u8>>,
    ) -> Result<(), TreeError> {
        self.budget.write()?;
        if let Some(touched) = self.touched.get_mut(&position) {
            touched.after = encode();
        } else {
            // Reserve before allocating/retaining a new entry, including a
            // subsequently reverted intermediate write. Two trees share this cap.
            self.budget.position()?;
            let before = self.read(position)?;
            self.touched.insert(
                position,
                Touched {
                    before,
                    after: encode(),
                },
            );
        }
        Ok(())
    }

    fn mutations(self) -> Result<Vec<AccumulatorMutation>, TreeError> {
        self.touched
            .into_iter()
            .filter(|(_, touched)| touched.before != touched.after)
            .map(|(position, touched)| {
                AccumulatorMutation::new(
                    position,
                    touched.before,
                    touched.after,
                    codec::MAX_BRANCH_BYTES,
                )
                .map_err(|_| TreeError::CorruptedProof)
            })
            .collect()
    }
}

fn branch_position(key: &BranchKey) -> AccumulatorPosition {
    AccumulatorPosition::Branch {
        height: key.height,
        path: key.node_key.into(),
    }
}

impl StoreReadOps<H256> for SnapshotOverlay<'_> {
    fn get_branch(&self, key: &BranchKey) -> Result<Option<BranchNode>, TreeError> {
        self.read(branch_position(key))?
            .as_deref()
            .map(codec::branch)
            .transpose()
            .map_err(|error| self.budget.fail(error.into()))
    }
    fn get_leaf(&self, key: &H256) -> Result<Option<H256>, TreeError> {
        self.read(AccumulatorPosition::Leaf((*key).into()))?
            .as_deref()
            .map(codec::hash)
            .transpose()
            .map_err(|error| self.budget.fail(error.into()))
    }
}

impl StoreWriteOps<H256> for SnapshotOverlay<'_> {
    fn insert_branch(&mut self, key: BranchKey, node: BranchNode) -> Result<(), TreeError> {
        self.write(branch_position(&key), || Some(codec::encode_branch(&node)))
    }
    fn insert_leaf(&mut self, key: H256, value: H256) -> Result<(), TreeError> {
        self.write(AccumulatorPosition::Leaf(key.into()), || {
            Some(value.as_slice().to_vec())
        })
    }
    fn remove_branch(&mut self, key: &BranchKey) -> Result<(), TreeError> {
        self.write(branch_position(key), || None)
    }
    fn remove_leaf(&mut self, key: &H256) -> Result<(), TreeError> {
        self.write(AccumulatorPosition::Leaf((*key).into()), || None)
    }
}

fn root_hash(root: &ArtifactDigest) -> Result<H256, TreeError> {
    let mut bytes = [0; 32];
    hex::decode_to_slice(root.as_str(), &mut bytes).map_err(|_| TreeError::CorruptedProof)?;
    Ok(bytes.into())
}

fn root_digest(root: H256) -> Result<ArtifactDigest, TreeError> {
    ArtifactDigest::parse(hex::encode(root.as_slice())).map_err(|_| TreeError::CorruptedProof)
}

// Pinned compile() has no bounded writer. Bound its encoded output using its
// public proof structure, without reproducing its ordering/stack/compiler:
// one L per leaf, at most n-1 H merges, one P(33) or Q(66) per path item;
// at most one two-byte zero run before each merge and after each leaf.
// This bounds encoded bytes, not Vec spare capacity or allocator overhead.
fn compiled_proof_bound(proof: &MerkleProof) -> Option<usize> {
    let leaves = proof.leaves_count();
    let merges = leaves.checked_sub(1)?;
    let base = leaves.checked_add(merges)?.checked_mul(3)?;
    proof.merkle_path().iter().try_fold(base, |bound, node| {
        let size = match node {
            MergeValue::Value(_) => 35,
            MergeValue::MergeWithZero { .. } => 68,
            // compile's unreachable branch cannot receive private nodes.
            MergeValue::ShortCut { .. } => return None,
        };
        bound.checked_add(size)
    })
}

fn verify_selected_proof(
    tree: &SparseMerkleTree<CheckpointHasher, H256, SnapshotOverlay<'_>>,
    leaves: Vec<(H256, H256)>,
) -> Result<sparse_merkle_tree::CompiledMerkleProof, TreeError> {
    let keys: Vec<_> = leaves.iter().map(|(key, _)| *key).collect();
    let proof = tree.merkle_proof(keys.clone())?;
    let bound = compiled_proof_bound(&proof).ok_or(TreeError::CorruptedProof)?;
    tree.store().budget.ensure_proof_fits(bound)?;
    let proof = proof.compile(keys)?;
    if proof.0.len() > bound {
        return Err(TreeError::CorruptedProof);
    }
    tree.store().budget.proof(proof.0.len())?;
    if !proof.verify::<CheckpointHasher>(tree.root(), leaves)? {
        return Err(TreeError::CorruptedProof);
    }
    Ok(proof)
}

fn prepare_tree(
    overlay: SnapshotOverlay<'_>,
    root: &ArtifactDigest,
    changes: &[AccumulatorLeafChange],
) -> Result<(ArtifactDigest, Vec<AccumulatorMutation>), TreeError> {
    if changes.is_empty() {
        return Ok((root.clone(), Vec::new()));
    }
    // Pinned proof construction: <=256 path entries/leaf; compilation emits
    // at most a leaf opcode plus 256*(66-byte sibling + 2-byte zero run).
    // Guard arithmetic BEFORE upstream allocation. Proof construction is bounded by leaf/read
    // ceilings and this fixed-depth growth. The actual proof structure is
    // checked against remaining encoded-byte budget before compile below.
    changes
        .len()
        .checked_mul(1 + 256 * 68)
        .filter(|bound| *bound <= isize::MAX as usize)
        .ok_or_else(|| {
            overlay
                .budget
                .fail(OperationAccumulatorPreparationError::Limit)
        })?;
    let old_root = root_hash(root)?;
    let mut tree = SparseMerkleTree::<CheckpointHasher, H256, _>::new(old_root, overlay);
    let mut before = Vec::with_capacity(changes.len());
    let mut after = Vec::with_capacity(changes.len());
    for change in changes {
        let key = root_hash(change.key())?;
        let expected = change.predecessor().map(root_hash).transpose()?;
        // Read directly, including when root is zero: tree.get shortcuts empty
        // trees and would hide an unexpected persisted selected leaf.
        if tree.store().get_leaf(&key)? != expected {
            return Err(TreeError::CorruptedProof);
        }
        before.push((key, expected.unwrap_or_default()));
        after.push((
            key,
            change
                .successor()
                .map(root_hash)
                .transpose()?
                .unwrap_or_default(),
        ));
    }
    let proof = verify_selected_proof(&tree, before)?;
    let proof_successor = proof.compute_root::<CheckpointHasher>(after.clone())?;
    drop(proof);
    tree.update_all(after)?;
    // Selected membership alone does not authenticate a shortcut's height.
    // This equality detects corrupted metadata that changes update mechanics.
    if *tree.root() != proof_successor {
        return Err(TreeError::CorruptedProof);
    }
    let root = root_digest(proof_successor)?;
    Ok((root, tree.take_store().mutations()?))
}

pub(crate) fn prepare(
    connection: &Connection,
    storage: Store,
    request: &OperationAccumulatorPreparation,
) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
    let basis = request.basis();
    let (state_prior, unresolved_prior) = match basis {
        AccumulatorPreparationBasis::Initial(_) => {
            if !domain_is_absent(connection, storage, basis.domain())? {
                return Err(invalid().into());
            }
            let empty = root_digest(H256::default()).map_err(|_| invalid())?;
            (empty.clone(), empty)
        }
        AccumulatorPreparationBasis::Successor(prior) => {
            if !selected_frontier_matches(
                connection,
                storage,
                prior.profile_ref(),
                prior.profile_sha256(),
                prior.checkpoint(),
                prior.state_root(),
                prior.unresolved_root(),
            )? {
                return Err(invalid().into());
            }
            (prior.state_root().clone(), prior.unresolved_root().clone())
        }
    };
    let budget = Budget {
        maximum_reads: request.limits().maximum_reads(),
        maximum_writes: request.limits().maximum_writes(),
        maximum_positions: request.limits().maximum_positions(),
        maximum_proof_bytes: request.limits().maximum_proof_bytes(),
        work: RefCell::new(Work::default()),
    };
    let overlay = |tree| SnapshotOverlay {
        storage,
        connection,
        domain: basis.domain(),
        epoch: basis.epoch(),
        profile_ref: basis.profile_ref(),
        profile_sha256: basis.profile_sha256(),
        tree,
        budget: &budget,
        touched: BTreeMap::new(),
    };
    let (state_root, state) =
        prepare_tree(overlay("state"), &state_prior, request.state().changes())
            .map_err(|error| budget.error(error))?;
    let (unresolved_root, unresolved) = prepare_tree(
        overlay("unresolved"),
        &unresolved_prior,
        request.unresolved().changes(),
    )
    .map_err(|error| budget.error(error))?;
    let state = StateAccumulatorBatch::new(
        state_prior,
        state_root,
        state,
        request.limits().maximum_positions(),
    )
    .map_err(|_| invalid())?;
    let unresolved = UnresolvedCustodyAccumulatorBatch::new(
        unresolved_prior,
        unresolved_root,
        unresolved,
        request.limits().maximum_positions(),
    )
    .map_err(|_| invalid())?;
    PreparedOperationAccumulator::new(request.clone(), state, unresolved)
        .map_err(|_| invalid().into())
}

pub(crate) fn verify_membership(
    connection: &Connection,
    storage: Store,
    request: &OperationAccumulatorMembership,
) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
    let checkpoint = request.checkpoint();
    if !selected_frontier_matches(
        connection,
        storage,
        checkpoint.profile_ref(),
        checkpoint.profile_sha256(),
        checkpoint.checkpoint(),
        checkpoint.state_root(),
        checkpoint.unresolved_root(),
    )? {
        return Err(invalid().into());
    }
    // No writes or retained positions are permitted by this read-only path.
    let budget = Budget {
        maximum_reads: request.limits().maximum_reads(),
        maximum_writes: 0,
        maximum_positions: 0,
        maximum_proof_bytes: request.limits().maximum_proof_bytes(),
        work: RefCell::new(Work::default()),
    };
    request
        .leaves()
        .len()
        .checked_mul(1 + 256 * 68)
        .filter(|bound| *bound <= isize::MAX as usize)
        .ok_or(OperationAccumulatorPreparationError::Limit)?;
    let overlay = SnapshotOverlay {
        storage,
        connection,
        domain: checkpoint.checkpoint().checkpoint_domain_id(),
        epoch: checkpoint.checkpoint().domain_epoch(),
        profile_ref: checkpoint.profile_ref(),
        profile_sha256: checkpoint.profile_sha256(),
        tree: "state",
        budget: &budget,
        touched: BTreeMap::new(),
    };
    let verify = || -> Result<(), TreeError> {
        let tree = SparseMerkleTree::<CheckpointHasher, H256, _>::new(
            root_hash(checkpoint.state_root())?,
            overlay,
        );
        let mut leaves = Vec::with_capacity(request.leaves().len());
        for leaf in request.leaves() {
            let key = root_hash(leaf.key())?;
            let value = root_hash(leaf.value())?;
            if tree.store().get_leaf(&key)? != Some(value) {
                return Err(TreeError::CorruptedProof);
            }
            leaves.push((key, value));
        }
        drop(verify_selected_proof(&tree, leaves)?);
        Ok(())
    };
    verify().map_err(|error| budget.error(error))?;
    Ok(VerifiedOperationAccumulatorMembership::new(request.clone()))
}

#[cfg(test)]
#[path = "../../tests/internal/accumulator/preparation.rs"]
mod tests;
