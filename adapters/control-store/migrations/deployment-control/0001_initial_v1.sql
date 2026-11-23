PRAGMA application_id = 1346456644;
PRAGMA user_version = 1;

-- Derived physical accumulator custody, not a public record/schema family.
-- Store incarnation is the database header; domain epoch and exact profile
-- are explicit keys so incompatible histories never share mutable positions.
CREATE TABLE pale_deployment_operation_accumulator_position (
    checkpoint_domain_id TEXT NOT NULL CHECK (length(checkpoint_domain_id) BETWEEN 1 AND 512),
    domain_epoch INTEGER NOT NULL CHECK (domain_epoch > 0),
    checkpoint_profile_ref TEXT NOT NULL CHECK (length(checkpoint_profile_ref) BETWEEN 1 AND 512),
    checkpoint_profile_sha256 TEXT NOT NULL CHECK (length(checkpoint_profile_sha256)=64 AND checkpoint_profile_sha256 NOT GLOB '*[^0-9a-f]*'),
    tree_kind TEXT NOT NULL CHECK (tree_kind IN ('state','unresolved')),
    position_kind TEXT NOT NULL CHECK (position_kind IN ('branch','leaf')),
    position_key BLOB NOT NULL CHECK (length(position_key)=33),
    node_bytes BLOB NOT NULL CHECK ((position_kind='leaf' AND length(node_bytes)=32) OR (position_kind='branch' AND length(node_bytes) IN (66,99,132))),
    PRIMARY KEY (checkpoint_domain_id,domain_epoch,checkpoint_profile_ref,checkpoint_profile_sha256,tree_kind,position_kind,position_key)
) STRICT;

CREATE TABLE pale_deployment_operation_accumulator_frontier (
    checkpoint_domain_id TEXT NOT NULL CHECK (length(checkpoint_domain_id) BETWEEN 1 AND 512),
    domain_epoch INTEGER NOT NULL CHECK (domain_epoch > 0),
    checkpoint_profile_ref TEXT NOT NULL CHECK (length(checkpoint_profile_ref) BETWEEN 1 AND 512),
    checkpoint_profile_sha256 TEXT NOT NULL CHECK (length(checkpoint_profile_sha256)=64 AND checkpoint_profile_sha256 NOT GLOB '*[^0-9a-f]*'),
    control_checkpoint_ref TEXT NOT NULL,
    control_checkpoint_sha256 TEXT NOT NULL CHECK (length(control_checkpoint_sha256)=64 AND control_checkpoint_sha256 NOT GLOB '*[^0-9a-f]*'),
    state_root_sha256 TEXT NOT NULL CHECK (length(state_root_sha256)=64 AND state_root_sha256 NOT GLOB '*[^0-9a-f]*'),
    unresolved_root_sha256 TEXT NOT NULL CHECK (length(unresolved_root_sha256)=64 AND unresolved_root_sha256 NOT GLOB '*[^0-9a-f]*'),
    PRIMARY KEY (checkpoint_domain_id,domain_epoch),
    FOREIGN KEY (control_checkpoint_ref,control_checkpoint_sha256) REFERENCES pale_deployment_control_record(record_ref,record_sha256)
) STRICT;

CREATE TABLE pale_control_store_header (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    store_kind TEXT NOT NULL CHECK (store_kind = 'deployment_control'),
    logical_store_id TEXT NOT NULL CHECK (length(logical_store_id) BETWEEN 1 AND 512),
    control_incarnation_id TEXT NOT NULL CHECK (length(control_incarnation_id) BETWEEN 1 AND 512),
    logical_transaction_sequence INTEGER NOT NULL
        CHECK (logical_transaction_sequence BETWEEN 1 AND 9007199254740991),
    store_schema_id TEXT NOT NULL,
    store_schema_sha256 TEXT NOT NULL
        CHECK (length(store_schema_sha256) = 64 AND store_schema_sha256 NOT GLOB '*[^0-9a-f]*'),
    store_schema_version INTEGER NOT NULL CHECK (store_schema_version = 1),
    storage_manifest_sha256 TEXT NOT NULL
        CHECK (length(storage_manifest_sha256) = 64 AND storage_manifest_sha256 NOT GLOB '*[^0-9a-f]*'),
    migration_head_sha256 TEXT NOT NULL
        CHECK (length(migration_head_sha256) = 64 AND migration_head_sha256 NOT GLOB '*[^0-9a-f]*')
) STRICT;

