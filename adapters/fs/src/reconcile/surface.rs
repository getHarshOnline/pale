//! Typed ownership checks and byte-level staging for generated surfaces.
//!
//! Whole-file ownership authenticates every byte. Managed-block ownership
//! authenticates the governed block while preserving surrounding user content;
//! the later mutation precondition still snapshots the complete file so concurrent
//! edits are detected.

use std::borrow::Cow;
use std::collections::BTreeMap;

use pale_core::managed::{split_managed_block, ManagedBlockError};
use pale_core::records::{
    ArtifactDigest, BaselineEntry, BaselineStateV1, Operation, SurfaceAction, SurfaceDigest,
    SurfaceOwnership,
};

use super::{Mutation, Staged};
use crate::confine::{ConfinedLocation, ConfinedRoot, ReadLimit, MATERIALIZED_SURFACE_READ_LIMIT};
use crate::errors::FsError;

/// Existing UTF-8 surface admitted for ownership comparison.
struct Existing {
    content: String,
    sha256: ArtifactDigest,
    mode: u32,
}

/// Complete-file ceiling shared by reconciliation and rollback.
const OWNED_SURFACE_LIMIT: ReadLimit = MATERIALIZED_SURFACE_READ_LIMIT;

fn read_existing(location: &ConfinedLocation, rel_path: &str) -> Result<Option<Existing>, FsError> {
    let Some(file) = location.read_file(OWNED_SURFACE_LIMIT)? else {
        return Ok(None);
    };
    let content = String::from_utf8(file.bytes).map_err(|_| {
        FsError::Collision(rel_path.into(), "non-UTF8 file at an owned path".into())
    })?;
    Ok(Some(Existing {
        sha256: ArtifactDigest::hash_bytes(content.as_bytes()),
        content,
        mode: file.mode,
    }))
}

fn managed<'a>(content: &'a str, rel_path: &str) -> Result<&'a str, FsError> {
    split_managed_block(content)
        .map(|split| split.block())
        .map_err(|error| {
            FsError::Collision(
                rel_path.into(),
                format!("invalid managed block at an owned path: {error}"),
            )
        })
}

fn verify_prior<'a>(
    existing: &'a Existing,
    prior: &SurfaceDigest,
) -> Result<Option<pale_core::managed::ManagedBlock<'a>>, FsError> {
    let actual = match prior.ownership {
        SurfaceOwnership::WholeFile => existing.sha256.clone(),
        SurfaceOwnership::ManagedBlock => {
            ArtifactDigest::hash_bytes(managed(&existing.content, &prior.rel_path)?.as_bytes())
        }
    };
    if actual != prior.sha256 {
        return Err(FsError::Collision(
            prior.rel_path.clone(),
            "prior owned content was modified".into(),
        ));
    }
    match prior.ownership {
        SurfaceOwnership::WholeFile => Ok(None),
        SurfaceOwnership::ManagedBlock => {
            split_managed_block(&existing.content)
                .map(Some)
                .map_err(|error| {
                    FsError::Collision(
                        prior.rel_path.clone(),
                        format!("invalid managed block at an owned path: {error}"),
                    )
                })
        }
    }
}

fn desired_payload<'a>(content: &'a str, desired: &SurfaceDigest) -> Result<&'a str, FsError> {
    let owned = match desired.ownership {
        SurfaceOwnership::WholeFile => content,
        SurfaceOwnership::ManagedBlock => managed(content, &desired.rel_path)?,
    };
    if ArtifactDigest::hash_bytes(owned.as_bytes()) != desired.sha256 {
        return Err(FsError::StageMismatch(desired.rel_path.clone()));
    }
    Ok(owned)
}

/// Authenticate and stage one exact generated-surface transition.
pub(super) fn stage<'payload, T: AsRef<str>>(
    root: &ConfinedRoot,
    operation: &Operation,
    prior: Option<&SurfaceDigest>,
    desired: Option<&SurfaceDigest>,
    baseline: Option<&BaselineEntry>,
    contents: &'payload BTreeMap<String, T>,
) -> Result<Staged<'payload>, FsError> {
    let location = root.location(&operation.rel_path)?;
    location.validate_mode(operation.mode.get())?;
    let existing = read_existing(&location, &operation.rel_path)?;
    if operation.transition.action() == SurfaceAction::Adopt {
        let admitted = baseline.ok_or_else(|| {
            FsError::InvalidState(format!(
                "adoption has no admitted baseline at {}",
                operation.rel_path
            ))
        })?;
        let actual = existing.as_ref().ok_or_else(|| {
            FsError::Collision(
                operation.rel_path.clone(),
                "adoption target disappeared after baseline capture".into(),
            )
        })?;
        let baseline_matches = matches!(
            &admitted.observation,
            BaselineStateV1::Present { sha256, mode }
                if sha256 == &actual.sha256 && mode.get() == actual.mode
        );
        if !baseline_matches {
            return Err(FsError::Collision(
                operation.rel_path.clone(),
                "adoption target differs from the admitted baseline".into(),
            ));
        }
    }
    let desired_content = desired
        .map(|surface| {
            let content = contents
                .get(&operation.rel_path)
                .ok_or_else(|| FsError::MissingContent(operation.rel_path.clone()))?;
            desired_payload(content.as_ref(), surface).map(|owned| (content.as_ref(), owned))
        })
        .transpose()?;

    let (mutation, mode) = planned_mutation(
        operation,
        prior,
        desired,
        existing.as_ref(),
        desired_content,
    )?;
    let pre_sha256 = existing.as_ref().map(|entry| entry.sha256.clone());
    let post_sha256 = match &mutation {
        Mutation::Unchanged => pre_sha256.clone(),
        Mutation::Write(bytes) => Some(ArtifactDigest::hash_bytes(bytes)),
        Mutation::Delete => None,
    };
    Ok(Staged {
        location,
        rel_path: operation.rel_path.clone(),
        action: operation.transition.action(),
        mutation,
        pre_sha256,
        pre_mode: existing.as_ref().map(|entry| entry.mode),
        post_sha256,
        mode,
        record_receipt: true,
    })
}

