//! Keep a page's closing questions after generated host and tool instructions.

use std::collections::BTreeSet;

use pale_core::records::{PackageResourceKindV1, PackageResourceRefV1};

use crate::packages::PackageResourceUse;
use crate::semantic::RepositorySemanticPlan;

use super::RenderError;

/// Preserve authored questions, or use the one set supplied by required reading.
/// Optional reading never supplies a closing condition. Conflicting sets require
/// the page to state its own questions instead of choosing one by traversal order.
pub(super) fn finish(
    resource: &PackageResourceRefV1,
    kind: PackageResourceKindV1,
    semantics: &RepositorySemanticPlan,
    prefix: &str,
    mut text: String,
) -> Result<String, RenderError> {
    let Some(reading) = semantics
        .page_reading
        .get(&PackageResourceUse::new(resource.clone(), kind))
    else {
        return Ok(text);
    };
    if !reading.authored_page {
        return Ok(text);
    }
    let ranges = crate::markdown::closing_questions(&text);
    validate(resource, &ranges)?;
    if ranges.len() > 1 {
        return Err(conflict(resource));
    }
    let questions = if let Some(section) = ranges.into_iter().next() {
        let range = section.range;
        let questions = question_text(&text[range.clone()]);
        text.replace_range(range, "");
        Some(questions)
    } else {
        let mut questions = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for required in &reading.required {
            collect(required, semantics, prefix, &mut visited, &mut questions)?;
        }
        if questions.len() > 1 {
            return Err(conflict(resource));
        }
        questions.pop_first()
    };
    if let Some(questions) = questions {
        Ok(format!("{}\n\n{}\n", text.trim_end(), questions.trim()))
    } else {
        Ok(text)
    }
}

fn collect(
    resource: &PackageResourceRefV1,
    semantics: &RepositorySemanticPlan,
    prefix: &str,
    visited: &mut BTreeSet<PackageResourceRefV1>,
    questions: &mut BTreeSet<String>,
) -> Result<(), RenderError> {
    if !visited.insert(resource.clone()) {
        return Ok(());
    }
    let guide = semantics
        .doctrine
        .iter()
        .find(|guide| guide.record.id == *resource)
        .ok_or_else(|| {
            RenderError::invalid_input("closing questions", "required guide is missing")
        })?;
    let body = super::body_links::linked_body(&guide.body, semantics, prefix)?;
    let ranges = crate::markdown::closing_questions(&body);
    validate(resource, &ranges)?;
    if !ranges.is_empty() {
        for section in ranges {
            questions.insert(question_text(&body[section.range]));
        }
    } else if let Some(reading) = semantics.page_reading.get(&PackageResourceUse::new(
        resource.clone(),
        PackageResourceKindV1::Doctrine,
    )) {
        for required in &reading.required {
            collect(required, semantics, prefix, visited, questions)?;
        }
    }
    Ok(())
}

fn question_text(section: &str) -> String {
    let heading_end = section.find(['\r', '\n']).unwrap_or(section.len());
    format!("## Before handing off{}", &section[heading_end..])
        .trim()
        .to_owned()
}

fn validate(
    resource: &PackageResourceRefV1,
    sections: &[crate::markdown::ClosingQuestions],
) -> Result<(), RenderError> {
    if sections
        .iter()
        .any(|section| !(3..=4).contains(&section.count))
    {
        return Err(RenderError::invalid_input(
            "closing questions",
            format!("page `{resource}` needs three or four unchecked closing questions"),
        ));
    }
    Ok(())
}

fn conflict(resource: &PackageResourceRefV1) -> RenderError {
    RenderError::invalid_input(
        "closing questions",
        format!("page `{resource}` needs one authored closing checklist to resolve multiple sets"),
    )
}
