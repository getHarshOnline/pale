//! Deterministic identity of the resultant persistent `SQLite` schema.

use pale_core::sha256_hex;
use rusqlite::Connection;
use serde::Serialize;

use crate::ControlSqliteError;

/// One closed row in the resultant persistent-schema projection.
///
/// `rootpage` is intentionally absent because page allocation is physical
/// placement, not schema meaning. `sql` remains optional so SQLite-owned
/// automatic indexes are represented instead of silently filtered out.
#[derive(Debug, PartialEq, Eq, Serialize)]
struct PersistentSchemaRow {
    /// `SQLite` object class (`table`, `index`, `view`, or `trigger`).
    #[serde(rename = "type")]
    object_type: String,
    /// Persistent object name.
    name: String,
    /// Table or view associated with the object.
    #[serde(rename = "tbl_name")]
    table_name: String,
    /// SQLite-normalized creation SQL, absent for implicit objects.
    sql: Option<String>,
}

/// Derive the SHA-256 identity of every persistent `main.sqlite_schema` row.
///
/// Rows are sorted in Rust by the byte tuples
/// `(type, name, tbl_name, sql)` and then encoded as canonical JCS. The exact
/// pinned `SQLite` build therefore owns SQL normalization, while this function
/// owns ordering and byte identity. Migration source remains the sole authored
/// schema; this projection is derived evidence rather than a second definition.
///
/// # Errors
///
/// Returns [`ControlSqliteError`] when `SQLite` cannot prepare or read the schema
/// projection, a row cannot be decoded, or canonical JCS encoding fails.
pub(crate) fn resultant_schema_sha256(
    connection: &Connection,
) -> Result<String, ControlSqliteError> {
    let mut statement = connection
        .prepare("SELECT type, name, tbl_name, sql FROM main.sqlite_schema")
        .map_err(|source| {
            ControlSqliteError::sqlite("prepare resultant-schema projection", source)
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(PersistentSchemaRow {
                object_type: row.get(0)?,
                name: row.get(1)?,
                table_name: row.get(2)?,
                sql: row.get(3)?,
            })
        })
        .map_err(|source| ControlSqliteError::sqlite("read resultant-schema projection", source))?;
    let mut rows = rows.collect::<Result<Vec<_>, _>>().map_err(|source| {
        ControlSqliteError::sqlite("decode resultant-schema projection", source)
    })?;
    rows.sort_by(|left, right| schema_sort_key(left).cmp(&schema_sort_key(right)));
    let canonical = serde_jcs::to_vec(&rows)
        .map_err(|_| ControlSqliteError::SourceInvariant("resultant-schema JCS encoding"))?;
    Ok(sha256_hex(&canonical))
}

/// Borrow the exact byte tuple used to order one schema row.
type SchemaSortKey<'row> = (&'row [u8], &'row [u8], &'row [u8], Option<&'row [u8]>);

/// Borrow the exact byte tuple used to order one schema row.
fn schema_sort_key(row: &PersistentSchemaRow) -> SchemaSortKey<'_> {
    (
        row.object_type.as_bytes(),
        row.name.as_bytes(),
        row.table_name.as_bytes(),
        row.sql.as_deref().map(str::as_bytes),
    )
}

#[cfg(test)]
#[path = "../tests/internal/schema.rs"]
mod tests;
