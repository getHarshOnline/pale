//! Client-scoped, phase-typed projection of the application-owned review lifecycle.
//!
//! Preparation borrows its explanation; consuming transitions remove their API
//! owners only after request and handle decoding. Neither preview nor acceptance
//! mutates configuration or grants authority.

use crate::operations::{api, json_response};

#[derive(serde::Serialize)]
struct InspectionState<'a> {
    phase: pale_sdk::ConfigurationImprovementPhase,
    workspace_id: &'a str,
    principal_universe_digest: &'a str,
}

pub(crate) fn inspection_state(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        let value = client
            .improvement_inspections
            .get(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement inspections", request.handle)
            })?;
        json_response(&InspectionState {
            phase: value.phase(),
            workspace_id: value.workspace_id(),
            principal_universe_digest: value.principal_universe_digest(),
        })
    })
}

#[derive(serde::Serialize)]
struct ExplanationState<'a> {
    phase: pale_sdk::ConfigurationImprovementPhase,
    profile_digest: &'a str,
    instance_digest: &'a str,
}

pub(crate) fn explanation_state(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        let value = client
            .improvement_explanations
            .get(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement explanations", request.handle)
            })?;
        json_response(&ExplanationState {
            phase: value.phase(),
            profile_digest: value.profile_digest(),
            instance_digest: value.instance_digest(),
        })
    })
}

#[derive(serde::Serialize)]
struct EvaluationState<'a> {
    phase: pale_sdk::ConfigurationImprovementPhase,
    candidate_principal_universe_digest: &'a str,
    candidate_profile_digest: &'a str,
    candidate_instance_digest: &'a str,
}

pub(crate) fn evaluation_state(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        let value = client
            .improvement_evaluations
            .get(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement evaluations", request.handle)
            })?;
        json_response(&EvaluationState {
            phase: value.phase(),
            candidate_principal_universe_digest: value.candidate_principal_universe_digest(),
            candidate_profile_digest: value.candidate_profile_digest(),
            candidate_instance_digest: value.candidate_instance_digest(),
        })
    })
}

#[derive(serde::Serialize)]
struct DiagnosisState<'a> {
    phase: pale_sdk::ConfigurationImprovementPhase,
    derivation_receipt_digest: &'a str,
}

pub(crate) fn diagnosis_state(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        let value = client
            .improvement_diagnoses
            .get(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement diagnoses", request.handle)
            })?;
        json_response(&DiagnosisState {
            phase: value.phase(),
            derivation_receipt_digest: value.derivation_receipt_digest(),
        })
    })
}

pub(crate) fn proposal_phase(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        let value = client
            .improvement_proposals
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("improvement proposal", request.handle))?;
        json_response(&value.phase())
    })
}

pub(crate) fn preview_phase(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        let value = client
            .improvement_previews
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("improvement preview", request.handle))?;
        json_response(&value.phase())
    })
}

