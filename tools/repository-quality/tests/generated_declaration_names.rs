//! Repository-level ownership check for generated declaration-name guards.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[path = "support/repository.rs"]
mod repository;

use repository::repository_root;

const HEADER: &str = concat!(
    "// Generated from tools/repository-quality/templates/generated-declaration-names.mjs.\n",
    "// Regenerate with tools/repository-quality/sync-generated-declaration-name-checks.mjs.\n",
    "\n",
);

const LOCAL_CHECKS: [&str; 4] = [
    "interfaces/lsp/tools/declaration-names.mjs",
    "sdk/js/tools/declaration-names.mjs",
    "sdk/js/contracts/tools/declaration-names.mjs",
    "sdk/scene/tools/declaration-names.mjs",
];

const GENERATED_OUTPUT_ROOTS: [&str; 9] = [
    "integrations/vscode/dist/generated/protocol",
    "integrations/vscode/src/generated/protocol",
    "sdk/go/contracts/types",
    "sdk/js/contracts/dist/generated",
    "sdk/js/contracts/src/generated",
    "sdk/js/index.d.ts",
    "sdk/python/python/pypale/__init__.pyi",
    "sdk/python/python/pypale/_native.pyi",
    "sdk/scene/src",
];

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_file() {
        files.push(path.to_path_buf());
        return;
    }
    for entry in
        fs::read_dir(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
    {
        let path = entry.expect("read generated output entry").path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn public_declaration_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    name.ends_with(".ts")
        || name.ends_with(".cts")
        || name.ends_with(".pyi")
        || name.ends_with(".go")
}

fn declaration_name(line: &str, path: &Path) -> Option<String> {
    let line = line.trim_start();
    if path.extension().and_then(|value| value.to_str()) == Some("go") {
        return line
            .strip_prefix("type ")?
            .split_whitespace()
            .next()
            .map(str::to_owned);
    }
    if path.extension().and_then(|value| value.to_str()) == Some("pyi") {
        return line
            .strip_prefix("class ")?
            .split(['(', ':', ' '])
            .next()
            .map(str::to_owned);
    }

    let mut words = line.split_whitespace();
    let mut word = words.next()?;
    while matches!(word, "export" | "declare" | "default" | "abstract") {
        word = words.next()?;
    }
    if !matches!(word, "class" | "enum" | "interface" | "type") {
        return None;
    }
    words
        .next()
        .map(|name| name.trim_end_matches(['<', '=', '{', ';']).to_owned())
}

fn numeric_suffix(name: &str) -> Option<(&str, &str)> {
    let suffix_start = name
        .char_indices()
        .rev()
        .take_while(|(_, character)| character.is_ascii_digit())
        .last()
        .map(|(index, _)| index)?;
    let suffix = &name[suffix_start..];
    if suffix.is_empty()
        || !suffix.bytes().all(|byte| byte.is_ascii_digit())
        || suffix.starts_with('0')
    {
        return None;
    }
    Some((&name[..suffix_start], suffix))
}

fn known_collision_name(name: &str) -> bool {
    let version_counter = name.rsplit_once("V1").is_some_and(|(_, suffix)| {
        !suffix.is_empty()
            && !suffix.starts_with('0')
            && suffix.bytes().all(|byte| byte.is_ascii_digit())
    });
    let variant_counter = name.rsplit_once("Variant").is_some_and(|(_, suffix)| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    });
    version_counter || variant_counter
}

fn collision_names(names: &[String]) -> BTreeSet<String> {
    let declared = names.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut groups = BTreeMap::<&str, Vec<&str>>::new();
    let mut collisions = names
        .iter()
        .filter(|name| known_collision_name(name))
        .cloned()
        .collect::<BTreeSet<_>>();
    for name in names {
        if let Some((base, _)) = numeric_suffix(name) {
            groups.entry(base).or_default().push(name);
        }
    }
    for (base, members) in groups {
        if declared.contains(base) || members.len() > 1 {
            collisions.extend(members.into_iter().map(str::to_owned));
        }
    }
    collisions
}

