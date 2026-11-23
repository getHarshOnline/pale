//! Cold-restart laws for one committed pre-Task A2A send.

use super::a2a_delegate::{run_committed_a2a_send_cold_restart, CommittedA2aSendRestartCase};

#[test]
fn committed_send_recovers_as_durable_uncertainty_without_carrier_replay() {
    run_committed_a2a_send_cold_restart(CommittedA2aSendRestartCase::Exact);
}

#[test]
fn committed_send_recovery_rejects_unlike_process_associations_without_replay() {
    for case in [
        CommittedA2aSendRestartCase::Selector,
        CommittedA2aSendRestartCase::Store,
        CommittedA2aSendRestartCase::RouteSaturated,
        CommittedA2aSendRestartCase::RouteConflict,
        CommittedA2aSendRestartCase::Bounds,
        CommittedA2aSendRestartCase::Request,
        CommittedA2aSendRestartCase::Route,
        CommittedA2aSendRestartCase::DeploymentAuthority,
        CommittedA2aSendRestartCase::Checkpoint,
        CommittedA2aSendRestartCase::Anchor,
    ] {
        run_committed_a2a_send_cold_restart(case);
    }
}
