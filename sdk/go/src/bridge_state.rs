//! Process-local custody for FFI requests, responses, and opaque API handles.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use pale_sdk::{
    AuthoringIdentity, AuthoringOccurrencePageCursor, EffectiveAuthoringContinuation,
    EffectiveProfileCompilation, LocalMaterializationAdmittedRun, LocalOneShotCompilation,
    LocalOneShotPlan, NativeActionRequest, ObservationLensCursor, ObservationPageCursor,
    ObservationProfileContinuation, ObservationSnapshot, ObservationSubject, OneShotAdmittedPlan,
    OneShotBoundPlan, OneShotCompilation, OneShotPlanCandidate, PortableWorkspaceComparison,
    PortableWorkspaceCompilation, ResidentHookAdmission,
};

/// First valid process-local handle. Zero is reserved for an invalid C call.
const FIRST_HANDLE: u64 = 1;

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(FIRST_HANDLE);
static REQUESTS: OnceLock<Mutex<BTreeMap<u64, Vec<u8>>>> = OnceLock::new();
static RESPONSES: OnceLock<Mutex<BTreeMap<u64, Response>>> = OnceLock::new();
type SharedClient = Arc<Mutex<ClientState>>;

static CLIENTS: OnceLock<Mutex<BTreeMap<u64, SharedClient>>> = OnceLock::new();

/// Exact FFI response class. Go maps diagnostics and bridge failures to
/// separate native error types.
#[repr(u8)]
#[derive(Clone, Copy)]
pub(crate) enum ResponseKind {
    Success = 0,
    Diagnostic = 1,
    BridgeFailure = 2,
}

/// One owned response retained until Go copies and drops it.
pub(crate) struct Response {
    pub(crate) kind: ResponseKind,
    pub(crate) object: u64,
    pub(crate) metadata: Vec<u8>,
    pub(crate) payload: Vec<u8>,
    pub(crate) pending_values: Option<Box<PendingResponse>>,
}

/// Native values remain response-owned until Go decodes the complete result.
pub(crate) struct PendingResponse {
    pub(crate) client: u64,
    pub(crate) values: PendingResponseValues,
}

pub(crate) enum PendingResponseValues {
    LocalCompilation {
        handle: u64,
        compilation: LocalOneShotCompilation,
    },
    LocalPlan {
        handle: u64,
        plan: LocalOneShotPlan,
    },
    LocalRun {
        handle: u64,
        run: LocalMaterializationAdmittedRun,
    },
    PortableWorkspace {
        handle: u64,
        workspace: PortableWorkspaceCompilation,
    },
    WorkspaceComparison {
        handle: u64,
        comparison: PortableWorkspaceComparison,
    },
    OneShotCompilation {
        handle: u64,
        compilation: OneShotCompilation,
    },
    OneShotCandidate {
        handle: u64,
        candidate: OneShotPlanCandidate,
    },
    OneShotBoundPlan {
        handle: u64,
        plan: OneShotBoundPlan,
    },
    OneShotAdmittedPlan {
        handle: u64,
        plan: OneShotAdmittedPlan,
    },
    AuthoringIdentities(Vec<(u64, AuthoringIdentity)>),
    AuthoringOccurrenceCursor {
        handle: u64,
        cursor: AuthoringOccurrencePageCursor,
    },
    EffectiveAuthoringCursor {
        handle: u64,
        cursor: EffectiveAuthoringContinuation,
    },
    ObservationProfileCursor {
        handle: u64,
        cursor: ObservationProfileContinuation,
    },
    EffectiveProfile {
        handle: u64,
        profile: EffectiveProfileCompilation,
    },
    Snapshot {
        handle: u64,
        snapshot: ObservationSnapshot,
    },
    Page {
        continuation: Option<(u64, Box<ObservationPageCursor>)>,
        subjects: Vec<(u64, ObservationSubject)>,
    },
    Lens {
        continuation: Option<(u64, Box<ObservationLensCursor>)>,
    },
}

