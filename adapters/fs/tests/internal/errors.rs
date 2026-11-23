//! Verifies that nested filesystem failures preserve the original publication effect boundary.

use super::{EffectVisibility, FsError};
use std::path::PathBuf;

fn rejected(detail: &str) -> FsError {
    FsError::Invariant {
        operation: "publish",
        path: PathBuf::from("record"),
        detail: detail.into(),
    }
}

fn uncertain(source: FsError) -> FsError {
    FsError::EffectUncertain {
        operation: "publish",
        path: PathBuf::from("record"),
        source: Box::new(source),
    }
}

fn cleanup_failed(source: FsError, cleanup: FsError) -> FsError {
    FsError::CleanupFailed {
        operation: "publish",
        path: PathBuf::from("record"),
        source: Box::new(source),
        cleanup: Box::new(cleanup),
    }
}

#[test]
fn visibility_follows_only_the_original_publication_chain() {
    assert_eq!(
        uncertain(rejected("post-publication failure")).effect_visibility(),
        EffectVisibility::MayBeVisible
    );

    let recursively_wrapped = cleanup_failed(
        cleanup_failed(
            uncertain(rejected("post-publication failure")),
            rejected("inner cleanup failure"),
        ),
        rejected("outer cleanup failure"),
    );
    assert_eq!(
        recursively_wrapped.effect_visibility(),
        EffectVisibility::MayBeVisible
    );

    let cleanup_only = cleanup_failed(
        rejected("rejected before publication"),
        uncertain(rejected("cleanup durability unknown")),
    );
    assert_eq!(
        cleanup_only.effect_visibility(),
        EffectVisibility::NoVisibleEffectReported
    );
}
