// Mechanical organization values and endpoint selectors; the SDK owns each query.

/// Namespace-qualified scope, not a generic resource or resident workspace ID.
#[napi(object, use_nullable = true)]
pub struct OrganizationScopeRef {
    /// Owning namespace.
    pub namespace: String,
    /// Scope-local identity.
    pub scope_id: String,
}
impl OrganizationScopeRef {
    fn into_api(self) -> pale_sdk::OrganizationScopeRef {
        pale_sdk::OrganizationScopeRef {
            namespace: self.namespace,
            scope_id: self.scope_id,
        }
    }

    fn from_api(value: &pale_sdk::OrganizationScopeRef) -> Self {
        Self {
            namespace: value.namespace.clone(),
            scope_id: value.scope_id.clone(),
        }
    }
}

/// Namespace-qualified organization.
#[napi(object, use_nullable = true)]
pub struct OrganizationRef {
    /// Owning namespace.
    pub namespace: String,
    /// Organization-local identity.
    pub organization_id: String,
}
impl OrganizationRef {
    fn from_api(value: &pale_sdk::OrganizationRef) -> Self {
        Self {
            namespace: value.namespace.clone(),
            organization_id: value.organization_id.clone(),
        }
    }
}

/// Namespace-qualified group, independently associated with its organization.
#[napi(object, use_nullable = true)]
pub struct OrganizationGroupRef {
    /// Owning namespace.
    pub namespace: String,
    /// Group-local identity.
    pub group_id: String,
}
impl OrganizationGroupRef {
    fn from_api(value: &pale_sdk::OrganizationGroupRef) -> Self {
        Self {
            namespace: value.namespace.clone(),
            group_id: value.group_id.clone(),
        }
    }
}

/// Namespace-qualified discipline tree.
#[napi(object, use_nullable = true)]
pub struct OrganizationTreeRef {
    /// Owning namespace.
    pub namespace: String,
    /// Tree-local identity.
    pub tree_id: String,
}
impl OrganizationTreeRef {
    fn from_api(value: &pale_sdk::OrganizationTreeRef) -> Self {
        Self {
            namespace: value.namespace.clone(),
            tree_id: value.tree_id.clone(),
        }
    }
}

/// Namespace-qualified stable role, not its occupant or provider.
#[napi(object, use_nullable = true)]
pub struct OrganizationRoleRef {
    /// Owning namespace.
    pub namespace: String,
    /// Role-local identity.
    pub role_id: String,
}
impl OrganizationRoleRef {
    fn from_api(value: &pale_sdk::OrganizationRoleRef) -> Self {
        Self {
            namespace: value.namespace.clone(),
            role_id: value.role_id.clone(),
        }
    }
}

/// Exact directional-service identity; both scope coordinates participate.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationServiceRef {
    /// Contract-owning namespace.
    pub namespace: String,
    /// Contract-local identity.
    pub contract_id: String,
    /// Exact provider scope.
    pub provider_scope: OrganizationScopeRef,
    /// Exact consumer scope.
    pub consumer_scope: OrganizationScopeRef,
}
impl OrganizationServiceRef {
    fn from_api(value: &pale_sdk::OrganizationServiceRef) -> Self {
        Self {
            namespace: value.namespace.clone(),
            contract_id: value.contract_id.clone(),
            provider_scope: OrganizationScopeRef::from_api(&value.provider_scope),
            consumer_scope: OrganizationScopeRef::from_api(&value.consumer_scope),
        }
    }
}

/// Existing endpoint kind; no identity is inferred from a display label.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    use_nullable = true
)]
pub enum OrganizationNode {
    /// Logical scope.
    Scope {
        /// Exact owning variant payload.
        scope: OrganizationScopeRef,
    },
    /// Logical organization.
    Organization {
        /// Exact owning variant payload.
        organization: OrganizationRef,
    },
    /// Group.
    Group {
        /// Exact owning variant payload.
        group: OrganizationGroupRef,
    },
    /// Discipline tree.
    DisciplineTree {
        /// Exact owning variant payload.
        tree: OrganizationTreeRef,
    },
    /// Stable role.
    Role {
        /// Exact owning variant payload.
        role: OrganizationRoleRef,
    },
    /// Occupant identity, separate from a role.
    Occupant {
        /// Exact owning variant payload.
        occupant: String,
    },
    /// Provider identity, separate from an occupant.
    Provider {
        /// Exact owning variant payload.
        provider: String,
    },
    /// Owner-native resource reference, not its body.
    NativeResource {
        /// Exact owning variant payload.
        native_resource: String,
    },
    /// Object endpoint in the existing owning node-reference grammar.
    Object {
        /// Exact owning variant payload.
        object: String,
    },
}
impl OrganizationNode {
    /// Move the typed selector into the SDK without parsing or normalizing identifiers here.
    fn into_api(self) -> pale_sdk::OrganizationNode {
        use pale_sdk::OrganizationNode as N;
        match self {
            Self::Scope { scope } => N::Scope(scope.into_api()),
            Self::Organization { organization } => N::Organization(pale_sdk::OrganizationRef {
                namespace: organization.namespace,
                organization_id: organization.organization_id,
            }),
            Self::Group { group } => N::Group(pale_sdk::OrganizationGroupRef {
                namespace: group.namespace,
                group_id: group.group_id,
            }),
            Self::DisciplineTree { tree } => N::DisciplineTree(pale_sdk::OrganizationTreeRef {
                namespace: tree.namespace,
                tree_id: tree.tree_id,
            }),
            Self::Role { role } => N::Role(pale_sdk::OrganizationRoleRef {
                namespace: role.namespace,
                role_id: role.role_id,
            }),
            Self::Occupant { occupant } => N::Occupant(occupant),
            Self::Provider { provider } => N::Provider(provider),
            Self::NativeResource { native_resource } => N::NativeResource(native_resource),
            Self::Object { object } => N::Object(object),
        }
    }

    fn from_api(value: &pale_sdk::OrganizationNode) -> Self {
        match value {
            pale_sdk::OrganizationNode::Scope(item) => Self::Scope {
                scope: OrganizationScopeRef::from_api(item),
            },
            pale_sdk::OrganizationNode::Organization(item) => Self::Organization {
                organization: OrganizationRef::from_api(item),
            },
            pale_sdk::OrganizationNode::Group(item) => Self::Group {
                group: OrganizationGroupRef::from_api(item),
            },
            pale_sdk::OrganizationNode::DisciplineTree(item) => Self::DisciplineTree {
                tree: OrganizationTreeRef::from_api(item),
            },
            pale_sdk::OrganizationNode::Role(item) => Self::Role {
                role: OrganizationRoleRef::from_api(item),
            },
            pale_sdk::OrganizationNode::Occupant(item) => Self::Occupant {
                occupant: item.clone(),
            },
            pale_sdk::OrganizationNode::Provider(item) => Self::Provider {
                provider: item.clone(),
            },
            pale_sdk::OrganizationNode::NativeResource(item) => Self::NativeResource {
                native_resource: item.clone(),
            },
            pale_sdk::OrganizationNode::Object(item) => Self::Object {
                object: item.clone(),
            },
        }
    }
}

/// Copied owner interval, never an editor generation or presentation clock.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationInterval {
    /// Inclusive logical start.
    pub start: BigInt,
    /// Optional exclusive logical end.
    pub end_exclusive: Option<BigInt>,
}
impl OrganizationInterval {
    fn from_api(value: &pale_sdk::OrganizationInterval) -> Self {
        Self {
            start: BigInt::from(value.start),
            end_exclusive: value.end_exclusive.as_ref().map(|item| BigInt::from(*item)),
        }
    }
}

