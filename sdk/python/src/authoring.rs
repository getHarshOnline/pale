//! Python-native projection of compiler-owned authoring intelligence.
//!
//! Structural paths are explicit host values while inspection and navigation
//! results retain the closed [`pale_sdk`] algebras. This leaf translates those
//! values only; it neither parses TOML nor recreates grammar or linker policy.

use std::sync::Arc;

use pale_sdk::{
    AuthoringCorrection, AuthoringCorrectionKind, AuthoringFieldFact, AuthoringFieldPresence,
    AuthoringIdentity, AuthoringIdentityKind, AuthoringIngressClass, AuthoringInspection,
    AuthoringInspectionRequest, AuthoringInspectionState, AuthoringInstancePathSegment,
    AuthoringLiteral, AuthoringLiteralFact, AuthoringNavigation, AuthoringNavigationRequest,
    AuthoringNodeFact, AuthoringOccurrenceRole, AuthoringPathSegment, AuthoringSourceLocation,
    AuthoringValueKind, StandaloneAuthoringRole,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyInt};

use crate::error::diagnostic_error;
use crate::{PalePython, PyEffectiveProfileCompilation, PyPortableWorkspaceCompilation};

/// Admit an integer page limit without Python's Boolean or `__index__` coercions.
pub(crate) fn page_limit(value: &Bound<'_, PyAny>) -> PyResult<u16> {
    if value.is_instance_of::<PyBool>() || !value.is_instance_of::<PyInt>() {
        return Err(PyTypeError::new_err(
            "page limit must be an integer, not a Boolean",
        ));
    }
    value.extract()
}

/// Closed class of source correction proved by the compiler.
#[pyclass(
    name = "AuthoringCorrectionKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyAuthoringCorrectionKind {
    /// Narrow an import visibility ceiling to the selected export's ceiling.
    NarrowImportVisibility,
}

/// Copied compiler-proved source edit, not permission to write a document.
///
/// Consumers must match the exact expected source before applying a replacement.
#[pyclass(name = "AuthoringCorrection", frozen)]
pub struct PyAuthoringCorrection {
    inner: AuthoringCorrection,
}

impl PyAuthoringCorrection {
    pub(crate) fn from_api(value: &AuthoringCorrection) -> Self {
        Self {
            inner: value.clone(),
        }
    }
}

#[pymethods]
impl PyAuthoringCorrection {
    /// Compiler-proved correction class.
    #[getter]
    pub fn kind(&self) -> PyAuthoringCorrectionKind {
        match self.inner.kind() {
            AuthoringCorrectionKind::NarrowImportVisibility => {
                PyAuthoringCorrectionKind::NarrowImportVisibility
            }
        }
    }

    /// Logical label of the edited source.
    #[getter]
    pub fn source_label(&self) -> &str {
        self.inner.source_label()
    }

    /// Exact decoded-key and array-index path.
    #[getter]
    pub fn path(&self) -> Vec<PyAuthoringInstancePathSegment> {
        self.inner
            .path()
            .iter()
            .map(PyAuthoringInstancePathSegment::from_api)
            .collect()
    }

    /// Exact source spelling required before any edit.
    #[getter]
    pub fn expected_source(&self) -> &str {
        self.inner.expected_source()
    }

    /// Exact replacement spelling selected by the compiler.
    #[getter]
    pub fn replacement_source(&self) -> &str {
        self.inner.replacement_source()
    }

    /// Retained semantic subject proving this correction, not an edit capability.
    #[getter]
    pub fn proof_subject(&self) -> PyAuthoringIdentity {
        PyAuthoringIdentity {
            inner: self.inner.proof_subject().clone(),
        }
    }

    /// Identity of the exact rejected diagnostic.
    #[getter]
    pub fn diagnostic_identity(&self) -> &str {
        self.inner.diagnostic_identity().as_str()
    }
}

/// Source-custody class whose API-owned bounds apply before parsing.
#[pyclass(name = "AuthoringIngressClass", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyAuthoringIngressClass {
    /// Portable workspace-language documents.
    Workspace,
    /// Private selection, binding, and lock documents.
    Private,
    /// Immutable Package distribution members.
    Package,
}

impl From<PyAuthoringIngressClass> for AuthoringIngressClass {
    fn from(value: PyAuthoringIngressClass) -> Self {
        match value {
            PyAuthoringIngressClass::Workspace => Self::Workspace,
            PyAuthoringIngressClass::Private => Self::Private,
            PyAuthoringIngressClass::Package => Self::Package,
        }
    }
}

/// Finite compiler-owned capture limits for one source-custody class.
#[pyclass(name = "AuthoringIngressLimits", frozen)]
pub struct PyAuthoringIngressLimits {
    /// Maximum logical documents in the captured source family.
    #[pyo3(get)]
    max_documents: usize,
    /// Maximum bytes in one logical document.
    #[pyo3(get)]
    max_document_bytes: u64,
    /// Maximum aggregate bytes in the source family.
    #[pyo3(get)]
    max_total_bytes: u64,
}