CREATE TABLE pale_deployment_control_store_binding (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    workspace_id TEXT NOT NULL CHECK (length(workspace_id) BETWEEN 1 AND 512),
    deployment_binding_ref TEXT NOT NULL
        CHECK (length(deployment_binding_ref) BETWEEN 1 AND 512),
    deployment_binding_sha256 TEXT NOT NULL
        CHECK (
            length(deployment_binding_sha256) = 64
            AND deployment_binding_sha256 NOT GLOB '*[^0-9a-f]*'
        )
) STRICT;

CREATE TABLE pale_control_store_migration_receipt (
    migration_ordinal INTEGER PRIMARY KEY CHECK (migration_ordinal > 0),
    migration_id TEXT NOT NULL UNIQUE,
    migration_sha256 TEXT NOT NULL UNIQUE
        CHECK (length(migration_sha256) = 64 AND migration_sha256 NOT GLOB '*[^0-9a-f]*')
) STRICT;

CREATE TABLE pale_control_store_profile_observation (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    sqlite_version TEXT NOT NULL,
    compile_options_sha256 TEXT NOT NULL
        CHECK (length(compile_options_sha256) = 64 AND compile_options_sha256 NOT GLOB '*[^0-9a-f]*'),
    journal_mode TEXT NOT NULL CHECK (journal_mode = 'wal'),
    foreign_keys INTEGER NOT NULL CHECK (foreign_keys = 1),
    trusted_schema INTEGER NOT NULL CHECK (trusted_schema = 0)
) STRICT;

CREATE TABLE pale_deployment_authority_head (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    journal_sequence INTEGER
        CHECK (journal_sequence IS NULL OR journal_sequence BETWEEN 1 AND 9007199254740991)
) STRICT;

INSERT INTO pale_deployment_authority_head (singleton, journal_sequence)
VALUES (1, NULL);

CREATE TABLE pale_deployment_authority_journal (
    journal_sequence INTEGER PRIMARY KEY
        CHECK (journal_sequence BETWEEN 1 AND 9007199254740991),
    physical_frame BLOB NOT NULL CHECK (length(physical_frame) > 0)
) STRICT;

-- Collection sequences belong to an operation handle, not the authority fold.
CREATE TABLE pale_deployment_collection_head (
    handle_ref TEXT NOT NULL CHECK (length(handle_ref) BETWEEN 1 AND 512),
    handle_sha256 TEXT NOT NULL CHECK (length(handle_sha256) = 64 AND handle_sha256 NOT GLOB '*[^0-9a-f]*'),
    journal_sequence INTEGER
        CHECK (journal_sequence IS NULL OR journal_sequence BETWEEN 1 AND 9007199254740991),
    PRIMARY KEY (handle_ref, handle_sha256)
) STRICT;

CREATE TABLE pale_deployment_collection_journal (
    handle_ref TEXT NOT NULL CHECK (length(handle_ref) BETWEEN 1 AND 512),
    handle_sha256 TEXT NOT NULL CHECK (length(handle_sha256) = 64 AND handle_sha256 NOT GLOB '*[^0-9a-f]*'),
    journal_sequence INTEGER NOT NULL CHECK (journal_sequence BETWEEN 1 AND 9007199254740991),
    physical_frame BLOB NOT NULL CHECK (length(physical_frame) > 0),
    PRIMARY KEY (handle_ref, handle_sha256, journal_sequence)
) STRICT;

