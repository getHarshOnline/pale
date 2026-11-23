//! Bounded-work text comparison using the exact SDK-observed before and after.

use pale_sdk::{LocalSourceGuidanceBaseline, LocalSourceGuidanceChange, LocalSourceGuidanceFile};

pub(super) fn append(output: &mut String, file: &LocalSourceGuidanceFile) {
    let after = match file.change() {
        LocalSourceGuidanceChange::Keep => return,
        LocalSourceGuidanceChange::Write { content, .. } => content.as_str(),
        LocalSourceGuidanceChange::Delete => "",
    };
    let before = file.previous_content().unwrap_or_default();
    output.push_str(&format!(
        "\n--- {} (current)\n+++ {} (proposed)\n",
        file.rel_path(),
        file.rel_path()
    ));
    if file.previous_content().is_none()
        && matches!(file.baseline(), LocalSourceGuidanceBaseline::Present { .. })
    {
        output.push_str(
            "Current file is not UTF-8; its byte identity is available in the JSON plan.\n",
        );
    }
    let old = before.split_inclusive('\n').collect::<Vec<_>>();
    let new = after.split_inclusive('\n').collect::<Vec<_>>();
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    if prefix == old.len() && prefix == new.len() {
        output.push_str("Text unchanged; file permissions change.\n");
        return;
    }
    let start = prefix.saturating_sub(3);
    let old_end = (old.len() - suffix + 3).min(old.len());
    let new_end = (new.len() - suffix + 3).min(new.len());
    output.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        if old.is_empty() { 0 } else { start + 1 },
        old_end - start,
        if new.is_empty() { 0 } else { start + 1 },
        new_end - start
    ));
    for line in &old[start..prefix] {
        line_out(output, ' ', line);
    }
    for line in &old[prefix..old.len() - suffix] {
        line_out(output, '-', line);
    }
    for line in &new[prefix..new.len() - suffix] {
        line_out(output, '+', line);
    }
    for line in &old[old.len() - suffix..old_end] {
        line_out(output, ' ', line);
    }
}

fn line_out(output: &mut String, prefix: char, line: &str) {
    output.push(prefix);
    for ch in line.chars() {
        if ch.is_control() && ch != '\n' && ch != '\t' {
            output.extend(ch.escape_default());
        } else {
            output.push(ch);
        }
    }
    if !line.ends_with('\n') {
        output.push_str("\n\\ No newline at end of file\n");
    }
}
