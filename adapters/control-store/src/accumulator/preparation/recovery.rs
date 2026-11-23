//! Prove the original logical transition backwards against the committed tree.

use super::*;
use pale_ports::OperationAccumulatorCheckpoint;

pub(crate) fn recover_committed(
    connection: &Connection,
    storage: Store,
    request: &OperationAccumulatorPreparation,
    successor: &OperationAccumulatorCheckpoint,
) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
    let AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
        return Err(invalid().into());
    };
    if !selected_frontier_matches(
        connection,
        storage,
        successor.profile_ref(),
        successor.profile_sha256(),
        successor.checkpoint(),
        successor.state_root(),
        successor.unresolved_root(),
    )? {
        return Err(invalid().into());
    }
    let budget = Budget {
        maximum_reads: request.limits().maximum_reads(),
        maximum_writes: request.limits().maximum_writes(),
        maximum_positions: request.limits().maximum_positions(),
        maximum_proof_bytes: request.limits().maximum_proof_bytes(),
        work: RefCell::new(Work::default()),
    };
    let reverse = |tree, changes: &[AccumulatorLeafChange], root, expected| {
        let inverse = changes
            .iter()
            .map(|change| {
                AccumulatorLeafChange::new(
                    change.key().clone(),
                    change.successor().cloned(),
                    change.predecessor().cloned(),
                )
                .map_err(|_| invalid())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let overlay = SnapshotOverlay {
            storage,
            connection,
            domain: request.basis().domain(),
            epoch: request.basis().epoch(),
            profile_ref: prior.profile_ref(),
            profile_sha256: prior.profile_sha256(),
            tree,
            budget: &budget,
            touched: BTreeMap::new(),
        };
        let (reconstructed, mutations) =
            prepare_tree(overlay, root, &inverse).map_err(|error| budget.error(error))?;
        if &reconstructed != expected {
            return Err(invalid().into());
        }
        mutations
            .into_iter()
            .map(|mutation| {
                AccumulatorMutation::new(
                    mutation.position(),
                    mutation.successor().map(<[u8]>::to_vec),
                    mutation.predecessor().map(<[u8]>::to_vec),
                    codec::MAX_BRANCH_BYTES,
                )
                .map_err(|_| invalid().into())
            })
            .collect::<Result<Vec<_>, OperationAccumulatorPreparationError>>()
    };
    let state = reverse(
        "state",
        request.state().changes(),
        successor.state_root(),
        prior.state_root(),
    )?;
    let unresolved = reverse(
        "unresolved",
        request.unresolved().changes(),
        successor.unresolved_root(),
        prior.unresolved_root(),
    )?;
    let state = StateAccumulatorBatch::new(
        prior.state_root().clone(),
        successor.state_root().clone(),
        state,
        request.limits().maximum_positions(),
    )
    .map_err(|_| invalid())?;
    let unresolved = UnresolvedCustodyAccumulatorBatch::new(
        prior.unresolved_root().clone(),
        successor.unresolved_root().clone(),
        unresolved,
        request.limits().maximum_positions(),
    )
    .map_err(|_| invalid())?;
    PreparedOperationAccumulator::new(request.clone(), state, unresolved)
        .map_err(|_| invalid().into())
}
