//! Move-only proof that a prompt came from an admitted direct owner boundary.

use std::{borrow::Cow, fmt};

use crate::terminal_adapters::TerminalAdapterDescriptor;
use pale_core::records::ProviderBindingRefV1;

use super::StdinEventFields;
use crate::hooks::{host_event_name, HookEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectPromptOrigin {
    Submitted,
    ProjectExpansion,
}

/// Owner-originated prompt input bound to one native harness.
///
/// The proof is move-only and can be minted only from a parsed native event
/// whose event name and host-specific origin agree. It proves no neutral
/// experience-path identity or delegation history; those remain separately
/// admitted compiler coordinates.
pub struct DirectOwnerPrompt<'a> {
    invocation: Cow<'a, str>,
    provider_binding: ProviderBindingRefV1,
    harness: TerminalAdapterDescriptor,
    event: HookEvent,
    origin: DirectPromptOrigin,
}

impl fmt::Debug for DirectOwnerPrompt<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOwnerPrompt")
            .field("harness", &self.harness)
            .field("event", &self.event)
            .field("origin", &self.origin)
            .field("invocation_bytes", &self.invocation.len())
            .finish()
    }
}

impl DirectOwnerPrompt<'_> {
    /// Canonical host invocation carried by this admitted owner boundary.
    pub fn invocation(&self) -> &str {
        self.invocation.as_ref()
    }

    /// Confirm that this proof was minted for the same exact provider binding.
    pub(crate) fn binds_provider(&self, provider: &ProviderBindingRefV1) -> bool {
        &self.provider_binding == provider
    }

    /// Native command sigil admitted by the provider adapter.
    pub(crate) const fn sigil(&self) -> char {
        if self.harness.is_claude_code() {
            '/'
        } else {
            '$'
        }
    }
}

impl StdinEventFields {
    /// Admit owner prompt input from the strongest boundary of one native harness.
    ///
    /// Claude requires a project-owned slash-command expansion; Codex admits
    /// only its native submitted-user-prompt event. The encoded event name must
    /// agree with the supplied [`HookEvent`]. This boundary does not infer or
    /// reconstruct a neutral experience path.
    pub fn direct_owner_prompt<'a>(
        &'a self,
        event: HookEvent,
        provider_binding: &ProviderBindingRefV1,
        harness: TerminalAdapterDescriptor,
    ) -> Option<DirectOwnerPrompt<'a>> {
        if self.hook_event_name() != Some(host_event_name(event)) {
            return None;
        }
        let (invocation, origin) = match (harness.is_codex(), event) {
            (true, HookEvent::UserPromptSubmit) => (
                Cow::Borrowed(self.message()?),
                DirectPromptOrigin::Submitted,
            ),
            (false, HookEvent::UserPromptExpansion) => {
                let expansion = self.expansion.as_ref()?;
                if expansion.expansion_type.as_deref() != Some("slash_command")
                    || expansion.command_source.as_deref() != Some("project")
                {
                    return None;
                }
                let name = expansion.command_name.as_deref()?.trim_start_matches('/');
                if name.is_empty()
                    || !name.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':')
                    })
                {
                    return None;
                }
                let command = format!("/{name}");
                let arguments = expansion.command_args.as_deref().unwrap_or("").trim();
                let invocation = if arguments.is_empty() {
                    command
                } else {
                    format!("{command} {arguments}")
                };
                (Cow::Owned(invocation), DirectPromptOrigin::ProjectExpansion)
            }
            _ => return None,
        };
        Some(DirectOwnerPrompt {
            invocation,
            provider_binding: provider_binding.clone(),
            harness,
            event,
            origin,
        })
    }
}
