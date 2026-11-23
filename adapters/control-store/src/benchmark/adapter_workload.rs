//! Exact unadmitted adapter mechanics exercised through the neutral authority port.

use std::path::Path;
use std::time::{Duration, Instant};

use pale_core::records::{
    ArtifactDigest, ControlIncarnationIdV1, ControlStoreIdV1, JournalSequenceV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1, ResidentWorkspaceIdV1,
};
use pale_core::sha256_hex;
use pale_ports::{
    AuthorityByteClass, AuthorityStoreCoordinate, BoundedAuthorityEntryBytes,
    BoundedRecoveryCheckpointBytes, DurableAuthorityStorePort, JournalTailPageBoundary,
    JournalTailPageRead, PreparedAuthorityAppend, PreparedRecoveryCheckpoint,
    RecoveryCheckpointRead, SequencedAuthorityEntry, StoredRecoveryCheckpoint,
};

use super::input::BenchmarkCase;
use super::workload::{
    deterministic_payload, file_size, file_sizes, private_tempdir, set_private_permissions,
    CaseMeasurements, CrashWriterRequest,
};
use crate::authority::DeploymentControlSqliteStore;
use crate::location::{
    ControlStorageRoot, ControlStoreLocation, ControlStoreScope, DeploymentStoreScope,
    PrincipalStoreScope,
};
use crate::store::{
    classify_store, initialize_unadmitted_store, ControlStoreIdentity, StoreOpenClassification,
};
use crate::{verify_embedded_storage_sources, ControlStoreKind};

/// Maximum journal entries read per adapter benchmark recovery page.
const BENCHMARK_RECOVERY_PAGE_ENTRIES: usize = 64;

/// Run one exact-port append/checkpoint sample in an isolated unadmitted store.
pub(crate) fn run_case_sample(case: &BenchmarkCase) -> CaseMeasurements {
    assert!(
        case.pragma_candidate.is_none(),
        "adapter mechanics cannot carry SQLite pragma candidates"
    );
    let directory = private_tempdir();
    preflight_principal_store(directory.path());
    let fixture = initialize_deployment_store(directory.path());
    let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
        &fixture.location,
        fixture.coordinate.clone(),
    )
    .expect("open exact unadmitted deployment adapter");
    let payload = deterministic_payload(case.payload_bytes);
    let checkpoint_payload = deterministic_payload(case.checkpoint_bytes);
    let mut sample = CaseMeasurements::default();
    append_records(&store, &fixture.coordinate, case, &payload, &mut sample);
    sample
        .before_checkpoint
        .push(file_sizes(&fixture.location.path()));
    sample.checkpoint_projection_commit.push(store_checkpoint(
        &store,
        &fixture.coordinate,
        case.records,
        &checkpoint_payload,
        case.limits.maximum_checkpoint_bytes(),
    ));
    sample
        .after_checkpoint
        .push(file_sizes(&fixture.location.path()));
    sample
}

/// Create committed adapter state, then terminate without connection destructors.
pub(crate) fn run_crash_writer(request: &CrashWriterRequest) -> ! {
    assert!(
        request.pragma_candidate.is_none(),
        "adapter mechanics child cannot carry SQLite pragma candidates"
    );
    set_private_permissions(&request.root);
    preflight_principal_store(&request.root);
    let fixture = initialize_deployment_store(&request.root);
    let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
        &fixture.location,
        fixture.coordinate.clone(),
    )
    .expect("open crash-writer unadmitted deployment adapter");
    let payload = deterministic_payload(request.payload_bytes);
    let checkpoint_payload = deterministic_payload(request.checkpoint_bytes);
    let tail_entries = request
        .records
        .saturating_sub(1)
        .min(request.limits.maximum_tail_entries() as u64);
    let checkpoint_frontier = request.records - tail_entries;
    let mut expected_head = None;
    for journal_sequence in 1..=request.records {
        append_record(
            &store,
            &fixture.coordinate,
            expected_head,
            journal_sequence,
            &payload,
            request.limits.maximum_entry_bytes(),
        );
        expected_head = Some(sequence(journal_sequence));
        if journal_sequence == checkpoint_frontier {
            std::hint::black_box(store_checkpoint(
                &store,
                &fixture.coordinate,
                journal_sequence,
                &checkpoint_payload,
                request.limits.maximum_checkpoint_bytes(),
            ));
        }
    }
    std::hint::black_box(&store);
    std::process::exit(0)
}

