//! Bounded native key/value representation; semantic laws remain in the port.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, PrincipalTrustCheckpointEpochV1,
    ResidentPrincipalIdV1,
};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorKey, ControlDomainAntiRollbackAnchor,
    ControlRecordCoordinate, PrincipalTrustMinimumAnchor,
};
use serde::{Deserialize, Serialize};

use crate::StoreFailure;

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Value {
    Principal {
        version: u8,
        principal: ResidentPrincipalIdV1,
        epoch: PrincipalTrustCheckpointEpochV1,
        coordinate: ControlRecordCoordinate,
    },
    Control {
        version: u8,
        store: ControlStoreIdV1,
        incarnation: ControlIncarnationIdV1,
        domain: ControlCheckpointDomainIdV1,
        epoch: ControlDomainEpochV1,
        sequence: ControlDomainSequenceV1,
        coordinate: ControlRecordCoordinate,
    },
}

pub(super) fn physical_key(
    prefix: &[u8],
    key: &AntiRollbackAnchorKey,
) -> Result<Vec<u8>, StoreFailure> {
    let preimage = match key {
        AntiRollbackAnchorKey::PrincipalTrustMinimum(principal) => {
            serde_json::to_vec(&(1, "principal", principal))
        }
        AntiRollbackAnchorKey::ControlDomain {
            control_store_id,
            control_incarnation_id,
            checkpoint_domain_id,
        } => serde_json::to_vec(&(
            1,
            "control",
            control_store_id,
            control_incarnation_id,
            checkpoint_domain_id,
        )),
    }
    .map_err(|_| StoreFailure::Protocol)?;
    let mut result = prefix.to_vec();
    result.extend_from_slice(URL_SAFE_NO_PAD.encode(preimage).as_bytes());
    Ok(result)
}

pub(super) fn encode(anchor: &AntiRollbackAnchor) -> Result<Vec<u8>, StoreFailure> {
    let value = match anchor {
        AntiRollbackAnchor::PrincipalTrustMinimum(anchor) => Value::Principal {
            version: 1,
            principal: anchor.principal_id().clone(),
            epoch: anchor.checkpoint_epoch(),
            coordinate: anchor.coordinate().clone(),
        },
        AntiRollbackAnchor::ControlDomain(anchor) => Value::Control {
            version: 1,
            store: anchor.control_store_id().clone(),
            incarnation: anchor.control_incarnation_id().clone(),
            domain: anchor.checkpoint_domain_id().clone(),
            epoch: anchor.domain_epoch(),
            sequence: anchor.domain_sequence(),
            coordinate: anchor.coordinate().clone(),
        },
    };
    serde_json::to_vec(&value).map_err(|_| StoreFailure::Protocol)
}

pub(super) fn decode(bytes: &[u8]) -> Result<AntiRollbackAnchor, StoreFailure> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| StoreFailure::Protocol)?;
    let anchor = match value {
        Value::Principal {
            version: 1,
            principal,
            epoch,
            coordinate,
        } => PrincipalTrustMinimumAnchor::new(principal, epoch, coordinate).into(),
        Value::Control {
            version: 1,
            store,
            incarnation,
            domain,
            epoch,
            sequence,
            coordinate,
        } => ControlDomainAntiRollbackAnchor::new(
            store,
            incarnation,
            domain,
            epoch,
            sequence,
            coordinate,
        )
        .into(),
        _ => return Err(StoreFailure::Protocol),
    };
    if encode(&anchor)? != bytes {
        return Err(StoreFailure::Protocol);
    }
    Ok(anchor)
}