fn planned_mutation<'payload>(
    operation: &Operation,
    prior: Option<&SurfaceDigest>,
    desired: Option<&SurfaceDigest>,
    existing: Option<&Existing>,
    desired_content: Option<(&'payload str, &'payload str)>,
) -> Result<(Mutation<'payload>, u32), FsError> {
    let (mutation, mode) = match (prior, desired, existing, desired_content) {
        (None, Some(next), None, Some((content, block))) => (
            Mutation::Write(match next.ownership {
                SurfaceOwnership::WholeFile => Cow::Borrowed(content.as_bytes()),
                SurfaceOwnership::ManagedBlock => Cow::Owned(block.as_bytes().to_vec()),
            }),
            operation.mode.get(),
        ),
        (None, Some(next), Some(existing), Some((content, block))) => match next.ownership {
            SurfaceOwnership::WholeFile => {
                if operation.transition.action() != SurfaceAction::Adopt {
                    return Err(FsError::Collision(
                        operation.rel_path.clone(),
                        "unowned file already exists at a whole-file target".into(),
                    ));
                }
                (
                    Mutation::Write(Cow::Borrowed(content.as_bytes())),
                    operation.mode.get(),
                )
            }
            SurfaceOwnership::ManagedBlock => {
                match split_managed_block(&existing.content) {
                    Err(ManagedBlockError::Missing) => {
                        if operation.transition.action() == SurfaceAction::Adopt {
                            return Err(FsError::Collision(
                                operation.rel_path.clone(),
                                "adoption target has no existing managed block".into(),
                            ));
                        }
                    }
                    Err(error) => {
                        return Err(FsError::Collision(
                            operation.rel_path.clone(),
                            format!("invalid pre-existing managed markers: {error}"),
                        ));
                    }
                    Ok(split) => {
                        if operation.transition.action() != SurfaceAction::Adopt {
                            return Err(FsError::Collision(
                                operation.rel_path.clone(),
                                "managed block exists without a matching ownership state".into(),
                            ));
                        }
                        let adopted = format!("{}{}{}", split.before(), block, split.after());
                        let mutation = if adopted.as_bytes() == existing.content.as_bytes()
                            && existing.mode == operation.mode.get()
                        {
                            Mutation::Unchanged
                        } else {
                            Mutation::Write(Cow::Owned(adopted.into_bytes()))
                        };
                        return Ok((mutation, operation.mode.get()));
                    }
                }
                (
                    Mutation::Write(Cow::Owned(
                        format!("{}{block}", existing.content).into_bytes(),
                    )),
                    operation.mode.get(),
                )
            }
        },
        (Some(_), _, None, _) => {
            return Err(FsError::Collision(
                operation.rel_path.clone(),
                "prior owned path is missing".into(),
            ));
        }
        (Some(previous), Some(next), Some(existing), Some((content, block))) => {
            let split = verify_prior(existing, previous)?;
            let bytes = match next.ownership {
                SurfaceOwnership::WholeFile => Cow::Borrowed(content.as_bytes()),
                SurfaceOwnership::ManagedBlock => {
                    let split = split.ok_or_else(|| {
                        FsError::InvalidState(format!(
                            "managed prior has no authenticated block at {}",
                            operation.rel_path
                        ))
                    })?;
                    Cow::Owned(format!("{}{}{}", split.before(), block, split.after()).into_bytes())
                }
            };
            if bytes == existing.content.as_bytes() && existing.mode == operation.mode.get() {
                (Mutation::Unchanged, operation.mode.get())
            } else {
                (Mutation::Write(bytes), operation.mode.get())
            }
        }
        (Some(previous), None, Some(existing), None) => {
            let split = verify_prior(existing, previous)?;
            match previous.ownership {
                SurfaceOwnership::WholeFile => (Mutation::Delete, operation.mode.get()),
                SurfaceOwnership::ManagedBlock => {
                    let split = split.ok_or_else(|| {
                        FsError::InvalidState(format!(
                            "managed prior has no authenticated block at {}",
                            operation.rel_path
                        ))
                    })?;
                    (
                        Mutation::Write(Cow::Owned(
                            format!("{}{}", split.before(), split.after()).into_bytes(),
                        )),
                        operation.mode.get(),
                    )
                }
            }
        }
        _ => {
            return Err(FsError::InvalidState(format!(
                "incomplete transition at {}",
                operation.rel_path
            )));
        }
    };
    Ok((mutation, mode))
}