/// Reopen after process loss through the adapter and validate bounded port reads.
pub(crate) fn run_recovery_sample(case: &BenchmarkCase, root: &Path) -> CaseMeasurements {
    let fixture = existing_deployment_store(root);
    let mut sample = CaseMeasurements::default();
    sample
        .recovery_tail
        .push(file_size(&sqlite_sidecar(&fixture.location.path(), "-wal")));
    let started = Instant::now();
    require_profile_unselected(&fixture.location, &fixture.identity);
    let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
        &fixture.location,
        fixture.coordinate.clone(),
    )
    .expect("reopen exact unadmitted deployment adapter after process loss");
    let checkpoints = store
        .read_recovery_checkpoints(&RecoveryCheckpointRead::new(
            fixture.coordinate.clone(),
            case.limits,
        ))
        .expect("read bounded recovery checkpoints through adapter port");
    let checkpoint = checkpoints
        .iter()
        .find_map(|candidate| candidate.trusted())
        .expect("at least one trusted checkpoint remains after process loss");
    let frontier = checkpoint.through_sequence();
    let expected_checkpoint = deterministic_payload(case.checkpoint_bytes);
    assert_eq!(checkpoint.bytes().as_slice(), expected_checkpoint);
    let head = store
        .read_journal_head(&fixture.coordinate)
        .expect("read the exact adapter recovery head")
        .expect("benchmark recovery journal is nonempty");
    assert_eq!(head, sequence(case.records));
    let mut request = JournalTailPageRead::new(
        fixture.coordinate,
        Some(frontier),
        head,
        case.limits,
        BENCHMARK_RECOVERY_PAGE_ENTRIES,
    )
    .expect("construct bounded fixed-head recovery page request");
    let expected_payload = deterministic_payload(case.payload_bytes);
    let mut expected_sequence = frontier
        .get()
        .checked_add(1)
        .expect("recovery sequence overflow");
    loop {
        let page = store
            .read_journal_page(&request)
            .expect("read bounded fixed-head recovery page through adapter port");
        for entry in page.entries() {
            assert_eq!(entry.sequence().get(), expected_sequence);
            assert_eq!(entry.bytes().as_slice(), expected_payload);
            expected_sequence = expected_sequence
                .checked_add(1)
                .expect("recovery sequence overflow");
        }
        match page.boundary() {
            JournalTailPageBoundary::Complete => break,
            JournalTailPageBoundary::More { .. } => {
                request = request
                    .continue_after(&page)
                    .expect("continue within whole-pass recovery budget")
                    .expect("a continuation boundary yields another request");
            }
            JournalTailPageBoundary::QuarantinedFrom { .. } => {
                panic!("benchmark recovery journal remains physically trusted")
            }
        }
    }
    assert_eq!(expected_sequence.saturating_sub(1), case.records);
    sample.recovery.push(started.elapsed());
    sample
}

/// Append every record through the real port and retain operation observations.
fn append_records(
    store: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    case: &BenchmarkCase,
    payload: &[u8],
    sample: &mut CaseMeasurements,
) {
    let mut expected_head = None;
    for journal_sequence in 1..=case.records {
        let checksum_started = Instant::now();
        std::hint::black_box(sha256_hex(std::hint::black_box(payload)));
        sample.checksum_reference.push(checksum_started.elapsed());
        let started = Instant::now();
        append_record(
            store,
            coordinate,
            expected_head,
            journal_sequence,
            payload,
            case.limits.maximum_entry_bytes(),
        );
        sample.append.push(started.elapsed());
        expected_head = Some(sequence(journal_sequence));
    }
}

