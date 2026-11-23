//! Read direct compiler-proof dependencies from one retained frontier.

use pale_core::records::{
    WorkspaceDerivationNodeV1, WorkspaceDerivationOriginV1, WorkspaceDerivationOutputV1,
};

/// Default number of direct input facts returned by one page.
pub const DEFAULT_DEPENDENCY_PAGE_LIMIT: u16 = 64;
/// Largest number of direct input facts returned by one page.
pub const MAX_DEPENDENCY_PAGE_LIMIT: u16 = 256;
/// Maximum UTF-8 bytes copied into dependency-step string fields per page.
///
/// This is deliberately larger than one maximally sized admitted source-content
/// step, so the byte ceiling always permits progress without splitting a fact.
/// The repeated root and canonical encoding have separate fixed/derived costs.
pub const MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES: usize = 8 * 1024;

/// A position beyond this subject's direct input list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidDependencyOffset;

impl std::fmt::Display for InvalidDependencyOffset {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("dependency page position exceeds the retained input list")
    }
}

impl std::error::Error for InvalidDependencyOffset {}

/// Borrow a selected fact and a page of its direct proof inputs.
///
/// Input order is the compiler's sorted proof-digest order. This is a direct
/// prerequisite query, not dependent impact or transitive reachability. The view
/// borrows the admitted graph; it allocates no graph copy, visited set or index.
/// Returned dependency string fields also share one fixed aggregate byte ceiling;
/// fixed enum/vector storage and canonical encoding remain separate costs. Neither
/// limit grants disclosure rights.
pub struct PortableWorkspaceDependencyPage<'a> {
    root: &'a WorkspaceDerivationNodeV1,
    dependencies: Vec<&'a WorkspaceDerivationNodeV1>,
    copied_string_bytes: usize,
    next_offset: Option<usize>,
}

impl<'a> PortableWorkspaceDependencyPage<'a> {
    pub(in crate::workspace) fn new(
        root: &'a WorkspaceDerivationNodeV1,
        nodes: &'a [WorkspaceDerivationNodeV1],
        offset: usize,
        requested_limit: u16,
    ) -> Result<Self, InvalidDependencyOffset> {
        let remaining = root.inputs.get(offset..).ok_or(InvalidDependencyOffset)?;
        let limit = if requested_limit == 0 {
            DEFAULT_DEPENDENCY_PAGE_LIMIT
        } else {
            requested_limit.min(MAX_DEPENDENCY_PAGE_LIMIT)
        };
        let mut dependencies = Vec::with_capacity(remaining.len().min(usize::from(limit)));
        let mut copied_string_bytes: usize = 0;
        for digest in remaining.iter().take(usize::from(limit)) {
            let index = nodes
                .binary_search_by(|node| node.node_digest.cmp(digest))
                .expect("admitted dependency refers to one retained proof node");
            let dependency = &nodes[index];
            let dependency_bytes = causal_step_copied_string_bytes(dependency);
            assert!(
                dependency_bytes <= MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES,
                "one admitted dependency step exceeds the fixed copied-string budget"
            );
            if copied_string_bytes.saturating_add(dependency_bytes)
                > MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES
            {
                break;
            }
            dependencies.push(dependency);
            copied_string_bytes += dependency_bytes;
        }
        let count = dependencies.len();
        Ok(Self {
            root,
            dependencies,
            copied_string_bytes,
            next_offset: (count < remaining.len()).then_some(offset + count),
        })
    }

    /// Fact whose direct inputs this page describes.
    #[must_use]
    pub const fn root(&self) -> &'a WorkspaceDerivationNodeV1 {
        self.root
    }

    /// Borrow each selected input after its single lookup in the sealed node array.
    pub fn dependencies(
        &self,
    ) -> impl ExactSizeIterator<Item = &'a WorkspaceDerivationNodeV1> + '_ {
        self.dependencies.iter().copied()
    }

    /// UTF-8 bytes copied into this page's dependency-step string fields.
    ///
    /// This excludes the repeated root, fixed enum/vector storage and the SDK's
    /// separately produced canonical encoding.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> usize {
        self.copied_string_bytes
    }

    /// Next direct-input position, or none when the list is exhausted.
    ///
    /// This application-local position is not a transferable continuation.
    /// The SDK must bind it to the exact owner, full subject and raw limit.
    #[must_use]
    pub const fn next_offset(&self) -> Option<usize> {
        self.next_offset
    }
}

fn causal_step_copied_string_bytes(node: &WorkspaceDerivationNodeV1) -> usize {
    node.node_digest.as_str().len()
        + super::comparison_subjects::subject_identity_bytes(&node.subject)
        + origin_copied_string_bytes(&node.origin)
        + output_copied_string_bytes(&node.output)
}

fn origin_copied_string_bytes(origin: &WorkspaceDerivationOriginV1) -> usize {
    match origin {
        WorkspaceDerivationOriginV1::Source { source_id } => source_id.as_str().len(),
        WorkspaceDerivationOriginV1::ModuleRelease {
            module_id,
            release_id,
            module_digest,
        } => module_id.as_str().len() + release_id.as_str().len() + module_digest.as_str().len(),
        WorkspaceDerivationOriginV1::Import {
            importer_module,
            upstream_module,
            upstream_release,
            export_id,
            source_namespace,
            local_namespace,
        } => {
            importer_module.as_str().len()
                + upstream_module.as_str().len()
                + upstream_release.as_str().len()
                + export_id.as_str().len()
                + source_namespace.as_str().len()
                + local_namespace.as_str().len()
        }
        WorkspaceDerivationOriginV1::Extension {
            owner_module,
            namespace,
            target,
        } => {
            owner_module.as_str().len()
                + namespace.as_str().len()
                + target.namespace.as_str().len()
                + target.declaration_id.as_str().len()
        }
        WorkspaceDerivationOriginV1::Compilation => 0,
    }
}

fn output_copied_string_bytes(output: &WorkspaceDerivationOutputV1) -> usize {
    match output {
        WorkspaceDerivationOutputV1::SourceContent {
            source_path,
            source_digest,
            ..
        } => source_path.as_str().len() + source_digest.as_str().len(),
        WorkspaceDerivationOutputV1::Component { component_digest } => {
            component_digest.as_str().len()
        }
        WorkspaceDerivationOutputV1::Module { module_digest } => module_digest.as_str().len(),
        WorkspaceDerivationOutputV1::Declaration { value_digest }
        | WorkspaceDerivationOutputV1::Extension { value_digest }
        | WorkspaceDerivationOutputV1::EffectivePolicy { value_digest }
        | WorkspaceDerivationOutputV1::Resource { value_digest } => value_digest.as_str().len(),
        WorkspaceDerivationOutputV1::ImportSurface { surface_digest } => {
            surface_digest.as_str().len()
        }
        WorkspaceDerivationOutputV1::ResponsibilityClosure { closure_digest } => {
            closure_digest.as_str().len()
        }
        WorkspaceDerivationOutputV1::OrganizationalTranslationClosure { closure_digest } => {
            closure_digest.as_str().len()
        }
        WorkspaceDerivationOutputV1::ExperienceComposition { source_digest } => {
            source_digest.as_str().len()
        }
        WorkspaceDerivationOutputV1::ResolutionLock { resolution_digest } => {
            resolution_digest.as_str().len()
        }
        WorkspaceDerivationOutputV1::UniverseLock { universe_digest } => {
            universe_digest.as_str().len()
        }
        WorkspaceDerivationOutputV1::SemanticIr {
            workspace_semantic_ir_digest,
        } => workspace_semantic_ir_digest.as_str().len(),
    }
}
