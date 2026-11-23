//! Closed physical selection for the two native checkpoint stores.
//! SQL text is compiled once; callers cannot provide table names.

#[derive(Clone, Copy)]
pub(crate) enum Store {
    DeploymentOperation,
    PrincipalTrust,
}

pub(super) struct Statements {
    pub(super) domain_absent: &'static str,
    pub(super) position: &'static str,
    pub(super) frontier_matches: &'static str,
    pub(super) upsert_position: &'static str,
    pub(super) remove_position: &'static str,
    pub(super) insert_frontier: &'static str,
    pub(super) advance_frontier: &'static str,
}

macro_rules! statements {
    ($position:literal, $frontier:literal, $head:literal) => {
        Statements {
            domain_absent: concat!("SELECT NOT EXISTS(SELECT 1 FROM ", $head, " WHERE checkpoint_domain_id=?1) \
         AND NOT EXISTS(SELECT 1 FROM ", $frontier, " WHERE checkpoint_domain_id=?1) \
         AND NOT EXISTS(SELECT 1 FROM ", $position, " WHERE checkpoint_domain_id=?1)"),
            position: concat!("SELECT CASE WHEN typeof(node_bytes)='blob' AND length(node_bytes) BETWEEN 1 AND 132 THEN node_bytes END \
         FROM ", $position, " WHERE checkpoint_domain_id=?1 AND domain_epoch=?2 \
         AND checkpoint_profile_ref=?3 AND checkpoint_profile_sha256=?4 AND tree_kind=?5 AND position_kind=?6 AND position_key=?7"),
            frontier_matches: concat!("SELECT EXISTS(SELECT 1 FROM ", $frontier, " WHERE checkpoint_domain_id=?1 AND domain_epoch=?2 \
         AND checkpoint_profile_ref=?3 AND checkpoint_profile_sha256=?4 AND control_checkpoint_ref=?5 AND control_checkpoint_sha256=?6 \
         AND state_root_sha256=?7 AND unresolved_root_sha256=?8)"),
            upsert_position: concat!("INSERT INTO ", $position, " \
                     (checkpoint_domain_id,domain_epoch,checkpoint_profile_ref,checkpoint_profile_sha256,tree_kind,position_kind,position_key,node_bytes) \
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8) \
                     ON CONFLICT(checkpoint_domain_id,domain_epoch,checkpoint_profile_ref,checkpoint_profile_sha256,tree_kind,position_kind,position_key) \
                     DO UPDATE SET node_bytes=excluded.node_bytes"),
            remove_position: concat!("DELETE FROM ", $position, " WHERE checkpoint_domain_id=?1 AND domain_epoch=?2 \
                     AND checkpoint_profile_ref=?3 AND checkpoint_profile_sha256=?4 AND tree_kind=?5 AND position_kind=?6 AND position_key=?7"),
            insert_frontier: concat!("INSERT INTO ", $frontier, " \
             (checkpoint_domain_id,domain_epoch,checkpoint_profile_ref,checkpoint_profile_sha256,control_checkpoint_ref,control_checkpoint_sha256,state_root_sha256,unresolved_root_sha256) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)"),
            advance_frontier: concat!("UPDATE ", $frontier, " SET control_checkpoint_ref=?1, control_checkpoint_sha256=?2, state_root_sha256=?3, unresolved_root_sha256=?4 \
         WHERE checkpoint_domain_id=?5 AND domain_epoch=?6 AND checkpoint_profile_ref=?7 AND checkpoint_profile_sha256=?8 \
         AND control_checkpoint_ref=?9 AND control_checkpoint_sha256=?10 AND state_root_sha256=?11 AND unresolved_root_sha256=?12"),
        }
    };
}

const DEPLOYMENT: Statements = statements!(
    "pale_deployment_operation_accumulator_position",
    "pale_deployment_operation_accumulator_frontier",
    "pale_deployment_operation_checkpoint_head"
);
const PRINCIPAL: Statements = statements!(
    "pale_principal_control_accumulator_position",
    "pale_principal_control_accumulator_frontier",
    "pale_principal_control_checkpoint_head"
);

impl Store {
    pub(super) const fn statements(self) -> &'static Statements {
        match self {
            Self::DeploymentOperation => &DEPLOYMENT,
            Self::PrincipalTrust => &PRINCIPAL,
        }
    }
}