fn version_segment_count(name: &str) -> usize {
    let bytes = name.as_bytes();
    let mut count = 0;
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'V' && matches!(bytes[index + 1], b'1'..=b'9') {
            count += 1;
            index += 2;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
        } else {
            index += 1;
        }
    }
    count
}

fn versioned_name(name: &str) -> bool {
    let Some(index) = name.rfind('V') else {
        return false;
    };
    let suffix = &name[index + 1..];
    !suffix.is_empty()
        && !suffix.starts_with('0')
        && suffix.bytes().all(|byte| byte.is_ascii_digit())
}

fn nonsemantic_names(names: &[String], reject_redundant_owner: bool) -> BTreeSet<String> {
    let owners = names
        .iter()
        .filter(|name| versioned_name(name))
        .collect::<Vec<_>>();
    names
        .iter()
        .filter(|name| {
            version_segment_count(name) > 1
                || (reject_redundant_owner
                    && owners.iter().any(|owner| {
                        name.as_str() != owner.as_str()
                            && name.starts_with(owner.as_str())
                            && name[owner.len()..]
                                .as_bytes()
                                .first()
                                .is_some_and(u8::is_ascii_uppercase)
                    }))
        })
        .cloned()
        .collect()
}

#[test]
fn standalone_packages_receive_the_repository_owned_name_check() {
    let root = repository_root();
    let template_path =
        root.join("tools/repository-quality/templates/generated-declaration-names.mjs");
    let template = fs::read_to_string(&template_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", template_path.display()));
    let expected = format!("{HEADER}{template}");

    for relative_path in LOCAL_CHECKS {
        let path = root.join(relative_path);
        let actual = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        assert_eq!(
            actual, expected,
            "{relative_path} diverged from the repository-owned declaration-name check"
        );
    }
}

#[test]
fn checked_in_public_declarations_have_semantic_names() {
    let root = repository_root();
    let mut files = Vec::new();
    for relative_path in GENERATED_OUTPUT_ROOTS {
        let path = root.join(relative_path);
        assert!(
            path.exists(),
            "missing public declaration output: {relative_path}"
        );
        collect_files(&path, &mut files);
    }
    files.retain(|path| public_declaration_file(path));
    assert!(
        !files.is_empty(),
        "no public generated declaration outputs found"
    );

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let names = source
            .lines()
            .filter_map(|line| declaration_name(line, &path))
            .collect::<Vec<_>>();
        assert!(
            collision_names(&names).is_empty(),
            "{} contains generated collision names: {:?}",
            path.display(),
            collision_names(&names)
        );
        let reject_redundant_owner =
            path.extension().and_then(|value| value.to_str()) != Some("go");
        assert!(
            nonsemantic_names(&names, reject_redundant_owner).is_empty(),
            "{} contains repeated version or owner names: {:?}",
            path.display(),
            nonsemantic_names(&names, reject_redundant_owner)
        );
    }

    assert!(collision_names(&[
        "ProfileInspectionContext".to_owned(),
        "ProfileInspectionContext2".to_owned(),
    ])
    .contains("ProfileInspectionContext2"));
    assert!(
        collision_names(&["AttributableAnswerV114".to_owned()]).contains("AttributableAnswerV114")
    );
    assert!(collision_names(&["SigningAuthorityBindingV11".to_owned()])
        .contains("SigningAuthorityBindingV11"));
    assert!(
        collision_names(&["Sha256DigestV1".to_owned(), "Http2PolicyV1".to_owned(),]).is_empty()
    );
    assert!(nonsemantic_names(
        &[
            "TypedDigestV1".to_owned(),
            "TypedDigestV1TypedDigestCanonicalizationV1".to_owned(),
        ],
        true
    )
    .contains("TypedDigestV1TypedDigestCanonicalizationV1"));
    assert!(nonsemantic_names(
        &[
            "ProviderBindingV1".to_owned(),
            "ProviderBindingV1ProviderCoordinates".to_owned(),
        ],
        true
    )
    .contains("ProviderBindingV1ProviderCoordinates"));
}
