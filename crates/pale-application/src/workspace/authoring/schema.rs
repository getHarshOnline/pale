//! Schema-derived structural facts for configuration authoring.
//!
//! The generated annotation graph is a presentation projection of the same
//! Rust producers used for contract generation. It owns no validation,
//! semantic admission, identity resolution, or proof state.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::AuthoringTomlGrammar;

/// One structural step from a TOML document root to the cursor context.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthoringPathSegment {
    /// Descend through one exact object property or TOML key.
    Field(String),
    /// Descend through an array or array-of-tables item.
    ArrayItem,
}

/// JSON-compatible literal offered by a generated authoring grammar.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthoringLiteral {
    /// UTF-8 string member.
    String(String),
    /// Signed integer member.
    Integer(i64),
    /// Boolean member.
    Boolean(bool),
}

/// One closed literal together with its Rustdoc-derived variant meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringLiteralFact {
    /// Exact TOML-compatible literal.
    pub value: AuthoringLiteral,
    /// Variant-specific meaning, when the producer declares it.
    pub description: Option<String>,
}

/// Structural value categories admitted by an authoring node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthoringValueKind {
    /// Boolean value.
    Boolean,
    /// Integer value.
    Integer,
    /// String value.
    String,
    /// Ordered array value.
    Array,
    /// TOML table or inline-table value.
    Table,
}

/// Whether a field is required across the candidate structural variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoringFieldPresence {
    /// Every applicable structural variant requires the field.
    Required,
    /// No applicable structural variant requires the field.
    Optional,
    /// The field is required only by some structural variants.
    VariantDependent,
}

/// One key offered beneath the selected authoring node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringFieldFact {
    /// Exact authored key spelling.
    pub name: String,
    /// Rustdoc-derived field meaning, when the producer declares it.
    pub description: Option<String>,
    /// Requiredness across the applicable schema variants.
    pub presence: AuthoringFieldPresence,
    /// Structural categories admitted for the field value.
    pub value_kinds: Vec<AuthoringValueKind>,
    /// Closed literal vocabulary, when one exists.
    pub allowed_values: Vec<AuthoringLiteralFact>,
}

/// Bounded structural facts at one authoring path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringNodeFact {
    /// Rustdoc-derived meaning of this exact field or value context.
    pub description: Option<String>,
    /// Structural categories admitted at the selected path.
    pub value_kinds: Vec<AuthoringValueKind>,
    /// Closed literal vocabulary at the selected path.
    pub allowed_values: Vec<AuthoringLiteralFact>,
    /// Deterministically ordered child keys.
    pub fields: Vec<AuthoringFieldFact>,
    /// Optional string format owned by the generated schema.
    pub format: Option<String>,
    /// Optional string pattern owned by the generated schema.
    pub pattern: Option<String>,
    /// Inclusive integer lower bound.
    pub minimum: Option<i64>,
    /// Inclusive integer upper bound.
    pub maximum: Option<i64>,
    /// Inclusive string-length lower bound.
    pub min_length: Option<u64>,
    /// Inclusive string-length upper bound.
    pub max_length: Option<u64>,
    /// Inclusive collection-size lower bound.
    pub min_items: Option<u64>,
    /// Inclusive collection-size upper bound.
    pub max_items: Option<u64>,
}