/// Typed observation of the owning effective-state enum, with no state algebra.
#[napi]
pub enum OrganizationFactState {
    /// Selected and effective.
    Present,
    /// Explicitly absent.
    Absent,
    /// Explicitly denied.
    Denied,
    /// Current fact cannot be established.
    Unknown,
    /// Outside the owner's freshness boundary.
    Stale,
    /// Replaced by another owner fact.
    Superseded,
    /// Explicitly revoked.
    Revoked,
}
impl OrganizationFactState {
    fn from_api(value: &pale_sdk::OrganizationFactState) -> Self {
        match value {
            pale_sdk::OrganizationFactState::Present => Self::Present,
            pale_sdk::OrganizationFactState::Absent => Self::Absent,
            pale_sdk::OrganizationFactState::Denied => Self::Denied,
            pale_sdk::OrganizationFactState::Unknown => Self::Unknown,
            pale_sdk::OrganizationFactState::Stale => Self::Stale,
            pale_sdk::OrganizationFactState::Superseded => Self::Superseded,
            pale_sdk::OrganizationFactState::Revoked => Self::Revoked,
        }
    }
}

/// Declared disclosure ceiling, not recipient admission.
#[napi]
pub enum OrganizationDisclosure {
    /// Declared private ceiling.
    Private,
    /// Declared protected ceiling.
    Protected,
    /// Declared public ceiling.
    Public,
}
impl OrganizationDisclosure {
    fn from_api(value: &pale_sdk::OrganizationDisclosure) -> Self {
        match value {
            pale_sdk::OrganizationDisclosure::Private => Self::Private,
            pale_sdk::OrganizationDisclosure::Protected => Self::Protected,
            pale_sdk::OrganizationDisclosure::Public => Self::Public,
        }
    }
}

/// Declared effect ceiling; inspection neither evaluates nor grants an effect.
#[napi]
pub enum OrganizationEffectCeiling {
    /// No effect permitted by this declaration.
    None,
    /// Declared read ceiling.
    Read,
    /// Declared mutation ceiling.
    Mutate,
}
impl OrganizationEffectCeiling {
    fn from_api(value: &pale_sdk::OrganizationEffectCeiling) -> Self {
        match value {
            pale_sdk::OrganizationEffectCeiling::None => Self::None,
            pale_sdk::OrganizationEffectCeiling::Read => Self::Read,
            pale_sdk::OrganizationEffectCeiling::Mutate => Self::Mutate,
        }
    }
}

/// Canonical explicit-relation locality, not native existence or crossing permission.
#[napi]
pub enum OrganizationRelationLocality {
    /// Endpoints remain within one owner and local container.
    Internal,
    /// Declared native presence without importing native ownership.
    NativeBoundary,
}
impl OrganizationRelationLocality {
    fn from_api(value: &pale_sdk::OrganizationRelationLocality) -> Self {
        match value {
            pale_sdk::OrganizationRelationLocality::Internal => Self::Internal,
            pale_sdk::OrganizationRelationLocality::NativeBoundary => Self::NativeBoundary,
        }
    }
}

/// Existing explicit relation class, not a renderer routing style.
#[napi]
pub enum OrganizationRelationClass {
    /// Exact group/tree coordination.
    Coordinates,
    /// Local integrated-roadmap coordination.
    Primary,
    /// Constitutive accepted-needs contribution.
    Leader,
    /// Bounded contribution.
    Supporting,
    /// Exact people-management relation.
    PeopleManagement,
    /// Exact roadmap-coordination relation.
    RoadmapCoordination,
    /// Exact discipline-judgment relation.
    DisciplineJudgment,
    /// Exact standards relation.
    Standards,
    /// Exact review relation.
    Review,
    /// Lens-scoped bounded block.
    BoundedBlock,
    /// Exact escalation relation.
    Escalation,
    /// Exact local acceptance relation.
    Acceptance,
    /// Native presence, not imported ownership.
    NativePresence,
    /// Exact accountable-role collaboration.
    Collaboration,
}
impl OrganizationRelationClass {
    fn from_api(value: &pale_sdk::OrganizationRelationClass) -> Self {
        match value {
            pale_sdk::OrganizationRelationClass::Coordinates => Self::Coordinates,
            pale_sdk::OrganizationRelationClass::Primary => Self::Primary,
            pale_sdk::OrganizationRelationClass::Leader => Self::Leader,
            pale_sdk::OrganizationRelationClass::Supporting => Self::Supporting,
            pale_sdk::OrganizationRelationClass::PeopleManagement => Self::PeopleManagement,
            pale_sdk::OrganizationRelationClass::RoadmapCoordination => Self::RoadmapCoordination,
            pale_sdk::OrganizationRelationClass::DisciplineJudgment => Self::DisciplineJudgment,
            pale_sdk::OrganizationRelationClass::Standards => Self::Standards,
            pale_sdk::OrganizationRelationClass::Review => Self::Review,
            pale_sdk::OrganizationRelationClass::BoundedBlock => Self::BoundedBlock,
            pale_sdk::OrganizationRelationClass::Escalation => Self::Escalation,
            pale_sdk::OrganizationRelationClass::Acceptance => Self::Acceptance,
            pale_sdk::OrganizationRelationClass::NativePresence => Self::NativePresence,
            pale_sdk::OrganizationRelationClass::Collaboration => Self::Collaboration,
        }
    }
}

/// Class already derived by the application from its dedicated authored fact.
#[napi]
pub enum OrganizationDerivedClass {
    /// Already derived from an occupant binding.
    Occupancy,
    /// Already derived from a provider binding.
    ProviderPresence,
    /// Already derived from the topic participation fact.
    LeadershipParticipation,
}
impl OrganizationDerivedClass {
    fn from_api(value: &pale_sdk::OrganizationDerivedClass) -> Self {
        match value {
            pale_sdk::OrganizationDerivedClass::Occupancy => Self::Occupancy,
            pale_sdk::OrganizationDerivedClass::ProviderPresence => Self::ProviderPresence,
            pale_sdk::OrganizationDerivedClass::LeadershipParticipation => {
                Self::LeadershipParticipation
            }
        }
    }
}

/// Provider-neutral declared projection class, not evidence of realization.
#[napi]
pub enum OrganizationProjectionClass {
    /// Accountable work and local acceptance.
    AccountableWork,
    /// Conversation, membership or bounded coordination.
    Communication,
    /// Relationship or commercial context.
    Relationship,
    /// Native request, result, effect or read-back.
    NativeApplication,
    /// Derived navigation or inspection.
    DerivedView,
    /// Bounded derived context.
    Context,
    /// Causal reference and occurrence lineage.
    Genealogy,
}
impl OrganizationProjectionClass {
    fn from_api(value: &pale_sdk::OrganizationProjectionClass) -> Self {
        match value {
            pale_sdk::OrganizationProjectionClass::AccountableWork => Self::AccountableWork,
            pale_sdk::OrganizationProjectionClass::Communication => Self::Communication,
            pale_sdk::OrganizationProjectionClass::Relationship => Self::Relationship,
            pale_sdk::OrganizationProjectionClass::NativeApplication => Self::NativeApplication,
            pale_sdk::OrganizationProjectionClass::DerivedView => Self::DerivedView,
            pale_sdk::OrganizationProjectionClass::Context => Self::Context,
            pale_sdk::OrganizationProjectionClass::Genealogy => Self::Genealogy,
        }
    }
}

