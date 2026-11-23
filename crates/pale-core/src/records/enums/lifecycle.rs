//! Lifecycle event, predicate, action, and delivery values.

wire_enum_all!(
    /// Host-neutral lifecycle event. The wire value is the runtime token; native
    /// host event names are projection concerns outside this crate.
    LifecycleEvent {
        /// A native-harness session begins.
        SessionStart => "session-start",
        /// The user submits a prompt before model execution.
        UserPromptSubmit => "user-prompt-submit",
        /// The host expands or augments a submitted prompt.
        UserPromptExpansion => "user-prompt-expansion",
        /// The host requests permission for a proposed effect.
        PermissionRequest => "permission-request",
        /// A tool request is visible before execution.
        PreToolUse => "pre-tool-use",
        /// A tool request completed successfully.
        PostToolUse => "post-tool-use",
        /// A tool request completed with failure.
        PostToolUseFailure => "post-tool-use-failure",
        /// A native-harness child agent starts.
        SubagentStart => "subagent-start",
        /// A native-harness child agent stops.
        SubagentStop => "subagent-stop",
        /// The host is about to compact session context.
        PreCompact => "pre-compact",
        /// The host has compacted session context.
        PostCompact => "post-compact",
        /// The harness is evaluating whether work may stop.
        Stop => "stop",
        /// The native-harness session ends.
        SessionEnd => "session-end",
    }
);

impl LifecycleEvent {
    /// Complete host-neutral lifecycle vocabulary in stable declaration order.
    pub fn vocabulary() -> impl ExactSizeIterator<Item = Self> {
        Self::declared_variants().iter().copied()
    }
}

wire_enum!(
    /// Deterministic predicate over bounded facts supplied by a native harness.
    /// Nuanced judgment remains in doctrine and specialist workflows.
    PredicateKind {
        /// Match every occurrence of the declared event.
        Always => "always",
        /// Match a validated native-harness tool-call identity.
        ToolMatch => "tool_match",
        /// Match a literal recursive-force deletion command.
        DestructiveShell => "destructive_shell",
        /// Match a literal Git invocation that bypasses repository hooks.
        GitHookBypass => "git_hook_bypass",
        /// Match when the host reports a structured write target.
        WriteEffect => "write_effect",
        /// Match a structured write target against a declared path glob.
        PathMatch => "path_match",
        /// Match a normalized message beginning with a declared command token.
        CommandPrefix => "command_prefix",
        /// Match a normalized whole message, never a substring.
        ExactMessage => "exact_message",
        /// Match when the sealed run-mode selection changed.
        ModeChanged => "mode_changed",
        /// Match while a post-compaction context refresh is armed.
        RebaselinePending => "rebaseline_pending",
        /// Match the first edit to a path in the current session.
        FirstTouch => "first_touch",
        /// Match only while the required connector is admitted.
        ConnectorBound => "connector_bound",
        /// Keep the declared behavior intentionally inert.
        Never => "never",
    }
);

wire_enum!(
    /// Deterministic lifecycle effect applied when a behavior predicate holds.
    LifecycleAction {
        /// Add compact guidance to the current session or child.
        Inject => "inject",
        /// Add only the sealed active-mode payload.
        InjectModes => "inject_modes",
        /// Block the observed tool request.
        Deny => "deny",
        /// Ask for confirmation before the observed tool request.
        Ask => "ask",
        /// Surface a non-blocking warning.
        Warn => "warn",
        /// Emit model-executed verification or review guidance.
        Guide => "guide",
        /// Emit a completion or status notification.
        Notify => "notify",
        /// Record a bounded audit entry through its governed sink.
        Capture => "capture",
        /// Select the run mode named by the behavior.
        Activate => "activate",
        /// Restore the affected run-mode axis to its resolved default.
        Deactivate => "deactivate",
        /// Arm one sealed context refresh after compaction.
        Rebaseline => "rebaseline",
        /// Emit a bounded budget, cost, or economy signal.
        Signal => "signal",
        /// Recognize the event without producing an effect.
        NoOp => "no_op",
    }
);

wire_enum!(
    /// Policy for an unevaluable predicate or failed lifecycle effect.
    FailurePosture {
        /// Deny or block when a safety-relevant result is uncertain.
        FailClosed => "fail_closed",
        /// Continue when declared guidance is unavailable.
        FailOpen => "fail_open",
        /// Retry a bounded transient failure.
        Retry => "retry",
        /// Steer the model with explicit fallback guidance.
        Steer => "steer",
        /// Escalate the unresolved condition to the host or owner.
        Escalate => "escalate",
    }
);

wire_enum!(
    /// Where a behavior's state lives. Durable state requires an explicitly
    /// named owner; message and session state are not durable-memory substitutes.
    StateScope {
        /// Retain no state after evaluating the event.
        Stateless => "stateless",
        /// Retain state only for one prompt/response turn.
        Message => "message",
        /// Retain state for one native-harness session.
        Session => "session",
        /// Retain cross-session state through an explicitly governed owner.
        Durable => "durable",
    }
);