impl PendingResponse {
    fn transfer(self) -> bool {
        with_client_mut(self.client, |client| {
            match self.values {
                PendingResponseValues::LocalCompilation {
                    handle,
                    compilation,
                } => {
                    client.local_compilations.insert(handle, compilation);
                }
                PendingResponseValues::LocalPlan { handle, plan } => {
                    client.local_plans.insert(handle, plan);
                }
                PendingResponseValues::LocalRun { handle, run } => {
                    client.local_runs.insert(handle, run);
                }
                PendingResponseValues::PortableWorkspace { handle, workspace } => {
                    client.workspaces.insert(handle, workspace);
                }
                PendingResponseValues::WorkspaceComparison { handle, comparison } => {
                    client.comparisons.insert(handle, comparison);
                }
                PendingResponseValues::OneShotCompilation {
                    handle,
                    compilation,
                } => {
                    client.compilations.insert(handle, compilation);
                }
                PendingResponseValues::OneShotCandidate { handle, candidate } => {
                    client.candidates.insert(handle, candidate);
                }
                PendingResponseValues::OneShotBoundPlan { handle, plan } => {
                    client.bound_plans.insert(handle, plan);
                }
                PendingResponseValues::OneShotAdmittedPlan { handle, plan } => {
                    client.admitted_plans.insert(handle, plan);
                }
                PendingResponseValues::AuthoringIdentities(identities) => {
                    client.authoring_identities.extend(identities);
                }
                PendingResponseValues::AuthoringOccurrenceCursor { handle, cursor } => {
                    client.authoring_occurrence_cursors.insert(handle, cursor);
                }
                PendingResponseValues::EffectiveAuthoringCursor { handle, cursor } => {
                    client.effective_authoring_cursors.insert(handle, cursor);
                }
                PendingResponseValues::ObservationProfileCursor { handle, cursor } => {
                    client.observation_profile_cursors.insert(handle, cursor);
                }
                PendingResponseValues::EffectiveProfile { handle, profile } => {
                    client.profiles.insert(handle, profile);
                }
                PendingResponseValues::Snapshot { handle, snapshot } => {
                    client.observation_snapshots.insert(handle, snapshot);
                }
                PendingResponseValues::Page {
                    continuation,
                    subjects,
                } => {
                    if let Some((handle, cursor)) = continuation {
                        client.observation_page_cursors.insert(handle, *cursor);
                    }
                    client.observation_subjects.extend(subjects);
                }
                PendingResponseValues::Lens { continuation } => {
                    if let Some((handle, cursor)) = continuation {
                        client.observation_lens_cursors.insert(handle, *cursor);
                    }
                }
            }
            Ok(())
        })
        .is_ok()
    }
}

/// All opaque values owned by one Go client.
pub(crate) struct ClientState {
    pub(crate) api: pale_sdk::PaleApi,
    pub(crate) improvement_inspections: BTreeMap<u64, pale_sdk::ConfigurationImprovementInspection>,
    pub(crate) improvement_explanations:
        BTreeMap<u64, pale_sdk::ConfigurationImprovementExplanation>,
    pub(crate) improvement_proposals: BTreeMap<u64, pale_sdk::ConfigurationImprovementProposal>,
    pub(crate) improvement_evaluations: BTreeMap<u64, pale_sdk::ConfigurationImprovementEvaluation>,
    pub(crate) improvement_diagnoses: BTreeMap<u64, pale_sdk::ConfigurationImprovementDiagnosis>,
    pub(crate) improvement_prepared:
        BTreeMap<u64, pale_sdk::PreparedConfigurationImprovementPreview>,
    pub(crate) improvement_previews: BTreeMap<u64, pale_sdk::ConfigurationImprovementPreview>,
    pub(crate) workspaces: BTreeMap<u64, PortableWorkspaceCompilation>,
    pub(crate) comparisons: BTreeMap<u64, PortableWorkspaceComparison>,
    pub(crate) profiles: BTreeMap<u64, EffectiveProfileCompilation>,
    pub(crate) observation_snapshots: BTreeMap<u64, ObservationSnapshot>,
    pub(crate) observation_page_cursors: BTreeMap<u64, ObservationPageCursor>,
    pub(crate) observation_lens_cursors: BTreeMap<u64, ObservationLensCursor>,
    pub(crate) observation_profile_cursors: BTreeMap<u64, ObservationProfileContinuation>,
    pub(crate) observation_subjects: BTreeMap<u64, ObservationSubject>,
    pub(crate) authoring_identities: BTreeMap<u64, AuthoringIdentity>,
    pub(crate) authoring_occurrence_cursors: BTreeMap<u64, AuthoringOccurrencePageCursor>,
    pub(crate) effective_authoring_cursors: BTreeMap<u64, EffectiveAuthoringContinuation>,
    pub(crate) compilations: BTreeMap<u64, OneShotCompilation>,
    pub(crate) candidates: BTreeMap<u64, OneShotPlanCandidate>,
    pub(crate) bound_plans: BTreeMap<u64, OneShotBoundPlan>,
    pub(crate) admitted_plans: BTreeMap<u64, OneShotAdmittedPlan>,
    pub(crate) resident_hook_admissions: BTreeMap<u64, ResidentHookAdmission>,
    pub(crate) native_action_requests: BTreeMap<u64, NativeActionRequest>,
    pub(crate) local_compilations: BTreeMap<u64, LocalOneShotCompilation>,
    pub(crate) local_plans: BTreeMap<u64, LocalOneShotPlan>,
    pub(crate) local_runs: BTreeMap<u64, LocalMaterializationAdmittedRun>,
}