/// Lifecycle asserted by the responsibility author, not newly verified by inspection.
#[napi]
pub enum OrganizationResponsibilityLifecycle {
    /// Author asserts verified current owner truth.
    VerifiedCurrent,
    /// Accepted future semantic target.
    AcceptedTarget,
    /// Principal-owner desired configuration.
    OwnerDesiredConfiguration,
    /// Candidate design, not accepted ABI status.
    Candidate,
    /// Implementation remains open.
    ImplementationOpen,
    /// Private population remains open.
    ConfigurationOpen,
    /// Evidence must close before use.
    EvidenceGated,
    /// Historical evidence only.
    Historical,
    /// Rejected proposition retained as genealogy.
    Rejected,
}
impl OrganizationResponsibilityLifecycle {
    fn from_api(value: &pale_sdk::OrganizationResponsibilityLifecycle) -> Self {
        match value {
            pale_sdk::OrganizationResponsibilityLifecycle::VerifiedCurrent => Self::VerifiedCurrent,
            pale_sdk::OrganizationResponsibilityLifecycle::AcceptedTarget => Self::AcceptedTarget,
            pale_sdk::OrganizationResponsibilityLifecycle::OwnerDesiredConfiguration => {
                Self::OwnerDesiredConfiguration
            }
            pale_sdk::OrganizationResponsibilityLifecycle::Candidate => Self::Candidate,
            pale_sdk::OrganizationResponsibilityLifecycle::ImplementationOpen => {
                Self::ImplementationOpen
            }
            pale_sdk::OrganizationResponsibilityLifecycle::ConfigurationOpen => {
                Self::ConfigurationOpen
            }
            pale_sdk::OrganizationResponsibilityLifecycle::EvidenceGated => Self::EvidenceGated,
            pale_sdk::OrganizationResponsibilityLifecycle::Historical => Self::Historical,
            pale_sdk::OrganizationResponsibilityLifecycle::Rejected => Self::Rejected,
        }
    }
}

/// Independent scalar authority axes; resources have a separate bounded query.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationAxes {
    /// Semantic owner.
    pub owner: String,
    /// Declared controller, not controller proof.
    pub controller: String,
    /// Declared authority, not a grant.
    pub authority: String,
    /// Declared disclosure ceiling.
    pub disclosure: OrganizationDisclosure,
}
impl OrganizationAxes {
    fn from_api(value: &pale_sdk::OrganizationAxes) -> Self {
        Self {
            owner: value.owner.clone(),
            controller: value.controller.clone(),
            authority: value.authority.clone(),
            disclosure: OrganizationDisclosure::from_api(&value.disclosure),
        }
    }
}

/// Owner and evidence reference for one separately named lifecycle requirement.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationEvidenceRequirement {
    /// Owner that can establish the fact.
    pub owner: String,
    /// Exact evidence reference, without its body.
    pub evidence_ref: String,
}
impl OrganizationEvidenceRequirement {
    fn from_api(value: &pale_sdk::OrganizationEvidenceRequirement) -> Self {
        Self {
            owner: value.owner.clone(),
            evidence_ref: value.evidence_ref.clone(),
        }
    }
}

/// Attribution preserved for a supplied answer or explicit omission.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationAttribution {
    /// Exact source coordinate.
    pub source_id: String,
    /// Attributable owner.
    pub owner: String,
    /// Exact evidence reference.
    pub evidence_ref: String,
}
impl OrganizationAttribution {
    fn from_api(value: &pale_sdk::OrganizationAttribution) -> Self {
        Self {
            source_id: value.source_id.clone(),
            owner: value.owner.clone(),
            evidence_ref: value.evidence_ref.clone(),
        }
    }
}

/// Actual attributed responsibility placement.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationPlacementAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationResponsibilityPlacement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}
impl OrganizationPlacementAnswer {
    fn from_api(value: &pale_sdk::OrganizationPlacementAnswer) -> Self {
        match value {
            pale_sdk::OrganizationPlacementAnswer::Supplied { value, attribution } => {
                Self::Supplied {
                    value: OrganizationResponsibilityPlacement::from_api(value),
                    attribution: OrganizationAttribution::from_api(attribution),
                }
            }
            pale_sdk::OrganizationPlacementAnswer::Omitted {
                reason,
                attribution,
            } => Self::Omitted {
                reason: reason.clone(),
                attribution: OrganizationAttribution::from_api(attribution),
            },
        }
    }
}

/// Actual attributed generic resource answer; no identity-class cast.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationResourceAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: String,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}
impl OrganizationResourceAnswer {
    fn from_api(value: &pale_sdk::OrganizationResourceAnswer) -> Self {
        match value {
            pale_sdk::OrganizationResourceAnswer::Supplied { value, attribution } => {
                Self::Supplied {
                    value: value.clone(),
                    attribution: OrganizationAttribution::from_api(attribution),
                }
            }
            pale_sdk::OrganizationResourceAnswer::Omitted {
                reason,
                attribution,
            } => Self::Omitted {
                reason: reason.clone(),
                attribution: OrganizationAttribution::from_api(attribution),
            },
        }
    }
}

/// Provider completion answer, separate from provider acceptance.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationProviderCompletionAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}
impl OrganizationProviderCompletionAnswer {
    fn from_api(value: &pale_sdk::OrganizationProviderCompletionAnswer) -> Self {
        match value {
            pale_sdk::OrganizationProviderCompletionAnswer::Supplied { value, attribution } => {
                Self::Supplied {
                    value: OrganizationEvidenceRequirement::from_api(value),
                    attribution: OrganizationAttribution::from_api(attribution),
                }
            }
            pale_sdk::OrganizationProviderCompletionAnswer::Omitted {
                reason,
                attribution,
            } => Self::Omitted {
                reason: reason.clone(),
                attribution: OrganizationAttribution::from_api(attribution),
            },
        }
    }
}

/// Provider acceptance answer, separate from completion and consumer acceptance.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationProviderAcceptanceAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}
impl OrganizationProviderAcceptanceAnswer {
    fn from_api(value: &pale_sdk::OrganizationProviderAcceptanceAnswer) -> Self {
        match value {
            pale_sdk::OrganizationProviderAcceptanceAnswer::Supplied { value, attribution } => {
                Self::Supplied {
                    value: OrganizationEvidenceRequirement::from_api(value),
                    attribution: OrganizationAttribution::from_api(attribution),
                }
            }
            pale_sdk::OrganizationProviderAcceptanceAnswer::Omitted {
                reason,
                attribution,
            } => Self::Omitted {
                reason: reason.clone(),
                attribution: OrganizationAttribution::from_api(attribution),
            },
        }
    }
}

/// Consumer-local consequence acceptance answer.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationConsumerAcceptanceAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}
impl OrganizationConsumerAcceptanceAnswer {
    fn from_api(value: &pale_sdk::OrganizationConsumerAcceptanceAnswer) -> Self {
        match value {
            pale_sdk::OrganizationConsumerAcceptanceAnswer::Supplied { value, attribution } => {
                Self::Supplied {
                    value: OrganizationEvidenceRequirement::from_api(value),
                    attribution: OrganizationAttribution::from_api(attribution),
                }
            }
            pale_sdk::OrganizationConsumerAcceptanceAnswer::Omitted {
                reason,
                attribution,
            } => Self::Omitted {
                reason: reason.clone(),
                attribution: OrganizationAttribution::from_api(attribution),
            },
        }
    }
}

/// Native result/read-back answer, not a settlement result.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationNativeReadBackAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}
impl OrganizationNativeReadBackAnswer {
    fn from_api(value: &pale_sdk::OrganizationNativeReadBackAnswer) -> Self {
        match value {
            pale_sdk::OrganizationNativeReadBackAnswer::Supplied { value, attribution } => {
                Self::Supplied {
                    value: OrganizationEvidenceRequirement::from_api(value),
                    attribution: OrganizationAttribution::from_api(attribution),
                }
            }
            pale_sdk::OrganizationNativeReadBackAnswer::Omitted {
                reason,
                attribution,
            } => Self::Omitted {
                reason: reason.clone(),
                attribution: OrganizationAttribution::from_api(attribution),
            },
        }
    }
}

