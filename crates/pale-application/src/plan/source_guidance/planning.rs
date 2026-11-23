//! Exact observation join and deterministic source-guidance proposals.

use std::collections::{BTreeMap, BTreeSet};

use super::*;

pub(super) fn required_paths(preparation: &SourceGuidancePreparation) -> Vec<String> {
    let mut paths = BTreeSet::from([preparation.entry_path().to_owned()]);
    paths.insert(excludes::PATH.to_owned());
    if let Some(state) = &preparation.state {
        paths.extend(state.files.iter().map(|file| file.path.clone()));
    }
    if let RequestedChange::Apply(projection) = &preparation.requested {
        paths.extend(
            projection
                .files()
                .iter()
                .map(|file| file.rel_path().to_owned()),
        );
    }
    paths.into_iter().collect()
}

pub(super) fn validate_observation(
    path: &str,
    root: &MachineLocalRootIdentity,
    observation: &MachineLocalFileObservation,
) -> Result<(), GuidancePlanError> {
    let relative = pale_core::paths::ConfinedRelativePath::parse(path)
        .ok_or(GuidancePlanError::Baseline("target path is not confined"))?;
    if &observation.root != root {
        return Err(GuidancePlanError::Baseline("native roots differ"));
    }
    match (&observation.prior, &observation.bytes) {
        (None, None) => {}
        (Some(prior), Some(bytes)) => {
            if bytes.len() > MAX_GUIDANCE_FILE_BYTES {
                return Err(GuidancePlanError::Limit("file bytes"));
            }
            if prior.sha256 != ArtifactDigest::hash_bytes(bytes)
                || !observation.missing_parent_directories.is_empty()
            {
                return Err(GuidancePlanError::Baseline(
                    "file bytes, metadata or parent absence disagree",
                ));
            }
        }
        _ => {
            return Err(GuidancePlanError::Baseline(
                "file presence and bytes disagree",
            ))
        }
    }
    let components = relative.components().collect::<Vec<_>>();
    let mut parents = Vec::new();
    let mut prefix = String::new();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        parents.push(prefix.clone());
    }
    let supplied = observation
        .retained_parent_directories
        .iter()
        .map(|entry| entry.rel_path.as_str())
        .chain(
            observation
                .missing_parent_directories
                .iter()
                .map(String::as_str),
        );
    if !supplied.eq(parents.iter().map(String::as_str)) {
        return Err(GuidancePlanError::Baseline(
            "parent partition is incomplete or unordered",
        ));
    }
    Ok(())
}