/// Generated authoring metadata contradicted its Rust-owned schema graph.
#[derive(Debug, thiserror::Error)]
pub enum AuthoringSchemaError {
    /// Embedded registry lookup or strict annotation parsing failed.
    #[error("authoring schema registry failed: {0}")]
    Registry(#[from] pale_contracts::SchemaError),
    /// A named definition selected by the compiler is absent.
    #[error("authoring schema `{schema_id}` has no definition `{definition}`")]
    UnknownDefinition {
        /// Descriptor identity selected by the compiler.
        schema_id: String,
        /// Exact missing `$defs` member.
        definition: String,
    },
    /// A generated reference does not resolve inside the authoring closure.
    #[error("authoring schema reference `{0}` is not resolvable")]
    InvalidReference(String),
    /// The requested structural path is absent from every schema variant.
    #[error("authoring path is not present in the selected grammar")]
    UnknownPath,
    /// Generated reference expansion exceeded the finite safety bound.
    #[error("authoring schema reference expansion exceeded its finite bound")]
    ReferenceDepth,
}

#[derive(Clone, Copy)]
struct NodeRef {
    schema_id: &'static str,
    value: &'static Value,
}

/// Inspect one exact path within a compiler-selected authoring grammar.
///
/// This operation follows only generated JSON Schema structure. It does not
/// parse TOML, resolve semantic identities, compile a workspace, or derive an
/// admission state. Union results remain explicit by accumulating compatible
/// fields, kinds, and literals rather than selecting a branch heuristically.
///
/// # Errors
///
/// Returns [`AuthoringSchemaError`] when generated annotations contradict the
/// registry, a compiler-selected definition is absent, or `path` is not part
/// of the selected structural grammar.
pub fn inspect_authoring_node(
    grammar: AuthoringTomlGrammar,
    path: &[AuthoringPathSegment],
) -> Result<AuthoringNodeFact, AuthoringSchemaError> {
    let schema = pale_contracts::authoring_schema(grammar.schema_id())?;
    let root = schema.value()?;
    let selected = match grammar.definition_name() {
        Some(definition) => root
            .pointer(&format!("/$defs/{}", escape_pointer(definition)))
            .ok_or_else(|| AuthoringSchemaError::UnknownDefinition {
                schema_id: grammar.schema_id().to_owned(),
                definition: definition.to_owned(),
            })?,
        None => root,
    };
    let mut nodes = vec![NodeRef {
        schema_id: schema.schema_id(),
        value: selected,
    }];
    for segment in path {
        nodes = descend(&nodes, segment)?;
        if nodes.is_empty() {
            return Err(AuthoringSchemaError::UnknownPath);
        }
    }
    project_node(&nodes)
}

fn descend(
    nodes: &[NodeRef],
    segment: &AuthoringPathSegment,
) -> Result<Vec<NodeRef>, AuthoringSchemaError> {
    let mut expanded = Vec::new();
    for node in nodes {
        expand(*node, 0, &mut BTreeSet::new(), &mut expanded)?;
    }
    let mut children = Vec::new();
    for node in expanded {
        let child = match segment {
            AuthoringPathSegment::Field(field) => node
                .value
                .get("properties")
                .and_then(|properties| properties.get(field)),
            AuthoringPathSegment::ArrayItem => node.value.get("items"),
        };
        if let Some(value) = child {
            children.push(NodeRef {
                schema_id: node.schema_id,
                value,
            });
        }
    }
    Ok(children)
}

fn expand(
    node: NodeRef,
    depth: usize,
    visited: &mut BTreeSet<String>,
    output: &mut Vec<NodeRef>,
) -> Result<(), AuthoringSchemaError> {
    const MAX_REFERENCE_DEPTH: usize = 64;
    if depth > MAX_REFERENCE_DEPTH {
        return Err(AuthoringSchemaError::ReferenceDepth);
    }
    if let Some(reference) = node.value.get("$ref").and_then(Value::as_str) {
        let key = format!("{}:{reference}", node.schema_id);
        if visited.insert(key.clone()) {
            expand(
                resolve_reference(node.schema_id, reference)?,
                depth + 1,
                visited,
                output,
            )?;
            visited.remove(&key);
        }
    }
    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(variants) = node.value.get(keyword).and_then(Value::as_array) {
            for variant in variants {
                expand(
                    NodeRef {
                        schema_id: node.schema_id,
                        value: variant,
                    },
                    depth + 1,
                    visited,
                    output,
                )?;
            }
        }
    }
    output.push(node);
    Ok(())
}

fn resolve_reference(
    current_schema_id: &'static str,
    reference: &str,
) -> Result<NodeRef, AuthoringSchemaError> {
    let (schema_id, pointer) = if let Some(pointer) = reference.strip_prefix('#') {
        (current_schema_id, pointer)
    } else {
        let (resource, pointer) = reference
            .split_once('#')
            .map_or((reference, ""), |(resource, pointer)| (resource, pointer));
        let schema_id = resource
            .rsplit('/')
            .next()
            .and_then(|name| name.strip_suffix(".json"))
            .ok_or_else(|| AuthoringSchemaError::InvalidReference(reference.to_owned()))?;
        let schema = pale_contracts::authoring_schema(schema_id)?;
        let declared_id = schema
            .value()?
            .get("$id")
            .and_then(Value::as_str)
            .ok_or_else(|| AuthoringSchemaError::InvalidReference(reference.to_owned()))?;
        if declared_id != resource {
            return Err(AuthoringSchemaError::InvalidReference(reference.to_owned()));
        }
        (schema.schema_id(), pointer)
    };
    let schema = pale_contracts::authoring_schema(schema_id)?;
    let root = schema.value()?;
    let value = if pointer.is_empty() {
        root
    } else {
        root.pointer(pointer)
            .ok_or_else(|| AuthoringSchemaError::InvalidReference(reference.to_owned()))?
    };
    Ok(NodeRef {
        schema_id: schema.schema_id(),
        value,
    })
}

