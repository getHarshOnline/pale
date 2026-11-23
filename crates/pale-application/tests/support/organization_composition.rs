//! Organizational fixtures used only by focused composition-law tests.

use pale_core::records::*;

use super::organization_fixtures::axes_owned;

/// Build independent authority axes.
pub(super) fn axes(name: &str) -> OrganizationalAuthorityAxesV1 {
    axes_owned(name, name)
}

/// Build the entirely optional empty organizational grammar.
pub(super) fn empty_organizational_translation() -> OrganizationalTranslationV1 {
    OrganizationalTranslationV1 {
        schema: ORGANIZATIONAL_TRANSLATION_V1_SCHEMA.into(),
        schema_version: ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_VERSION,
        scopes: Vec::new(),
        organizations: Vec::new(),
        groups: Vec::new(),
        discipline_trees: Vec::new(),
        roles: Vec::new(),
        occupant_bindings: Vec::new(),
        provider_bindings: Vec::new(),
        communication_actor_bindings: Vec::new(),
        leadership: Vec::new(),
        relations: Vec::new(),
        objects: Vec::new(),
        population_rules: Vec::new(),
        group_lenses: Vec::new(),
        services: Vec::new(),
        native_projections: Vec::new(),
        inspection: None,
    }
}
