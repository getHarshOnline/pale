//! Managed-block byte-preservation and malformed-marker tests.

use pale_core::{
    managed::{split_managed_block, ManagedBlockError},
    MANAGED_BEGIN, MANAGED_END,
};

#[test]
fn caller_owned_marker_pairs_preserve_bytes_and_reject_ambiguous_boundaries() {
    use pale_core::managed::split_delimited_block;
    let split = split_delimited_block(
        "local\r\n<guide>method</guide>\nmore",
        "<guide>",
        "</guide>",
    )
    .unwrap();
    assert_eq!(split.before(), "local\r\n");
    assert_eq!(split.block(), "<guide>method</guide>");
    assert_eq!(split.after(), "\nmore");
    for (text, begin, end) in [
        ("x", "", "x"),
        ("x", "x", "x"),
        ("xy", "x", "xy"),
        ("<guide>x<guide>y</guide>", "<guide>", "</guide>"),
        ("</guide><guide>", "<guide>", "</guide>"),
    ] {
        assert!(split_delimited_block(text, begin, end).is_err());
    }
}

#[test]
fn splits_only_the_owned_block() {
    let content = format!("user before\n{MANAGED_BEGIN}\nowned\n{MANAGED_END}\nuser after\n");
    let split = split_managed_block(&content).unwrap();

    assert_eq!(split.before(), "user before\n");
    assert_eq!(
        split.block(),
        format!("{MANAGED_BEGIN}\nowned\n{MANAGED_END}")
    );
    assert_eq!(split.after(), "\nuser after\n");
}

#[test]
fn rejects_missing_duplicate_and_reversed_markers() {
    assert_eq!(
        split_managed_block("user text").unwrap_err(),
        ManagedBlockError::Missing
    );
    assert_eq!(
        split_managed_block(&format!("{MANAGED_BEGIN}{MANAGED_BEGIN}{MANAGED_END}")).unwrap_err(),
        ManagedBlockError::Malformed
    );
    assert_eq!(
        split_managed_block(&format!("{MANAGED_END}{MANAGED_BEGIN}")).unwrap_err(),
        ManagedBlockError::OutOfOrder
    );
}