fn project_node(nodes: &[NodeRef]) -> Result<AuthoringNodeFact, AuthoringSchemaError> {
    let mut expanded = Vec::new();
    for node in nodes {
        expand(*node, 0, &mut BTreeSet::new(), &mut expanded)?;
    }
    if expanded.is_empty() {
        return Err(AuthoringSchemaError::UnknownPath);
    }
    let description = join_distinct(
        expanded
            .iter()
            .filter(|node| node.value.get("const").is_none() && node.value.get("enum").is_none())
            .filter_map(|node| node.value.get("description").and_then(Value::as_str))
            .map(str::to_owned)
            .collect(),
    );
    let format = unique_string(&expanded, "format");
    let pattern = unique_string(&expanded, "pattern");
    let value_kinds = value_kinds(&expanded);
    let allowed_values = allowed_values(&expanded);
    let minimum = integer_bound(&expanded, "minimum", BoundChoice::Lowest);
    let maximum = integer_bound(&expanded, "maximum", BoundChoice::Highest);
    let min_length = unsigned_bound(&expanded, "minLength", BoundChoice::Lowest);
    let max_length = unsigned_bound(&expanded, "maxLength", BoundChoice::Highest);
    let min_items = unsigned_bound(&expanded, "minItems", BoundChoice::Lowest);
    let max_items = unsigned_bound(&expanded, "maxItems", BoundChoice::Highest);
    let fields = project_fields(&expanded)?;
    Ok(AuthoringNodeFact {
        description,
        value_kinds,
        allowed_values,
        fields,
        format,
        pattern,
        minimum,
        maximum,
        min_length,
        max_length,
        min_items,
        max_items,
    })
}

fn project_fields(nodes: &[NodeRef]) -> Result<Vec<AuthoringFieldFact>, AuthoringSchemaError> {
    #[derive(Default)]
    struct FieldAccumulator {
        descriptions: BTreeSet<String>,
        value_kinds: BTreeSet<AuthoringValueKind>,
        allowed_values: BTreeMap<AuthoringLiteral, BTreeSet<String>>,
        present_in: usize,
        required_in: usize,
    }

    let object_nodes = nodes
        .iter()
        .filter(|node| {
            node.value
                .get("properties")
                .and_then(Value::as_object)
                .is_some()
        })
        .copied()
        .collect::<Vec<_>>();
    let variant_count = object_nodes.len();
    let mut fields = BTreeMap::<String, FieldAccumulator>::new();
    for node in object_nodes {
        let properties = node
            .value
            .get("properties")
            .and_then(Value::as_object)
            .expect("object node was selected by its properties map");
        let required = node
            .value
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        for (name, value) in properties {
            let entry = fields.entry(name.clone()).or_default();
            entry.present_in += 1;
            entry.required_in += usize::from(required.contains(name.as_str()));
            if let Some(description) = value.get("description").and_then(Value::as_str) {
                entry.descriptions.insert(description.to_owned());
            }
            let mut values = Vec::new();
            expand(
                NodeRef {
                    schema_id: node.schema_id,
                    value,
                },
                0,
                &mut BTreeSet::new(),
                &mut values,
            )?;
            entry.value_kinds.extend(value_kinds(&values));
            for value in allowed_values(&values) {
                let descriptions = entry.allowed_values.entry(value.value).or_default();
                descriptions.extend(value.description);
            }
        }
    }
    Ok(fields
        .into_iter()
        .map(|(name, field)| {
            let presence = if variant_count != 0 && field.required_in == variant_count {
                AuthoringFieldPresence::Required
            } else if field.required_in == 0 {
                AuthoringFieldPresence::Optional
            } else {
                AuthoringFieldPresence::VariantDependent
            };
            AuthoringFieldFact {
                name,
                description: join_distinct(field.descriptions),
                presence,
                value_kinds: field.value_kinds.into_iter().collect(),
                allowed_values: field
                    .allowed_values
                    .into_iter()
                    .map(|(value, descriptions)| AuthoringLiteralFact {
                        value,
                        description: join_distinct(descriptions),
                    })
                    .collect(),
            }
        })
        .collect())
}