/// Pale obligation-settlement answer, separate from native-owner acceptance.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationSettlementAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}
impl OrganizationSettlementAnswer {
    fn from_api(value: &pale_sdk::OrganizationSettlementAnswer) -> Self {
        match value {
            pale_sdk::OrganizationSettlementAnswer::Supplied { value, attribution } => {
                Self::Supplied {
                    value: OrganizationEvidenceRequirement::from_api(value),
                    attribution: OrganizationAttribution::from_api(attribution),
                }
            }
            pale_sdk::OrganizationSettlementAnswer::Omitted {
                reason,
                attribution,
            } => Self::Omitted {
                reason: reason.clone(),
                attribution: OrganizationAttribution::from_api(attribution),
            },
        }
    }
}

/// Actual responsibility placement without guessing a scope from a resource.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationResponsibilityPlacement {
    /// Generic local resource exactly as declared; no logical-scope cast.
    VerticalLocal {
        /// Exact owning variant payload.
        resource: String,
    },
    /// Exact service coordinate already checked by the owning linker.
    DirectionalService {
        /// Exact owning variant payload.
        contract: OrganizationServiceRef,
    },
}
impl OrganizationResponsibilityPlacement {
    fn from_api(value: &pale_sdk::OrganizationResponsibilityPlacement) -> Self {
        match value {
            pale_sdk::OrganizationResponsibilityPlacement::VerticalLocal(item) => {
                Self::VerticalLocal {
                    resource: item.clone(),
                }
            }
            pale_sdk::OrganizationResponsibilityPlacement::DirectionalService(item) => {
                Self::DirectionalService {
                    contract: OrganizationServiceRef::from_api(item),
                }
            }
        }
    }
}