CREATE TABLE pale_deployment_recovery_checkpoint (
    through_journal_sequence INTEGER PRIMARY KEY
        CHECK (through_journal_sequence BETWEEN 1 AND 9007199254740991),
    physical_frame BLOB NOT NULL CHECK (length(physical_frame) > 0)
) STRICT;

CREATE TABLE pale_deployment_session_lease_body (
    session_admission_lease_digest TEXT PRIMARY KEY
        CHECK (
            length(session_admission_lease_digest) = 64
            AND session_admission_lease_digest NOT GLOB '*[^0-9a-f]*'
        ),
    schema_id TEXT NOT NULL CHECK (length(schema_id) BETWEEN 1 AND 512),
    schema_sha256 TEXT NOT NULL
        CHECK (length(schema_sha256) = 64 AND schema_sha256 NOT GLOB '*[^0-9a-f]*'),
    canonical_wire_jcs BLOB NOT NULL CHECK (length(canonical_wire_jcs) > 0),
    private_admission_basis_sha256 TEXT
        CHECK (
            private_admission_basis_sha256 IS NULL
            OR (
                length(private_admission_basis_sha256) = 64
                AND private_admission_basis_sha256 NOT GLOB '*[^0-9a-f]*'
            )
        ),
    canonical_private_admission_basis_jcs BLOB
        CHECK (
            canonical_private_admission_basis_jcs IS NULL
            OR length(canonical_private_admission_basis_jcs) > 0
        ),
    CHECK (
        (private_admission_basis_sha256 IS NULL)
        = (canonical_private_admission_basis_jcs IS NULL)
    )
) STRICT;

CREATE TABLE pale_deployment_control_record (
    record_ref TEXT PRIMARY KEY CHECK (length(record_ref) BETWEEN 1 AND 512),
    inserted_transaction_sequence INTEGER NOT NULL
        CHECK (inserted_transaction_sequence BETWEEN 1 AND 9007199254740991),
    record_sha256 TEXT NOT NULL
        CHECK (length(record_sha256) = 64 AND record_sha256 NOT GLOB '*[^0-9a-f]*'),
    schema_id TEXT NOT NULL CHECK (length(schema_id) BETWEEN 1 AND 512),
    schema_sha256 TEXT NOT NULL
        CHECK (length(schema_sha256) = 64 AND schema_sha256 NOT GLOB '*[^0-9a-f]*'),
    canonical_record_jcs BLOB NOT NULL CHECK (length(canonical_record_jcs) > 0),
    UNIQUE (record_ref, record_sha256)
) STRICT;

-- Singleton append-only genesis closure. Ordered members retain the exact
-- historical checkpoints even after later operation frontiers advance.
CREATE TABLE pale_deployment_control_store_genesis (
    singleton INTEGER PRIMARY KEY CHECK (singleton=1),
    genesis_id TEXT NOT NULL UNIQUE CHECK (length(genesis_id) BETWEEN 1 AND 512),
    principal_id TEXT NOT NULL CHECK (length(principal_id) BETWEEN 1 AND 512),
    workspace_id TEXT NOT NULL CHECK (length(workspace_id) BETWEEN 1 AND 512),
    deployment_binding_ref TEXT NOT NULL, deployment_binding_sha256 TEXT NOT NULL CHECK (length(deployment_binding_sha256)=64),
    checkpoint_signer_ref TEXT NOT NULL, checkpoint_signer_sha256 TEXT NOT NULL CHECK (length(checkpoint_signer_sha256)=64),
    runtime_authority_ref TEXT NOT NULL, runtime_authority_sha256 TEXT NOT NULL CHECK (length(runtime_authority_sha256)=64),
    finalization_store_id TEXT NOT NULL CHECK (length(finalization_store_id) BETWEEN 1 AND 512),
    finalization_incarnation_id TEXT NOT NULL CHECK (length(finalization_incarnation_id) BETWEEN 1 AND 512),
    receipt_ref TEXT NOT NULL, receipt_sha256 TEXT NOT NULL CHECK (length(receipt_sha256)=64), receipt_body_sha256 TEXT NOT NULL CHECK (length(receipt_body_sha256)=64),
    transaction_sequence INTEGER NOT NULL CHECK (transaction_sequence BETWEEN 1 AND 9007199254740991),
    checkpoint_count INTEGER NOT NULL CHECK (checkpoint_count BETWEEN 1 AND 1024),
    candidate_identity_sha256 TEXT NOT NULL CHECK (length(candidate_identity_sha256)=64 AND candidate_identity_sha256 NOT GLOB '*[^0-9a-f]*'),
    FOREIGN KEY (receipt_ref,receipt_sha256) REFERENCES pale_deployment_control_record(record_ref,record_sha256)
) STRICT;

