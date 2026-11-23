//! Test-only construction for the private resident operation client failure.

use super::{ResidentOperationClientError, ResidentRequestTransmissionCertainty};

impl ResidentOperationClientError {
    pub(crate) fn test(message: &'static str) -> Self {
        Self {
            source: std::io::Error::other(message).into(),
            transmission: ResidentRequestTransmissionCertainty::MayHaveBeenSent,
        }
    }
}