pub(super) fn finish(
    preparation: SourceGuidancePreparation,
    mut observations: BTreeMap<String, MachineLocalFileObservation>,
) -> Result<SourceGuidancePlan, GuidancePlanError> {
    if !observations.keys().eq(required_paths(&preparation).iter()) {
        return Err(GuidancePlanError::Baseline(
            "target observations do not match the requested inventory",
        ));
    }
    let state_path = ownership_path(preparation.host);
    observations.insert(state_path.clone(), preparation.state_observation.clone());
    if observations.len() > pale_ports::MAX_SOURCE_GUIDANCE_TRANSACTION_FILES {
        return Err(GuidancePlanError::Limit("transaction files"));
    }
    let mut bytes = 0_usize;
    let mut parents = BTreeMap::new();
    for (path, observation) in &observations {
        validate_observation(path, &preparation.root, observation)?;
        bytes = bytes
            .checked_add(observation.bytes.as_ref().map_or(0, Vec::len))
            .ok_or(GuidancePlanError::Limit("aggregate bytes"))?;
        if bytes > MAX_GUIDANCE_PLAN_BYTES {
            return Err(GuidancePlanError::Limit("aggregate bytes"));
        }
        for directory in &observation.retained_parent_directories {
            match parents.insert(directory.rel_path.clone(), Some(directory.clone())) {
                Some(prior) if prior.as_ref() != Some(directory) => {
                    return Err(GuidancePlanError::Baseline(
                        "ancestor identity differs across observations",
                    ))
                }
                _ => {}
            }
        }
        for directory in &observation.missing_parent_directories {
            if parents.insert(directory.clone(), None).flatten().is_some() {
                return Err(GuidancePlanError::Baseline(
                    "ancestor is both present and absent",
                ));
            }
        }
    }
    let entry_path = preparation.entry_path();
    let entry_observation = observations
        .get(entry_path)
        .ok_or(GuidancePlanError::Baseline(
            "entrypoint observation is missing",
        ))?;
    let (entry_desired, entry_ownership) = entrypoint::reconcile(&preparation, entry_observation)?;
    let mut desired = BTreeMap::new();
    if let RequestedChange::Apply(projection) = &preparation.requested {
        for file in projection.files() {
            if file.content().len() > MAX_GUIDANCE_FILE_BYTES {
                return Err(GuidancePlanError::Limit("proposed file bytes"));
            }
            bytes = bytes
                .checked_add(file.content().len())
                .ok_or(GuidancePlanError::Limit("aggregate bytes"))?;
            if bytes > MAX_GUIDANCE_PLAN_BYTES {
                return Err(GuidancePlanError::Limit("aggregate bytes"));
            }
            desired.insert(
                file.rel_path().to_owned(),
                (file.content().to_owned(), UnixFileMode::DEFAULT_FILE),
            );
        }
    }
    if let Some(state) = &preparation.state {
        for file in &state.files {
            let observation = observations
                .get(&file.path)
                .ok_or(GuidancePlanError::Baseline(
                    "owned target observation is missing",
                ))?;
            if observation
                .prior
                .as_ref()
                .is_some_and(|prior| prior.sha256 != file.sha256 || prior.mode != file.mode)
            {
                return Err(GuidancePlanError::Collision {
                    path: file.path.clone(),
                    reason: "owned file bytes or permissions changed",
                });
            }
        }
    }
    for path in desired.keys() {
        let was_owned = preparation
            .state
            .as_ref()
            .is_some_and(|state| state.files.iter().any(|file| &file.path == path));
        if !was_owned
            && observations
                .get(path)
                .is_some_and(|observation| observation.prior.is_some())
        {
            return Err(GuidancePlanError::Collision {
                path: path.clone(),
                reason: "target exists without source-guidance ownership",
            });
        }
    }
    if let Some((content, mode)) = entry_desired {
        bytes = bytes
            .checked_add(content.len())
            .ok_or(GuidancePlanError::Limit("aggregate bytes"))?;
        desired.insert(entry_path.to_owned(), (content, mode));
    }
    if let Some((content, mode)) = excludes::reconcile(
        &observations[excludes::PATH],
        matches!(preparation.requested, RequestedChange::Apply(_)),
    )? {
        bytes = bytes
            .checked_add(content.len())
            .ok_or(GuidancePlanError::Limit("aggregate bytes"))?;
        desired.insert(excludes::PATH.to_owned(), (content, mode));
    }
    let intent = match &preparation.requested {
        RequestedChange::Apply(projection) => {
            let entry = entry_ownership.ok_or(GuidancePlanError::Ownership(
                "proposed entrypoint ownership is absent",
            ))?;
            let state = ownership::Ownership::proposed(
                preparation.context.clone(),
                preparation.root.clone(),
                projection,
                entry,
            )?;
            let content =
                serde_json::to_string_pretty(&state).map_err(GuidancePlanError::Encode)? + "\n";
            if content.len() > MAX_OWNERSHIP_BYTES {
                return Err(GuidancePlanError::Limit("proposed ownership bytes"));
            }
            bytes = bytes
                .checked_add(content.len())
                .ok_or(GuidancePlanError::Limit("aggregate bytes"))?;
            let mode = UnixFileMode::new(0o600).map_err(|_| {
                GuidancePlanError::Baseline("private metadata mode cannot be represented")
            })?;
            desired.insert(state_path.clone(), (content, mode));
            GuidancePlanIntent::Apply
        }
        RequestedChange::Remove => GuidancePlanIntent::Remove,
    };
    if bytes > MAX_GUIDANCE_PLAN_BYTES {
        return Err(GuidancePlanError::Limit("aggregate bytes"));
    }
    let mut files = Vec::with_capacity(observations.len());
    for (path, observation) in observations {
        let change = match desired.remove(&path) {
            Some((content, mode)) => {
                if observation.prior.as_ref().is_some_and(|prior| {
                    prior.sha256 == ArtifactDigest::hash_bytes(content.as_bytes())
                        && prior.mode == mode
                }) {
                    GuidanceFileChange::Keep
                } else {
                    GuidanceFileChange::Write { content, mode }
                }
            }
            None if observation.prior.is_some() => GuidanceFileChange::Delete,
            None => GuidanceFileChange::Keep,
        };
        files.push(GuidancePlannedFile {
            rel_path: path,
            baseline: observation,
            change,
        });
    }
    if !desired.is_empty() {
        return Err(GuidancePlanError::Baseline(
            "proposed target was not observed",
        ));
    }
    let file_preimages = files.iter().map(|file| {
        let after = match &file.change {
            GuidanceFileChange::Keep => serde_json::json!({"action":"keep"}),
            GuidanceFileChange::Delete => serde_json::json!({"action":"delete"}),
            GuidanceFileChange::Write { content, mode } => serde_json::json!({"action":"write", "sha256":ArtifactDigest::hash_bytes(content.as_bytes()), "mode":mode}),
        };
        serde_json::json!({
            "path":file.rel_path, "before":file.baseline.prior,
            "parents":file.baseline.retained_parent_directories,
            "missing_parents":file.baseline.missing_parent_directories, "after":after,
        })
    }).collect::<Vec<_>>();
    let index_path = format!(
        ".pale/guidance/{}/index.md",
        preparation.host.adapter().as_str()
    );
    let mut ordered = files
        .iter()
        .filter(|file| !matches!(file.change(), GuidanceFileChange::Keep))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|file| {
        let path = file.rel_path();
        let priority = if path == excludes::PATH {
            0
        } else if path == state_path {
            5
        } else if intent == GuidancePlanIntent::Remove && path == entry_path {
            0
        } else if matches!(file.change(), GuidanceFileChange::Delete) {
            4
        } else if path == entry_path {
            3
        } else if path == index_path {
            2
        } else {
            1
        };
        (priority, path)
    });
    let publication_order = ordered
        .iter()
        .map(|file| file.rel_path().to_owned())
        .collect::<Vec<_>>();
    let preimage = serde_json::json!({
        "domain":"pale.local-source-guidance-plan.v1", "owner":preparation.context,
        "profile":preparation.profile_digest, "root":preparation.root, "intent":intent,
        "ownership_path":state_path, "files":file_preimages, "publication_order":publication_order,
    });
    let digest =
        ArtifactDigest::hash_bytes(pale_core::canon::to_canonical_string(&preimage)?.as_bytes());
    Ok(SourceGuidancePlan {
        owner_sha256: ownership::identity_digest("owner", &preparation.context)?,
        root: preparation.root,
        intent,
        digest,
        ownership_path: state_path,
        files,
        publication_order,
    })
}