CREATE TABLE pale_deployment_control_store_genesis_checkpoint (
    genesis_id TEXT NOT NULL, checkpoint_ordinal INTEGER NOT NULL CHECK (checkpoint_ordinal BETWEEN 0 AND 1023),
    checkpoint_domain_id TEXT NOT NULL CHECK (length(checkpoint_domain_id) BETWEEN 1 AND 512),
    domain_epoch INTEGER NOT NULL, domain_sequence INTEGER NOT NULL,
    checkpoint_ref TEXT NOT NULL, checkpoint_sha256 TEXT NOT NULL CHECK (length(checkpoint_sha256)=64), checkpoint_body_sha256 TEXT NOT NULL CHECK (length(checkpoint_body_sha256)=64),
    checkpoint_profile_ref TEXT NOT NULL, checkpoint_profile_sha256 TEXT NOT NULL CHECK (length(checkpoint_profile_sha256)=64),
    state_root_sha256 TEXT NOT NULL CHECK (length(state_root_sha256)=64),
    unresolved_root_sha256 TEXT NOT NULL CHECK (length(unresolved_root_sha256)=64),
    mutation_count INTEGER NOT NULL CHECK (mutation_count >= 0),
    PRIMARY KEY (genesis_id,checkpoint_ordinal), UNIQUE (genesis_id,checkpoint_domain_id),
    FOREIGN KEY (checkpoint_ref,checkpoint_sha256) REFERENCES pale_deployment_control_record(record_ref,record_sha256),
    FOREIGN KEY (genesis_id) REFERENCES pale_deployment_control_store_genesis(genesis_id) DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TABLE pale_deployment_control_store_genesis_mutation (
    genesis_id TEXT NOT NULL, checkpoint_ordinal INTEGER NOT NULL,
    tree_kind TEXT NOT NULL CHECK (tree_kind IN ('state','unresolved')),
    position_kind TEXT NOT NULL CHECK (position_kind IN ('branch','leaf')),
    position_key BLOB NOT NULL CHECK (length(position_key)=33),
    successor_node_bytes BLOB,
    PRIMARY KEY (genesis_id,checkpoint_ordinal,tree_kind,position_kind,position_key),
    CHECK (successor_node_bytes IS NULL OR (position_kind='leaf' AND length(successor_node_bytes)=32) OR (position_kind='branch' AND length(successor_node_bytes) IN (66,99,132))),
    FOREIGN KEY (genesis_id,checkpoint_ordinal)
      REFERENCES pale_deployment_control_store_genesis_checkpoint(genesis_id,checkpoint_ordinal)
      DEFERRABLE INITIALLY DEFERRED
) STRICT;

-- Rebuildable discovery index; observation/checkpoint custody owns the evidence.
CREATE INDEX pale_deployment_control_record_source_order
    ON pale_deployment_control_record(schema_id, inserted_transaction_sequence, record_ref);

CREATE TABLE pale_deployment_event_index (
    index_role TEXT NOT NULL CHECK (
        index_role IN (
            'event_identity',
            'event_commit_witness',
            'transition_witness',
            'owner_source',
            'identity_conflict',
            'subscription',
            'inbox_delivery',
            'delivery_attempt',
            'delivery_stream',
            'acknowledgement',
            'delivery_cursor',
            'source_frontier',
            'interaction_occurrence',
            'genealogy_edge',
            'reconciliation_frontier',
            'pale_derived_outbox'
        )
    ),
    index_key TEXT NOT NULL
        CHECK (length(index_key) = 64 AND index_key NOT GLOB '*[^0-9a-f]*'),
    record_ref TEXT NOT NULL,
    record_sha256 TEXT NOT NULL
        CHECK (length(record_sha256) = 64 AND record_sha256 NOT GLOB '*[^0-9a-f]*'),
    PRIMARY KEY (index_role, index_key),
    FOREIGN KEY (record_ref, record_sha256)
        REFERENCES pale_deployment_control_record(record_ref, record_sha256)
) STRICT;

CREATE TABLE pale_deployment_event_inbox_order (
    subscription_key TEXT NOT NULL
        CHECK (length(subscription_key) = 64 AND subscription_key NOT GLOB '*[^0-9a-f]*'),
    journal_sequence INTEGER NOT NULL
        CHECK (journal_sequence BETWEEN 1 AND 9007199254740991),
    delivery_key TEXT NOT NULL
        CHECK (length(delivery_key) = 64 AND delivery_key NOT GLOB '*[^0-9a-f]*'),
    index_role TEXT NOT NULL DEFAULT 'inbox_delivery'
        CHECK (index_role = 'inbox_delivery'),
    PRIMARY KEY (subscription_key, journal_sequence, delivery_key),
    UNIQUE (delivery_key),
    FOREIGN KEY (index_role, delivery_key)
        REFERENCES pale_deployment_event_index(index_role, index_key)
) STRICT;

CREATE TABLE pale_deployment_event_inbox_version (
    subscription_key TEXT NOT NULL
        CHECK (length(subscription_key) = 64 AND subscription_key NOT GLOB '*[^0-9a-f]*'),
    initial_journal_sequence INTEGER NOT NULL
        CHECK (initial_journal_sequence BETWEEN 1 AND 9007199254740991),
    delivery_key TEXT NOT NULL
        CHECK (length(delivery_key) = 64 AND delivery_key NOT GLOB '*[^0-9a-f]*'),
    mutation_journal_sequence INTEGER NOT NULL
        CHECK (mutation_journal_sequence BETWEEN initial_journal_sequence AND 9007199254740991),
    record_ref TEXT NOT NULL,
    record_sha256 TEXT NOT NULL
        CHECK (length(record_sha256) = 64 AND record_sha256 NOT GLOB '*[^0-9a-f]*'),
    PRIMARY KEY (delivery_key, mutation_journal_sequence),
    FOREIGN KEY (subscription_key, initial_journal_sequence, delivery_key)
        REFERENCES pale_deployment_event_inbox_order(
            subscription_key,
            journal_sequence,
            delivery_key
        ),
    FOREIGN KEY (record_ref, record_sha256)
        REFERENCES pale_deployment_control_record(record_ref, record_sha256)
) STRICT;

CREATE TABLE pale_deployment_effect_case_index (
    index_role TEXT NOT NULL CHECK (
        index_role IN (
            'request_identity',
            'logical_action',
            'action_disposition',
            'active_claim',
            'active_custody',
            'action_intent',
            'case_result'
        )
    ),
    index_key TEXT NOT NULL
        CHECK (length(index_key) = 64 AND index_key NOT GLOB '*[^0-9a-f]*'),
    record_ref TEXT NOT NULL,
    record_sha256 TEXT NOT NULL
        CHECK (length(record_sha256) = 64 AND record_sha256 NOT GLOB '*[^0-9a-f]*'),
    PRIMARY KEY (index_role, index_key),
    FOREIGN KEY (record_ref, record_sha256)
        REFERENCES pale_deployment_control_record(record_ref, record_sha256)
) STRICT;

CREATE TABLE pale_deployment_effect_case_checkpoint_head (
    checkpoint_domain_id TEXT PRIMARY KEY CHECK (length(checkpoint_domain_id) BETWEEN 1 AND 512),
    domain_epoch INTEGER NOT NULL CHECK (domain_epoch BETWEEN 1 AND 9007199254740991),
    domain_sequence INTEGER NOT NULL CHECK (domain_sequence BETWEEN 1 AND 9007199254740991),
    control_checkpoint_ref TEXT NOT NULL,
    control_checkpoint_sha256 TEXT NOT NULL
        CHECK (
            length(control_checkpoint_sha256) = 64
            AND control_checkpoint_sha256 NOT GLOB '*[^0-9a-f]*'
        ),
    UNIQUE (
        checkpoint_domain_id,
        domain_epoch,
        domain_sequence,
        control_checkpoint_ref,
        control_checkpoint_sha256
    ),
    FOREIGN KEY (control_checkpoint_ref, control_checkpoint_sha256)
        REFERENCES pale_deployment_control_record(record_ref, record_sha256)
) STRICT;

CREATE TABLE pale_deployment_operation_index (
    index_role TEXT NOT NULL CHECK (
        index_role IN (
            'request_evidence',
            'request_for_handle',
            'start_observation',
            'observation_checkpoint',
            'request_operation',
            'operation_handle',
            'parent_child_membership',
            'collection_head',
            'latest_observation',
            'a2a_send_preparation',
            'a2a_send_preparation_checkpoint',
            'current_a2a_get_task',
            'a2a_get_task_checkpoint',
            'native_outcome',
            'a2a_task_binding',
            'latest_a2a_task_observation',
            'first_a2a_task_observation',
            'a2a_task_observation_checkpoint'
        )
    ),
    index_key TEXT NOT NULL
        CHECK (length(index_key) = 64 AND index_key NOT GLOB '*[^0-9a-f]*'),
    record_ref TEXT NOT NULL,
    record_sha256 TEXT NOT NULL
        CHECK (length(record_sha256) = 64 AND record_sha256 NOT GLOB '*[^0-9a-f]*'),
    PRIMARY KEY (index_role, index_key),
    FOREIGN KEY (record_ref, record_sha256)
        REFERENCES pale_deployment_control_record(record_ref, record_sha256)
) STRICT;

-- Binding discovery is a derived, one-to-many index, not an authenticated set.
CREATE TABLE pale_deployment_operation_a2a_binding (
    operation_key TEXT NOT NULL CHECK (length(operation_key) = 64 AND operation_key NOT GLOB '*[^0-9a-f]*'),
    binding_key TEXT NOT NULL CHECK (length(binding_key) = 64 AND binding_key NOT GLOB '*[^0-9a-f]*'),
    record_ref TEXT NOT NULL CHECK (length(record_ref) BETWEEN 1 AND 512),
    record_sha256 TEXT NOT NULL CHECK (length(record_sha256) = 64 AND record_sha256 NOT GLOB '*[^0-9a-f]*'),
    inserted_transaction_sequence INTEGER NOT NULL CHECK (inserted_transaction_sequence BETWEEN 1 AND 9007199254740991),
    PRIMARY KEY (operation_key, binding_key),
    UNIQUE (binding_key),
    UNIQUE (record_ref),
    FOREIGN KEY (record_ref, record_sha256) REFERENCES pale_deployment_control_record(record_ref, record_sha256)
) STRICT;
CREATE INDEX pale_deployment_operation_a2a_binding_order
    ON pale_deployment_operation_a2a_binding (operation_key, inserted_transaction_sequence, record_ref);
CREATE INDEX pale_deployment_operation_a2a_binding_transaction
    ON pale_deployment_operation_a2a_binding (inserted_transaction_sequence, record_ref);

-- Exact selected protocol evidence is not canonical JSON. Its coordinate and
-- optional SSE metadata are bound by the application-owned checkpoint leaf;
-- this private table preserves bytes and their atomic insertion transaction.
CREATE TABLE pale_deployment_operation_a2a_evidence (
    record_ref TEXT PRIMARY KEY CHECK (length(record_ref) BETWEEN 1 AND 512),
    record_sha256 TEXT NOT NULL
        CHECK (length(record_sha256) = 64 AND record_sha256 NOT GLOB '*[^0-9a-f]*'),
    evidence_bytes BLOB NOT NULL,
    sse_event_id TEXT,
    sse_event_type TEXT,
    inserted_transaction_sequence INTEGER NOT NULL
        CHECK (inserted_transaction_sequence BETWEEN 1 AND 9007199254740991),
    CHECK ((sse_event_id IS NULL) = (sse_event_type IS NULL))
) STRICT;

CREATE INDEX pale_deployment_operation_a2a_evidence_transaction
    ON pale_deployment_operation_a2a_evidence (inserted_transaction_sequence, record_ref);

CREATE TABLE pale_deployment_operation_checkpoint_head (
    checkpoint_domain_id TEXT PRIMARY KEY CHECK (length(checkpoint_domain_id) BETWEEN 1 AND 512),
    domain_epoch INTEGER NOT NULL CHECK (domain_epoch BETWEEN 1 AND 9007199254740991),
    domain_sequence INTEGER NOT NULL CHECK (domain_sequence BETWEEN 1 AND 9007199254740991),
    control_checkpoint_ref TEXT NOT NULL,
    control_checkpoint_sha256 TEXT NOT NULL
        CHECK (
            length(control_checkpoint_sha256) = 64
            AND control_checkpoint_sha256 NOT GLOB '*[^0-9a-f]*'
        ),
    UNIQUE (
        checkpoint_domain_id,
        domain_epoch,
        domain_sequence,
        control_checkpoint_ref,
        control_checkpoint_sha256
    ),
    FOREIGN KEY (control_checkpoint_ref, control_checkpoint_sha256)
        REFERENCES pale_deployment_control_record(record_ref, record_sha256)
) STRICT;

-- Operation-scoped private admission provenance. No independent record
-- reference, public schema or storage-owned session admission is introduced.
CREATE TABLE pale_deployment_operation_session_basis (
    handle_ref TEXT PRIMARY KEY CHECK (length(handle_ref) BETWEEN 1 AND 512),
    handle_sha256 TEXT NOT NULL
        CHECK (length(handle_sha256) = 64 AND handle_sha256 NOT GLOB '*[^0-9a-f]*'),
    request_ref TEXT NOT NULL CHECK (length(request_ref) BETWEEN 1 AND 512),
    request_sha256 TEXT NOT NULL
        CHECK (length(request_sha256) = 64 AND request_sha256 NOT GLOB '*[^0-9a-f]*'),
    session_lease_sha256 TEXT NOT NULL
        CHECK (length(session_lease_sha256) = 64 AND session_lease_sha256 NOT GLOB '*[^0-9a-f]*'),
    basis_sha256 TEXT NOT NULL
        CHECK (length(basis_sha256) = 64 AND basis_sha256 NOT GLOB '*[^0-9a-f]*'),
    canonical_basis_jcs BLOB NOT NULL CHECK (length(canonical_basis_jcs) BETWEEN 1 AND 65536),
    inserted_transaction_sequence INTEGER NOT NULL
        CHECK (inserted_transaction_sequence BETWEEN 1 AND 9007199254740991),
    CHECK (handle_ref <> request_ref),
    FOREIGN KEY (handle_ref, handle_sha256)
        REFERENCES pale_deployment_control_record(record_ref, record_sha256),
    FOREIGN KEY (request_ref, request_sha256)
        REFERENCES pale_deployment_control_record(record_ref, record_sha256)
) STRICT;

CREATE INDEX pale_deployment_operation_session_basis_transaction
    ON pale_deployment_operation_session_basis (inserted_transaction_sequence, handle_ref);
