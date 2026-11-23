//! Exact source-guidance contribution, preserving the project's other bytes.

use super::{ownership::EntryOwnership, *};

type EntryReconciliation = (Option<(String, UnixFileMode)>, Option<EntryOwnership>);

fn markers(host: SourceGuidanceHost) -> (String, String) {
    let host = host.adapter().as_str();
    (
        format!("<!-- pale:source-guidance {host} begin -->"),
        format!("<!-- pale:source-guidance {host} end -->"),
    )
}

pub(super) fn validate_owned_block(
    host: SourceGuidanceHost,
    block: &str,
) -> Result<(), GuidancePlanError> {
    let (begin, end) = markers(host);
    let split = pale_core::managed::split_delimited_block(block, &begin, &end)
        .map_err(|_| GuidancePlanError::Ownership("entrypoint marker pair is invalid"))?;
    if !matches!(split.before(), "" | "\n")
        || split.after() != "\n"
        || block.matches("<!-- pale:source-guidance ").count() != 2
    {
        return Err(GuidancePlanError::Ownership(
            "entrypoint contribution has unexpected surrounding bytes",
        ));
    }
    Ok(())
}

pub(super) fn reconcile(
    preparation: &SourceGuidancePreparation,
    observation: &MachineLocalFileObservation,
) -> Result<EntryReconciliation, GuidancePlanError> {
    let path = preparation.entry_path();
    let collision = |reason| GuidancePlanError::Collision {
        path: path.to_owned(),
        reason,
    };
    let text = observation
        .bytes
        .as_deref()
        .map(std::str::from_utf8)
        .transpose()
        .map_err(|_| collision("project instruction file is not UTF-8"))?;
    let mode = observation
        .prior
        .as_ref()
        .map_or(UnixFileMode::DEFAULT_FILE, |file| file.mode);
    if text.is_some_and(|value| {
        value.contains(pale_core::MANAGED_BEGIN) || value.contains(pale_core::MANAGED_END)
    }) {
        return Err(collision(
            "existing Pale runtime block requires its own qualified handoff",
        ));
    }
    let prior = preparation.state.as_ref().map(|state| &state.entry);
    if let Some(text) = text {
        // Each host owns its contribution; the other host's well-formed block
        // remains part of the surrounding project text, never our authority.
        let mut marker_count = 0;
        for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
            let (begin, end) = markers(host);
            if text.contains(&begin) || text.contains(&end) {
                pale_core::managed::split_delimited_block(text, &begin, &end)
                    .map_err(|_| collision("entrypoint markers changed or were duplicated"))?;
                marker_count += 2;
                if host == preparation.host && prior.is_none() {
                    return Err(collision(
                        "source-guidance markers exist without their ownership record",
                    ));
                }
            }
        }
        if text.matches("<!-- pale:source-guidance ").count() != marker_count {
            return Err(collision(
                "unrecognised source-guidance markers in entrypoint",
            ));
        }
    }
    if let Some(prior) = prior {
        let text = text.ok_or_else(|| collision("owned entrypoint contribution is missing"))?;
        let (begin, end) = markers(preparation.host);
        pale_core::managed::split_delimited_block(text, &begin, &end)
            .map_err(|_| collision("entrypoint markers changed or were duplicated"))?;
        if text.matches(&prior.block).count() != 1 {
            return Err(collision("owned entrypoint contribution was edited"));
        }
        return match &preparation.requested {
            RequestedChange::Remove => {
                let (before, after) = text
                    .split_once(&prior.block)
                    .ok_or_else(|| collision("owned entrypoint contribution is missing"))?;
                // Preserve both native text chunks. Retain a line boundary if
                // removing our separator would glue later project additions
                // onto an original file that had no final newline.
                let joiner = if !before.is_empty()
                    && !after.is_empty()
                    && !before.ends_with('\n')
                    && !after.starts_with('\n')
                {
                    "\n"
                } else {
                    ""
                };
                let remainder = format!("{before}{joiner}{after}");
                let desired = if prior.created_file && remainder.is_empty() {
                    None
                } else {
                    Some((remainder, mode))
                };
                Ok((desired, None))
            }
            RequestedChange::Apply(projection) => {
                let separator = if prior.block.starts_with('\n') {
                    "\n"
                } else {
                    ""
                };
                let block = format!(
                    "{separator}{begin}\n{}{end}\n",
                    projection.entrypoint_contribution()
                );
                let desired = text.replacen(&prior.block, &block, 1);
                Ok((
                    Some((desired, mode)),
                    Some(EntryOwnership {
                        block,
                        created_file: prior.created_file,
                    }),
                ))
            }
        };
    }
    match &preparation.requested {
        RequestedChange::Remove => Ok((text.map(|text| (text.to_owned(), mode)), None)),
        RequestedChange::Apply(projection) => {
            let (begin, end) = markers(preparation.host);
            let separator = if text.is_some_and(|value| !value.is_empty() && !value.ends_with('\n'))
            {
                "\n"
            } else {
                ""
            };
            let block = format!(
                "{separator}{begin}\n{}{end}\n",
                projection.entrypoint_contribution()
            );
            let desired = format!("{}{block}", text.unwrap_or_default());
            Ok((
                Some((desired, mode)),
                Some(EntryOwnership {
                    block,
                    created_file: text.is_none(),
                }),
            ))
        }
    }
}