/// Complete source coordinate of an already-derived edge.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationDerivedSource {
    /// Exact temporal occupant binding.
    OccupantBinding {
        /// Stable role.
        role: OrganizationRoleRef,
        /// Occupant.
        occupant: String,
        /// Source binding interval.
        interval: OrganizationInterval,
    },
    /// Exact temporal provider binding.
    ProviderBinding {
        /// Stable role.
        role: OrganizationRoleRef,
        /// Provider.
        provider: String,
        /// Source binding interval.
        interval: OrganizationInterval,
    },
    /// Topic is only the source coordinate; class/endpoints/interval still identify the edge.
    LeadershipParticipation {
        /// Exact owning variant payload.
        topic: String,
    },
}
impl OrganizationDerivedSource {
    fn from_api(value: &pale_sdk::OrganizationDerivedSource) -> Self {
        match value {
            pale_sdk::OrganizationDerivedSource::OccupantBinding {
                role,
                occupant,
                interval,
            } => Self::OccupantBinding {
                role: OrganizationRoleRef::from_api(role),
                occupant: occupant.clone(),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationDerivedSource::ProviderBinding {
                role,
                provider,
                interval,
            } => Self::ProviderBinding {
                role: OrganizationRoleRef::from_api(role),
                provider: provider.clone(),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationDerivedSource::LeadershipParticipation(item) => {
                Self::LeadershipParticipation {
                    topic: item.clone(),
                }
            }
        }
    }
}

/// Shallow responsibility header with independent attributed answers.
///
/// Kept in one allocation when held by [`OrganizationRow`], so its complete
/// attribution payload does not determine every other row's inline size.
/// The copied-string budget excludes this allocation and allocator overhead.
#[napi]
pub struct OrganizationResponsibility {
    inner: Box<pale_sdk::OrganizationResponsibility>,
}
impl OrganizationResponsibility {
    fn from_api(value: &pale_sdk::OrganizationResponsibility) -> Self {
        Self {
            inner: Box::new(value.clone()),
        }
    }
}
#[napi]
impl OrganizationResponsibility {
    /// Exact responsibility derivation identity.
    #[napi(getter)]
    pub fn derivation_id(&self) -> String {
        self.inner.derivation_id.clone()
    }
    /// Lifecycle asserted by the author; not reverified by this query.
    #[napi(getter)]
    pub fn lifecycle(&self) -> OrganizationResponsibilityLifecycle {
        OrganizationResponsibilityLifecycle::from_api(&self.inner.lifecycle)
    }
    /// Original complete-record source/owner attribution.
    #[napi(getter)]
    pub fn provenance(&self) -> OrganizationAttribution {
        OrganizationAttribution::from_api(&self.inner.provenance)
    }
    /// Actual supplied placement or explicit attributed omission.
    #[napi(getter)]
    pub fn placement(&self) -> OrganizationPlacementAnswer {
        OrganizationPlacementAnswer::from_api(&self.inner.placement)
    }
    /// Independent provider-local completion requirement.
    #[napi(getter)]
    pub fn provider_completion(&self) -> OrganizationProviderCompletionAnswer {
        OrganizationProviderCompletionAnswer::from_api(&self.inner.provider_completion)
    }
    /// Independent provider-local acceptance requirement.
    #[napi(getter)]
    pub fn provider_acceptance(&self) -> OrganizationProviderAcceptanceAnswer {
        OrganizationProviderAcceptanceAnswer::from_api(&self.inner.provider_acceptance)
    }
    /// Independent consumer-local consequence acceptance requirement.
    #[napi(getter)]
    pub fn consumer_acceptance(&self) -> OrganizationConsumerAcceptanceAnswer {
        OrganizationConsumerAcceptanceAnswer::from_api(&self.inner.consumer_acceptance)
    }
    /// Independent supplied native-result/read-back requirement or omission.
    #[napi(getter)]
    pub fn native_result(&self) -> OrganizationNativeReadBackAnswer {
        OrganizationNativeReadBackAnswer::from_api(&self.inner.native_result)
    }
    /// Independent Pale obligation-settlement requirement.
    #[napi(getter)]
    pub fn pale_settlement(&self) -> OrganizationSettlementAnswer {
        OrganizationSettlementAnswer::from_api(&self.inner.pale_settlement)
    }
}

/// Shallow copied records from the existing eighteen-family owner.
///
/// These public values can be copied or constructed as observations, but no
/// query accepts them as authority or reconstructs an opaque record handle.
/// Nested collections are intentionally accessed through their owning handle.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationRow {
    /// Logical scope and independent axes.
    Scope {
        /// Exact namespace-qualified logical scope.
        scope: OrganizationScopeRef,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Organization and its actual scope.
    Organization {
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Exact namespace-qualified logical scope.
        scope: OrganizationScopeRef,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Group and its actual organization.
    Group {
        /// Actual group association; not substituted for the discipline tree.
        group: OrganizationGroupRef,
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Tree and its actual organization.
    DisciplineTree {
        /// Exact namespace-qualified discipline tree.
        tree: OrganizationTreeRef,
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Role with independent group and tree associations.
    Role {
        /// Exact stable role, independent of its occupant/provider.
        role: OrganizationRoleRef,
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Actual group association; not substituted for the discipline tree.
        group: Option<OrganizationGroupRef>,
        /// Actual optional tree association, independently of group.
        discipline_tree: Option<OrganizationTreeRef>,
        /// Owner-selected role class; its spelling grants no authority.
        role_class: String,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Occupant is not a provider or the role itself.
    OccupantBinding {
        /// Exact stable role, independent of its occupant/provider.
        role: OrganizationRoleRef,
        /// Actual occupant identity, not the role or provider.
        occupant: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Provider is not an occupant or the role itself.
    ProviderBinding {
        /// Exact stable role, independent of its occupant/provider.
        role: OrganizationRoleRef,
        /// Actual provider identity, not an occupant or native readiness proof.
        provider: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Identity-only communication binding, with separate local-resolution detail.
    CommunicationActor {
        /// Actual provider identity, not an occupant or native readiness proof.
        provider: String,
        /// Canonical provider-native communication handle; not an authorization union.
        handle: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Topic and declared visibility; participant and scope lists are separate.
    Leadership {
        /// Exact topic coordinate; derived edges also require endpoints and interval.
        topic: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Declared disclosure ceiling, not admitted recipient context.
        disclosure: OrganizationDisclosure,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Exact explicit multigraph edge.
    Relation {
        /// Exact explicit relation identity, preserving parallel relations.
        relation_id: String,
        /// Actual owner-defined class, not a renderer style.
        class: OrganizationRelationClass,
        /// Locality already selected by the canonical class owner.
        locality: OrganizationRelationLocality,
        /// Exact typed directional source endpoint.
        from: OrganizationNode,
        /// Exact typed directional destination endpoint.
        to: OrganizationNode,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Object preserves its actual namespace and both independent associations.
    Object {
        /// Actual owner-selected object identity.
        object_id: String,
        /// Actual owning namespace; not discarded during projection.
        namespace: String,
        /// Exact namespace-qualified logical scope.
        scope: OrganizationScopeRef,
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Owner-selected object class; not inferred from a label.
        object_class: String,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Declared class pairing, not inferred membership.
    PopulationRule {
        /// Owner-selected object class; not inferred from a label.
        object_class: String,
        /// Explicit accountable role class in the population rule.
        accountable_role_class: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Named lens and its explicit holders/resolvers.
    GroupLens {
        /// Exact group-lens authority-profile identity.
        profile_id: String,
        /// Exact role carrying the declared lens-scoped profile.
        holder: OrganizationRoleRef,
        /// Existing relation that bounds the lens context.
        alignment_relation: String,
        /// Owner-selected governed lens.
        lens: String,
        /// Actual optional accountable review role.
        review_owner: Option<OrganizationRoleRef>,
        /// Actual optional escalation resolver role.
        escalation_resolver: Option<OrganizationRoleRef>,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Directional service and five non-substituted requirement fields.
    Service {
        /// Complete contract identity, including namespace and both directional scopes.
        contract: OrganizationServiceRef,
        /// Owner of the relationship/contract, not a selected provider.
        contract_owner: String,
        /// Independent provider-local completion requirement.
        provider_completion: OrganizationEvidenceRequirement,
        /// Independent provider-local acceptance requirement.
        provider_acceptance: OrganizationEvidenceRequirement,
        /// Independent consumer-local consequence acceptance requirement.
        consumer_acceptance: OrganizationEvidenceRequirement,
        /// Independent native owner effect/read-back requirement.
        native_read_back: OrganizationEvidenceRequirement,
        /// Independent Pale obligation-settlement requirement.
        pale_settlement: OrganizationEvidenceRequirement,
        /// Declared disclosure ceiling, not admitted recipient context.
        disclosure: OrganizationDisclosure,
        /// Declared effect ceiling; inspection does not grant or perform an effect.
        effect: OrganizationEffectCeiling,
        /// Declared model-use ceiling, not a dispatch decision.
        model_use_permitted: bool,
        /// Declared retention ceiling in seconds, not a live retention receipt.
        retention_seconds: BigInt,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Desired native links are queried separately and do not prove realization.
    NativeProjection {
        /// Exact native-projection identity.
        projection_id: String,
        /// Actual typed semantic subject of the desired projection.
        subject: OrganizationNode,
        /// Actual owner-defined class, not a renderer style.
        class: OrganizationProjectionClass,
        /// Native owner retaining record/effect truth.
        native_owner: String,
        /// Declared disclosure ceiling, not admitted recipient context.
        disclosure: OrganizationDisclosure,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Declared inspection selection, not authenticated recipient context.
    Inspection {
        /// Exact configured inspection-contract identity.
        inspection_id: String,
        /// Declared principal reference; no authenticated authority is inferred.
        principal: String,
        /// Declared disclosure ceiling, not admitted recipient context.
        disclosure: OrganizationDisclosure,
        /// Optional configured view provider, not proof of availability.
        view_provider: Option<String>,
    },
    /// Source, class, both endpoints and interval jointly preserve parallel edges.
    DerivedRelation {
        /// Complete dedicated source-fact coordinate for this derived edge.
        source: OrganizationDerivedSource,
        /// Actual owner-defined class, not a renderer style.
        class: OrganizationDerivedClass,
        /// Exact typed directional source endpoint.
        from: OrganizationNode,
        /// Exact typed directional destination endpoint.
        to: OrganizationNode,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Shallow responsibility structure; omitted answers retain reason and attribution.
    Responsibility {
        /// Complete shallow responsibility header, not a custody token.
        record: OrganizationResponsibility,
    },
}
impl OrganizationRow {
    fn from_api(value: &pale_sdk::OrganizationRow) -> Self {
        match value {
            pale_sdk::OrganizationRow::Scope {
                scope,
                axes,
                state,
                interval,
            } => Self::Scope {
                scope: OrganizationScopeRef::from_api(scope),
                axes: OrganizationAxes::from_api(axes),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::Organization {
                organization,
                scope,
                axes,
                state,
                interval,
            } => Self::Organization {
                organization: OrganizationRef::from_api(organization),
                scope: OrganizationScopeRef::from_api(scope),
                axes: OrganizationAxes::from_api(axes),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::Group {
                group,
                organization,
                axes,
                state,
                interval,
            } => Self::Group {
                group: OrganizationGroupRef::from_api(group),
                organization: OrganizationRef::from_api(organization),
                axes: OrganizationAxes::from_api(axes),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::DisciplineTree {
                tree,
                organization,
                axes,
                state,
                interval,
            } => Self::DisciplineTree {
                tree: OrganizationTreeRef::from_api(tree),
                organization: OrganizationRef::from_api(organization),
                axes: OrganizationAxes::from_api(axes),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::Role {
                role,
                organization,
                group,
                discipline_tree,
                role_class,
                axes,
                state,
                interval,
            } => Self::Role {
                role: OrganizationRoleRef::from_api(role),
                organization: OrganizationRef::from_api(organization),
                group: group.as_ref().map(OrganizationGroupRef::from_api),
                discipline_tree: discipline_tree.as_ref().map(OrganizationTreeRef::from_api),
                role_class: role_class.clone(),
                axes: OrganizationAxes::from_api(axes),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::OccupantBinding {
                role,
                occupant,
                issuer,
                state,
                interval,
            } => Self::OccupantBinding {
                role: OrganizationRoleRef::from_api(role),
                occupant: occupant.clone(),
                issuer: issuer.clone(),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::ProviderBinding {
                role,
                provider,
                issuer,
                state,
                interval,
            } => Self::ProviderBinding {
                role: OrganizationRoleRef::from_api(role),
                provider: provider.clone(),
                issuer: issuer.clone(),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::CommunicationActor {
                provider,
                handle,
                issuer,
                state,
                interval,
            } => Self::CommunicationActor {
                provider: provider.clone(),
                handle: handle.clone(),
                issuer: issuer.clone(),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::Leadership {
                topic,
                issuer,
                disclosure,
                state,
                interval,
            } => Self::Leadership {
                topic: topic.clone(),
                issuer: issuer.clone(),
                disclosure: OrganizationDisclosure::from_api(disclosure),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::Relation {
                relation_id,
                class,
                locality,
                from,
                to,
                issuer,
                state,
                interval,
            } => Self::Relation {
                relation_id: relation_id.clone(),
                class: OrganizationRelationClass::from_api(class),
                locality: OrganizationRelationLocality::from_api(locality),
                from: OrganizationNode::from_api(from),
                to: OrganizationNode::from_api(to),
                issuer: issuer.clone(),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::Object {
                object_id,
                namespace,
                scope,
                organization,
                object_class,
                axes,
                state,
                interval,
            } => Self::Object {
                object_id: object_id.clone(),
                namespace: namespace.clone(),
                scope: OrganizationScopeRef::from_api(scope),
                organization: OrganizationRef::from_api(organization),
                object_class: object_class.clone(),
                axes: OrganizationAxes::from_api(axes),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::PopulationRule {
                object_class,
                accountable_role_class,
                issuer,
                state,
                interval,
            } => Self::PopulationRule {
                object_class: object_class.clone(),
                accountable_role_class: accountable_role_class.clone(),
                issuer: issuer.clone(),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::GroupLens {
                profile_id,
                holder,
                alignment_relation,
                lens,
                review_owner,
                escalation_resolver,
                issuer,
                state,
                interval,
            } => Self::GroupLens {
                profile_id: profile_id.clone(),
                holder: OrganizationRoleRef::from_api(holder),
                alignment_relation: alignment_relation.clone(),
                lens: lens.clone(),
                review_owner: review_owner.as_ref().map(OrganizationRoleRef::from_api),
                escalation_resolver: escalation_resolver
                    .as_ref()
                    .map(OrganizationRoleRef::from_api),
                issuer: issuer.clone(),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::Service {
                contract,
                contract_owner,
                provider_completion,
                provider_acceptance,
                consumer_acceptance,
                native_read_back,
                pale_settlement,
                disclosure,
                effect,
                model_use_permitted,
                retention_seconds,
                state,
                interval,
            } => Self::Service {
                contract: OrganizationServiceRef::from_api(contract),
                contract_owner: contract_owner.clone(),
                provider_completion: OrganizationEvidenceRequirement::from_api(provider_completion),
                provider_acceptance: OrganizationEvidenceRequirement::from_api(provider_acceptance),
                consumer_acceptance: OrganizationEvidenceRequirement::from_api(consumer_acceptance),
                native_read_back: OrganizationEvidenceRequirement::from_api(native_read_back),
                pale_settlement: OrganizationEvidenceRequirement::from_api(pale_settlement),
                disclosure: OrganizationDisclosure::from_api(disclosure),
                effect: OrganizationEffectCeiling::from_api(effect),
                model_use_permitted: *model_use_permitted,
                retention_seconds: BigInt::from(*retention_seconds),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::NativeProjection {
                projection_id,
                subject,
                class,
                native_owner,
                disclosure,
                state,
                interval,
            } => Self::NativeProjection {
                projection_id: projection_id.clone(),
                subject: OrganizationNode::from_api(subject),
                class: OrganizationProjectionClass::from_api(class),
                native_owner: native_owner.clone(),
                disclosure: OrganizationDisclosure::from_api(disclosure),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::Inspection {
                inspection_id,
                principal,
                disclosure,
                view_provider,
            } => Self::Inspection {
                inspection_id: inspection_id.clone(),
                principal: principal.clone(),
                disclosure: OrganizationDisclosure::from_api(disclosure),
                view_provider: view_provider.clone(),
            },
            pale_sdk::OrganizationRow::DerivedRelation {
                source,
                class,
                from,
                to,
                issuer,
                state,
                interval,
            } => Self::DerivedRelation {
                source: OrganizationDerivedSource::from_api(source),
                class: OrganizationDerivedClass::from_api(class),
                from: OrganizationNode::from_api(from),
                to: OrganizationNode::from_api(to),
                issuer: issuer.clone(),
                state: OrganizationFactState::from_api(state),
                interval: OrganizationInterval::from_api(interval),
            },
            pale_sdk::OrganizationRow::Responsibility { record } => Self::Responsibility {
                record: OrganizationResponsibility::from_api(record.as_ref()),
            },
        }
    }
}

/// Actual parent custody of a detail result, independently retained after return.
///
/// These are issued opaque handles, not generic semantic cell values. A nested
/// parent remains distinguishable even when its returned child list is empty.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationDetailParent {
    /// A top-level record section.
    Record {
        /// Exact owning variant payload.
        handle: OrganizationRecordHandle,
    },
    /// One actual communication local resolution.
    Communication {
        /// Exact owning variant payload.
        handle: OrganizationCommunicationResolutionHandle,
    },
    /// One actual service assignment.
    Assignment {
        /// Exact owning variant payload.
        handle: OrganizationServiceAssignmentHandle,
    },
    /// One actual responsibility-native projection.
    Projection {
        /// Exact owning variant payload.
        handle: OrganizationResponsibilityProjectionHandle,
    },
    /// One actual genealogy link.
    Genealogy {
        /// Exact owning variant payload.
        handle: OrganizationGenealogyHandle,
    },
}
impl OrganizationDetailParent {
    fn from_api(value: &pale_sdk::OrganizationDetailParent) -> Self {
        match value {
            pale_sdk::OrganizationDetailParent::Record(item) => Self::Record {
                handle: OrganizationRecordHandle {
                    inner: item.clone(),
                },
            },
            pale_sdk::OrganizationDetailParent::Communication(item) => Self::Communication {
                handle: OrganizationCommunicationResolutionHandle {
                    inner: item.clone(),
                },
            },
            pale_sdk::OrganizationDetailParent::Assignment(item) => Self::Assignment {
                handle: OrganizationServiceAssignmentHandle {
                    inner: item.clone(),
                },
            },
            pale_sdk::OrganizationDetailParent::Projection(item) => Self::Projection {
                handle: OrganizationResponsibilityProjectionHandle {
                    inner: item.clone(),
                },
            },
            pale_sdk::OrganizationDetailParent::Genealogy(item) => Self::Genealogy {
                handle: OrganizationGenealogyHandle {
                    inner: item.clone(),
                },
            },
        }
    }
}

/// Closed structural section payloads; each variant retains its exact owning meaning.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationDetailRows {
    /// Actual authority-resource members.
    AxisResources {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Actual participating role references.
    LeadershipParticipants {
        /// Exact owning variant payload.
        values: Vec<OrganizationRoleRef>,
    },
    /// Independent affected scope references.
    LeadershipScopes {
        /// Exact owning variant payload.
        values: Vec<OrganizationScopeRef>,
    },
    /// Separately retained local-resolution headers.
    CommunicationResolutions {
        /// Exact owning variant payload.
        values: Vec<OrganizationCommunicationResolution>,
    },
    /// One exact local resolution's authorized closure.
    CommunicationClosure {
        /// Exact owning variant payload.
        values: Vec<OrganizationNode>,
    },
    /// Exact named standards.
    LensStandards {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Exact bounded-block conditions.
    LensConditions {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Actual declared lens rights.
    LensRights {
        /// Exact owning variant payload.
        values: Vec<OrganizationLensRight>,
    },
    /// Shallow account/mandate assignments with exact nested custody.
    ServiceAssignments {
        /// Exact owning variant payload.
        values: Vec<OrganizationServiceAssignment>,
    },
    /// Stewards of one actual assignment.
    AssignmentStewards {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Pods of one actual assignment.
    AssignmentPods {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Independent service use-right members.
    ServiceRights {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Independent service controller references.
    ServiceControllers {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Independent service authority references.
    ServiceAuthorities {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Desired native references of one organizational projection.
    NativeReferences {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Configured coordinate classes, not a grant.
    InspectionCoordinates {
        /// Exact owning variant payload.
        values: Vec<OrganizationInspectionCoordinate>,
    },
    /// Configured effective-state classes.
    InspectionStates {
        /// Exact owning variant payload.
        values: Vec<OrganizationFactState>,
    },
    /// Exact structural answers and omissions.
    ResponsibilitySelections {
        /// Exact owning variant payload.
        values: Vec<OrganizationStructuralSelection>,
    },
    /// Actual responsibility residence decisions.
    ResponsibilityResidences {
        /// Exact owning variant payload.
        values: Vec<OrganizationResidence>,
    },
    /// Shallow desired native projections.
    ResponsibilityProjections {
        /// Exact owning variant payload.
        values: Vec<OrganizationResponsibilityProjection>,
    },
    /// Native references of one exact responsibility projection.
    ProjectionReferences {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Shallow links retaining three separate member sets.
    ResponsibilityGenealogy {
        /// Exact owning variant payload.
        values: Vec<OrganizationGenealogyLink>,
    },
    /// Responsibility identities of one actual link.
    GenealogyResponsibilities {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Projection identities of one actual link.
    GenealogyProjections {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
    /// Body-free occurrence identities of one actual link.
    GenealogyOccurrences {
        /// Exact owning variant payload.
        values: Vec<String>,
    },
}
impl OrganizationDetailRows {
    fn from_api(value: &pale_sdk::OrganizationDetailRows) -> Self {
        match value {
            pale_sdk::OrganizationDetailRows::AxisResources(item) => Self::AxisResources {
                values: item.to_vec(),
            },
            pale_sdk::OrganizationDetailRows::LeadershipParticipants(item) => {
                Self::LeadershipParticipants {
                    values: item.iter().map(OrganizationRoleRef::from_api).collect(),
                }
            }
            pale_sdk::OrganizationDetailRows::LeadershipScopes(item) => Self::LeadershipScopes {
                values: item.iter().map(OrganizationScopeRef::from_api).collect(),
            },
            pale_sdk::OrganizationDetailRows::CommunicationResolutions(item) => {
                Self::CommunicationResolutions {
                    values: item
                        .iter()
                        .map(OrganizationCommunicationResolution::from_api)
                        .collect(),
                }
            }
            pale_sdk::OrganizationDetailRows::CommunicationClosure(item) => {
                Self::CommunicationClosure {
                    values: item.iter().map(OrganizationNode::from_api).collect(),
                }
            }
            pale_sdk::OrganizationDetailRows::LensStandards(item) => Self::LensStandards {
                values: item.to_vec(),
            },
            pale_sdk::OrganizationDetailRows::LensConditions(item) => Self::LensConditions {
                values: item.to_vec(),
            },
            pale_sdk::OrganizationDetailRows::LensRights(item) => Self::LensRights {
                values: item.iter().map(OrganizationLensRight::from_api).collect(),
            },
            pale_sdk::OrganizationDetailRows::ServiceAssignments(item) => {
                Self::ServiceAssignments {
                    values: item
                        .iter()
                        .map(OrganizationServiceAssignment::from_api)
                        .collect(),
                }
            }
            pale_sdk::OrganizationDetailRows::AssignmentStewards(item) => {
                Self::AssignmentStewards {
                    values: item.to_vec(),
                }
            }
            pale_sdk::OrganizationDetailRows::AssignmentPods(item) => Self::AssignmentPods {
                values: item.to_vec(),
            },
            pale_sdk::OrganizationDetailRows::ServiceRights(item) => Self::ServiceRights {
                values: item.to_vec(),
            },
            pale_sdk::OrganizationDetailRows::ServiceControllers(item) => {
                Self::ServiceControllers {
                    values: item.to_vec(),
                }
            }
            pale_sdk::OrganizationDetailRows::ServiceAuthorities(item) => {
                Self::ServiceAuthorities {
                    values: item.to_vec(),
                }
            }
            pale_sdk::OrganizationDetailRows::NativeReferences(item) => Self::NativeReferences {
                values: item.to_vec(),
            },
            pale_sdk::OrganizationDetailRows::InspectionCoordinates(item) => {
                Self::InspectionCoordinates {
                    values: item
                        .iter()
                        .map(OrganizationInspectionCoordinate::from_api)
                        .collect(),
                }
            }
            pale_sdk::OrganizationDetailRows::InspectionStates(item) => Self::InspectionStates {
                values: item.iter().map(OrganizationFactState::from_api).collect(),
            },
            pale_sdk::OrganizationDetailRows::ResponsibilitySelections(item) => {
                Self::ResponsibilitySelections {
                    values: item
                        .iter()
                        .map(OrganizationStructuralSelection::from_api)
                        .collect(),
                }
            }
            pale_sdk::OrganizationDetailRows::ResponsibilityResidences(item) => {
                Self::ResponsibilityResidences {
                    values: item.iter().map(OrganizationResidence::from_api).collect(),
                }
            }
            pale_sdk::OrganizationDetailRows::ResponsibilityProjections(item) => {
                Self::ResponsibilityProjections {
                    values: item
                        .iter()
                        .map(OrganizationResponsibilityProjection::from_api)
                        .collect(),
                }
            }
            pale_sdk::OrganizationDetailRows::ProjectionReferences(item) => {
                Self::ProjectionReferences {
                    values: item.to_vec(),
                }
            }
            pale_sdk::OrganizationDetailRows::ResponsibilityGenealogy(item) => {
                Self::ResponsibilityGenealogy {
                    values: item
                        .iter()
                        .map(OrganizationGenealogyLink::from_api)
                        .collect(),
                }
            }
            pale_sdk::OrganizationDetailRows::GenealogyResponsibilities(item) => {
                Self::GenealogyResponsibilities {
                    values: item.to_vec(),
                }
            }
            pale_sdk::OrganizationDetailRows::GenealogyProjections(item) => {
                Self::GenealogyProjections {
                    values: item.to_vec(),
                }
            }
            pale_sdk::OrganizationDetailRows::GenealogyOccurrences(item) => {
                Self::GenealogyOccurrences {
                    values: item.to_vec(),
                }
            }
        }
    }
}

/// One shallow local resolution with its own separately queryable closure.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationCommunicationResolution {
    /// Actual local node, not a merged provider-wide authorization set.
    pub local_node: OrganizationNode,
    /// Exact retained member handle.
    pub handle: OrganizationCommunicationResolutionHandle,
}
impl OrganizationCommunicationResolution {
    fn from_api(value: &pale_sdk::OrganizationCommunicationResolution) -> Self {
        Self {
            local_node: OrganizationNode::from_api(&value.local_node),
            handle: OrganizationCommunicationResolutionHandle {
                inner: value.handle().clone(),
            },
        }
    }
}

/// One shallow assignment; stewards and pods remain separate actual member sets.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationServiceAssignment {
    /// Exact account.
    pub account: String,
    /// Exact mandate.
    pub mandate: String,
    /// Independently declared state.
    pub state: OrganizationFactState,
    /// Actual assignment interval.
    pub interval: OrganizationInterval,
    /// Exact retained member handle.
    pub handle: OrganizationServiceAssignmentHandle,
}
impl OrganizationServiceAssignment {
    fn from_api(value: &pale_sdk::OrganizationServiceAssignment) -> Self {
        Self {
            account: value.account.clone(),
            mandate: value.mandate.clone(),
            state: OrganizationFactState::from_api(&value.state),
            interval: OrganizationInterval::from_api(&value.interval),
            handle: OrganizationServiceAssignmentHandle {
                inner: value.handle().clone(),
            },
        }
    }
}

/// One desired native projection; no native body or realization proof is copied.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationResponsibilityProjection {
    /// Exact projection identity.
    pub projection_id: String,
    /// Existing provider-neutral class.
    pub class: OrganizationProjectionClass,
    /// Owner retaining native truth.
    pub native_owner: String,
    /// Actual independent state.
    pub state: OrganizationFactState,
    /// Exact retained member handle.
    pub handle: OrganizationResponsibilityProjectionHandle,
}
impl OrganizationResponsibilityProjection {
    fn from_api(value: &pale_sdk::OrganizationResponsibilityProjection) -> Self {
        Self {
            projection_id: value.projection_id.clone(),
            class: OrganizationProjectionClass::from_api(&value.class),
            native_owner: value.native_owner.clone(),
            state: OrganizationFactState::from_api(&value.state),
            handle: OrganizationResponsibilityProjectionHandle {
                inner: value.handle().clone(),
            },
        }
    }
}

/// One genealogy link with separately paged responsibility/projection/occurrence sets.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationGenealogyLink {
    /// Exact link identity.
    pub link_id: String,
    /// Exact retained member handle.
    pub handle: OrganizationGenealogyHandle,
}
impl OrganizationGenealogyLink {
    fn from_api(value: &pale_sdk::OrganizationGenealogyLink) -> Self {
        Self {
            link_id: value.link_id.clone(),
            handle: OrganizationGenealogyHandle {
                inner: value.handle().clone(),
            },
        }
    }
}

/// Existing declared group-lens right; no independent rights policy.
#[napi]
pub enum OrganizationLensRight {
    /// Declared named standards right.
    Standards,
    /// Declared named review right.
    Review,
    /// Block only under declared conditions.
    BoundedBlock,
    /// Escalate only through the declared resolver.
    Escalation,
}
impl OrganizationLensRight {
    fn from_api(value: &pale_sdk::OrganizationLensRight) -> Self {
        match value {
            pale_sdk::OrganizationLensRight::Standards => Self::Standards,
            pale_sdk::OrganizationLensRight::Review => Self::Review,
            pale_sdk::OrganizationLensRight::BoundedBlock => Self::BoundedBlock,
            pale_sdk::OrganizationLensRight::Escalation => Self::Escalation,
        }
    }
}

/// Configured inspection coordinate, not authenticated disclosure context.
#[napi]
pub enum OrganizationInspectionCoordinate {
    /// Logical scopes and organizations.
    ScopeOrganization,
    /// Groups and discipline trees.
    GroupTree,
    /// Stable roles and bindings.
    RoleBinding,
    /// Owner-selected atomic objects.
    AtomicObject,
    /// Group-lens authority profiles.
    GroupLens,
    /// Directional service assignments.
    DirectionalService,
    /// Independent acceptance/effect frontiers.
    AcceptanceFrontier,
    /// Desired native projections.
    NativeProjection,
    /// Explicit negative/unresolved states.
    EffectiveState,
}
impl OrganizationInspectionCoordinate {
    fn from_api(value: &pale_sdk::OrganizationInspectionCoordinate) -> Self {
        match value {
            pale_sdk::OrganizationInspectionCoordinate::ScopeOrganization => {
                Self::ScopeOrganization
            }
            pale_sdk::OrganizationInspectionCoordinate::GroupTree => Self::GroupTree,
            pale_sdk::OrganizationInspectionCoordinate::RoleBinding => Self::RoleBinding,
            pale_sdk::OrganizationInspectionCoordinate::AtomicObject => Self::AtomicObject,
            pale_sdk::OrganizationInspectionCoordinate::GroupLens => Self::GroupLens,
            pale_sdk::OrganizationInspectionCoordinate::DirectionalService => {
                Self::DirectionalService
            }
            pale_sdk::OrganizationInspectionCoordinate::AcceptanceFrontier => {
                Self::AcceptanceFrontier
            }
            pale_sdk::OrganizationInspectionCoordinate::NativeProjection => Self::NativeProjection,
            pale_sdk::OrganizationInspectionCoordinate::EffectiveState => Self::EffectiveState,
        }
    }
}

/// Actual structural question class, not a name-based inferred object type.
#[napi]
pub enum OrganizationStructuralKind {
    /// Independent logical organization.
    LogicalOrganization,
    /// Stable role surviving occupant replacement.
    StableRole,
    /// Current occupant or provider binding.
    OccupantProvider,
    /// Exact directional service contract.
    DirectionalServiceContract,
    /// Accountable-work native projection.
    AccountableWorkProjection,
    /// Communication native projection.
    CommunicationProjection,
    /// Relationship/commercial native projection.
    RelationshipProjection,
    /// Replayable native application.
    NativeApplication,
    /// Transferable procedure Package.
    Package,
    /// Owner-attested normative Pack.
    NormativePack,
    /// Exact reference without copied body.
    ReferenceOnly,
}
impl OrganizationStructuralKind {
    fn from_api(value: &pale_sdk::OrganizationStructuralKind) -> Self {
        match value {
            pale_sdk::OrganizationStructuralKind::LogicalOrganization => Self::LogicalOrganization,
            pale_sdk::OrganizationStructuralKind::StableRole => Self::StableRole,
            pale_sdk::OrganizationStructuralKind::OccupantProvider => Self::OccupantProvider,
            pale_sdk::OrganizationStructuralKind::DirectionalServiceContract => {
                Self::DirectionalServiceContract
            }
            pale_sdk::OrganizationStructuralKind::AccountableWorkProjection => {
                Self::AccountableWorkProjection
            }
            pale_sdk::OrganizationStructuralKind::CommunicationProjection => {
                Self::CommunicationProjection
            }
            pale_sdk::OrganizationStructuralKind::RelationshipProjection => {
                Self::RelationshipProjection
            }
            pale_sdk::OrganizationStructuralKind::NativeApplication => Self::NativeApplication,
            pale_sdk::OrganizationStructuralKind::Package => Self::Package,
            pale_sdk::OrganizationStructuralKind::NormativePack => Self::NormativePack,
            pale_sdk::OrganizationStructuralKind::ReferenceOnly => Self::ReferenceOnly,
        }
    }
}

/// Actual owner-declared residence class.
#[napi]
pub enum OrganizationResidenceClass {
    /// Reusable method or obligations.
    Package,
    /// Owner-attested normative content.
    NormativePack,
    /// Replayable native truth or effect.
    NativeApplication,
    /// Principal-private selection or binding.
    Configuration,
    /// Exact reference to another owner's truth.
    Reference,
}
impl OrganizationResidenceClass {
    fn from_api(value: &pale_sdk::OrganizationResidenceClass) -> Self {
        match value {
            pale_sdk::OrganizationResidenceClass::Package => Self::Package,
            pale_sdk::OrganizationResidenceClass::NormativePack => Self::NormativePack,
            pale_sdk::OrganizationResidenceClass::NativeApplication => Self::NativeApplication,
            pale_sdk::OrganizationResidenceClass::Configuration => Self::Configuration,
            pale_sdk::OrganizationResidenceClass::Reference => Self::Reference,
        }
    }
}

/// Existing structural selection or attributed omission.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationStructuralSelection {
    /// Named structural question.
    pub kind: OrganizationStructuralKind,
    /// Actual generic resource answer; not cast into a role or scope identity.
    pub selection: OrganizationResourceAnswer,
}
impl OrganizationStructuralSelection {
    fn from_api(value: &pale_sdk::OrganizationStructuralSelection) -> Self {
        Self {
            kind: OrganizationStructuralKind::from_api(&value.kind),
            selection: OrganizationResourceAnswer::from_api(&value.selection),
        }
    }
}

/// Existing residence decision, without copying another owner's content.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct OrganizationResidence {
    /// Actual residence class.
    pub class: OrganizationResidenceClass,
    /// Exact selected resource/release reference.
    pub resource: String,
    /// Owner of that resource.
    pub owner: String,
}
impl OrganizationResidence {
    fn from_api(value: &pale_sdk::OrganizationResidence) -> Self {
        Self {
            class: OrganizationResidenceClass::from_api(&value.class),
            resource: value.resource.clone(),
            owner: value.owner.clone(),
        }
    }
}
