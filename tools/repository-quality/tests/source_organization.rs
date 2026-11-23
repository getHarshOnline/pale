//! Repository-level regression for test, fixture, benchmark, and Rustdoc-math custody.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use syn::parse::{ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, Item, ItemMod, Lit, Meta, Token};

#[path = "support/repository.rs"]
mod repository;
#[path = "support/rust_sources.rs"]
mod rust_sources;

use repository::{repository_root, rust_crate_roots};
use rust_sources::collect_rust_sources;

const RAW_RUSTDOC_MATH_DELIMITERS: [&str; 7] =
    [r"\(", r"\)", r"\[", r"\]", "$$", r"\begin{", r"\end{"];
const RAW_RUSTDOC_MATH_SYMBOLS: [&str; 41] = [
    "∀", "∃", "∈", "∉", "∩", "∪", "⊆", "⊂", "⇔", "⇒", "⇏", "→", "↔", "↦", "μ", "ι", "≺", "≤", "≥",
    "≠", "≈", "⊕", "⊥", "∖", "∅", "∧", "∨", "¬", "Σ", "Π", "𝒫", "⊎", "⊔", "⊓", "∑", "∞", "×", "÷",
    "·", "⁺", "⁻",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustdocMathInclude {
    member: String,
    source_path: PathBuf,
    location: RustLocation,
}

#[derive(Default)]
struct RustdocMathVisitor {
    source_path: PathBuf,
    location: Option<RustLocation>,
    includes: Vec<RustdocMathInclude>,
    raw_delimiters: Vec<String>,
}

fn rustdoc_math_include(attribute: &Attribute) -> Option<String> {
    if !attribute.path().is_ident("doc") {
        return None;
    }
    let Meta::NameValue(meta) = &attribute.meta else {
        return None;
    };
    let Expr::Macro(include) = &meta.value else {
        return None;
    };
    if !include.mac.path.is_ident("include_str") {
        return None;
    }
    let Expr::Macro(concat) = syn::parse2::<Expr>(include.mac.tokens.clone()).ok()? else {
        return None;
    };
    if !concat.mac.path.is_ident("concat") {
        return None;
    }
    let arguments = concat
        .mac
        .parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
        .ok()?;
    if arguments.len() != 2 {
        return None;
    }
    let mut arguments = arguments.iter();
    let Expr::Macro(manifest_root) = arguments.next()? else {
        return None;
    };
    if !manifest_root.mac.path.is_ident("env") {
        return None;
    }
    let Lit::Str(variable) = manifest_root.mac.parse_body::<Lit>().ok()? else {
        return None;
    };
    if variable.value() != "CARGO_MANIFEST_DIR" {
        return None;
    }
    let Expr::Lit(suffix) = arguments.next()? else {
        return None;
    };
    let Lit::Str(suffix) = &suffix.lit else {
        return None;
    };
    let suffix = suffix.value();
    suffix
        .strip_prefix("/docs/math/")
        .filter(|member| member.ends_with(".mathml"))
        .map(str::to_owned)
}

fn is_rustdoc_mathml_include(attribute: &Attribute) -> bool {
    if !attribute.path().is_ident("doc") {
        return false;
    }
    let Meta::NameValue(meta) = &attribute.meta else {
        return false;
    };
    let Expr::Macro(include) = &meta.value else {
        return false;
    };
    include.mac.path.is_ident("include_str") && include.mac.tokens.to_string().contains(".mathml")
}

impl<'ast> Visit<'ast> for RustdocMathVisitor {
    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        self.inspect_attribute(attribute);
        visit::visit_attribute(self, attribute);
    }

    fn visit_macro(&mut self, macro_: &'ast syn::Macro) {
        for attribute in macro_attributes(macro_) {
            self.inspect_attribute(&attribute);
        }
        visit::visit_macro(self, macro_);
    }
}

impl RustdocMathVisitor {
    fn inspect_attribute(&mut self, attribute: &Attribute) {
        if attribute.path().is_ident("doc") {
            if let Meta::NameValue(meta) = &attribute.meta {
                if let Expr::Lit(value) = &meta.value {
                    if let Lit::Str(documentation) = &value.lit {
                        for delimiter in RAW_RUSTDOC_MATH_DELIMITERS {
                            if documentation.value().contains(delimiter) {
                                self.raw_delimiters.push(format!(
                                    "{} contains raw Rustdoc delimiter `{delimiter}`",
                                    self.source_path.display()
                                ));
                            }
                        }
                        let value = documentation.value();
                        if contains_raw_mathematics(&value) {
                            self.raw_delimiters.push(format!(
                                "{} contains raw Rustdoc mathematics; use one crate-local TeX/MathML owner",
                                self.source_path.display()
                            ));
                        }
                    }
                }
            }
            if let Some(member) = rustdoc_math_include(attribute) {
                self.includes.push(RustdocMathInclude {
                    member,
                    source_path: self.source_path.clone(),
                    location: self
                        .location
                        .expect("Rustdoc visitor has a source location"),
                });
            } else if is_rustdoc_mathml_include(attribute) {
                self.raw_delimiters.push(format!(
                    "{} contains a noncanonical Rustdoc MathML include; use the crate-local docs/math form",
                    self.source_path.display()
                ));
            }
        }
    }
}

fn contains_paired_dollar_math(value: &str) -> bool {
    let mut positions = value.match_indices('$').map(|(position, _)| position);
    matches!((positions.next(), positions.next()), (Some(first), Some(second)) if second > first + 1)
}

fn contains_raw_mathematics(value: &str) -> bool {
    if RAW_RUSTDOC_MATH_SYMBOLS
        .iter()
        .any(|symbol| value.contains(symbol))
        || contains_paired_dollar_math(value)
    {
        return true;
    }

    let mut has_inline_code = false;
    for (index, fragment) in value.split('`').enumerate() {
        if index % 2 == 1 {
            has_inline_code = true;
            if contains_formula_shape(fragment) {
                return true;
            }
        }
    }
    !has_inline_code && value.lines().any(contains_formula_shape)
}

