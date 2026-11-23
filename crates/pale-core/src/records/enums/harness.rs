//! Closed wire vocabulary for child propagation.

wire_enum!(
    /// Whether the selected project context enters a native-harness child.
    /// Selectors narrow `propagate`; `exempt` and `none` withhold context for
    /// distinct policy reasons without becoming free-form strings.
    ChildPropagation {
        /// Carry selected context into the child under the same ceilings.
        Propagate => "propagate",
        /// Withhold context because the child is explicitly exempt.
        Exempt => "exempt",
        /// Withhold context because this policy declares no propagation.
        None => "none",
    }
);
