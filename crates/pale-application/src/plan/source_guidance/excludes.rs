//! Repository-local exclusions for guidance working files, preserving user rules.

use super::*;

pub(super) const PATH: &str = ".gitignore";
const BEGIN: &str = "# BEGIN Pale guidance working files";
const END: &str = "# END Pale guidance working files";
const RULES: &str = "/.pale-source-guidance.lock\n/.pale-source-guidance-recovery.json\n/.pale-source-guidance-review-codex.json\n/.pale-source-guidance-review-claude-code.json\n";

pub(super) fn reconcile(
    observation: &MachineLocalFileObservation,
    install: bool,
) -> Result<Option<(String, UnixFileMode)>, GuidancePlanError> {
    let previous = observation.bytes.as_deref().unwrap_or_default();
    let text = std::str::from_utf8(previous).map_err(|_| GuidancePlanError::Collision {
        path: PATH.into(),
        reason: "Git ignore rules are not UTF-8",
    })?;
    let block = format!("{BEGIN}\n{RULES}{END}\n");
    let content = if text.contains(BEGIN) || text.contains(END) {
        // An edited managed block needs a deliberate repair; never replace a
        // user's additions merely because they occur between our markers.
        if text.matches(BEGIN).count() != 1
            || text.matches(END).count() != 1
            || !text.contains(&block)
        {
            return Err(GuidancePlanError::Collision {
                path: PATH.into(),
                reason: "Pale's Git ignore block was edited; review it before planning again",
            });
        }
        text.to_owned()
    } else if install {
        format!(
            "{text}{}{block}",
            if text.is_empty() || text.ends_with('\n') {
                ""
            } else {
                "\n"
            }
        )
    } else if observation.prior.is_some() {
        text.to_owned()
    } else {
        return Ok(None);
    };
    // Keep these exclusions after removal: the persistent cooperating lock and
    // the last reviewed proposal remain useful to the next invocation.
    let mode = observation
        .prior
        .as_ref()
        .map_or(UnixFileMode::DEFAULT_FILE, |p| p.mode);
    Ok(Some((content, mode)))
}