fn contains_formula_shape(fragment: &str) -> bool {
    let fragment = fragment.trim();
    if fragment.is_empty() {
        return false;
    }
    if fragment.match_indices("O(").any(|(index, _)| {
        index == 0
            || fragment[..index]
                .chars()
                .next_back()
                .is_some_and(|character| !character.is_alphanumeric() && character != '_')
    }) {
        return true;
    }
    if fragment.contains("-->") {
        return true;
    }
    let lowercase = fragment.to_ascii_lowercase();
    if lowercase.starts_with("mu ") && lowercase.contains(" union ") {
        return true;
    }
    if contains_implicit_coefficient_sum(fragment) {
        return true;
    }
    let states = fragment.split(" -> ").map(str::trim).collect::<Vec<_>>();
    if states.len() >= 3 && states.iter().all(|state| is_state_token(state)) {
        return true;
    }

    let Some((left, right)) = fragment.split_once(" = ") else {
        return false;
    };
    if fragment.contains(" == ")
        || fragment.contains(" != ")
        || fragment.contains(" <= ")
        || fragment.contains(" >= ")
        || left.contains("..")
    {
        return false;
    }
    let left = left.trim();
    let right = right.trim();
    let left_is_call = left
        .split_once('(')
        .is_some_and(|(name, _)| is_symbolic_name(name.trim()));
    let left_is_indexed = left.contains('[') && left.ends_with(']');
    let left_is_symbol =
        left.chars().count() == 1 && left.chars().all(|character| character.is_alphabetic());
    let left_is_derived_symbol = left
        .split_once('_')
        .is_some_and(|(head, tail)| is_symbolic_name(head) && !tail.is_empty());
    let right_has_operation = ['+', '-', '*', '/', '^', '(', '[', '{']
        .into_iter()
        .any(|operator| right.contains(operator));
    (left_is_call || left_is_indexed || left_is_symbol)
        || (left_is_derived_symbol && right_has_operation)
}

fn contains_implicit_coefficient_sum(fragment: &str) -> bool {
    if !fragment.contains(" + ") || !fragment.contains('(') {
        return false;
    }
    let expression_only = fragment.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || character.is_ascii_whitespace()
            || matches!(
                character,
                '(' | ')' | '+' | '-' | '*' | '/' | '^' | ',' | '_'
            )
    });
    expression_only
        && fragment
            .chars()
            .zip(fragment.chars().skip(1))
            .any(|(left, right)| left.is_ascii_digit() && right.is_ascii_alphabetic())
}

fn is_symbolic_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
}

fn is_state_token(value: &str) -> bool {
    !value.contains(char::is_whitespace)
        && value.chars().next().is_some_and(char::is_uppercase)
        && value
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
}

fn macro_attributes(macro_: &syn::Macro) -> Vec<Attribute> {
    let parser = |input: ParseStream<'_>| {
        let mut attributes = Vec::new();
        while !input.is_empty() {
            if input.peek(Token![#]) && input.peek2(syn::token::Bracket) {
                attributes.extend(input.call(Attribute::parse_outer)?);
            } else {
                input.step(|cursor| {
                    cursor
                        .token_tree()
                        .map(|(_, next)| ((), next))
                        .ok_or_else(|| cursor.error("expected macro token"))
                })?;
            }
        }
        Ok(attributes)
    };
    parser
        .parse2(macro_.tokens.clone())
        .unwrap_or_else(|error| panic!("parse repository macro attributes: {error}"))
}

fn inspect_rustdoc_math(crate_root: &Path, source_path: &Path, source: &str) -> RustdocMathVisitor {
    let syntax = syn::parse_file(source).unwrap_or_else(|error| {
        panic!(
            "parse repository-owned Rust {}: {error}",
            source_path.display()
        )
    });
    let mut visitor = RustdocMathVisitor {
        source_path: source_path.to_path_buf(),
        location: Some(rust_location(crate_root, source_path)),
        ..RustdocMathVisitor::default()
    };
    visitor.visit_file(&syntax);
    visitor
}

