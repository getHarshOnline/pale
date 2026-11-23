//! Orthogonal wire vocabularies for session mode and reasoning effort.
//!
//! [`ModeAxis`] separates persistent task stance from intensity, while
//! [`ModeScope`] prevents session-only selections from becoming defaults.
//! Renderers map [`ReasoningEffort`] to a harness without naming models here.

wire_enum!(
    /// Orthogonal session-mode dimensions. Intensity controls how much process
    /// guidance is applied; task stance controls the kind of work being done.
    ModeAxis {
        /// Amount of process and verification guidance applied.
        Intensity => "intensity",
        /// Kind of specialist work being performed.
        TaskStance => "task_stance",
    }
);

wire_enum!(
    /// Declarative layer that supplied an effective session-mode default.
    ModeDefaultSource {
        /// Distribution fallback supplied the default.
        Distribution => "distribution",
        /// Principal-level preference supplied the default.
        User => "user",
        /// Project configuration supplied the default.
        Project => "project",
    }
);

wire_enum!(
    /// Persistence scope of a session run mode. Session-only modes cannot become
    /// persisted defaults.
    ModeScope {
        /// The mode may be selected as a persisted project default.
        Runtime => "runtime",
        /// The mode may be selected only for the current session.
        SessionOnly => "session_only",
    }
);

wire_enum!(
    /// Host-neutral reasoning-effort tier for specialist work. A renderer maps
    /// the tier to a native harness setting without naming a model here.
    ReasoningEffort {
        /// Strongest tier for architecture, planning, and difficult judgment.
        Deep => "deep",
        /// Ordinary tier for specialist work.
        Standard => "standard",
        /// Lightest tier for shallow or mechanical work.
        Fast => "fast",
    }
);