/// Perform one exact compare-and-append port operation.
fn append_record(
    store: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    expected_head: Option<JournalSequenceV1>,
    journal_sequence: u64,
    payload: &[u8],
    maximum_entry_bytes: usize,
) {
    let bytes = BoundedAuthorityEntryBytes::new(payload.to_vec(), maximum_entry_bytes)
        .expect("benchmark entry fits its explicit port bound");
    let entry = SequencedAuthorityEntry::new(sequence(journal_sequence), bytes);
    let prepared = PreparedAuthorityAppend::new(coordinate.clone(), expected_head, entry)
        .expect("benchmark append is an exact successor");
    store
        .compare_and_append(&prepared)
        .expect("append through exact deployment adapter port");
}

/// Store one handle-free checkpoint through the real port.
fn store_checkpoint(
    store: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    through_sequence: u64,
    payload: &[u8],
    maximum_checkpoint_bytes: usize,
) -> Duration {
    let through_sequence = sequence(through_sequence);
    let bytes = BoundedRecoveryCheckpointBytes::new(payload.to_vec(), maximum_checkpoint_bytes)
        .expect("benchmark checkpoint fits its explicit port bound");
    let checkpoint = StoredRecoveryCheckpoint::new(through_sequence, bytes);
    let prepared =
        PreparedRecoveryCheckpoint::new(coordinate.clone(), through_sequence, checkpoint)
            .expect("benchmark checkpoint matches the exact journal head");
    let started = Instant::now();
    store
        .compare_and_store_checkpoint(&prepared)
        .expect("store checkpoint through exact deployment adapter port");
    started.elapsed()
}

/// Initialize and classify a principal store as untimed exact-source preflight.
fn preflight_principal_store(root: &Path) {
    let scope = ControlStoreScope::Principal(PrincipalStoreScope::new(
        ResidentPrincipalIdV1::parse("benchmark-principal").expect("fixed principal ID parses"),
    ));
    let (location, identity) = location_and_identity(
        root,
        scope,
        "benchmark-principal-store",
        "benchmark-principal-incarnation",
    );
    let initialized = initialize_unadmitted_store(&location, identity.clone())
        .expect("initialize principal preflight store from exact sources");
    assert_initialized(&initialized, &location, &identity);
    require_profile_unselected(&location, &identity);
}

/// Initialize and classify the deployment store used by an adapter sample.
pub(super) fn initialize_deployment_store(root: &Path) -> AdapterFixture {
    let fixture = new_deployment_store(root);
    let initialized = initialize_unadmitted_store(&fixture.location, fixture.identity.clone())
        .expect("initialize deployment benchmark store from exact sources");
    assert_initialized(&initialized, &fixture.location, &fixture.identity);
    require_profile_unselected(&fixture.location, &fixture.identity);
    fixture
}