fn validate_math_ownership(
    expected: &BTreeSet<String>,
    includes: &[RustdocMathInclude],
) -> Result<(), String> {
    let mut owners = BTreeMap::<&str, Vec<&RustdocMathInclude>>::new();
    for include in includes {
        if include.location != RustLocation::Source {
            return Err(format!(
                "{} includes `{}` outside the owning src/ tree",
                include.source_path.display(),
                include.member
            ));
        }
        if !expected.contains(&include.member) {
            return Err(format!(
                "{} includes nonexistent Rustdoc math member `{}`",
                include.source_path.display(),
                include.member
            ));
        }
        owners.entry(&include.member).or_default().push(include);
    }
    for member in expected {
        let count = owners.get(member.as_str()).map_or(0, Vec::len);
        if count != 1 {
            return Err(format!(
                "Rustdoc math member `{member}` has {count} owning includes; expected exactly one"
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct CfgTruthSet {
    can_be_false: bool,
    can_be_true: bool,
}

impl CfgTruthSet {
    const FALSE: Self = Self {
        can_be_false: true,
        can_be_true: false,
    };
    const TRUE: Self = Self {
        can_be_false: false,
        can_be_true: true,
    };
    const UNKNOWN: Self = Self {
        can_be_false: true,
        can_be_true: true,
    };

    const fn negate(self) -> Self {
        Self {
            can_be_false: self.can_be_true,
            can_be_true: self.can_be_false,
        }
    }
}

fn nested_meta(list: &syn::MetaList) -> Punctuated<Meta, Token![,]> {
    list.parse_args_with(Punctuated::parse_terminated)
        .unwrap_or_else(|error| panic!("parse cfg expression: {error}"))
}

fn cfg_truth(meta: &Meta, test: bool) -> CfgTruthSet {
    match meta {
        Meta::Path(path) if path.is_ident("test") => {
            if test {
                CfgTruthSet::TRUE
            } else {
                CfgTruthSet::FALSE
            }
        }
        Meta::Path(_) | Meta::NameValue(_) => CfgTruthSet::UNKNOWN,
        Meta::List(list) if list.path.is_ident("all") => nested_meta(list)
            .iter()
            .map(|nested| cfg_truth(nested, test))
            .fold(CfgTruthSet::TRUE, |left, right| CfgTruthSet {
                can_be_false: left.can_be_false || right.can_be_false,
                can_be_true: left.can_be_true && right.can_be_true,
            }),
        Meta::List(list) if list.path.is_ident("any") => nested_meta(list)
            .iter()
            .map(|nested| cfg_truth(nested, test))
            .fold(CfgTruthSet::FALSE, |left, right| CfgTruthSet {
                can_be_false: left.can_be_false && right.can_be_false,
                can_be_true: left.can_be_true || right.can_be_true,
            }),
        Meta::List(list) if list.path.is_ident("not") => {
            let nested = nested_meta(list);
            assert_eq!(nested.len(), 1, "cfg(not(...)) must have one operand");
            cfg_truth(nested.first().expect("one cfg operand"), test).negate()
        }
        Meta::List(_) => CfgTruthSet::UNKNOWN,
    }
}

fn is_positive_test_cfg(attribute: &Attribute) -> bool {
    if !attribute.path().is_ident("cfg") {
        return false;
    }
    let expression = attribute
        .parse_args::<Meta>()
        .unwrap_or_else(|error| panic!("parse cfg attribute: {error}"));
    !cfg_truth(&expression, false).can_be_true && cfg_truth(&expression, true).can_be_true
}

fn test_attribute_path(path: &syn::Path) -> bool {
    let Some(name) = path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
    else {
        return false;
    };
    name == "test"
        || matches!(
            name.as_str(),
            "parameterized" | "proptest" | "rstest" | "test_case" | "wasm_bindgen_test"
        )
}

fn meta_enables_test(meta: &Meta) -> bool {
    if test_attribute_path(meta.path()) {
        return true;
    }
    match meta {
        Meta::List(list) if list.path.is_ident("cfg_attr") => {
            nested_meta(list).iter().skip(1).any(meta_enables_test)
        }
        _ => false,
    }
}

fn attribute_enables_test(attribute: &Attribute) -> bool {
    if test_attribute_path(attribute.path()) {
        return true;
    }
    if !attribute.path().is_ident("cfg_attr") {
        return false;
    }
    let nested = attribute
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .unwrap_or_else(|error| panic!("parse cfg_attr attribute: {error}"));
    nested.iter().skip(1).any(meta_enables_test)
}

fn nomenclature_tokens(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
}

fn is_test_material_name(value: &str) -> bool {
    let lowercase = value.to_ascii_lowercase();
    let compact = lowercase
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();
    compact.starts_with("testdata")
        || nomenclature_tokens(&lowercase).any(|token| {
            matches!(
                token,
                "corpus"
                    | "fixture"
                    | "fixtures"
                    | "mock"
                    | "mocks"
                    | "synthetic"
                    | "test"
                    | "tests"
            )
        })
}

fn is_benchmark_name(value: &str) -> bool {
    value.to_ascii_lowercase().contains("bench")
}

fn is_exact_feature_cfg(attribute: &Attribute, expected: &str) -> bool {
    if !attribute.path().is_ident("cfg") {
        return false;
    }
    let Ok(Meta::NameValue(feature)) = attribute.parse_args::<Meta>() else {
        return false;
    };
    if !feature.path.is_ident("feature") {
        return false;
    }
    matches!(&feature.value, Expr::Lit(value) if matches!(&value.lit, Lit::Str(name) if name.value() == expected))
}

fn is_hidden(attribute: &Attribute) -> bool {
    attribute.path().is_ident("doc")
        && matches!(&attribute.meta, Meta::List(list) if list.tokens.to_string() == "hidden")
}

fn source_module_owner(crate_root: &Path, path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    if parent == crate_root.join("src") {
        return [parent.join("lib.rs"), parent.join("main.rs")]
            .into_iter()
            .find(|candidate| candidate.is_file());
    }
    let directory_owner = parent.join("mod.rs");
    if directory_owner.is_file() {
        return Some(directory_owner);
    }
    let sibling_owner = parent.with_extension("rs");
    sibling_owner.is_file().then_some(sibling_owner)
}

fn is_feature_gated_benchmark_support(crate_root: &Path, path: &Path) -> bool {
    if rust_location(crate_root, path) != RustLocation::Source {
        return false;
    }
    let manifest = fs::read_to_string(crate_root.join("Cargo.toml"))
        .expect("read benchmark-support crate manifest")
        .parse::<toml::Table>()
        .expect("parse benchmark-support crate manifest");
    let owns_feature = manifest
        .get("features")
        .and_then(toml::Value::as_table)
        .is_some_and(|features| features.contains_key("bench"));
    let selected_by_target = manifest
        .get("bench")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_table)
        .filter_map(|target| target.get("required-features"))
        .filter_map(toml::Value::as_array)
        .flatten()
        .filter_map(toml::Value::as_str)
        .any(|feature| feature == "bench");
    if !owns_feature || !selected_by_target {
        return false;
    }

    let Some(owner_path) = source_module_owner(crate_root, path) else {
        return false;
    };
    let source = fs::read_to_string(&owner_path).expect("read benchmark-support module owner");
    let syntax = syn::parse_file(&source).expect("parse benchmark-support module owner");
    let Some(module_name) = path.file_stem().and_then(|name| name.to_str()) else {
        return false;
    };
    syntax.items.into_iter().any(|item| {
        matches!(item, Item::Mod(module)
            if module.ident == module_name
                && module.content.is_none()
                && module.attrs.iter().any(|attribute| is_exact_feature_cfg(attribute, "bench"))
                && module.attrs.iter().any(is_hidden))
    })
}

fn is_inline_test_module(module: &ItemMod) -> bool {
    module.content.is_some() && is_test_material_name(&module.ident.to_string())
}

fn is_support_module_name(value: &str) -> bool {
    value.contains("fixture") || value.ends_with("test_support")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustLocation {
    Source,
    Tests,
    Benches,
    Examples,
    Other,
}

fn rust_location(crate_root: &Path, path: &Path) -> RustLocation {
    match path
        .strip_prefix(crate_root)
        .expect("Rust file belongs to its crate")
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
    {
        Some("src") => RustLocation::Source,
        Some("tests") => RustLocation::Tests,
        Some("benches") => RustLocation::Benches,
        Some("examples") => RustLocation::Examples,
        _ => RustLocation::Other,
    }
}

struct RustOrganizationVisitor<'a> {
    source_path: &'a Path,
    canonical_tests_root: &'a Path,
    location: RustLocation,
    allowed_test_cfgs: BTreeSet<usize>,
}

impl RustOrganizationVisitor<'_> {
    fn validate_test_module(&mut self, module: &ItemMod) {
        let test_cfgs = module
            .attrs
            .iter()
            .enumerate()
            .filter(|(_, attribute)| is_positive_test_cfg(attribute))
            .collect::<Vec<_>>();
        if test_cfgs.is_empty() {
            assert!(
                !is_inline_test_module(module),
                "{} contains inline test-only module `{}`",
                self.source_path.display(),
                module.ident
            );
            return;
        }

        assert_eq!(
            test_cfgs.len(),
            1,
            "{} test hook must have one positive cfg(test) attribute",
            self.source_path.display()
        );
        assert!(
            module.content.is_none(),
            "{} test hook `{}` must be an external module",
            self.source_path.display(),
            module.ident
        );
        let (cfg_index, cfg_attribute) = test_cfgs[0];
        let path_attribute = module.attrs.get(cfg_index + 1).unwrap_or_else(|| {
            panic!(
                "{} test hook `{}` requires an immediate path attribute",
                self.source_path.display(),
                module.ident
            )
        });
        assert!(
            path_attribute.path().is_ident("path"),
            "{} test hook `{}` requires an immediate path attribute",
            self.source_path.display(),
            module.ident
        );
        assert!(
            module.attrs.iter().enumerate().all(|(index, attribute)| {
                index == cfg_index || index == cfg_index + 1 || attribute.path().is_ident("doc")
            }),
            "{} test hook `{}` may carry only documentation, cfg, and path attributes",
            self.source_path.display(),
            module.ident
        );
        let Meta::NameValue(path_meta) = &path_attribute.meta else {
            panic!(
                "{} test hook `{}` path must use #[path = \"...\"]",
                self.source_path.display(),
                module.ident
            );
        };
        let Expr::Lit(path_value) = &path_meta.value else {
            panic!(
                "{} test hook `{}` path must be a string literal",
                self.source_path.display(),
                module.ident
            );
        };
        let Lit::Str(relative) = &path_value.lit else {
            panic!(
                "{} test hook `{}` path must be a string literal",
                self.source_path.display(),
                module.ident
            );
        };
        let target = self
            .source_path
            .parent()
            .expect("Rust source has a parent")
            .join(relative.value())
            .canonicalize()
            .unwrap_or_else(|_| {
                panic!(
                    "{} test hook target does not exist",
                    self.source_path.display()
                )
            });
        let relative_target = target
            .strip_prefix(self.canonical_tests_root)
            .unwrap_or_else(|_| {
                panic!(
                    "{} test hook escapes the owning crate's tests root",
                    self.source_path.display()
                )
            });
        let class = relative_target
            .components()
            .next()
            .and_then(|component| component.as_os_str().to_str())
            .expect("test hook has a path below tests/");
        assert!(
            matches!(class, "internal" | "support"),
            "{} test hook enters public integration scope",
            self.source_path.display()
        );
        let module_name = module.ident.to_string();
        if class == "support" {
            assert!(
                is_support_module_name(&module_name),
                "{} shared support hook `{module_name}` needs fixture/test_support nomenclature",
                self.source_path.display()
            );
        } else {
            assert!(
                !module_name.contains("fixture"),
                "{} fixture hook belongs under tests/support",
                self.source_path.display()
            );
        }
        self.allowed_test_cfgs
            .insert(cfg_attribute as *const Attribute as usize);
    }
}

impl<'ast> Visit<'ast> for RustOrganizationVisitor<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if self.location == RustLocation::Source {
            if let Item::Mod(module) = item {
                self.validate_test_module(module);
            }
        }
        visit::visit_item(self, item);
    }

    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        assert!(
            self.location == RustLocation::Tests || !attribute_enables_test(attribute),
            "{} contains a test body outside the owning tests tree",
            self.source_path.display()
        );
        assert!(
            self.location != RustLocation::Source
                || !is_positive_test_cfg(attribute)
                || self
                    .allowed_test_cfgs
                    .contains(&(attribute as *const Attribute as usize)),
            "{} contains a test-gated helper, import, or indirect module",
            self.source_path.display()
        );
        visit::visit_attribute(self, attribute);
    }
}

