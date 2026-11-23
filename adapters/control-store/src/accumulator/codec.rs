//! Finite private encoding of the pinned trie's physical node variants.
//! Decoding establishes representation only, never authenticated membership.

use pale_ports::{AccumulatorPosition, ControlStoreError};
use sparse_merkle_tree::{merge::MergeValue, BranchNode, H256};

pub(super) const MAX_BRANCH_BYTES: usize = 132;

pub(super) fn hash(bytes: &[u8]) -> Result<H256, ControlStoreError> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| super::invalid())?;
    Ok(bytes.into())
}

fn take<const N: usize>(bytes: &mut &[u8]) -> Result<[u8; N], ControlStoreError> {
    let result = bytes
        .get(..N)
        .ok_or_else(super::invalid)?
        .try_into()
        .map_err(|_| super::invalid())?;
    *bytes = &bytes[N..];
    Ok(result)
}

fn merge(bytes: &mut &[u8]) -> Result<MergeValue, ControlStoreError> {
    match take::<1>(bytes)?[0] {
        1 => Ok(MergeValue::Value(take::<32>(bytes)?.into())),
        2 => Ok(MergeValue::MergeWithZero {
            base_node: take::<32>(bytes)?.into(),
            zero_bits: take::<32>(bytes)?.into(),
            zero_count: take::<1>(bytes)?[0],
        }),
        3 => Ok(MergeValue::ShortCut {
            key: take::<32>(bytes)?.into(),
            value: take::<32>(bytes)?.into(),
            height: take::<1>(bytes)?[0],
        }),
        _ => Err(super::invalid()),
    }
}

pub(super) fn branch(mut bytes: &[u8]) -> Result<BranchNode, ControlStoreError> {
    if !(66..=MAX_BRANCH_BYTES).contains(&bytes.len()) {
        return Err(super::invalid());
    }
    let node = BranchNode {
        left: merge(&mut bytes)?,
        right: merge(&mut bytes)?,
    };
    if !bytes.is_empty() {
        return Err(super::invalid());
    }
    Ok(node)
}

pub(super) fn valid_bytes(position: AccumulatorPosition, bytes: &[u8]) -> bool {
    match position {
        AccumulatorPosition::Leaf(_) => hash(bytes).is_ok(),
        AccumulatorPosition::Branch { .. } => branch(bytes).is_ok(),
    }
}

fn encode_merge(value: &MergeValue, bytes: &mut Vec<u8>) {
    match value {
        MergeValue::Value(hash) => {
            bytes.push(1);
            bytes.extend_from_slice(hash.as_slice());
        }
        MergeValue::MergeWithZero {
            base_node,
            zero_bits,
            zero_count,
        } => {
            bytes.push(2);
            bytes.extend_from_slice(base_node.as_slice());
            bytes.extend_from_slice(zero_bits.as_slice());
            bytes.push(*zero_count);
        }
        MergeValue::ShortCut { key, value, height } => {
            bytes.push(3);
            bytes.extend_from_slice(key.as_slice());
            bytes.extend_from_slice(value.as_slice());
            bytes.push(*height);
        }
    }
}

pub(super) fn encode_branch(node: &BranchNode) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(MAX_BRANCH_BYTES);
    encode_merge(&node.left, &mut bytes);
    encode_merge(&node.right, &mut bytes);
    bytes
}
