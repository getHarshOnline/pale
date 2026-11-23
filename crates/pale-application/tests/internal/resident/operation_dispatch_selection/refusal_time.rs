//! Exercise retained elapsed time through public re-entry, without sleeps.

use super::*;

impl NativeOperationInvocationRefusal<'_, '_> {
    pub(in crate::resident) fn assert_elapsed_budget_retained(
        mut self,
        input: NativeOperationExecutionContext<'_>,
        authority: &mut impl AmbientSessionAuthorityPort,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Self {
        let timeout = Duration::from_millis(
            self.retained
                .release
                .dispatch
                .attempt
                .context
                .lease
                .timeout_millis()
                .get(),
        );
        // Only this test-owned method changes an Instant. Production always
        // captures Instant::now once at initial invocation and retains it.
        let aged = self.retained.release.started.checked_sub(timeout).unwrap();
        self.retained.release.started = aged;
        let again = || NativeOperationExecutionContext {
            current: input.current,
            resolve_store: input.resolve_store,
            clock: input.clock,
            maximum_record_bytes: input.maximum_record_bytes,
        };
        let refused = match self.recheck(again(), authority, custody, anchor) {
            Err(value) => value,
            Ok(_) => panic!("expired monotonic budget cannot release the handler"),
        };
        assert!(matches!(refused.reason(), OperationAttemptError::Deadline));
        assert_eq!(refused.retained.release.started, aged);
        let refused = match refused.recheck(again(), authority, custody, anchor) {
            Err(value) => value,
            Ok(_) => panic!("deadline refusal remains closed"),
        };
        assert!(matches!(refused.reason(), OperationAttemptError::Deadline));
        assert_eq!(refused.retained.release.started, aged);
        refused
    }
}