fn validate_rust_organization(crate_root: &Path, path: &Path, canonical_tests_root: &Path) {
    let source = fs::read_to_string(path).expect("read repository-owned Rust source");
    let syntax = syn::parse_file(&source)
        .unwrap_or_else(|error| panic!("parse repository-owned Rust {}: {error}", path.display()));
    let location = rust_location(crate_root, path);
    assert!(
        location == RustLocation::Tests
            || location == RustLocation::Benches
            || !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_benchmark_name)
            || is_feature_gated_benchmark_support(crate_root, path),
        "benchmark Rust must live under benches/ or be an opt-in hidden production module selected by a Cargo benchmark; correctness tests remain under tests/: {}",
        path.display()
    );
    RustOrganizationVisitor {
        source_path: path,
        canonical_tests_root,
        location,
        allowed_test_cfgs: BTreeSet::new(),
    }
    .visit_file(&syntax);
}

fn collect_paths(root: &Path, output: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }
    for entry in fs::read_dir(root).expect("read owned path tree") {
        let entry = entry.expect("read owned path entry");
        let path = entry.path();
        if entry
            .file_type()
            .expect("inspect owned path entry")
            .is_dir()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(repository::is_excluded_source_directory)
        {
            continue;
        }
        output.push(path.clone());
        if entry
            .file_type()
            .expect("inspect owned path entry")
            .is_dir()
        {
            collect_paths(&path, output);
        }
    }
}

fn is_generated_or_external_directory(name: &str) -> bool {
    repository::is_excluded_source_directory(name)
        || matches!(
            name,
            ".codegraph" | "generated" | "third-party" | "third_party" | "vendor"
        )
}

fn simple_plural_forms(name: &str) -> BTreeSet<String> {
    let mut forms = BTreeSet::from([format!("{name}s"), format!("{name}es")]);
    if let Some(stem) = name.strip_suffix('y') {
        forms.insert(format!("{stem}ies"));
    }
    forms
}

fn plural_directory_pairs(names: &BTreeSet<String>) -> Vec<(String, String)> {
    names
        .iter()
        .flat_map(|singular| {
            simple_plural_forms(singular)
                .into_iter()
                .filter(|plural| names.contains(plural))
                .map(|plural| (singular.clone(), plural))
        })
        .collect()
}

fn collect_confusable_directory_homes(
    directory: &Path,
    conflicts: &mut Vec<(PathBuf, String, String)>,
) {
    let children = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read authored directory {}: {error}", directory.display()))
        .map(|entry| entry.expect("read authored directory entry"))
        .filter(|entry| {
            entry
                .file_type()
                .expect("inspect authored directory entry")
                .is_dir()
        })
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| !is_generated_or_external_directory(name))
        })
        .collect::<Vec<_>>();
    let names = children
        .iter()
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .collect::<BTreeSet<_>>();
    conflicts.extend(
        plural_directory_pairs(&names)
            .into_iter()
            .map(|(singular, plural)| (directory.to_path_buf(), singular, plural)),
    );
    for child in children {
        collect_confusable_directory_homes(&child.path(), conflicts);
    }
}

fn collect_rustdoc_math_assets(root: &Path, directory: &Path, output: &mut BTreeSet<String>) {
    if !directory.exists() {
        return;
    }
    for entry in fs::read_dir(directory).expect("read Rustdoc math directory") {
        let entry = entry.expect("read Rustdoc math entry");
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).expect("inspect Rustdoc math entry");
        assert!(
            !metadata.file_type().is_symlink(),
            "Rustdoc math custody forbids symlinks: {}",
            path.display()
        );
        if metadata.is_dir() {
            collect_rustdoc_math_assets(root, &path, output);
        } else if metadata.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension, "tex" | "mathml"))
        {
            output.insert(
                path.strip_prefix(root)
                    .expect("Rustdoc math member belongs to its root")
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
            );
        }
    }
}

