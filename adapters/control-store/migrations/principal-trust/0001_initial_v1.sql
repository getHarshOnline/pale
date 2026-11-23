PRAGMA application_id = 1346456660;
PRAGMA user_version = 1;

-- Derived physical accumulator custody, not a public record/schema family.
-- Store incarnation is the database header; domain epoch and exact profile
-- are explicit keys so incompatible histories never share mutable positions.
CREATE TABLE pale_principal_control_accumulator_position (
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

CREATE TABLE pale_principal_control_accumulator_frontier (
    checkpoint_domain_id TEXT NOT NULL CHECK (length(checkpoint_domain_id) BETWEEN 1 AND 512),
    domain_epoch INTEGER NOT NULL CHECK (domain_epoch > 0),
    checkpoint_profile_ref TEXT NOT NULL CHECK (length(checkpoint_profile_ref) BETWEEN 1 AND 512),
    checkpoint_profile_sha256 TEXT NOT NULL CHECK (length(checkpoint_profile_sha256)=64 AND checkpoint_profile_sha256 NOT GLOB '*[^0-9a-f]*'),
    control_checkpoint_ref TEXT NOT NULL,
    control_checkpoint_sha256 TEXT NOT NULL CHECK (length(control_checkpoint_sha256)=64 AND control_checkpoint_sha256 NOT GLOB '*[^0-9a-f]*'),
    state_root_sha256 TEXT NOT NULL CHECK (length(state_root_sha256)=64 AND state_root_sha256 NOT GLOB '*[^0-9a-f]*'),
    unresolved_root_sha256 TEXT NOT NULL CHECK (length(unresolved_root_sha256)=64 AND unresolved_root_sha256 NOT GLOB '*[^0-9a-f]*'),
    PRIMARY KEY (checkpoint_domain_id,domain_epoch),
    FOREIGN KEY (control_checkpoint_ref,control_checkpoint_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256)
) STRICT;

CREATE TABLE pale_control_store_header (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    store_kind TEXT NOT NULL CHECK (store_kind = 'principal_trust'),
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

CREATE TABLE pale_principal_trust_store_binding (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    principal_id TEXT NOT NULL UNIQUE CHECK (length(principal_id) BETWEEN 1 AND 512),
    FOREIGN KEY (singleton) REFERENCES pale_control_store_header(singleton)
) STRICT;

CREATE TABLE pale_principal_trust_record (
    record_ref TEXT PRIMARY KEY CHECK (length(record_ref) BETWEEN 1 AND 512),
    record_sha256 TEXT NOT NULL
        CHECK (length(record_sha256) = 64 AND record_sha256 NOT GLOB '*[^0-9a-f]*'),
    schema_id TEXT NOT NULL CHECK (length(schema_id) BETWEEN 1 AND 512),
    schema_sha256 TEXT NOT NULL
        CHECK (length(schema_sha256) = 64 AND schema_sha256 NOT GLOB '*[^0-9a-f]*'),
    principal_id TEXT NOT NULL CHECK (length(principal_id) BETWEEN 1 AND 512),
    canonical_record_jcs BLOB NOT NULL CHECK (length(canonical_record_jcs) > 0),
    inserted_transaction_sequence INTEGER NOT NULL
        CHECK (inserted_transaction_sequence BETWEEN 1 AND 9007199254740991),
    UNIQUE (record_ref, record_sha256),
    FOREIGN KEY (principal_id)
        REFERENCES pale_principal_trust_store_binding(principal_id)
) STRICT;

CREATE TABLE pale_principal_trust_checkpoint_head (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    principal_id TEXT NOT NULL UNIQUE CHECK (length(principal_id) BETWEEN 1 AND 512),
    checkpoint_epoch INTEGER NOT NULL
        CHECK (checkpoint_epoch BETWEEN 1 AND 9007199254740991),
    trust_checkpoint_ref TEXT NOT NULL,
    trust_checkpoint_sha256 TEXT NOT NULL
        CHECK (length(trust_checkpoint_sha256) = 64 AND trust_checkpoint_sha256 NOT GLOB '*[^0-9a-f]*'),
    UNIQUE (
        principal_id,
        checkpoint_epoch,
        trust_checkpoint_ref,
        trust_checkpoint_sha256
    ),
    FOREIGN KEY (principal_id)
        REFERENCES pale_principal_trust_store_binding(principal_id),
    FOREIGN KEY (trust_checkpoint_ref, trust_checkpoint_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256)
) STRICT;

-- Append-only replay and historical closure for deployment finalization.  The
-- authority coordinate scopes the idempotency key; every other column is exact
-- physical evidence, never a semantic or signer selection.
CREATE TABLE pale_principal_deployment_finalization (
    admin_authority_ref TEXT NOT NULL CHECK (length(admin_authority_ref) BETWEEN 1 AND 512),
    admin_authority_sha256 TEXT NOT NULL CHECK (length(admin_authority_sha256)=64 AND admin_authority_sha256 NOT GLOB '*[^0-9a-f]*'),
    idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 512),
    admin_command_id TEXT NOT NULL CHECK (length(admin_command_id) BETWEEN 1 AND 512),
    command_ref TEXT NOT NULL, command_sha256 TEXT NOT NULL CHECK (length(command_sha256)=64), command_body_sha256 TEXT NOT NULL CHECK (length(command_body_sha256)=64),
    receipt_id TEXT NOT NULL CHECK (length(receipt_id) BETWEEN 1 AND 512),
    receipt_ref TEXT NOT NULL, receipt_sha256 TEXT NOT NULL CHECK (length(receipt_sha256)=64), receipt_body_sha256 TEXT NOT NULL CHECK (length(receipt_body_sha256)=64),
    binding_ref TEXT NOT NULL, binding_sha256 TEXT NOT NULL CHECK (length(binding_sha256)=64), binding_body_sha256 TEXT NOT NULL CHECK (length(binding_body_sha256)=64),
    checkpoint_signer_ref TEXT NOT NULL, checkpoint_signer_sha256 TEXT NOT NULL CHECK (length(checkpoint_signer_sha256)=64), checkpoint_signer_body_sha256 TEXT NOT NULL CHECK (length(checkpoint_signer_body_sha256)=64),
    runtime_authority_ref TEXT NOT NULL, runtime_authority_sha256 TEXT NOT NULL CHECK (length(runtime_authority_sha256)=64), runtime_authority_body_sha256 TEXT NOT NULL CHECK (length(runtime_authority_body_sha256)=64),
    trust_checkpoint_ref TEXT NOT NULL, trust_checkpoint_sha256 TEXT NOT NULL CHECK (length(trust_checkpoint_sha256)=64), trust_checkpoint_body_sha256 TEXT NOT NULL CHECK (length(trust_checkpoint_body_sha256)=64),
    control_checkpoint_ref TEXT NOT NULL, control_checkpoint_sha256 TEXT NOT NULL CHECK (length(control_checkpoint_sha256)=64), control_checkpoint_body_sha256 TEXT NOT NULL CHECK (length(control_checkpoint_body_sha256)=64),
    transaction_sequence INTEGER NOT NULL CHECK (transaction_sequence BETWEEN 1 AND 9007199254740991),
    principal_id TEXT NOT NULL,
    predecessor_trust_epoch INTEGER NOT NULL, predecessor_trust_ref TEXT NOT NULL, predecessor_trust_sha256 TEXT NOT NULL CHECK (length(predecessor_trust_sha256)=64),
    successor_trust_epoch INTEGER NOT NULL, successor_trust_ref TEXT NOT NULL, successor_trust_sha256 TEXT NOT NULL CHECK (length(successor_trust_sha256)=64),
    control_domain_id TEXT NOT NULL, predecessor_control_epoch INTEGER NOT NULL, predecessor_control_sequence INTEGER NOT NULL,
    predecessor_control_ref TEXT NOT NULL, predecessor_control_sha256 TEXT NOT NULL CHECK (length(predecessor_control_sha256)=64),
    successor_control_epoch INTEGER NOT NULL, successor_control_sequence INTEGER NOT NULL,
    successor_control_ref TEXT NOT NULL, successor_control_sha256 TEXT NOT NULL CHECK (length(successor_control_sha256)=64),
    checkpoint_profile_ref TEXT NOT NULL, checkpoint_profile_sha256 TEXT NOT NULL CHECK (length(checkpoint_profile_sha256)=64),
    predecessor_state_root_sha256 TEXT NOT NULL CHECK (length(predecessor_state_root_sha256)=64),
    successor_state_root_sha256 TEXT NOT NULL CHECK (length(successor_state_root_sha256)=64),
    predecessor_unresolved_root_sha256 TEXT NOT NULL CHECK (length(predecessor_unresolved_root_sha256)=64),
    successor_unresolved_root_sha256 TEXT NOT NULL CHECK (length(successor_unresolved_root_sha256)=64),
    mutation_count INTEGER NOT NULL CHECK (mutation_count >= 0),
    candidate_identity_sha256 TEXT NOT NULL CHECK (length(candidate_identity_sha256)=64 AND candidate_identity_sha256 NOT GLOB '*[^0-9a-f]*'),
    PRIMARY KEY (admin_authority_ref,admin_authority_sha256,idempotency_key),
    FOREIGN KEY (admin_authority_ref,admin_authority_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256),
    FOREIGN KEY (command_ref,command_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256),
    FOREIGN KEY (receipt_ref,receipt_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256),
    FOREIGN KEY (binding_ref,binding_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256),
    FOREIGN KEY (checkpoint_signer_ref,checkpoint_signer_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256),
    FOREIGN KEY (runtime_authority_ref,runtime_authority_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256),
    FOREIGN KEY (trust_checkpoint_ref,trust_checkpoint_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256),
    FOREIGN KEY (control_checkpoint_ref,control_checkpoint_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256),
    FOREIGN KEY (predecessor_trust_ref,predecessor_trust_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256),
    FOREIGN KEY (predecessor_control_ref,predecessor_control_sha256) REFERENCES pale_principal_trust_record(record_ref,record_sha256)
) STRICT;

CREATE TABLE pale_principal_deployment_finalization_mutation (
    admin_authority_ref TEXT NOT NULL, admin_authority_sha256 TEXT NOT NULL, idempotency_key TEXT NOT NULL,
    tree_kind TEXT NOT NULL CHECK (tree_kind IN ('state','unresolved')),
    position_kind TEXT NOT NULL CHECK (position_kind IN ('branch','leaf')),
    position_key BLOB NOT NULL CHECK (length(position_key)=33),
    predecessor_node_bytes BLOB, successor_node_bytes BLOB,
    PRIMARY KEY (admin_authority_ref,admin_authority_sha256,idempotency_key,tree_kind,position_kind,position_key),
    CHECK (predecessor_node_bytes IS NOT NULL OR successor_node_bytes IS NOT NULL),
    CHECK (predecessor_node_bytes IS NULL OR (position_kind='leaf' AND length(predecessor_node_bytes)=32) OR (position_kind='branch' AND length(predecessor_node_bytes) IN (66,99,132))),
    CHECK (successor_node_bytes IS NULL OR (position_kind='leaf' AND length(successor_node_bytes)=32) OR (position_kind='branch' AND length(successor_node_bytes) IN (66,99,132))),
    FOREIGN KEY (admin_authority_ref,admin_authority_sha256,idempotency_key)
      REFERENCES pale_principal_deployment_finalization(admin_authority_ref,admin_authority_sha256,idempotency_key)
      DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TABLE pale_principal_producer_incarnation (
    allocation_ref TEXT PRIMARY KEY,
    allocation_sha256 TEXT NOT NULL
        CHECK (length(allocation_sha256) = 64 AND allocation_sha256 NOT GLOB '*[^0-9a-f]*'),
    origin_principal_id TEXT NOT NULL CHECK (length(origin_principal_id) BETWEEN 1 AND 512),
    producer_id TEXT NOT NULL CHECK (length(producer_id) BETWEEN 1 AND 512),
    producer_epoch TEXT NOT NULL CHECK (
        length(producer_epoch) BETWEEN 32 AND 512
        AND length(producer_epoch) % 2 = 0
        AND producer_epoch NOT GLOB '*[^0-9a-f]*'
    ),
    previous_allocation_ref TEXT,
    previous_allocation_sha256 TEXT CHECK (
        previous_allocation_sha256 IS NULL
        OR (length(previous_allocation_sha256) = 64
            AND previous_allocation_sha256 NOT GLOB '*[^0-9a-f]*')
    ),
    UNIQUE (allocation_ref, allocation_sha256),
    UNIQUE (origin_principal_id, producer_epoch),
    CHECK ((previous_allocation_ref IS NULL) = (previous_allocation_sha256 IS NULL)),
    FOREIGN KEY (origin_principal_id)
        REFERENCES pale_principal_trust_store_binding(principal_id),
    FOREIGN KEY (allocation_ref, allocation_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256),
    FOREIGN KEY (previous_allocation_ref, previous_allocation_sha256)
        REFERENCES pale_principal_producer_incarnation(allocation_ref, allocation_sha256)
) STRICT;

CREATE TABLE pale_principal_producer_incarnation_retirement (
    retirement_ref TEXT PRIMARY KEY,
    retirement_sha256 TEXT NOT NULL
        CHECK (length(retirement_sha256) = 64 AND retirement_sha256 NOT GLOB '*[^0-9a-f]*'),
    origin_principal_id TEXT NOT NULL CHECK (length(origin_principal_id) BETWEEN 1 AND 512),
    producer_id TEXT NOT NULL CHECK (length(producer_id) BETWEEN 1 AND 512),
    producer_epoch TEXT NOT NULL CHECK (
        length(producer_epoch) BETWEEN 32 AND 512
        AND length(producer_epoch) % 2 = 0
        AND producer_epoch NOT GLOB '*[^0-9a-f]*'
    ),
    allocation_ref TEXT NOT NULL,
    allocation_sha256 TEXT NOT NULL
        CHECK (length(allocation_sha256) = 64 AND allocation_sha256 NOT GLOB '*[^0-9a-f]*'),
    UNIQUE (retirement_ref, retirement_sha256),
    UNIQUE (allocation_ref, allocation_sha256),
    FOREIGN KEY (origin_principal_id)
        REFERENCES pale_principal_trust_store_binding(principal_id),
    FOREIGN KEY (retirement_ref, retirement_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256),
    FOREIGN KEY (allocation_ref, allocation_sha256)
        REFERENCES pale_principal_producer_incarnation(allocation_ref, allocation_sha256)
) STRICT;

CREATE TABLE pale_principal_producer_incarnation_head (
    origin_principal_id TEXT NOT NULL,
    producer_id TEXT NOT NULL,
    producer_epoch TEXT NOT NULL,
    producer_state TEXT NOT NULL CHECK (producer_state IN ('active', 'retired')),
    allocation_ref TEXT NOT NULL,
    allocation_sha256 TEXT NOT NULL
        CHECK (length(allocation_sha256) = 64 AND allocation_sha256 NOT GLOB '*[^0-9a-f]*'),
    retirement_ref TEXT,
    retirement_sha256 TEXT CHECK (
        retirement_sha256 IS NULL
        OR (length(retirement_sha256) = 64
            AND retirement_sha256 NOT GLOB '*[^0-9a-f]*')
    ),
    PRIMARY KEY (origin_principal_id, producer_id),
    CHECK (
        (producer_state = 'active' AND retirement_ref IS NULL AND retirement_sha256 IS NULL)
        OR
        (producer_state = 'retired' AND retirement_ref IS NOT NULL AND retirement_sha256 IS NOT NULL)
    ),
    FOREIGN KEY (origin_principal_id)
        REFERENCES pale_principal_trust_store_binding(principal_id),
    FOREIGN KEY (allocation_ref, allocation_sha256)
        REFERENCES pale_principal_producer_incarnation(allocation_ref, allocation_sha256),
    FOREIGN KEY (retirement_ref, retirement_sha256)
        REFERENCES pale_principal_producer_incarnation_retirement(retirement_ref, retirement_sha256)
) STRICT;

CREATE TABLE pale_principal_trust_bootstrap_head (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    bootstrap_id TEXT NOT NULL UNIQUE CHECK (length(bootstrap_id) BETWEEN 1 AND 512),
    principal_record_ref TEXT NOT NULL,
    principal_record_sha256 TEXT NOT NULL
        CHECK (length(principal_record_sha256) = 64 AND principal_record_sha256 NOT GLOB '*[^0-9a-f]*'),
    recovery_policy_ref TEXT NOT NULL,
    recovery_policy_sha256 TEXT NOT NULL
        CHECK (length(recovery_policy_sha256) = 64 AND recovery_policy_sha256 NOT GLOB '*[^0-9a-f]*'),
    initial_admin_authority_ref TEXT NOT NULL,
    initial_admin_authority_sha256 TEXT NOT NULL
        CHECK (length(initial_admin_authority_sha256) = 64 AND initial_admin_authority_sha256 NOT GLOB '*[^0-9a-f]*'),
    initial_producer_incarnation_authority_ref TEXT NOT NULL,
    initial_producer_incarnation_authority_sha256 TEXT NOT NULL
        CHECK (length(initial_producer_incarnation_authority_sha256) = 64 AND initial_producer_incarnation_authority_sha256 NOT GLOB '*[^0-9a-f]*'),
    initial_checkpoint_signing_authority_ref TEXT NOT NULL,
    initial_checkpoint_signing_authority_sha256 TEXT NOT NULL
        CHECK (length(initial_checkpoint_signing_authority_sha256) = 64 AND initial_checkpoint_signing_authority_sha256 NOT GLOB '*[^0-9a-f]*'),
    trust_checkpoint_ref TEXT NOT NULL,
    trust_checkpoint_sha256 TEXT NOT NULL
        CHECK (length(trust_checkpoint_sha256) = 64 AND trust_checkpoint_sha256 NOT GLOB '*[^0-9a-f]*'),
    bootstrap_receipt_ref TEXT NOT NULL,
    bootstrap_receipt_sha256 TEXT NOT NULL
        CHECK (length(bootstrap_receipt_sha256) = 64 AND bootstrap_receipt_sha256 NOT GLOB '*[^0-9a-f]*'),
    initial_control_checkpoint_count INTEGER NOT NULL
        CHECK (initial_control_checkpoint_count BETWEEN 1 AND 9007199254740991),
    logical_committed_at INTEGER NOT NULL
        CHECK (logical_committed_at BETWEEN 0 AND 9007199254740991),
    FOREIGN KEY (principal_record_ref, principal_record_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256),
    FOREIGN KEY (recovery_policy_ref, recovery_policy_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256),
    FOREIGN KEY (initial_admin_authority_ref, initial_admin_authority_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256),
    FOREIGN KEY (
        initial_producer_incarnation_authority_ref,
        initial_producer_incarnation_authority_sha256
    ) REFERENCES pale_principal_trust_record(record_ref, record_sha256),
    FOREIGN KEY (
        initial_checkpoint_signing_authority_ref,
        initial_checkpoint_signing_authority_sha256
    ) REFERENCES pale_principal_trust_record(record_ref, record_sha256),
    FOREIGN KEY (trust_checkpoint_ref, trust_checkpoint_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256),
    FOREIGN KEY (bootstrap_receipt_ref, bootstrap_receipt_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256)
) STRICT;

CREATE TABLE pale_principal_bootstrap_control_checkpoint (
    bootstrap_id TEXT NOT NULL,
    checkpoint_ordinal INTEGER NOT NULL CHECK (checkpoint_ordinal BETWEEN 1 AND 9007199254740991),
    checkpoint_domain_id TEXT NOT NULL CHECK (length(checkpoint_domain_id) BETWEEN 1 AND 512),
    domain_epoch INTEGER NOT NULL CHECK (domain_epoch = 1),
    domain_sequence INTEGER NOT NULL CHECK (domain_sequence = 1),
    control_checkpoint_ref TEXT NOT NULL,
    control_checkpoint_sha256 TEXT NOT NULL
        CHECK (length(control_checkpoint_sha256) = 64 AND control_checkpoint_sha256 NOT GLOB '*[^0-9a-f]*'),
    PRIMARY KEY (bootstrap_id, checkpoint_ordinal),
    UNIQUE (bootstrap_id, checkpoint_domain_id),
    UNIQUE (bootstrap_id, control_checkpoint_ref, control_checkpoint_sha256),
    FOREIGN KEY (bootstrap_id)
        REFERENCES pale_principal_trust_bootstrap_head(bootstrap_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (control_checkpoint_ref, control_checkpoint_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256)
) STRICT;

CREATE TABLE pale_principal_control_checkpoint_head (
    checkpoint_domain_id TEXT PRIMARY KEY CHECK (length(checkpoint_domain_id) BETWEEN 1 AND 512),
    domain_epoch INTEGER NOT NULL CHECK (domain_epoch BETWEEN 1 AND 9007199254740991),
    domain_sequence INTEGER NOT NULL CHECK (domain_sequence BETWEEN 1 AND 9007199254740991),
    control_checkpoint_ref TEXT NOT NULL,
    control_checkpoint_sha256 TEXT NOT NULL
        CHECK (length(control_checkpoint_sha256) = 64 AND control_checkpoint_sha256 NOT GLOB '*[^0-9a-f]*'),
    UNIQUE (
        checkpoint_domain_id,
        domain_epoch,
        domain_sequence,
        control_checkpoint_ref,
        control_checkpoint_sha256
    ),
    FOREIGN KEY (control_checkpoint_ref, control_checkpoint_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256)
) STRICT;

CREATE TABLE pale_principal_resident_writer_head (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    writer_fence_id TEXT NOT NULL CHECK (length(writer_fence_id) BETWEEN 1 AND 512),
    principal_id TEXT NOT NULL CHECK (length(principal_id) BETWEEN 1 AND 512),
    writer_generation INTEGER NOT NULL
        CHECK (writer_generation BETWEEN 1 AND 9007199254740991),
    writer_state TEXT NOT NULL CHECK (writer_state IN ('active', 'retired')),
    writer_fence_ref TEXT NOT NULL,
    writer_fence_sha256 TEXT NOT NULL
        CHECK (length(writer_fence_sha256) = 64 AND writer_fence_sha256 NOT GLOB '*[^0-9a-f]*'),
    checkpoint_domain_id TEXT NOT NULL,
    checkpoint_domain_epoch INTEGER NOT NULL
        CHECK (checkpoint_domain_epoch BETWEEN 1 AND 9007199254740991),
    checkpoint_domain_sequence INTEGER NOT NULL
        CHECK (checkpoint_domain_sequence BETWEEN 1 AND 9007199254740991),
    control_checkpoint_ref TEXT NOT NULL,
    control_checkpoint_sha256 TEXT NOT NULL
        CHECK (length(control_checkpoint_sha256) = 64 AND control_checkpoint_sha256 NOT GLOB '*[^0-9a-f]*'),
    FOREIGN KEY (principal_id)
        REFERENCES pale_principal_trust_store_binding(principal_id),
    FOREIGN KEY (writer_fence_ref, writer_fence_sha256)
        REFERENCES pale_principal_trust_record(record_ref, record_sha256),
    FOREIGN KEY (
        checkpoint_domain_id,
        checkpoint_domain_epoch,
        checkpoint_domain_sequence,
        control_checkpoint_ref,
        control_checkpoint_sha256
    ) REFERENCES pale_principal_control_checkpoint_head (
        checkpoint_domain_id,
        domain_epoch,
        domain_sequence,
        control_checkpoint_ref,
        control_checkpoint_sha256
    ) DEFERRABLE INITIALLY DEFERRED
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
