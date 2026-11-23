//! Lexical relative-path and logical-target grammar tests.

use pale_core::paths::{ConfinedRelativePath, PlainFilename, SurfaceTarget};

#[test]
fn relative_path_types_accept_only_their_declared_grammar() {
    for valid in ["README.md", ".pale/routes/context.json", "name..toml"] {
        assert!(ConfinedRelativePath::parse(valid).is_some(), "{valid}");
    }
    for invalid in [
        "",
        "/absolute",
        "../escape",
        "a/./b",
        "a//b",
        "a\\b",
        "line\nbreak",
        "{townRoot}/CONTEXT.md",
    ] {
        assert!(ConfinedRelativePath::parse(invalid).is_none(), "{invalid}");
    }
    assert_eq!(
        PlainFilename::parse("name..toml").unwrap().as_str(),
        "name..toml"
    );
    assert!(PlainFilename::parse("nested/name.toml").is_none());
}

#[test]
fn logical_targets_are_explicit_and_keep_a_confined_tail() {
    let target = SurfaceTarget::parse("{townRoot}/CONTEXT.md").unwrap();
    assert!(!target.is_filesystem());
    assert_eq!(target.logical_root(), Some("townRoot"));
    assert_eq!(target.relative_path().as_str(), "CONTEXT.md");
    assert!(SurfaceTarget::parse("{}/CONTEXT.md").is_none());
    assert!(SurfaceTarget::parse("{townRoot}/../escape").is_none());
    assert!(SurfaceTarget::parse("{town root}/CONTEXT.md").is_none());
    let filesystem = SurfaceTarget::parse(".pale/context.json").unwrap();
    assert!(filesystem.is_filesystem());
    assert_eq!(filesystem.logical_root(), None);
    assert_eq!(filesystem.relative_path().as_str(), ".pale/context.json");
}