fn expected_rustdoc_math_members(crate_root: &Path) -> BTreeSet<String> {
    let math_root = crate_root.join("docs/math");
    let mut assets = BTreeSet::new();
    collect_rustdoc_math_assets(&math_root, &math_root, &mut assets);
    let sources = assets
        .iter()
        .filter_map(|member| member.strip_suffix(".tex"))
        .map(|stem| format!("{stem}.mathml"))
        .collect::<BTreeSet<_>>();
    let outputs = assets
        .iter()
        .filter(|member| member.ends_with(".mathml"))
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        outputs,
        sources,
        "every crate-owned TeX law must have exactly one same-stem MathML projection under {}",
        math_root.display()
    );
    outputs
}

fn path_has_test_material_nomenclature(relative: &Path) -> bool {
    relative.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(is_test_material_name)
    })
}

fn is_regular_file_entry(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
}

fn belongs_to_nested_package_tests(crate_root: &Path, path: &Path) -> bool {
    if path.strip_prefix(crate_root).is_err() {
        return false;
    }
    let mut owner = path.parent();
    while let Some(candidate) = owner {
        if candidate == crate_root {
            return false;
        }
        if candidate.join("package.json").is_file()
            && path
                .strip_prefix(candidate)
                .is_ok_and(|relative| relative.starts_with("tests"))
        {
            return true;
        }
        if is_regular_file_entry(path)
            && is_regular_file_entry(&candidate.join("go.mod"))
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("_test.go"))
        {
            return true;
        }
        owner = candidate.parent();
    }
    false
}

#[test]
fn nested_go_packages_admit_only_idiomatic_colocated_test_files() {
    struct TemporaryTree(PathBuf);
    impl Drop for TemporaryTree {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove this test's temporary tree");
        }
    }

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock follows Unix epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "pale-go-test-ownership-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("create an unused test directory");
    let tree = TemporaryTree(directory);
    let crate_root = tree.0.join("crate");
    let package = crate_root.join("tools/generator");
    fs::create_dir_all(package.join("nested")).expect("create nested Go package");
    fs::write(package.join("go.mod"), "module example.test/generator\n")
        .expect("write nested Go manifest");

    let colocated_test = package.join("main_test.go");
    let nested_test = package.join("nested/decoder_test.go");
    let test_named_directory = package.join("directory_test.go");
    let non_test_helper = package.join("test_helper.go");
    let unowned_test = crate_root.join("stray_test.go");
    fs::write(&colocated_test, "package main\n").expect("write colocated Go test");
    fs::write(&nested_test, "package nested\n").expect("write nested Go test");
    fs::create_dir(&test_named_directory).expect("create test-named directory");
    fs::write(&non_test_helper, "package main\n").expect("write non-test Go helper");
    fs::write(&unowned_test, "package crate\n").expect("write unowned Go test");

    let directory_manifest_package = crate_root.join("tools/directory-manifest");
    fs::create_dir_all(directory_manifest_package.join("go.mod"))
        .expect("create directory named as a Go manifest");
    let directory_manifest_test = directory_manifest_package.join("main_test.go");
    fs::write(&directory_manifest_test, "package main\n")
        .expect("write test beside directory manifest");

    assert!(belongs_to_nested_package_tests(
        &crate_root,
        &colocated_test
    ));
    assert!(belongs_to_nested_package_tests(&crate_root, &nested_test));
    assert!(!belongs_to_nested_package_tests(
        &crate_root,
        &test_named_directory
    ));
    assert!(!belongs_to_nested_package_tests(
        &crate_root,
        &non_test_helper
    ));
    assert!(!belongs_to_nested_package_tests(&crate_root, &unowned_test));
    assert!(!belongs_to_nested_package_tests(
        &crate_root,
        &directory_manifest_test
    ));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let linked_test = package.join("linked_test.go");
        symlink("main_test.go", &linked_test).expect("link a Go test candidate");
        assert!(!belongs_to_nested_package_tests(&crate_root, &linked_test));

        let linked_manifest_package = crate_root.join("tools/linked-manifest");
        fs::create_dir_all(&linked_manifest_package).expect("create linked-manifest package");
        fs::write(
            linked_manifest_package.join("go.mod.target"),
            "module example.test/linked\n",
        )
        .expect("write linked Go manifest target");
        symlink("go.mod.target", linked_manifest_package.join("go.mod"))
            .expect("link a Go manifest");
        let linked_manifest_test = linked_manifest_package.join("main_test.go");
        fs::write(&linked_manifest_test, "package main\n")
            .expect("write test beside linked manifest");
        assert!(!belongs_to_nested_package_tests(
            &crate_root,
            &linked_manifest_test
        ));
    }
}

// Nested build tools own their Cargo test and benchmark targets. They remain
// part of their enclosing repository for distribution and attribution policy.
fn owned_rust_packages(root: &Path) -> BTreeSet<PathBuf> {
    let mut packages = BTreeSet::new();
    for source_home in rust_crate_roots(root) {
        let mut paths = Vec::new();
        collect_paths(&source_home, &mut paths);
        paths.sort_by_key(|path| path.components().count());
        for manifest_path in paths.into_iter().filter(|path| {
            path.file_name().is_some_and(|name| name == "Cargo.toml") && path.is_file()
        }) {
            let directory = manifest_path.parent().expect("Cargo manifest has a parent");
            // A test can contain a complete or deliberately malformed package.
            // Its files remain governed as test material, not a new source home.
            if rust_package_owner(&packages, directory).is_some_and(|owner| {
                directory
                    .strip_prefix(owner)
                    .is_ok_and(|relative| relative.starts_with("tests"))
            }) {
                continue;
            }
            let manifest = fs::read_to_string(&manifest_path)
                .expect("read owned Cargo manifest")
                .parse::<toml::Table>()
                .expect("parse owned Cargo manifest");
            if manifest.get("package").is_some_and(toml::Value::is_table) {
                packages.insert(directory.to_path_buf());
            }
        }
    }
    packages
}

fn rust_package_owner<'a>(packages: &'a BTreeSet<PathBuf>, path: &Path) -> Option<&'a Path> {
    path.ancestors()
        .find_map(|ancestor| packages.get(ancestor).map(PathBuf::as_path))
}

#[test]
fn rustdoc_math_has_structural_single_ownership() {
    let root = repository_root();
    for crate_root in rust_crate_roots(&root) {
        let expected = expected_rustdoc_math_members(&crate_root);
        let mut rust_files = BTreeSet::new();
        collect_rust_sources(&crate_root, &mut rust_files);
        let mut includes = Vec::new();
        let mut raw_delimiters = Vec::new();
        for source_path in rust_files {
            let source =
                fs::read_to_string(&source_path).expect("read repository-owned Rust source");
            let inspected = inspect_rustdoc_math(&crate_root, &source_path, &source);
            includes.extend(inspected.includes);
            raw_delimiters.extend(inspected.raw_delimiters);
        }
        assert!(
            raw_delimiters.is_empty(),
            "raw mathematical delimiters are not a rendered Rustdoc contract:\n{}",
            raw_delimiters.join("\n")
        );
        validate_math_ownership(&expected, &includes)
            .unwrap_or_else(|error| panic!("{}: {error}", crate_root.display()));
    }
}

