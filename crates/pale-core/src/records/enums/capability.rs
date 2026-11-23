//! Capability availability, effect, approval, provider, and support values.

wire_enum!(
    /// Whether an operation strictly needs a tool capability or can proceed without it.
    Requirement {
        /// The operation cannot produce its declared result without the capability.
        Required => "required",
        /// Absence does not block the operation's core result.
        Optional => "optional",
    }
);

wire_enum!(
    /// What remains possible when an operation capability has no usable provider.
    CapabilityUnavailability {
        /// No truthful operation result can be produced.
        Block => "block",
        /// A separately declared lower-effect result remains valid.
        Degrade => "degrade",
    }
);

wire_enum!(
    /// Host-specific support after resolving every operation capability.
    OperationSupport {
        /// Every required capability has a usable provider.
        Full => "full",
        /// Only the operation's declared lower-effect result is available.
        Degraded => "degraded",
        /// A required capability has no truthful fallback.
        Blocked => "blocked",
    }
);

wire_enum!(
    /// Whether private configuration selected a concrete implementation.
    BindingState {
        /// No private provider selection exists for the semantic requirement.
        Unbound => "unbound",
        /// A private provider selection exists; this does not prove reachability.
        Bound => "bound",
    }
);

wire_enum!(
    /// Latest execution availability observed for a privately selected provider.
    ProviderExecutionAvailability {
        /// No current owner-native observation is available.
        Unknown => "unknown",
        /// A current observation found the provider reachable.
        Available => "available",
        /// A current observation found the provider unreachable.
        Unavailable => "unavailable",
    }
);

wire_enum!(
    /// Observable effect class used for approval and authority-boundary policy.
    EffectClass {
        /// Deterministically transform only caller-supplied data.
        Compute => "compute",
        /// Read state without mutation.
        Read => "read",
        /// Run a local command without a remote write effect.
        Execute => "execute",
        /// Mutate local state.
        Write => "write",
        /// Read from a remote service.
        Network => "network",
        /// Write to a remote service or outward-facing destination.
        Publish => "publish",
    }
);

wire_enum!(
    /// The approval a tool capability requires before it acts.
    Approval {
        /// The admitted effect needs no additional confirmation.
        None => "none",
        /// Ask the user before proceeding.
        Ask => "ask",
        /// Require an exact bounded authorization before proceeding.
        Gated => "gated",
    }
);

wire_enum!(
    /// Support state for a declared operation, role, tool, or provider binding.
    Availability {
        /// A real admitted mechanic exists in the current runtime.
        AvailableNow => "available_now",
        /// No current runtime can carry the declaration truthfully.
        HonestUnavailable => "honest_unavailable",
        /// Guidance exists while a named owner governs the unavailable effect.
        Governed => "governed",
    }
);

wire_enum!(
    /// Mechanism by which a [`crate::records::BoundedTool`] carries out its
    /// declared effect. Availability and provider admission are separate checks.
    Mechanics {
        /// Deterministic mechanics implemented in the Pale Rust runtime.
        NativeRust => "native_rust",
        /// External capability carried through an admitted connector.
        Connector => "connector",
        /// Bounded operation performed through a host command-line surface.
        HostCli => "host_cli",
        /// Bounded package-owned script executed by the native harness.
        Script => "script",
    }
);

wire_enum!(
    /// Concrete provider selected by private project configuration.
    ///
    /// Absence of a binding is represented by [`BindingState::Unbound`], and a
    /// provider outage is represented by [`ProviderExecutionAvailability::Unavailable`].
    /// Neither condition is a provider kind.
    CapabilityProvider {
        /// Native-harness tool bound by private project configuration.
        HostNative => "host_native",
        /// Pure or bounded mechanic implemented by Pale.
        NativeRust => "native_rust",
        /// External capability carried by an admitted connector.
        Connector => "connector",
    }
);

wire_enum!(
    /// How a native harness exposes one concrete tool call to Pale. This is a
    /// property of the host protocol, not of the capability's semantic effect.
    /// An unobservable tool may still be useful, but it cannot satisfy an
    /// operation contract enforced by Pale Runtime.
    ToolObservability {
        /// The exact request is visible before the tool executes.
        PreToolObservable => "pre_tool_observable",
        /// The request is visible only at the host permission boundary.
        PermissionOnly => "permission_only",
        /// The host does not expose the request to lifecycle admission.
        Unobservable => "unobservable",
    }
);