/// Derive the exact deployment coordinate while preparing a new location.
fn new_deployment_store(root: &Path) -> AdapterFixture {
    let scope = ControlStoreScope::Deployment(DeploymentStoreScope::new(
        ResidentWorkspaceIdV1::parse("benchmark-workspace").expect("fixed workspace ID parses"),
        ResidentRecordRefV1::parse("benchmark-deployment-binding")
            .expect("fixed deployment binding parses"),
        ArtifactDigest::hash_bytes(b"benchmark-deployment-binding"),
    ));
    let (location, identity) = location_and_identity(
        root,
        scope,
        "benchmark-deployment-store",
        "benchmark-deployment-incarnation",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    AdapterFixture {
        location,
        identity,
        coordinate,
    }
}

/// Reconstruct the exact deployment coordinate without creating directories.
fn existing_deployment_store(root: &Path) -> AdapterFixture {
    let scope = ControlStoreScope::Deployment(DeploymentStoreScope::new(
        ResidentWorkspaceIdV1::parse("benchmark-workspace").expect("fixed workspace ID parses"),
        ResidentRecordRefV1::parse("benchmark-deployment-binding")
            .expect("fixed deployment binding parses"),
        ArtifactDigest::hash_bytes(b"benchmark-deployment-binding"),
    ));
    let identity = ControlStoreIdentity::new(
        scope.clone(),
        ControlStoreIdV1::parse("benchmark-deployment-store")
            .expect("fixed benchmark store ID parses"),
        ControlIncarnationIdV1::parse("benchmark-deployment-incarnation")
            .expect("fixed benchmark incarnation ID parses"),
    );
    let location = ControlStorageRoot::retain(root)
        .expect("retain existing benchmark storage root")
        .lookup_store_location(scope)
        .expect("look up existing benchmark store location")
        .expect("benchmark store location exists");
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    AdapterFixture {
        location,
        identity,
        coordinate,
    }
}

/// Derive a store location and exact logical/incarnation identity.
fn location_and_identity(
    root: &Path,
    scope: ControlStoreScope,
    store_id: &str,
    incarnation_id: &str,
) -> (ControlStoreLocation, ControlStoreIdentity) {
    let location = ControlStorageRoot::retain(root)
        .expect("retain isolated benchmark storage root")
        .prepare_store_location(scope.clone())
        .expect("derive canonical benchmark store location");
    let identity = ControlStoreIdentity::new(
        scope,
        ControlStoreIdV1::parse(store_id).expect("fixed benchmark store ID parses"),
        ControlIncarnationIdV1::parse(incarnation_id)
            .expect("fixed benchmark incarnation ID parses"),
    );
    (location, identity)
}

/// Require the exact source frontier while retaining every profile field unselected.
fn require_profile_unselected(location: &ControlStoreLocation, identity: &ControlStoreIdentity) {
    match classify_store(location, identity).expect("classify exact benchmark store") {
        StoreOpenClassification::ProfileUnselected { unresolved } => {
            assert!(
                !unresolved.is_empty(),
                "unresolved profile requirements remain"
            )
        }
        other => panic!("benchmark store must remain ProfileUnselected, observed {other:?}"),
    }
}

/// Consume initialization evidence without upgrading it to Pale Runtime assembly.
fn assert_initialized(
    initialized: &crate::store::InitializedStore,
    location: &ControlStoreLocation,
    identity: &ControlStoreIdentity,
) {
    assert_eq!(initialized.path(), location.path());
    assert_eq!(initialized.identity(), identity);
    let sources = verify_embedded_storage_sources().expect("verify embedded storage sources");
    assert_eq!(initialized.manifest_sha256(), sources.manifest_sha256());
    assert!(!initialized.compile_options_sha256().is_empty());
    assert_eq!(initialized.identity().kind(), identity.kind());
    assert!(matches!(
        identity.kind(),
        ControlStoreKind::PrincipalTrust | ControlStoreKind::DeploymentControl
    ));
}

/// Construct an `SQLite` sidecar path without Unicode conversion.
fn sqlite_sidecar(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    value.into()
}

/// Construct one positive journal sequence.
fn sequence(value: u64) -> JournalSequenceV1 {
    JournalSequenceV1::new(value).expect("benchmark journal sequence is positive and bounded")
}

/// Exact initialized adapter coordinates for one isolated sample.
pub(super) struct AdapterFixture {
    pub(super) location: ControlStoreLocation,
    pub(super) identity: ControlStoreIdentity,
    pub(super) coordinate: AuthorityStoreCoordinate,
}

/// Compile-time assertion that adapter frames retain the journal byte class.
const _: AuthorityByteClass = AuthorityByteClass::JournalEntry;