#[test]
fn authored_directories_do_not_create_singular_and_plural_sibling_homes() {
    assert_eq!(
        plural_directory_pairs(&BTreeSet::from([
            "command".to_owned(),
            "commands".to_owned(),
            "profile".to_owned(),
        ])),
        [("command".to_owned(), "commands".to_owned())]
    );

    let root = repository_root();
    let mut conflicts = Vec::new();
    collect_confusable_directory_homes(&root, &mut conflicts);
    assert!(
        conflicts.is_empty(),
        "singular and plural sibling directories create confusable ownership homes:\n{}",
        conflicts
            .into_iter()
            .map(|(parent, singular, plural)| format!(
                "{} contains both `{singular}/` and `{plural}/`",
                parent.display()
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn manifest_target(
    crate_root: &Path,
    packages: &BTreeSet<PathBuf>,
    entry: &toml::Table,
    default_directory: &str,
    kind: &str,
) -> PathBuf {
    let name = entry
        .get("name")
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{kind} declaration has no name"));
    let relative = entry
        .get("path")
        .and_then(toml::Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(default_directory).join(format!("{name}.rs")));
    let target = crate_root
        .join(relative)
        .canonicalize()
        .unwrap_or_else(|_| {
            panic!(
                "{kind} `{name}` target does not exist under {}",
                crate_root.display()
            )
        });
    assert_eq!(
        rust_package_owner(packages, &target),
        Some(crate_root),
        "{kind} target belongs to another Cargo package: {}",
        target.display()
    );
    target
}

#[test]
fn tests_fixtures_and_benchmarks_remain_in_their_owning_roots() {
    let root = repository_root();
    let benchmark_documentation = root.join("benchmarks/resident-kernel/synthetic/README.md");
    let mut benchmark_paths = Vec::new();
    collect_paths(&root.join("benchmarks"), &mut benchmark_paths);
    let benchmark_files = benchmark_paths
        .into_iter()
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    assert_eq!(
        benchmark_files,
        [benchmark_documentation],
        "top-level benchmark custody may contain only the synthetic-characterization README"
    );

    let mut contract_paths = Vec::new();
    collect_paths(&root.join("contracts"), &mut contract_paths);
    for path in contract_paths {
        let relative = path
            .strip_prefix(root.join("contracts"))
            .expect("contract path belongs to contracts/");
        assert!(
            !path_has_test_material_nomenclature(relative),
            "test-only fixture or corpus escaped into contracts/: {}",
            path.display()
        );
    }

    let packages = owned_rust_packages(&root);
    for crate_root in &packages {
        let tests_root = crate_root.join("tests");
        let canonical_tests_root = tests_root
            .canonicalize()
            .unwrap_or_else(|_| panic!("{} has no tests root", crate_root.display()));
        let canonical_crate_root = crate_root
            .canonicalize()
            .expect("canonicalize workspace crate root");

        let mut rust_files = BTreeSet::new();
        collect_rust_sources(crate_root, &mut rust_files);
        for path in rust_files {
            if rust_package_owner(&packages, &path) != Some(crate_root.as_path()) {
                continue;
            }
            validate_rust_organization(crate_root, &path, &canonical_tests_root);
        }

        let mut owned_paths = Vec::new();
        collect_paths(crate_root, &mut owned_paths);
        for path in &owned_paths {
            if rust_package_owner(&packages, path) != Some(crate_root.as_path()) {
                continue;
            }
            let relative = path
                .strip_prefix(crate_root)
                .expect("owned crate path belongs to its crate");
            if relative.starts_with("tests") {
                continue;
            }
            if belongs_to_nested_package_tests(crate_root, path) {
                continue;
            }
            assert!(
                !path_has_test_material_nomenclature(relative),
                "test-only helper, fixture, or corpus escaped its owning tests tree: {}",
                path.display()
            );
        }

        let manifest = fs::read_to_string(crate_root.join("Cargo.toml"))
            .expect("read crate manifest")
            .parse::<toml::Table>()
            .expect("parse crate manifest");

        for test in manifest
            .get("test")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            let test = test.as_table().expect("test declaration is a table");
            let target = manifest_target(crate_root, &packages, test, "tests", "test");
            let relative = target
                .strip_prefix(&canonical_tests_root)
                .unwrap_or_else(|_| {
                    panic!("public test target escapes tests/: {}", target.display())
                });
            let class = relative
                .components()
                .next()
                .and_then(|component| component.as_os_str().to_str());
            assert!(
                !matches!(class, Some("fixtures" | "internal" | "support")),
                "public integration test enters private/support scope: {}",
                target.display()
            );
        }

        for bench in manifest
            .get("bench")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            let bench = bench.as_table().expect("bench declaration is a table");
            let target = manifest_target(crate_root, &packages, bench, "benches", "bench");
            let benches_root = crate_root
                .join("benches")
                .canonicalize()
                .expect("declared bench has a benches root");
            assert!(
                target.starts_with(&benches_root),
                "benchmark target escapes benches/: {}",
                target.display()
            );
        }

        for example in manifest
            .get("example")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            let example = example.as_table().expect("example declaration is a table");
            let name = example
                .get("name")
                .and_then(toml::Value::as_str)
                .expect("example declaration has a name");
            let path = example
                .get("path")
                .and_then(toml::Value::as_str)
                .unwrap_or(name);
            assert!(
                !is_benchmark_name(name) && !is_benchmark_name(path),
                "benchmark `{name}` is misclassified as an example"
            );
        }

        let examples_root = crate_root.join("examples");
        if examples_root.exists() {
            let mut examples = BTreeSet::new();
            collect_rust_sources(&examples_root, &mut examples);
            for example in examples {
                if rust_package_owner(&packages, &example) != Some(crate_root.as_path()) {
                    continue;
                }
                let relative = example
                    .strip_prefix(&examples_root)
                    .expect("example belongs to examples/");
                assert!(
                    !relative.components().any(|component| component
                        .as_os_str()
                        .to_str()
                        .is_some_and(is_benchmark_name)),
                    "benchmark-named auto example belongs under benches/: {}",
                    example.display()
                );
            }
        }

        assert!(
            canonical_tests_root.starts_with(&canonical_crate_root),
            "tests root escapes its owning crate: {}",
            tests_root.display()
        );
    }
}

#[test]
fn nested_cargo_packages_enforce_their_own_test_boundaries() {
    struct TemporaryTree(PathBuf);
    impl Drop for TemporaryTree {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove this test's temporary tree");
        }
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock follows Unix epoch")
        .as_nanos();
    let directory = std::env::temp_dir()
        .canonicalize()
        .expect("resolve temporary directory")
        .join(format!(
            "pale-cargo-ownership-{}-{nonce}",
            std::process::id()
        ));
    fs::create_dir(&directory).expect("create an unused test directory");
    let tree = TemporaryTree(directory);
    let outer = tree.0.join("sdk/example");
    let inner = outer.join("tools/helper");
    let fixture_package = outer.join("tests/fixtures/capsule");
    for owner in [&outer, &inner, &fixture_package] {
        fs::create_dir_all(owner.join("tests/internal")).unwrap();
        fs::write(owner.join("Cargo.toml"), "[package]\nname = 'example'\n").unwrap();
    }
    for (directory, manifest) in [
        (outer.join("tools/virtual"), "[workspace]\n"),
        (outer.join("target/cache"), "[package]\nname = 'cached'\n"),
        (
            outer.join(".venv/lib/external"),
            "[package]\nname = 'dependency'\n",
        ),
        (
            outer.join("__pycache__/external"),
            "[package]\nname = 'cache'\n",
        ),
        (
            inner.join("tests/fixtures/invalid"),
            "deliberately invalid Cargo input",
        ),
    ] {
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("Cargo.toml"), manifest).unwrap();
    }
    let packages = owned_rust_packages(&tree.0);
    assert_eq!(packages, BTreeSet::from([outer.clone(), inner.clone()]));
    assert_eq!(
        rust_package_owner(&packages, &fixture_package),
        Some(outer.as_path())
    );
    assert_eq!(
        rust_package_owner(&packages, &inner.join("tests/fixtures/invalid")),
        Some(inner.as_path())
    );
    assert_eq!(
        rust_package_owner(&packages, &inner.join("tests")),
        Some(inner.as_path())
    );
    assert_eq!(
        rust_package_owner(&packages, &outer.join("tools/helper-other/tests")),
        Some(outer.as_path())
    );

    let tests = inner.join("tests").canonicalize().unwrap();
    let source = inner.join("src/main.rs");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(
        tests.join("internal/model.rs"),
        "#[test] fn behavior() {}\n",
    )
    .unwrap();
    fs::write(tests.join("model.rs"), "#[test] fn behavior() {}\n").unwrap();
    fs::write(outer.join("tests/internal/other.rs"), "").unwrap();
    fs::write(
        &source,
        "#[cfg(test)]\n#[path = \"../tests/internal/model.rs\"]\nmod tests;\n",
    )
    .unwrap();
    validate_rust_organization(&inner, &source, &tests);
    validate_rust_organization(&inner, &tests.join("internal/model.rs"), &tests);
    for rejected in [
        "#[test] fn misplaced() {}\n",
        "#[cfg(test)]\n#[path = \"../tests/model.rs\"]\nmod tests;\n",
        "#[cfg(test)]\n#[path = \"../../../tests/internal/other.rs\"]\nmod tests;\n",
    ] {
        fs::write(&source, rejected).unwrap();
        assert!(
            std::panic::catch_unwind(|| validate_rust_organization(&inner, &source, &tests))
                .is_err()
        );
    }
    let fixture_source = fixture_package.join("src/fixture.rs");
    fs::create_dir_all(fixture_source.parent().unwrap()).unwrap();
    fs::write(&fixture_source, "#[test] fn fixture_test() {}\n").unwrap();
    validate_rust_organization(&outer, &fixture_source, &outer.join("tests"));
    for environment in [".venv", "__pycache__"] {
        let external = outer.join(environment).join("lib/external");
        fs::create_dir_all(&external).unwrap();
        fs::write(external.join("test_cases.py"), "dependency tests").unwrap();
        fs::write(external.join("ffi.rs"), "#[test] fn external_test() {}\n").unwrap();
    }
    let mut authored_paths = Vec::new();
    collect_paths(&outer, &mut authored_paths);
    let mut authored_rust = BTreeSet::new();
    collect_rust_sources(&outer, &mut authored_rust);
    assert!(authored_paths.contains(&source));
    assert!(authored_rust.contains(&source));
    for path in authored_paths.iter().chain(authored_rust.iter()) {
        assert!(!path
            .components()
            .any(|part| matches!(part.as_os_str().to_str(), Some(".venv" | "__pycache__"))));
    }
    let foreign = inner.join("tests/case.rs");
    fs::write(&foreign, "").unwrap();
    let target = "name = 'case'\npath = 'tools/helper/tests/case.rs'\n"
        .parse::<toml::Table>()
        .unwrap();
    for (directory, kind) in [("tests", "test"), ("benches", "bench")] {
        assert!(std::panic::catch_unwind(|| manifest_target(
            &outer, &packages, &target, directory, kind
        ))
        .is_err());
        assert_eq!(
            manifest_target(
                &inner,
                &packages,
                &toml::Table::from_iter([
                    ("name".into(), toml::Value::String("case".into())),
                    ("path".into(), toml::Value::String("tests/case.rs".into())),
                ]),
                directory,
                kind
            ),
            foreign
        );
    }
}