/// Explicit standalone grammar selected by the Python caller.
#[pyclass(name = "StandaloneAuthoringRole", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyStandaloneAuthoringRole {
    /// Root portable workspace manifest.
    WorkspaceManifest,
    /// Reusable workspace module.
    WorkspaceModule,
    /// Workspace-language primitive declarations.
    WorkspaceLanguagePrimitives,
    /// Responsibility derivation declarations.
    ResponsibilityDerivation,
    /// Organization-to-responsibility translations.
    OrganizationalTranslation,
    /// Neutral experience-composition declarations.
    ExperienceComposition,
    /// Exact module-resolution lock.
    WorkspaceResolutionLock,
    /// Exact workspace-universe lock.
    WorkspaceUniverseLock,
    /// Private capability policy and bindings.
    Capabilities,
    /// Private machine realization and authority bindings.
    MachineBinding,
    /// Exact admitted Package distribution lock.
    DistributionLock,
    /// Package manifest and resource inventory.
    PackageManifest,
    /// Package doctrine declarations.
    PackageDoctrine,
    /// Package operation declarations.
    PackageOperations,
    /// Package role declarations.
    PackageRoles,
    /// Package stack declarations.
    PackageStacks,
    /// Package run-mode declarations.
    PackageRunModes,
    /// Package policy-family declaration.
    PackagePolicyFamily,
    /// Package governing-order declaration.
    PackageGoverningOrder,
    /// Package capability-policy declaration.
    PackageCapabilityPolicy,
    /// Package tool-recipe declaration.
    PackageToolRecipe,
}

impl From<PyStandaloneAuthoringRole> for StandaloneAuthoringRole {
    fn from(value: PyStandaloneAuthoringRole) -> Self {
        match value {
            PyStandaloneAuthoringRole::WorkspaceManifest => Self::WorkspaceManifest,
            PyStandaloneAuthoringRole::WorkspaceModule => Self::WorkspaceModule,
            PyStandaloneAuthoringRole::WorkspaceLanguagePrimitives => {
                Self::WorkspaceLanguagePrimitives
            }
            PyStandaloneAuthoringRole::ResponsibilityDerivation => Self::ResponsibilityDerivation,
            PyStandaloneAuthoringRole::OrganizationalTranslation => Self::OrganizationalTranslation,
            PyStandaloneAuthoringRole::ExperienceComposition => Self::ExperienceComposition,
            PyStandaloneAuthoringRole::WorkspaceResolutionLock => Self::WorkspaceResolutionLock,
            PyStandaloneAuthoringRole::WorkspaceUniverseLock => Self::WorkspaceUniverseLock,
            PyStandaloneAuthoringRole::Capabilities => Self::Capabilities,
            PyStandaloneAuthoringRole::MachineBinding => Self::MachineBinding,
            PyStandaloneAuthoringRole::DistributionLock => Self::DistributionLock,
            PyStandaloneAuthoringRole::PackageManifest => Self::PackageManifest,
            PyStandaloneAuthoringRole::PackageDoctrine => Self::PackageDoctrine,
            PyStandaloneAuthoringRole::PackageOperations => Self::PackageOperations,
            PyStandaloneAuthoringRole::PackageRoles => Self::PackageRoles,
            PyStandaloneAuthoringRole::PackageStacks => Self::PackageStacks,
            PyStandaloneAuthoringRole::PackageRunModes => Self::PackageRunModes,
            PyStandaloneAuthoringRole::PackagePolicyFamily => Self::PackagePolicyFamily,
            PyStandaloneAuthoringRole::PackageGoverningOrder => Self::PackageGoverningOrder,
            PyStandaloneAuthoringRole::PackageCapabilityPolicy => Self::PackageCapabilityPolicy,
            PyStandaloneAuthoringRole::PackageToolRecipe => Self::PackageToolRecipe,
        }
    }
}

/// One structural grammar step supplied by a tolerant Python syntax layer.
#[pyclass(name = "AuthoringPathSegment", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyAuthoringPathSegment {
    inner: AuthoringPathSegment,
}

#[pymethods]
impl PyAuthoringPathSegment {
    /// Select one exact TOML key.
    #[staticmethod]
    pub fn field(name: String) -> Self {
        Self {
            inner: AuthoringPathSegment::Field(name),
        }
    }

    /// Select an arbitrary array or array-of-tables member.
    #[staticmethod]
    pub fn array_item() -> Self {
        Self {
            inner: AuthoringPathSegment::ArrayItem,
        }
    }
}

impl PyAuthoringPathSegment {
    fn into_api(self) -> AuthoringPathSegment {
        self.inner
    }
}

/// One exact decoded path step used for semantic source navigation.
#[pyclass(name = "AuthoringInstancePathSegment", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyAuthoringInstancePathSegment {
    inner: AuthoringInstancePathSegment,
}

#[pymethods]
impl PyAuthoringInstancePathSegment {
    /// Select one exact decoded key.
    #[staticmethod]
    pub fn field(name: String) -> Self {
        Self {
            inner: AuthoringInstancePathSegment::Field(name),
        }
    }