use crate::{bridge_state, success, wire, BridgeFailure};
use pale_sdk::{ConfigurationImprovementCandidateRequest, ConfigurationImprovementProposalRequest};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Handle {
    handle: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Explain {
    handle: u64,
    profile: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Proposal {
    handle: u64,
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    workspace: wire::PortableRequest,
    private_inputs: wire::PrivateInputs,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    handle: u64,
    candidate: u64,
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private_inputs: wire::PrivateInputs,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commit {
    handle: u64,
    prepared: u64,
}

/// A selected retained profile, never a machine realization claim.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProfileSide {
    Baseline,
    Candidate,
}

impl ProfileSide {
    fn select(
        self,
        preview: &pale_sdk::ConfigurationImprovementPreview,
    ) -> &pale_sdk::EffectiveProfileCompilation {
        match self {
            Self::Baseline => preview.baseline_profile(),
            Self::Candidate => preview.candidate_profile(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfilePage {
    handle: u64,
    side: ProfileSide,
    section: wire::ProfileSection,
    offset: u32,
    limit: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfileExplanation {
    handle: u64,
    side: ProfileSide,
    explanation: String,
}

pub(crate) fn inspect(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        let baseline = client
            .workspaces
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("portable workspace", request.handle))?;
        let value = client.api.inspect_configuration_improvement(baseline);
        let object = bridge_state::next_object_handle();
        client.improvement_inspections.insert(object, value);
        Ok(success(Vec::new(), Vec::new(), object))
    })
}

pub(crate) fn explain(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Explain = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        let profile = client
            .profiles
            .get(&request.profile)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.profile))?;
        let inspection = client
            .improvement_inspections
            .remove(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement inspection", request.handle)
            })?;
        api(
            client
                .api
                .explain_configuration_improvement(inspection, profile),
            |value| {
                let object = bridge_state::next_object_handle();
                client.improvement_explanations.insert(object, value);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

pub(crate) fn propose(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Proposal = crate::decode(bytes)?;
    let candidate = ConfigurationImprovementProposalRequest::new(
        request.project_id,
        request.execution_workspace_id,
        request.machine_id,
        request.workspace.into_api()?,
        request.private_inputs.into_api()?,
    );
    bridge_state::with_client_mut(client, |client| {
        let explanation = client
            .improvement_explanations
            .remove(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement explanation", request.handle)
            })?;
        api(
            client
                .api
                .propose_configuration_improvement(explanation, candidate),
            |value| {
                let object = bridge_state::next_object_handle();
                client.improvement_proposals.insert(object, value);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

pub(crate) fn evaluate(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        let prior = client
            .improvement_proposals
            .remove(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement proposals", request.handle)
            })?;
        api(
            client.api.evaluate_configuration_improvement(prior),
            |value| {
                let object = bridge_state::next_object_handle();
                client.improvement_evaluations.insert(object, value);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

pub(crate) fn diagnose(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        let prior = client
            .improvement_evaluations
            .remove(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement evaluations", request.handle)
            })?;
        api(
            client.api.diagnose_configuration_improvement(prior),
            |value| {
                let object = bridge_state::next_object_handle();
                client.improvement_diagnoses.insert(object, value);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

pub(crate) fn preview(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        let prior = client
            .improvement_diagnoses
            .remove(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement diagnoses", request.handle)
            })?;
        api(
            client.api.preview_configuration_improvement(prior),
            |value| {
                let object = bridge_state::next_object_handle();
                client.improvement_previews.insert(object, value);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

pub(crate) fn prepare(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Prepare = crate::decode(bytes)?;
    let candidate_request = ConfigurationImprovementCandidateRequest::new(
        request.project_id,
        request.execution_workspace_id,
        request.machine_id,
        request.private_inputs.into_api()?,
    );
    bridge_state::with_client_mut(client, |client| {
        let explanation = client
            .improvement_explanations
            .get(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("improvement explanation", request.handle)
            })?;
        let candidate = client.workspaces.get(&request.candidate).ok_or_else(|| {
            BridgeFailure::invalid_handle("portable workspace", request.candidate)
        })?;
        api(
            client.api.prepare_configuration_improvement_preview(
                explanation,
                candidate,
                candidate_request,
            ),
            |value| {
                let object = bridge_state::next_object_handle();
                client.improvement_prepared.insert(object, value);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

pub(crate) fn commit(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Commit = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        // Validate both client-scoped handles before consuming either. Exact
        // explanation-token equality remains exclusively the API's decision.
        if !client
            .improvement_explanations
            .contains_key(&request.handle)
        {
            return Err(BridgeFailure::invalid_handle(
                "improvement explanation",
                request.handle,
            ));
        }
        if !client.improvement_prepared.contains_key(&request.prepared) {
            return Err(BridgeFailure::invalid_handle(
                "prepared improvement",
                request.prepared,
            ));
        }
        let explanation = client
            .improvement_explanations
            .remove(&request.handle)
            .expect("preflight and removal hold the same client mutex");
        let prepared = client
            .improvement_prepared
            .remove(&request.prepared)
            .expect("preflight and removal hold the same client mutex");
        api(
            client
                .api
                .commit_configuration_improvement_preview(explanation, prepared),
            |value| {
                let object = bridge_state::next_object_handle();
                client.improvement_previews.insert(object, value);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

pub(crate) fn prepared_summary(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        let prepared = client
            .improvement_prepared
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("prepared improvement", request.handle))?;
        Ok(success(
            Vec::new(),
            prepared.summary().canonical_bytes().to_vec(),
            0,
        ))
    })
}

fn retained(
    client: &bridge_state::ClientState,
    handle: u64,
) -> Result<&pale_sdk::ConfigurationImprovementPreview, BridgeFailure> {
    client
        .improvement_previews
        .get(&handle)
        .ok_or_else(|| BridgeFailure::invalid_handle("improvement preview", handle))
}

pub(crate) fn summary(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        Ok(success(
            Vec::new(),
            retained(client, request.handle)?
                .summary()
                .canonical_bytes()
                .to_vec(),
            0,
        ))
    })
}

pub(crate) fn comparison_summary(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        Ok(success(
            Vec::new(),
            retained(client, request.handle)?
                .comparison()
                .summary()
                .canonical_bytes()
                .to_vec(),
            0,
        ))
    })
}

pub(crate) fn change_roots(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: wire::PageRequest = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        Ok(success(
            Vec::new(),
            retained(client, request.handle)?
                .comparison()
                .change_roots(request.offset, request.limit)
                .canonical_bytes()
                .to_vec(),
            0,
        ))
    })
}

pub(crate) fn impacts(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: wire::PageRequest = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        Ok(success(
            Vec::new(),
            retained(client, request.handle)?
                .comparison()
                .impacts(request.offset, request.limit)
                .canonical_bytes()
                .to_vec(),
            0,
        ))
    })
}

pub(crate) fn explain_comparison(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: wire::WorkspaceExplanationRequest = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        api(
            retained(client, request.handle)?
                .comparison()
                .explain_selector(request.subject.into()),
            |value| Ok(success(Vec::new(), value.canonical_bytes().to_vec(), 0)),
        )
    })
}

pub(crate) fn profile_page(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: ProfilePage = crate::decode(bytes)?;
    bridge_state::with_client(client, |client| {
        let profile = request.side.select(retained(client, request.handle)?);
        Ok(success(
            Vec::new(),
            profile
                .inspect(pale_sdk::EffectiveProfilePageRequest::new(
                    request.section.into(),
                    request.offset,
                    request.limit,
                ))
                .canonical_bytes()
                .to_vec(),
            0,
        ))
    })
}

pub(crate) fn explain_profile(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: ProfileExplanation = crate::decode(bytes)?;
    let explanation = pale_sdk::EffectiveProfileExplanationHandle::parse(request.explanation)
        .map_err(|error| BridgeFailure::request(error.to_string()))?;
    bridge_state::with_client(client, |client| {
        json_response(
            &request
                .side
                .select(retained(client, request.handle)?)
                .explain(&explanation),
        )
    })
}

pub(crate) fn accept(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        let preview = client
            .improvement_previews
            .remove(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("improvement preview", request.handle))?;
        let terminal = preview.accept();
        Ok(success(
            crate::encode(&terminal.disposition())?,
            terminal.preview().canonical_bytes().to_vec(),
            0,
        ))
    })
}

pub(crate) fn reject(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        let preview = client
            .improvement_previews
            .remove(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("improvement preview", request.handle))?;
        let terminal = preview.reject();
        Ok(success(
            crate::encode(&terminal.disposition())?,
            terminal.preview().canonical_bytes().to_vec(),
            0,
        ))
    })
}

pub(crate) fn cancel(client: u64, bytes: &[u8]) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        let preview = client
            .improvement_previews
            .remove(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("improvement preview", request.handle))?;
        let terminal = preview.cancel();
        Ok(success(
            crate::encode(&terminal.disposition())?,
            terminal.preview().canonical_bytes().to_vec(),
            0,
        ))
    })
}

pub(crate) fn mark_stale(
    client: u64,
    bytes: &[u8],
) -> Result<bridge_state::Response, BridgeFailure> {
    let request: Handle = crate::decode(bytes)?;
    bridge_state::with_client_mut(client, |client| {
        let preview = client
            .improvement_previews
            .remove(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("improvement preview", request.handle))?;
        let terminal = preview.mark_stale();
        Ok(success(
            crate::encode(&terminal.disposition())?,
            terminal.preview().canonical_bytes().to_vec(),
            0,
        ))
    })
}