impl ClientState {
    pub(crate) fn new(api: pale_sdk::PaleApi) -> Self {
        Self {
            api,
            improvement_inspections: BTreeMap::new(),
            improvement_explanations: BTreeMap::new(),
            improvement_proposals: BTreeMap::new(),
            improvement_evaluations: BTreeMap::new(),
            improvement_diagnoses: BTreeMap::new(),
            improvement_prepared: BTreeMap::new(),
            improvement_previews: BTreeMap::new(),
            workspaces: BTreeMap::new(),
            comparisons: BTreeMap::new(),
            profiles: BTreeMap::new(),
            observation_snapshots: BTreeMap::new(),
            observation_page_cursors: BTreeMap::new(),
            observation_lens_cursors: BTreeMap::new(),
            observation_profile_cursors: BTreeMap::new(),
            observation_subjects: BTreeMap::new(),
            authoring_identities: BTreeMap::new(),
            authoring_occurrence_cursors: BTreeMap::new(),
            effective_authoring_cursors: BTreeMap::new(),
            compilations: BTreeMap::new(),
            candidates: BTreeMap::new(),
            bound_plans: BTreeMap::new(),
            admitted_plans: BTreeMap::new(),
            resident_hook_admissions: BTreeMap::new(),
            native_action_requests: BTreeMap::new(),
            local_compilations: BTreeMap::new(),
            local_plans: BTreeMap::new(),
            local_runs: BTreeMap::new(),
        }
    }
}

fn next_handle() -> u64 {
    loop {
        let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        if handle != 0 {
            return handle;
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn requests() -> &'static Mutex<BTreeMap<u64, Vec<u8>>> {
    REQUESTS.get_or_init(Default::default)
}

fn responses() -> &'static Mutex<BTreeMap<u64, Response>> {
    RESPONSES.get_or_init(Default::default)
}

fn clients() -> &'static Mutex<BTreeMap<u64, SharedClient>> {
    CLIENTS.get_or_init(Default::default)
}

pub(crate) fn insert_request(bytes: Vec<u8>) -> u64 {
    let handle = next_handle();
    lock(requests()).insert(handle, bytes);
    handle
}

pub(crate) fn take_request(handle: u64) -> Option<Vec<u8>> {
    lock(requests()).remove(&handle)
}

pub(crate) fn drop_request(handle: u64) {
    lock(requests()).remove(&handle);
}

pub(crate) fn insert_response(response: Response) -> u64 {
    let handle = next_handle();
    lock(responses()).insert(handle, response);
    handle
}

pub(crate) fn with_response<R>(handle: u64, f: impl FnOnce(&Response) -> R) -> Option<R> {
    lock(responses()).get(&handle).map(f)
}

pub(crate) fn drop_response(handle: u64) {
    lock(responses()).remove(&handle);
}

/// Consume a decoded response and transfer only its pending native values.
pub(crate) fn claim_response(handle: u64) -> bool {
    // Release the response-map lock before acquiring the exact client lock.
    let response = { lock(responses()).remove(&handle) };
    match response {
        Some(Response {
            kind: ResponseKind::Success,
            pending_values,
            ..
        }) => pending_values.is_none_or(|pending| (*pending).transfer()),
        _ => false,
    }
}

pub(crate) fn insert_client(client: ClientState) -> u64 {
    let handle = next_handle();
    lock(clients()).insert(handle, Arc::new(Mutex::new(client)));
    handle
}

pub(crate) fn drop_client(handle: u64) {
    lock(clients()).remove(&handle);
}

pub(crate) fn with_client<R>(
    handle: u64,
    f: impl FnOnce(&ClientState) -> Result<R, crate::BridgeFailure>,
) -> Result<R, crate::BridgeFailure> {
    let client = lock(clients())
        .get(&handle)
        .cloned()
        .ok_or_else(|| crate::BridgeFailure::invalid_handle("client", handle))?;
    let result = f(&lock(&client));
    result
}

pub(crate) fn with_client_mut<R>(
    handle: u64,
    f: impl FnOnce(&mut ClientState) -> Result<R, crate::BridgeFailure>,
) -> Result<R, crate::BridgeFailure> {
    let client = lock(clients())
        .get(&handle)
        .cloned()
        .ok_or_else(|| crate::BridgeFailure::invalid_handle("client", handle))?;
    let result = f(&mut lock(&client));
    result
}

pub(crate) fn next_object_handle() -> u64 {
    next_handle()
}