    /// Select one exact decoded array member.
    #[staticmethod]
    pub fn index(index: usize) -> Self {
        Self {
            inner: AuthoringInstancePathSegment::Index(index),
        }
    }

    /// Return `field` or `index` without exposing the Rust enum.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match self.inner {
            AuthoringInstancePathSegment::Field(_) => "field",
            AuthoringInstancePathSegment::Index(_) => "index",
        }
    }

    /// Exact field value when [`Self::kind`] is `field`.
    #[getter]
    pub fn field_value(&self) -> Option<&str> {
        match &self.inner {
            AuthoringInstancePathSegment::Field(value) => Some(value),
            AuthoringInstancePathSegment::Index(_) => None,
        }
    }

    /// Exact array index when [`Self::kind`] is `index`.
    #[getter]
    pub fn index_value(&self) -> Option<usize> {
        match self.inner {
            AuthoringInstancePathSegment::Index(value) => Some(value),
            AuthoringInstancePathSegment::Field(_) => None,
        }
    }
}

impl PyAuthoringInstancePathSegment {
    fn into_api(self) -> AuthoringInstancePathSegment {
        self.inner
    }

    fn from_api(inner: &AuthoringInstancePathSegment) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

/// Availability of structural assistance at one admitted coordinate.
#[pyclass(
    name = "AuthoringInspectionState",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyAuthoringInspectionState {
    /// The selected grammar contains the path.
    Available,
    /// The admitted document is substantive text rather than TOML.
    TextDocument,
    /// The compilation does not own the source label.
    UnknownDocument,
    /// The document is known but the grammar does not contain the path.
    UnknownPath,
}

impl From<AuthoringInspectionState> for PyAuthoringInspectionState {
    fn from(value: AuthoringInspectionState) -> Self {
        match value {
            AuthoringInspectionState::Available => Self::Available,
            AuthoringInspectionState::TextDocument => Self::TextDocument,
            AuthoringInspectionState::UnknownDocument => Self::UnknownDocument,
            AuthoringInspectionState::UnknownPath => Self::UnknownPath,
        }
    }
}

/// Structural value category admitted by the compiler-owned grammar.
#[pyclass(name = "AuthoringValueKind", eq, eq_int, frozen, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyAuthoringValueKind {
    /// Boolean value.
    Boolean,
    /// Signed integer value.
    Integer,
    /// UTF-8 string value.
    String,
    /// Ordered array value.
    Array,
    /// TOML table or inline-table value.
    Table,
}

impl From<AuthoringValueKind> for PyAuthoringValueKind {
    fn from(value: AuthoringValueKind) -> Self {
        match value {
            AuthoringValueKind::Boolean => Self::Boolean,
            AuthoringValueKind::Integer => Self::Integer,
            AuthoringValueKind::String => Self::String,
            AuthoringValueKind::Array => Self::Array,
            AuthoringValueKind::Table => Self::Table,
        }
    }
}

/// Requiredness of one field across structural variants.
#[pyclass(
    name = "AuthoringFieldPresence",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyAuthoringFieldPresence {
    /// Every applicable variant requires the field.
    Required,
    /// No applicable variant requires the field.
    Optional,
    /// Requiredness depends on the selected structural variant.
    VariantDependent,
}

impl From<AuthoringFieldPresence> for PyAuthoringFieldPresence {
    fn from(value: AuthoringFieldPresence) -> Self {
        match value {
            AuthoringFieldPresence::Required => Self::Required,
            AuthoringFieldPresence::Optional => Self::Optional,
            AuthoringFieldPresence::VariantDependent => Self::VariantDependent,
        }
    }
}

/// Closed TOML-compatible literal family.
#[pyclass(name = "AuthoringLiteralKind", eq, eq_int, frozen, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyAuthoringLiteralKind {
    /// UTF-8 string member.
    String,
    /// Signed integer member.
    Integer,
    /// Boolean member.
    Boolean,
}

/// One closed literal retained by generated authoring metadata.
#[pyclass(name = "AuthoringLiteral", frozen)]
pub struct PyAuthoringLiteral {
    inner: AuthoringLiteral,
}

#[pymethods]
impl PyAuthoringLiteral {
    /// Return the non-interchangeable literal family.
    #[getter]
    pub fn kind(&self) -> PyAuthoringLiteralKind {
        match self.inner {
            AuthoringLiteral::String(_) => PyAuthoringLiteralKind::String,
            AuthoringLiteral::Integer(_) => PyAuthoringLiteralKind::Integer,
            AuthoringLiteral::Boolean(_) => PyAuthoringLiteralKind::Boolean,
        }
    }

    /// Exact string member, when selected.
    #[getter]
    pub fn string_value(&self) -> Option<&str> {
        match &self.inner {
            AuthoringLiteral::String(value) => Some(value),
            AuthoringLiteral::Integer(_) | AuthoringLiteral::Boolean(_) => None,
        }
    }

