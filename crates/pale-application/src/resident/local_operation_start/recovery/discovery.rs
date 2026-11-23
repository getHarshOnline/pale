//! Bounded discovery of committed operation starts after process restart.
//!
//! Physical enumeration is not semantic recovery. One turn reads at most one
//! source from a transaction prefix frozen by the first read. The caller may
//! advance the cursor only after that source has a closed application-owned
//! disposition.

use pale_ports::{
    AuthorityStoreCoordinate, ControlStoreError, OperationCustodyPort, OperationStartCursor,
    OperationStartReadError, OperationStartSource, MAX_OPERATION_START_SOURCE_BYTES,
};

/// Per-deployment position in one frozen committed-start prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct LocalOperationStartDiscoveryAttempt {
    store: AuthorityStoreCoordinate,
    cursor: Option<OperationStartCursor>,
    exhausted: bool,
}

impl LocalOperationStartDiscoveryAttempt {
    /// Start before the first committed operation in one exact store incarnation.
    pub(in crate::resident) const fn new(store: AuthorityStoreCoordinate) -> Self {
        Self {
            store,
            cursor: None,
            exhausted: false,
        }
    }
}

/// One discovered source plus the cursor that follows only after classification.
#[derive(Debug)]
#[must_use = "classify the source before advancing its frozen-prefix cursor"]
pub(in crate::resident) struct PendingLocalOperationStartDiscovery {
    source: OperationStartSource,
    successor: LocalOperationStartDiscoveryAttempt,
}

impl PendingLocalOperationStartDiscovery {
    /// Borrow the physical source. Its bytes still have no application meaning.
    pub(in crate::resident) const fn source(&self) -> &OperationStartSource {
        &self.source
    }

    /// Advance only after the source has a closed application-owned disposition.
    pub(in crate::resident) fn advance(self) -> LocalOperationStartDiscoveryAttempt {
        self.successor
    }
}

/// One bounded discovery turn.
#[derive(Debug)]
pub(in crate::resident) enum LocalOperationStartDiscoveryTurn {
    /// The frozen prefix is empty or fully processed.
    Exhausted(LocalOperationStartDiscoveryAttempt),
    /// Exactly one source awaits application admission and classification.
    Pending(PendingLocalOperationStartDiscovery),
}

/// Why physical discovery could not select one trustworthy source.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum LocalOperationStartDiscoveryError {
    /// The configured aggregate private-basis ceiling is invalid.
    #[error("operation recovery source byte bound is invalid")]
    Bounds,
    /// Storage could not return a complete frozen-prefix page.
    #[error(transparent)]
    Read(#[from] OperationStartReadError),
    /// The returned page did not preserve the requested cursor or one-source law.
    #[error("operation recovery source page is inconsistent")]
    Page,
    /// A continuation coordinate could not be formed from the returned source.
    #[error(transparent)]
    Cursor(#[from] ControlStoreError),
}

/// Read at most one committed start from this deployment's frozen prefix.
///
/// A successful pending turn does not mutate `attempt`. The returned linear
/// value carries both the retry position and the successor position so a failed
/// semantic recovery cannot silently skip work.
pub(in crate::resident) fn discover_local_operation_start(
    attempt: LocalOperationStartDiscoveryAttempt,
    custody: &mut dyn OperationCustodyPort,
    maximum_session_basis_bytes: usize,
) -> Result<LocalOperationStartDiscoveryTurn, LocalOperationStartDiscoveryError> {
    discover_local_operation_start_with(
        attempt,
        maximum_session_basis_bytes,
        |store, cursor, bytes| custody.read_operation_starts(store, cursor, 1, bytes),
    )
}

fn discover_local_operation_start_with<Read>(
    attempt: LocalOperationStartDiscoveryAttempt,
    maximum_session_basis_bytes: usize,
    read: Read,
) -> Result<LocalOperationStartDiscoveryTurn, LocalOperationStartDiscoveryError>
where
    Read: FnOnce(
        &AuthorityStoreCoordinate,
        Option<&OperationStartCursor>,
        usize,
    ) -> Result<pale_ports::OperationStartSources, OperationStartReadError>,
{
    if maximum_session_basis_bytes == 0
        || maximum_session_basis_bytes > MAX_OPERATION_START_SOURCE_BYTES
    {
        return Err(LocalOperationStartDiscoveryError::Bounds);
    }
    if attempt.exhausted {
        return Ok(LocalOperationStartDiscoveryTurn::Exhausted(attempt));
    }
    let page = read(
        &attempt.store,
        attempt.cursor.as_ref(),
        maximum_session_basis_bytes,
    )?;
    let through = page.through();
    if page.sources.len() > 1
        || attempt
            .cursor
            .as_ref()
            .is_some_and(|cursor| cursor.through() != through)
    {
        return Err(LocalOperationStartDiscoveryError::Page);
    }
    let Some(source) = page.sources.into_iter().next() else {
        if page.continuation.is_some() {
            return Err(LocalOperationStartDiscoveryError::Page);
        }
        let mut exhausted = attempt;
        exhausted.exhausted = true;
        return Ok(LocalOperationStartDiscoveryTurn::Exhausted(exhausted));
    };
    if source.transaction().get() > through.get() {
        return Err(LocalOperationStartDiscoveryError::Page);
    }
    let basis = source.session_basis();
    let next_cursor = OperationStartCursor::new(
        attempt.store.clone(),
        through,
        source.transaction(),
        basis.handle().clone(),
        basis.request().clone(),
        basis.session_lease_digest().clone(),
        basis.basis_sha256().clone(),
    )?;
    match page.continuation.as_ref() {
        Some(continuation) if continuation != &next_cursor => {
            return Err(LocalOperationStartDiscoveryError::Page);
        }
        None | Some(_) => {}
    }
    let successor = LocalOperationStartDiscoveryAttempt {
        store: attempt.store.clone(),
        cursor: Some(next_cursor),
        exhausted: page.continuation.is_none(),
    };
    Ok(LocalOperationStartDiscoveryTurn::Pending(
        PendingLocalOperationStartDiscovery { source, successor },
    ))
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/local_operation_start/recovery/discovery.rs"]
mod tests;