fn first_function_attribute_matches(source: &str, predicate: fn(&Attribute) -> bool) -> bool {
    let syntax = syn::parse_file(source).expect("parse synthetic Rust item");
    let Item::Fn(function) = &syntax.items[0] else {
        panic!("synthetic Rust item is not a function");
    };
    predicate(&function.attrs[0])
}

#[test]
fn test_cfg_detection_is_structural_and_polarity_aware() {
    for source in [
        "#[cfg(test)] fn item() {}",
        "#[cfg(all(test, unix))] fn item() {}",
        "#[cfg(all(test, not(unix)))] fn item() {}",
        "#[cfg(not(not(test)))] fn item() {}",
    ] {
        assert!(first_function_attribute_matches(
            source,
            is_positive_test_cfg
        ));
    }
    for source in [
        "#[cfg(not(test))] fn item() {}",
        "#[cfg(any(test, unix))] fn item() {}",
        "#[cfg(feature = \"contest\")] fn item() {}",
    ] {
        assert!(!first_function_attribute_matches(
            source,
            is_positive_test_cfg
        ));
    }
}

#[test]
fn test_attribute_detection_ignores_comments_strings_and_nearby_names() {
    for source in [
        "#[test] fn item() {}",
        "#[tokio::test] async fn item() {}",
        "#[rstest] fn item() {}",
        "#[test_case(1)] fn item(value: u8) {}",
        "#[cfg_attr(test, proptest)] fn item() {}",
    ] {
        assert!(first_function_attribute_matches(
            source,
            attribute_enables_test
        ));
    }
    assert!(!first_function_attribute_matches(
        "#[example::testing] fn item() {}",
        attribute_enables_test
    ));

    let syntax = syn::parse_file(
        r##"
        const TEXT: &str = r#"#[test]"#;
        /* #[tokio::test] */
        fn ordinary() {}
        "##,
    )
    .expect("parse comments and raw string");
    assert!(syntax.attrs.is_empty());
    assert!(syntax.items.iter().all(|item| match item {
        Item::Const(item) => item.attrs.is_empty(),
        Item::Fn(item) => item.attrs.is_empty(),
        _ => false,
    }));
    assert!(!is_test_material_name("contest.rs"));
    assert!(is_test_material_name("test_support.rs"));
    assert!(is_test_material_name("synthetic-contracts"));
}