    /// Exact integer member, when selected.
    #[getter]
    pub fn integer_value(&self) -> Option<i64> {
        match self.inner {
            AuthoringLiteral::Integer(value) => Some(value),
            AuthoringLiteral::String(_) | AuthoringLiteral::Boolean(_) => None,
        }
    }

    /// Exact Boolean member, when selected.
    #[getter]
    pub fn boolean_value(&self) -> Option<bool> {
        match self.inner {
            AuthoringLiteral::Boolean(value) => Some(value),
            AuthoringLiteral::String(_) | AuthoringLiteral::Integer(_) => None,
        }
    }
}

/// One allowed literal and its producer-owned description.
#[pyclass(name = "AuthoringLiteralFact", frozen)]
pub struct PyAuthoringLiteralFact {
    inner: AuthoringLiteralFact,
}

#[pymethods]
impl PyAuthoringLiteralFact {
    /// Return the exact allowed literal.
    #[getter]
    pub fn value(&self) -> PyAuthoringLiteral {
        PyAuthoringLiteral {
            inner: self.inner.value().clone(),
        }
    }

    /// Return variant-specific producer documentation when present.
    #[getter]
    pub fn description(&self) -> Option<&str> {
        self.inner.description()
    }
}

/// One child field offered at the selected structural path.
#[pyclass(name = "AuthoringFieldFact", frozen)]
pub struct PyAuthoringFieldFact {
    inner: AuthoringFieldFact,
}

#[pymethods]
impl PyAuthoringFieldFact {
    /// Exact authored key spelling.
    #[getter]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Producer documentation when present.
    #[getter]
    pub fn description(&self) -> Option<&str> {
        self.inner.description()
    }

    /// Requiredness across applicable structural variants.
    #[getter]
    pub fn presence(&self) -> PyAuthoringFieldPresence {
        self.inner.presence().into()
    }

    /// Closed structural categories admitted for this field.
    #[getter]
    pub fn value_kinds(&self) -> Vec<PyAuthoringValueKind> {
        self.inner
            .value_kinds()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    /// Closed literal vocabulary admitted for this field.
    #[getter]
    pub fn allowed_values(&self) -> Vec<PyAuthoringLiteralFact> {
        self.inner
            .allowed_values()
            .iter()
            .cloned()
            .map(|inner| PyAuthoringLiteralFact { inner })
            .collect()
    }
}

/// Compiler-owned structural facts for one editor position.
#[pyclass(name = "AuthoringNodeFact", frozen)]
pub struct PyAuthoringNodeFact {
    inner: AuthoringNodeFact,
}

#[pymethods]
impl PyAuthoringNodeFact {
    /// Producer documentation for the selected structural context.
    #[getter]
    pub fn description(&self) -> Option<&str> {
        self.inner.description()
    }

    /// Structural categories admitted at this path.
    #[getter]
    pub fn value_kinds(&self) -> Vec<PyAuthoringValueKind> {
        self.inner
            .value_kinds()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    /// Closed literals admitted at this path.
    #[getter]
    pub fn allowed_values(&self) -> Vec<PyAuthoringLiteralFact> {
        self.inner
            .allowed_values()
            .iter()
            .cloned()
            .map(|inner| PyAuthoringLiteralFact { inner })
            .collect()
    }

    /// Deterministically ordered child fields.
    #[getter]
    pub fn fields(&self) -> Vec<PyAuthoringFieldFact> {
        self.inner
            .fields()
            .iter()
            .cloned()
            .map(|inner| PyAuthoringFieldFact { inner })
            .collect()
    }

    /// Generated string format when present.
    #[getter]
    pub fn format(&self) -> Option<&str> {
        self.inner.format()
    }

    /// Generated string pattern when present.
    #[getter]
    pub fn pattern(&self) -> Option<&str> {
        self.inner.pattern()
    }

    /// Inclusive integer lower bound.
    #[getter]
    pub fn minimum(&self) -> Option<i64> {
        self.inner.minimum()
    }

    /// Inclusive integer upper bound.
    #[getter]
    pub fn maximum(&self) -> Option<i64> {
        self.inner.maximum()
    }

    /// Inclusive string-length lower bound.
    #[getter]
    pub fn min_length(&self) -> Option<u64> {
        self.inner.min_length()
    }

    /// Inclusive string-length upper bound.
    #[getter]
    pub fn max_length(&self) -> Option<u64> {
        self.inner.max_length()
    }

    /// Inclusive collection-size lower bound.
    #[getter]
    pub fn min_items(&self) -> Option<u64> {
        self.inner.min_items()
    }

    /// Inclusive collection-size upper bound.
    #[getter]
    pub fn max_items(&self) -> Option<u64> {
        self.inner.max_items()
    }
}

/// Complete source-role and schema-derived inspection result.
#[pyclass(name = "AuthoringInspection", frozen)]
pub struct PyAuthoringInspection {
    inner: AuthoringInspection,
}

#[pymethods]
impl PyAuthoringInspection {
    /// Availability of structural help at the exact request coordinate.
    #[getter]
    pub fn state(&self) -> PyAuthoringInspectionState {
        self.inner.state().into()
    }

