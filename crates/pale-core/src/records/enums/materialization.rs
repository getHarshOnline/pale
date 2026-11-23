//! Materialization plan, ownership, operation, and outcome values.

wire_enum!(
    /// The operation class a materialization plan belongs to. `apply` (fresh
    /// install) and `remove` (retire owned surfaces) are the only classes; a
    /// plan of one class cannot drive the other.
    PlanKind {
        /// Materialize the planned owned surface set.
        Apply => "apply",
        /// Retire the exact previously owned surface set.
        Remove => "remove"
    }
);

wire_enum!(
    /// A run's aggregate outcome, or a single target's outcome.
    Outcome {
        /// Every declared target completed successfully.
        Succeeded => "succeeded",
        /// At least one target succeeded and at least one did not.
        Partial => "partial",
        /// The run failed before publishing any target effect.
        FailedNoEffect => "failed_no_effect",
    }
);

wire_enum!(
    /// The action a receipt operation performed on a surface.
    SurfaceAction {
        /// Create a previously absent owned surface.
        Create => "create",
        /// Explicitly take ownership of a pre-existing generated surface while
        /// replacing either the whole file or one exact marker-delimited block
        /// under the admitted full-file baseline.
        Adopt => "adopt",
        /// Replace an existing owned whole-file surface.
        Update => "update",
        /// Verify an owned surface without changing it.
        Unchanged => "unchanged",
        /// Remove a previously owned surface.
        Delete => "delete",
        /// Replace only the marker-delimited block in a user-owned file.
        Splice => "splice",
    }
);

wire_enum!(
    /// The exact filesystem region Pale owns for one generated surface.
    /// Whole-file ownership binds every byte; managed-block ownership binds only
    /// the marker-delimited block inside a user-owned file.
    SurfaceOwnership {
        /// Own only the exact marker-delimited block.
        ManagedBlock => "managed_block",
        /// Own every byte of the target file.
        WholeFile => "whole_file",
    }
);