fn value_kinds(nodes: &[NodeRef]) -> Vec<AuthoringValueKind> {
    let mut kinds = BTreeSet::new();
    for node in nodes {
        match node.value.get("type") {
            Some(Value::String(kind)) => insert_value_kind(kind, &mut kinds),
            Some(Value::Array(types)) => {
                for kind in types.iter().filter_map(Value::as_str) {
                    insert_value_kind(kind, &mut kinds);
                }
            }
            _ => {}
        }
        if node.value.get("properties").is_some() {
            kinds.insert(AuthoringValueKind::Table);
        }
        if node.value.get("items").is_some() {
            kinds.insert(AuthoringValueKind::Array);
        }
    }
    kinds.into_iter().collect()
}

fn insert_value_kind(kind: &str, output: &mut BTreeSet<AuthoringValueKind>) {
    let kind = match kind {
        "boolean" => Some(AuthoringValueKind::Boolean),
        "integer" => Some(AuthoringValueKind::Integer),
        "string" => Some(AuthoringValueKind::String),
        "array" => Some(AuthoringValueKind::Array),
        "object" => Some(AuthoringValueKind::Table),
        _ => None,
    };
    if let Some(kind) = kind {
        output.insert(kind);
    }
}

fn allowed_values(nodes: &[NodeRef]) -> Vec<AuthoringLiteralFact> {
    let mut values = BTreeMap::<AuthoringLiteral, BTreeSet<String>>::new();
    for node in nodes {
        if let Some(value) = node.value.get("const").and_then(literal) {
            let descriptions = values.entry(value).or_default();
            if let Some(description) = node.value.get("description").and_then(Value::as_str) {
                descriptions.insert(description.to_owned());
            }
        }
        if let Some(members) = node.value.get("enum").and_then(Value::as_array) {
            for value in members.iter().filter_map(literal) {
                values.entry(value).or_default();
            }
        }
    }
    values
        .into_iter()
        .map(|(value, descriptions)| AuthoringLiteralFact {
            value,
            description: join_distinct(descriptions),
        })
        .collect()
}

fn literal(value: &Value) -> Option<AuthoringLiteral> {
    match value {
        Value::String(value) => Some(AuthoringLiteral::String(value.clone())),
        Value::Number(value) => value.as_i64().map(AuthoringLiteral::Integer),
        Value::Bool(value) => Some(AuthoringLiteral::Boolean(*value)),
        _ => None,
    }
}

fn unique_string(nodes: &[NodeRef], key: &str) -> Option<String> {
    join_distinct(
        nodes
            .iter()
            .filter_map(|node| node.value.get(key).and_then(Value::as_str))
            .map(str::to_owned)
            .collect(),
    )
}

fn join_distinct(values: BTreeSet<String>) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(values.into_iter().collect::<Vec<_>>().join("\n\n"))
    }
}

#[derive(Clone, Copy)]
enum BoundChoice {
    Lowest,
    Highest,
}

fn integer_bound(nodes: &[NodeRef], key: &str, choice: BoundChoice) -> Option<i64> {
    choose_bound(
        nodes
            .iter()
            .filter_map(|node| node.value.get(key).and_then(Value::as_i64)),
        choice,
    )
}

fn unsigned_bound(nodes: &[NodeRef], key: &str, choice: BoundChoice) -> Option<u64> {
    choose_bound(
        nodes
            .iter()
            .filter_map(|node| node.value.get(key).and_then(Value::as_u64)),
        choice,
    )
}

fn choose_bound<T: Ord>(values: impl Iterator<Item = T>, choice: BoundChoice) -> Option<T> {
    match choice {
        BoundChoice::Lowest => values.min(),
        BoundChoice::Highest => values.max(),
    }
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