    /// Selected grammar schema identity when present.
    #[getter]
    pub fn schema_id(&self) -> Option<&str> {
        self.inner.schema_id()
    }

    /// Selected shared definition when present.
    #[getter]
    pub fn definition(&self) -> Option<&str> {
        self.inner.definition()
    }

    /// Structural facts when [`Self::state`] is available.
    #[getter]
    pub fn node(&self) -> Option<PyAuthoringNodeFact> {
        self.inner
            .node()
            .cloned()
            .map(|inner| PyAuthoringNodeFact { inner })
    }
}

/// Output-only closed namespace of one compiler-resolved identity.
#[pyclass(name = "AuthoringIdentityKind", frozen)]
pub struct PyAuthoringIdentityKind {
    inner: AuthoringIdentityKind,
}

#[pymethods]
impl PyAuthoringIdentityKind {
    /// Stable lower-kebab-case namespace name.
    #[getter]
    pub fn name(&self) -> &'static str {
        authoring_identity_kind_name(self.inner)
    }
}

/// Semantic role played by one admitted source occurrence.
#[pyclass(
    name = "AuthoringOccurrenceRole",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyAuthoringOccurrenceRole {
    /// The occurrence establishes a canonical definition.
    Definition,
    /// The occurrence resolves to a separately defined identity.
    Reference,
    /// The occurrence carries a content-bearing interface projection.
    Projection,
}

impl From<AuthoringOccurrenceRole> for PyAuthoringOccurrenceRole {
    fn from(value: AuthoringOccurrenceRole) -> Self {
        match value {
            AuthoringOccurrenceRole::Definition => Self::Definition,
            AuthoringOccurrenceRole::Reference => Self::Reference,
            AuthoringOccurrenceRole::Projection => Self::Projection,
        }
    }
}

/// One compiler-resolved authoring identity.
#[pyclass(name = "AuthoringIdentity", frozen)]
pub struct PyAuthoringIdentity {
    pub(crate) inner: AuthoringIdentity,
}

#[pymethods]
impl PyAuthoringIdentity {
    /// Non-interchangeable identity namespace.
    #[getter]
    pub fn kind(&self) -> PyAuthoringIdentityKind {
        PyAuthoringIdentityKind {
            inner: self.inner.kind(),
        }
    }

    /// Canonical compiler-owned display form.
    #[getter]
    pub fn value(&self) -> &str {
        self.inner.value()
    }

    /// Exact TOML scalar spelling valid at the reference scope.
    #[getter]
    pub fn authored_value(&self) -> &str {
        self.inner.authored_value()
    }
}

/// One exact source location retained by the compiler, including a rejected input.
#[pyclass(name = "AuthoringSourceLocation", frozen)]
pub struct PyAuthoringSourceLocation {
    inner: AuthoringSourceLocation,
}

impl PyAuthoringSourceLocation {
    pub(crate) fn from_api(value: &AuthoringSourceLocation) -> Self {
        Self {
            inner: value.clone(),
        }
    }
}

/// A compiler-resolved location and its definition, reference or projection role.
#[pyclass(name = "AuthoringOccurrence", frozen)]
pub struct PyAuthoringOccurrence {
    inner: pale_sdk::AuthoringOccurrence,
}

#[pymethods]
impl PyAuthoringOccurrence {
    /// Exact logical source label and decoded field/index path.
    #[getter]
    pub fn location(&self) -> PyAuthoringSourceLocation {
        PyAuthoringSourceLocation::from_api(self.inner.location())
    }

    /// Semantic role assigned by the compiler.
    #[getter]
    pub fn role(&self) -> PyAuthoringOccurrenceRole {
        self.inner.role().into()
    }
}

/// Resume the same occurrence query; this token cannot be built from strings.
///
/// The token shares its Rust compilation owner until Python releases it.
/// It grants no source-disclosure permission and has no configuration effect.
#[pyclass(name = "AuthoringOccurrencePageCursor", frozen)]
pub struct PyAuthoringOccurrencePageCursor {
    inner: pale_sdk::AuthoringOccurrencePageCursor,
}

/// Matching locations, copied string bytes, and retained-index work performed.
#[pyclass(name = "AuthoringOccurrencePage", frozen)]
pub struct PyAuthoringOccurrencePage {
    inner: pale_sdk::AuthoringOccurrencePage,
}

#[pymethods]
impl PyAuthoringOccurrencePage {
    /// Matching occurrences in the compiler's source/path order.
    #[getter]
    pub fn occurrences(&self) -> Vec<PyAuthoringOccurrence> {
        self.inner
            .occurrences()
            .iter()
            .cloned()
            .map(|inner| PyAuthoringOccurrence { inner })
            .collect()
    }

    /// Index entries inspected, including nonmatches.
    #[getter]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }

    /// UTF-8 bytes copied into returned source labels and field names.
    #[getter]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }

    /// Resume token; an empty page can still require continuation.
    #[getter]
    pub fn continuation(&self) -> Option<PyAuthoringOccurrencePageCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| PyAuthoringOccurrencePageCursor { inner })
    }

    /// Whether the scan proved that it reached the end of the index.
    #[getter]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }
}

fn occurrence_request(
    identity: &PyAuthoringIdentity,
    maximum_inspected: u16,
    maximum_returned: u16,
    continuation: Option<&PyAuthoringOccurrencePageCursor>,
) -> pale_sdk::AuthoringOccurrencePageRequest {
    let request = pale_sdk::AuthoringOccurrencePageRequest::new(
        identity.inner.clone(),
        maximum_inspected,
        maximum_returned,
    );
    match continuation {
        Some(cursor) => request.after(cursor.inner.clone()),
        None => request,
    }
}

#[pymethods]
impl PyAuthoringSourceLocation {
    /// Compiler logical source label.
    #[getter]
    pub fn source_label(&self) -> &str {
        self.inner.source_label()
    }

    /// Exact decoded field/index path.
    #[getter]
    pub fn path(&self) -> Vec<PyAuthoringInstancePathSegment> {
        self.inner
            .path()
            .iter()
            .map(PyAuthoringInstancePathSegment::from_api)
            .collect()
    }
}

/// Semantic navigation result for one admitted source occurrence.
#[pyclass(name = "AuthoringNavigation", frozen)]
pub struct PyAuthoringNavigation {
    inner: AuthoringNavigation,
}

#[pymethods]
impl PyAuthoringNavigation {
    /// Resolved identity, or `None` for structural-only coordinates.
    #[getter]
    pub fn identity(&self) -> Option<PyAuthoringIdentity> {
        self.inner
            .identity()
            .cloned()
            .map(|inner| PyAuthoringIdentity { inner })
    }

    /// Exact semantic role of the queried occurrence.
    #[getter]
    pub fn occurrence_role(&self) -> Option<PyAuthoringOccurrenceRole> {
        self.inner.occurrence_role().map(Into::into)
    }

    /// Canonical definition location when present.
    #[getter]
    pub fn definition(&self) -> Option<PyAuthoringSourceLocation> {
        self.inner
            .definition()
            .map(PyAuthoringSourceLocation::from_api)
    }

    /// Retained references in deterministic source/path order.
    #[getter]
    pub fn references(&self) -> Vec<PyAuthoringSourceLocation> {
        self.inner
            .references()
            .iter()
            .map(PyAuthoringSourceLocation::from_api)
            .collect()
    }

    /// Same-namespace identities eligible for completion.
    #[getter]
    pub fn alternatives(&self) -> Vec<PyAuthoringIdentity> {
        self.inner
            .alternatives()
            .iter()
            .cloned()
            .map(|inner| PyAuthoringIdentity { inner })
            .collect()
    }

    /// Whether references were truncated at the API ceiling.
    #[getter]
    pub fn references_truncated(&self) -> bool {
        self.inner.references_truncated()
    }

    /// Whether alternatives were truncated at the API ceiling.
    #[getter]
    pub fn alternatives_truncated(&self) -> bool {
        self.inner.alternatives_truncated()
    }
}

fn inspection_request(
    source_label: String,
    path: Vec<PyAuthoringPathSegment>,
) -> AuthoringInspectionRequest {
    AuthoringInspectionRequest::new(
        source_label,
        path.into_iter()
            .map(PyAuthoringPathSegment::into_api)
            .collect(),
    )
}

fn navigation_request(
    source_label: String,
    path: Vec<PyAuthoringInstancePathSegment>,
) -> AuthoringNavigationRequest {
    AuthoringNavigationRequest::new(
        source_label,
        path.into_iter()
            .map(PyAuthoringInstancePathSegment::into_api)
            .collect(),
    )
}

#[pymethods]
impl PalePython {
    /// Return the canonical capture limits for one source-custody class.
    pub fn authoring_ingress_limits(
        &self,
        ingress_class: PyAuthoringIngressClass,
    ) -> PyAuthoringIngressLimits {
        let limits = pale_sdk::PaleApi::authoring_ingress_limits(ingress_class.into());
        PyAuthoringIngressLimits {
            max_documents: limits.max_documents(),
            max_document_bytes: limits.max_document_bytes(),
            max_total_bytes: limits.max_total_bytes(),
        }
    }

    /// Validate one explicitly typed source without selecting project context.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for unsafe labels, bounded-input
    /// failures, invalid TOML, or contract violations.
    pub fn validate_authoring_document(
        &self,
        py: Python<'_>,
        role: PyStandaloneAuthoringRole,
        source_label: String,
        bytes: Vec<u8>,
    ) -> PyResult<()> {
        let api = Arc::clone(&self.api);
        py.detach(move || api.validate_authoring_document(role.into(), source_label, bytes))
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Inspect one standalone compiler-owned TOML grammar.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] with the canonical API diagnostic
    /// for an oversized path or incoherent generated authoring metadata.
    pub fn inspect_standalone_authoring(
        &self,
        py: Python<'_>,
        role: PyStandaloneAuthoringRole,
        path: Vec<PyAuthoringPathSegment>,
    ) -> PyResult<PyAuthoringInspection> {
        let api = Arc::clone(&self.api);
        let path = path
            .into_iter()
            .map(PyAuthoringPathSegment::into_api)
            .collect();
        py.detach(move || api.inspect_standalone_authoring(role.into(), path))
            .map(|inner| PyAuthoringInspection { inner })
            .map_err(|error| diagnostic_error(py, error))
    }
}