#[test]
fn rustdoc_math_detection_is_structural() {
    let crate_root = Path::new("/synthetic/crate");
    let source_path = crate_root.join("src/lib.rs");
    let source = r##"
        const TEXT: &str = r#"#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/math/fake.display.mathml"))]"#;
        // #[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/math/comment.display.mathml"))]
        /// The owner exposes a rendered law below.
        #[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/math/law.display.mathml"))]
        pub struct Law;
    "##;
    let inspected = inspect_rustdoc_math(crate_root, &source_path, source);
    assert_eq!(
        inspected.includes,
        [RustdocMathInclude {
            member: "law.display.mathml".to_owned(),
            source_path,
            location: RustLocation::Source,
        }]
    );
    assert!(inspected.raw_delimiters.is_empty());
}

#[test]
fn rustdoc_math_detection_rejects_every_raw_delimiter_form() {
    for (index, delimiter) in RAW_RUSTDOC_MATH_DELIMITERS.into_iter().enumerate() {
        let crate_root = Path::new("/synthetic/crate");
        let source_path = crate_root.join(format!("src/raw-{index}.rs"));
        let documentation = format!("law {delimiter} x");
        let source = format!("#[doc = {documentation:?}] pub struct Law;");
        let inspected = inspect_rustdoc_math(crate_root, &source_path, &source);
        assert_eq!(inspected.raw_delimiters.len(), 1, "{delimiter}");
    }
    let inspected = inspect_rustdoc_math(
        Path::new("/synthetic/crate"),
        Path::new("/synthetic/crate/src/block.rs"),
        "/** Raw \\(x\\). */ pub struct Law;",
    );
    assert_eq!(inspected.raw_delimiters.len(), 2);
}

#[test]
fn rustdoc_math_detection_rejects_every_raw_symbol() {
    for (index, symbol) in RAW_RUSTDOC_MATH_SYMBOLS.into_iter().enumerate() {
        let crate_root = Path::new("/synthetic/crate");
        let source_path = crate_root.join(format!("src/raw-symbol-{index}.rs"));
        let documentation = format!("law {symbol} x");
        let source = format!("#[doc = {documentation:?}] pub struct Law;");
        let inspected = inspect_rustdoc_math(crate_root, &source_path, &source);
        assert_eq!(inspected.raw_delimiters.len(), 1, "{symbol}");
    }
}

#[test]
fn rustdoc_math_detection_rejects_formula_shapes_without_named_source_spelling() {
    let cases = [
        "/// `f(k + 1) = f(k) + 1`\npub struct Recurrence;",
        "/// The traversal costs `O(|V| + |E|)`.\npub struct Complexity;",
        "/// The meet obeys `A ∩ B ⊆ A`.\npub struct SetLaw;",
        "/// Ready -> Pending -> Complete\npub struct StateMachine;",
        "/// S --derive--> T --commit--> U\npub struct Transition;",
        "/// `mu X. roots union D(X)`\npub struct FixedPoint;",
        "/// At least `4rp + (5c + tp)` bytes are copied.\npub struct Bound;",
    ];
    for (index, source) in cases.into_iter().enumerate() {
        let inspected = inspect_rustdoc_math(
            Path::new("/synthetic/crate"),
            &Path::new("/synthetic/crate/src").join(format!("formula-{index}.rs")),
            source,
        );
        assert_eq!(inspected.raw_delimiters.len(), 1, "{source}");
    }
}

#[test]
fn rustdoc_math_detection_rejects_noncanonical_mathml_include() {
    let crate_root = Path::new("/synthetic/crate");
    let source_path = crate_root.join("src/noncanonical.rs");
    let source = r#"
        #[doc = include_str!("../law.mathml")]
        pub struct Law;
    "#;
    let inspected = inspect_rustdoc_math(crate_root, &source_path, source);
    assert!(inspected.includes.is_empty());
    assert_eq!(inspected.raw_delimiters.len(), 1);
    assert!(inspected.raw_delimiters[0].contains("noncanonical"));
}

#[test]
fn rustdoc_math_detection_preserves_code_configuration_and_pipeline_examples() {
    let source = r#"
        /// The setting is `state_scope = durable`; `0..=10` is inclusive.
        /// Authorization uses `requirement=digest`.
        /// captured documents -> parsed source -> admitted universe
        /// ```rust
        /// let value = builder.finish()?;
        /// assert_eq!(value.len(), 1);
        /// ```
        pub struct OrdinaryDocumentation;
    "#;
    let inspected = inspect_rustdoc_math(
        Path::new("/synthetic/crate"),
        Path::new("/synthetic/crate/src/ordinary.rs"),
        source,
    );
    assert!(inspected.raw_delimiters.is_empty());
}

#[test]
fn rustdoc_math_ownership_rejects_missing_duplicate_and_nonexistent_members() {
    let expected = BTreeSet::from(["law.display.mathml".to_owned()]);
    let owner = RustdocMathInclude {
        member: "law.display.mathml".to_owned(),
        source_path: PathBuf::from("/synthetic/crate/src/lib.rs"),
        location: RustLocation::Source,
    };
    assert!(validate_math_ownership(&expected, &[])
        .expect_err("missing owner must fail")
        .contains("0 owning includes"));
    assert!(
        validate_math_ownership(&expected, &[owner.clone(), owner.clone()])
            .expect_err("duplicate owner must fail")
            .contains("2 owning includes")
    );

    let nonexistent = RustdocMathInclude {
        member: "missing.display.mathml".to_owned(),
        ..owner.clone()
    };
    assert!(validate_math_ownership(&expected, &[nonexistent])
        .expect_err("nonexistent include must fail")
        .contains("nonexistent"));

    let test_owner = RustdocMathInclude {
        location: RustLocation::Tests,
        ..owner
    };
    assert!(validate_math_ownership(&expected, &[test_owner])
        .expect_err("test-owned law must fail")
        .contains("outside the owning src/ tree"));
}
