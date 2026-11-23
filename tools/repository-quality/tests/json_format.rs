//! Repository-level JSON presentation convention.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Serialize as _;
use serde_json::ser::{PrettyFormatter, Serializer};

#[path = "support/repository.rs"]
mod repository;

use repository::repository_root;

const DUPLICATE_KEY_FIXTURE: &str =
    "crates/pale-contracts/tests/fixtures/negative/strict-duplicate-key.json";

/// JSON value that retains source member order during formatting checks.
#[derive(Debug)]
enum OrderedJson {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl<'de> serde::Deserialize<'de> for OrderedJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(OrderedJsonVisitor)
    }
}

struct OrderedJsonVisitor;

impl<'de> serde::de::Visitor<'de> for OrderedJsonVisitor {
    type Value = OrderedJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value with ordered object members")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(OrderedJson::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(OrderedJson::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(OrderedJson::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(OrderedJson::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(OrderedJson::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(OrderedJson::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(OrderedJson::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(OrderedJson::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((key, value)) = map.next_entry::<String, OrderedJson>()? {
            entries.push((key, value));
        }
        Ok(OrderedJson::Object(entries))
    }
}

impl OrderedJson {
    fn duplicate_member_paths(&self) -> Vec<String> {
        let mut duplicates = Vec::new();
        self.collect_duplicate_member_paths("", &mut duplicates);
        duplicates
    }

    fn collect_duplicate_member_paths(&self, path: &str, duplicates: &mut Vec<String>) {
        match self {
            Self::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    value.collect_duplicate_member_paths(&format!("{path}/{index}"), duplicates);
                }
            }
            Self::Object(entries) => {
                let mut seen = BTreeSet::new();
                for (key, value) in entries {
                    let escaped = key.replace('~', "~0").replace('/', "~1");
                    let member_path = format!("{path}/{escaped}");
                    if !seen.insert(key) {
                        duplicates.push(member_path.clone());
                    }
                    value.collect_duplicate_member_paths(&member_path, duplicates);
                }
            }
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::String(_) => {}
        }
    }
}

impl serde::Serialize for OrderedJson {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Number(value) => value.serialize(serializer),
            Self::String(value) => serializer.serialize_str(value),
            Self::Array(values) => {
                use serde::ser::SerializeSeq as _;
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(value)?;
                }
                sequence.end()
            }
            Self::Object(entries) => {
                use serde::ser::SerializeMap as _;
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

fn pretty_document(value: &OrderedJson) -> Vec<u8> {
    let mut bytes = Vec::new();
    value
        .serialize(&mut Serializer::with_formatter(
            &mut bytes,
            PrettyFormatter::with_indent(b"  "),
        ))
        .expect("parsed JSON renders");
    bytes.push(b'\n');
    bytes
}

fn excluded_directory(relative: &Path) -> bool {
    relative.components().any(|component| {
        matches!(
            component,
            Component::Normal(name)
                if matches!(
                    name.to_str(),
                    Some(
                        ".git"
                            | ".codegraph"
                            | ".venv"
                            | "__pycache__"
                            | "build"
                            | "dist"
                            | "node_modules"
                            | "target"
                            | "venv"
                    )
                )
        )
    })
}

fn repository_json_paths(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut json = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("repository directory is readable") {
            let entry = entry.expect("repository entry is readable");
            let path = entry.path();
            let relative = path.strip_prefix(root).expect("entry remains below root");
            let file_type = entry.file_type().expect("entry type is readable");
            if file_type.is_dir() {
                if !excluded_directory(relative) {
                    pending.push(path);
                }
            } else if file_type.is_file()
                && path.extension().and_then(|value| value.to_str()) == Some("json")
            {
                json.push(path);
            }
        }
    }
    json.sort();
    json
}

#[test]
fn repository_json_is_deterministically_readable_at_rest() {
    let root = repository_root();
    let mut checked = 0_usize;
    for path in repository_json_paths(&root) {
        let relative = path.strip_prefix(&root).expect("JSON remains below root");
        let bytes = fs::read(&path).expect("JSON file is readable");
        let source = std::str::from_utf8(&bytes).expect("repository JSON is UTF-8");
        assert!(
            source.ends_with('\n') && !source.ends_with("\n\n"),
            "{} must end in exactly one newline",
            relative.display()
        );
        assert!(
            !source.contains('\r') && !source.contains('\t'),
            "{} must use LF and spaces",
            relative.display()
        );
        let value = serde_json::from_slice::<OrderedJson>(&bytes)
            .unwrap_or_else(|error| panic!("parse {}: {error}", relative.display()));
        let duplicates = value.duplicate_member_paths();
        if relative == Path::new(DUPLICATE_KEY_FIXTURE) {
            assert_eq!(duplicates, ["/exec"]);
        } else {
            assert!(
                duplicates.is_empty(),
                "{} repeats JSON members at {duplicates:?}",
                relative.display()
            );
        }
        assert_eq!(bytes, pretty_document(&value), "{}", relative.display());
        checked += 1;
    }
    assert!(
        checked > 100,
        "repository JSON inventory is unexpectedly empty"
    );
}
