//! A selected stack's own guidance, separate from its required topic reading.

use pale_core::records::PackageResourceKindV1;

use super::{
    semantic_helpers::{description, stack_name, yaml_string},
    RenderError,
};
use crate::semantic::{RepositorySemanticPlan, ResolvedStack};

pub(super) fn stack_skill(
    stack: &ResolvedStack,
    semantics: &RepositorySemanticPlan,
) -> Result<Option<String>, RenderError> {
    let Some(body) = &stack.body else {
        return Ok(None);
    };
    let body = super::body_links::skill_body(body, semantics)?;
    let document = crate::markdown::project(&body);
    let title = document.title(&body).unwrap_or(&stack.description);
    let content = &body[document.body_range()];
    let mut parents = String::new();
    for id in &stack.extends {
        let parent = semantics
            .stacks
            .iter()
            .find(|parent| parent.id == *id)
            .ok_or_else(|| {
                RenderError::invalid_input(
                    "stack prerequisites",
                    format!("stack `{}` requires missing stack `{id}`", stack.id),
                )
            })?;
        if parent.body.is_some() {
            let title = parent
                .description
                .replace('\\', "\\\\")
                .replace('[', "\\[")
                .replace(']', "\\]");
            parents.push_str(&format!("- [{title}](../{}/SKILL.md)\n", stack_name(id)));
        }
    }
    if !parents.is_empty() {
        parents = format!("## Parent stacks\n\n{parents}\n");
    }
    let official =
        super::semantic_helpers::official_source_guidance(&super::semantic_helpers::page_sources(
            &stack.id,
            PackageResourceKindV1::Stack,
            semantics,
            &stack.official_sources,
        ));
    let navigation = super::body_links::page_navigation(
        &stack.id,
        PackageResourceKindV1::Stack,
        semantics,
        "../",
    )?;
    Ok(Some(super::closing_questions::finish(
        &stack.id,
        PackageResourceKindV1::Stack,
        semantics,
        "../",
        format!(
            "---\nname: {}\ndescription: {}\n---\n\n# {}\n\n{parents}{navigation}{}\n\n{official}",
            stack_name(&stack.id),
            yaml_string(&description(&stack.description)),
            title,
            content.trim()
        ),
    )?))
}