#[pymethods]
impl PyPortableWorkspaceCompilation {
    /// Page compiler-resolved occurrences without reopening source.
    ///
    /// Zero selects 64; limits above 256 are capped by the SDK. Keep both raw
    /// limits unchanged when resuming. Empty pages can be incomplete.
    ///
    /// # Errors
    /// Invalid Python limits raise `TypeError` or `OverflowError`. A foreign
    /// cursor, changed identity or changed limit raises [`crate::PaleDiagnosticError`].
    #[pyo3(signature = (identity, maximum_inspected: "int", maximum_returned: "int", continuation=None))]
    pub fn authoring_occurrences(
        &self,
        py: Python<'_>,
        identity: &PyAuthoringIdentity,
        #[pyo3(from_py_with = page_limit)] maximum_inspected: u16,
        #[pyo3(from_py_with = page_limit)] maximum_returned: u16,
        continuation: Option<&PyAuthoringOccurrencePageCursor>,
    ) -> PyResult<PyAuthoringOccurrencePage> {
        let compilation = Arc::clone(&self.inner);
        let request =
            occurrence_request(identity, maximum_inspected, maximum_returned, continuation);
        py.detach(move || compilation.authoring_occurrences(request))
            .map(|inner| PyAuthoringOccurrencePage { inner })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Inspect structural grammar retained by this admitted workspace.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] with the canonical API diagnostic
    /// for an oversized path or incoherent generated authoring metadata.
    pub fn inspect_authoring(
        &self,
        py: Python<'_>,
        source_label: String,
        path: Vec<PyAuthoringPathSegment>,
    ) -> PyResult<PyAuthoringInspection> {
        let compilation = Arc::clone(&self.inner);
        let request = inspection_request(source_label, path);
        py.detach(move || compilation.inspect_authoring(request))
            .map(|inner| PyAuthoringInspection { inner })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Resolve one exact source occurrence through the canonical linker.
    pub fn navigate_authoring(
        &self,
        py: Python<'_>,
        source_label: String,
        path: Vec<PyAuthoringInstancePathSegment>,
    ) -> PyAuthoringNavigation {
        let compilation = Arc::clone(&self.inner);
        let request = navigation_request(source_label, path);
        py.detach(move || PyAuthoringNavigation {
            inner: compilation.navigate_authoring(request),
        })
    }
}

#[pymethods]
impl PyEffectiveProfileCompilation {
    /// Page exact occurrences in the retained workspace and private source index.
    ///
    /// Uses the same limits and continuation laws as
    /// [`PyPortableWorkspaceCompilation::authoring_occurrences`]. A portable
    /// cursor cannot resume this profile's scan. Permission to disclose source
    /// locations remains the caller's responsibility.
    ///
    /// # Errors
    /// Invalid Python limits raise `TypeError` or `OverflowError`; SDK query
    /// rejection raises [`crate::PaleDiagnosticError`].
    #[pyo3(signature = (identity, maximum_inspected: "int", maximum_returned: "int", continuation=None))]
    pub fn authoring_occurrences(
        &self,
        py: Python<'_>,
        identity: &PyAuthoringIdentity,
        #[pyo3(from_py_with = page_limit)] maximum_inspected: u16,
        #[pyo3(from_py_with = page_limit)] maximum_returned: u16,
        continuation: Option<&PyAuthoringOccurrencePageCursor>,
    ) -> PyResult<PyAuthoringOccurrencePage> {
        let compilation = Arc::clone(&self.inner);
        let request =
            occurrence_request(identity, maximum_inspected, maximum_returned, continuation);
        py.detach(move || compilation.authoring_occurrences(request))
            .map(|inner| PyAuthoringOccurrencePage { inner })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Inspect structural grammar retained by this admitted profile phase.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] with the canonical API diagnostic
    /// for an oversized path or incoherent generated authoring metadata.
    pub fn inspect_authoring(
        &self,
        py: Python<'_>,
        source_label: String,
        path: Vec<PyAuthoringPathSegment>,
    ) -> PyResult<PyAuthoringInspection> {
        let compilation = Arc::clone(&self.inner);
        let request = inspection_request(source_label, path);
        py.detach(move || compilation.inspect_authoring(request))
            .map(|inner| PyAuthoringInspection { inner })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Resolve one exact source occurrence through the retained canonical linker.
    pub fn navigate_authoring(
        &self,
        py: Python<'_>,
        source_label: String,
        path: Vec<PyAuthoringInstancePathSegment>,
    ) -> PyAuthoringNavigation {
        let compilation = Arc::clone(&self.inner);
        let request = navigation_request(source_label, path);
        py.detach(move || PyAuthoringNavigation {
            inner: compilation.navigate_authoring(request),
        })
    }
}

fn authoring_identity_kind_name(kind: AuthoringIdentityKind) -> &'static str {
    match kind {
        AuthoringIdentityKind::SourceDocument => "source-document",
        AuthoringIdentityKind::Workspace => "workspace",
        AuthoringIdentityKind::Project => "project",
        AuthoringIdentityKind::ExecutionWorkspace => "execution-workspace",
        AuthoringIdentityKind::Repository => "repository",
        AuthoringIdentityKind::ExecutionPath => "execution-path",
        AuthoringIdentityKind::Module => "module",
        AuthoringIdentityKind::ModuleRelease => "module-release",
        AuthoringIdentityKind::Component => "component",
        AuthoringIdentityKind::Export => "export",
        AuthoringIdentityKind::Namespace => "namespace",
        AuthoringIdentityKind::Declaration => "declaration",
        AuthoringIdentityKind::Resource => "resource",
        AuthoringIdentityKind::RunPolicy => "run-policy",
        AuthoringIdentityKind::LifecycleBehavior => "lifecycle-behavior",
        AuthoringIdentityKind::Capability => "capability",
        AuthoringIdentityKind::AuthorityRoot => "authority-root",
        AuthoringIdentityKind::ProviderBinding => "provider-binding",
        AuthoringIdentityKind::Machine => "machine",
        AuthoringIdentityKind::ResponsibilityDerivation => "responsibility-derivation",
        AuthoringIdentityKind::ResponsibilityProjection => "responsibility-projection",
        AuthoringIdentityKind::ResponsibilityGenealogyLink => "responsibility-genealogy-link",
        AuthoringIdentityKind::LogicalScope => "logical-scope",
        AuthoringIdentityKind::LogicalOrganization => "logical-organization",
        AuthoringIdentityKind::OrganizationalGroup => "organizational-group",
        AuthoringIdentityKind::DisciplineTree => "discipline-tree",
        AuthoringIdentityKind::OrganizationalRole => "organizational-role",
        AuthoringIdentityKind::OrganizationalOccupant => "organizational-occupant",
        AuthoringIdentityKind::OrganizationalProvider => "organizational-provider",
        AuthoringIdentityKind::CommunicationActor => "communication-actor",
        AuthoringIdentityKind::LeadershipTopic => "leadership-topic",
        AuthoringIdentityKind::OrganizationalRelation => "organizational-relation",
        AuthoringIdentityKind::OrganizationalObject => "organizational-object",
        AuthoringIdentityKind::GroupLensProfile => "group-lens-profile",
        AuthoringIdentityKind::DirectionalService => "directional-service",
        AuthoringIdentityKind::OrganizationalProjection => "organizational-projection",
        AuthoringIdentityKind::OrganizationalInspection => "organizational-inspection",
        AuthoringIdentityKind::CompositionParticipant => "composition-participant",
        AuthoringIdentityKind::CompositionResponsibilityContract => {
            "composition-responsibility-contract"
        }
        AuthoringIdentityKind::CompositionParticipantManifest => "composition-participant-manifest",
        AuthoringIdentityKind::CompositionProvider => "composition-provider",
        AuthoringIdentityKind::CompositionProviderBinding => "composition-provider-binding",
        AuthoringIdentityKind::CompositionEntryContract => "composition-entry-contract",
        AuthoringIdentityKind::CompositionRequestTranslation => "composition-request-translation",
        AuthoringIdentityKind::CompositionResultTranslation => "composition-result-translation",
        AuthoringIdentityKind::CompositionTerminationContract => "composition-termination-contract",
        AuthoringIdentityKind::CompositionSettlementContract => "composition-settlement-contract",
        AuthoringIdentityKind::CompositionContinuityContract => "composition-continuity-contract",
        AuthoringIdentityKind::CompositionEdge => "composition-edge",
        AuthoringIdentityKind::ExperiencePath => "experience-path",
        AuthoringIdentityKind::PackageDoctrine => "package-doctrine",
        AuthoringIdentityKind::PackageOperationArchetype => "package-operation-archetype",
        AuthoringIdentityKind::PackageOperation => "package-operation",
        AuthoringIdentityKind::PackageRoleArchetype => "package-role-archetype",
        AuthoringIdentityKind::PackageRole => "package-role",
        AuthoringIdentityKind::PackageStack => "package-stack",
        AuthoringIdentityKind::PackageRunMode => "package-run-mode",
        AuthoringIdentityKind::PackagePolicyFamily => "package-policy-family",
        AuthoringIdentityKind::PackageGoverningOrder => "package-governing-order",
        AuthoringIdentityKind::PackageCapabilityPolicy => "package-capability-policy",
        AuthoringIdentityKind::PackageToolRecipe => "package-tool-recipe",
        AuthoringIdentityKind::Package => "package",
    }
}
