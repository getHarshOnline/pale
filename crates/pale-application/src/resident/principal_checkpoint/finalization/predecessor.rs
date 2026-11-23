//! Compare the selected signed predecessor with current physical and independent custody.

use super::*;
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorPort, ControlDomainAntiRollbackAnchor,
    PrincipalTrustCheckpointHead, PrincipalTrustMinimumAnchor, PrincipalTrustReadPort,
    PrincipalTrustSnapshot, PrincipalTrustStoreBinding,
};

/// Exact admitted bytes and coordinates reused before and after native preparation.
pub(super) struct Expected {
    snapshot: PrincipalTrustSnapshot,
    minimum: AntiRollbackAnchor,
    domain: AntiRollbackAnchor,
    maximum_bytes: usize,
}

impl Expected {
    pub(super) fn new(
        trust_ref: &ResidentRecordRefV1,
        trust: &PrincipalTrustCheckpointV1,
        control_ref: &ResidentRecordRefV1,
        control: &ControlStateCheckpointV1,
        maximum_bytes: usize,
    ) -> Result<Self, PrincipalCheckpointError> {
        let coordinate = record_projection::coordinate(
            trust_ref,
            trust.canonical_trust_checkpoint_sha256().as_str(),
        )
        .map_err(PrincipalBootstrapProjectionError::from)?;
        let record = record_projection::record(coordinate.clone(), trust)
            .map_err(PrincipalBootstrapProjectionError::from)?;
        if maximum_bytes == 0 || record.canonical_record_jcs().as_slice().len() > maximum_bytes {
            return Err(PrincipalCheckpointError::TrustReadBound);
        }
        let snapshot = PrincipalTrustSnapshot::new(
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
            record,
        )
        .map_err(|_| PrincipalCheckpointError::Association)?;
        let minimum = PrincipalTrustMinimumAnchor::new(
            trust.principal_id().clone(),
            *trust.checkpoint_epoch(),
            coordinate,
        )
        .into();
        let domain = ControlDomainAntiRollbackAnchor::new(
            control.control_store_id().clone(),
            control.control_incarnation_id().clone(),
            control.checkpoint_domain_id().clone(),
            *control.domain_epoch(),
            *control.domain_sequence(),
            record_projection::coordinate(
                control_ref,
                control.canonical_control_checkpoint_sha256().as_str(),
            )
            .map_err(PrincipalBootstrapProjectionError::from)?,
        )
        .into();
        Ok(Self {
            snapshot,
            minimum,
            domain,
            maximum_bytes,
        })
    }

    pub(super) fn verify<P: PrincipalTrustReadPort + ?Sized>(
        &self,
        custody: &mut P,
        anchors: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<(), PrincipalCheckpointError> {
        let current = custody
            .read_current_checkpoint(self.snapshot.binding(), self.maximum_bytes)?
            .ok_or(PrincipalCheckpointError::CurrentTrust)?;
        if current
            .checkpoint_record()
            .canonical_record_jcs()
            .as_slice()
            .len()
            > self.maximum_bytes
        {
            return Err(PrincipalCheckpointError::TrustReadBound);
        }
        if current != self.snapshot {
            return Err(PrincipalCheckpointError::CurrentTrust);
        }
        if anchors.read(&self.minimum.key())?.as_ref() != Some(&self.minimum) {
            return Err(PrincipalCheckpointError::TrustMinimum);
        }
        if anchors.read(&self.domain.key())?.as_ref() != Some(&self.domain) {
            return Err(PrincipalCheckpointError::ControlAnchor);
        }
        Ok(())
    }
}
