//! Causal tests for the closed persistent-schema projection.

use super::*;
use rusqlite::OptionalExtension;

#[test]
fn schema_identity_is_order_independent_and_data_independent() {
    let first = Connection::open_in_memory().unwrap();
    first
        .execute_batch(
            "CREATE TABLE alpha (id INTEGER PRIMARY KEY, value TEXT) STRICT; \
             CREATE UNIQUE INDEX alpha_value ON alpha(value); \
             CREATE TABLE beta (id INTEGER PRIMARY KEY) STRICT;",
        )
        .unwrap();
    let second = Connection::open_in_memory().unwrap();
    second
        .execute_batch(
            "CREATE TABLE beta (id INTEGER PRIMARY KEY) STRICT; \
             CREATE TABLE alpha (id INTEGER PRIMARY KEY, value TEXT) STRICT; \
             CREATE UNIQUE INDEX alpha_value ON alpha(value);",
        )
        .unwrap();

    let initial = resultant_schema_sha256(&first).unwrap();
    assert_eq!(initial, resultant_schema_sha256(&second).unwrap());
    first
        .execute("INSERT INTO alpha (value) VALUES ('payload')", [])
        .unwrap();
    assert_eq!(initial, resultant_schema_sha256(&first).unwrap());
}

#[test]
fn every_persistent_object_changes_schema_identity() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("CREATE TABLE item (id INTEGER PRIMARY KEY, value TEXT) STRICT;")
        .unwrap();
    let table_only = resultant_schema_sha256(&connection).unwrap();
    connection
        .execute_batch("CREATE INDEX item_value ON item(value);")
        .unwrap();
    let indexed = resultant_schema_sha256(&connection).unwrap();
    connection
        .execute_batch("CREATE VIEW item_view AS SELECT id FROM item;")
        .unwrap();
    let viewed = resultant_schema_sha256(&connection).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER item_guard BEFORE DELETE ON item BEGIN SELECT RAISE(ABORT, 'no'); END;",
        )
        .unwrap();
    let triggered = resultant_schema_sha256(&connection).unwrap();

    assert_ne!(table_only, indexed);
    assert_ne!(indexed, viewed);
    assert_ne!(viewed, triggered);
}

#[test]
fn implicit_autoindexes_remain_in_the_closed_projection_with_absent_sql() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE item (id INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE) STRICT;",
        )
        .unwrap();
    let implicit_sql: Option<Option<String>> = connection
        .query_row(
            "SELECT sql FROM main.sqlite_schema WHERE type = 'index' AND name LIKE 'sqlite_autoindex_%'",
            [],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert_eq!(implicit_sql, Some(None));
    assert_eq!(resultant_schema_sha256(&connection).unwrap().len(), 64);
}

#[test]
fn dropping_or_replacing_each_object_class_changes_schema_identity() {
    for (definition, removal, replacement) in [
        (
            "CREATE TABLE subject (id INTEGER PRIMARY KEY) STRICT;",
            "DROP TABLE subject;",
            "CREATE TABLE subject (id INTEGER PRIMARY KEY, value TEXT) STRICT;",
        ),
        (
            "CREATE INDEX subject_index ON anchor(value);",
            "DROP INDEX subject_index;",
            "CREATE INDEX subject_index ON anchor(value DESC);",
        ),
        (
            "CREATE VIEW subject_view AS SELECT id FROM anchor;",
            "DROP VIEW subject_view;",
            "CREATE VIEW subject_view AS SELECT value FROM anchor;",
        ),
        (
            "CREATE TRIGGER subject_trigger BEFORE DELETE ON anchor BEGIN SELECT RAISE(ABORT, 'first'); END;",
            "DROP TRIGGER subject_trigger;",
            "CREATE TRIGGER subject_trigger BEFORE DELETE ON anchor BEGIN SELECT RAISE(ABORT, 'second'); END;",
        ),
    ] {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE anchor (id INTEGER PRIMARY KEY, value TEXT) STRICT;")
            .unwrap();
        connection.execute_batch(definition).unwrap();
        let original = resultant_schema_sha256(&connection).unwrap();
        connection.execute_batch(removal).unwrap();
        let missing = resultant_schema_sha256(&connection).unwrap();
        connection.execute_batch(replacement).unwrap();
        let replaced = resultant_schema_sha256(&connection).unwrap();
        assert_ne!(original, missing);
        assert_ne!(original, replaced);
        assert_ne!(missing, replaced);
    }
}
