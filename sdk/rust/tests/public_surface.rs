//! Structural guards against wrong-layer public signature leakage.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{
    Fields, ImplItem, Item, ItemImpl, Signature, TraitItem, Type, TypePath, UseTree, Visibility,
};

fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut sources = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).expect("API source directory is readable") {
            let path = entry.expect("directory entry is readable").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources
}

#[derive(Default)]
struct ImportAliases {
    paths: BTreeMap<String, String>,
}

impl ImportAliases {
    fn collect(file: &syn::File) -> Self {
        let mut aliases = Self::default();
        collect_item_imports(&file.items, &mut aliases);
        aliases
    }

    fn resolve(&self, path: &syn::Path) -> String {
        let mut segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string());
        let Some(first) = segments.next() else {
            return String::new();
        };
        let mut resolved = self.paths.get(&first).cloned().unwrap_or(first);
        for segment in segments {
            resolved.push_str("::");
            resolved.push_str(&segment);
        }
        resolved
    }
}

fn collect_item_imports(items: &[Item], aliases: &mut ImportAliases) {
    for item in items {
        match item {
            Item::Use(item_use) => collect_use_tree(&item_use.tree, "", aliases),
            Item::Mod(item_mod) => {
                if let Some((_, items)) = &item_mod.content {
                    collect_item_imports(items, aliases);
                }
            }
            _ => {}
        }
    }
}

fn collect_use_tree(tree: &UseTree, prefix: &str, aliases: &mut ImportAliases) {
    match tree {
        UseTree::Path(path) => {
            let next = join_path(prefix, &path.ident.to_string());
            collect_use_tree(&path.tree, &next, aliases);
        }
        UseTree::Name(name) => {
            let imported = join_path(prefix, &name.ident.to_string());
            aliases.paths.insert(name.ident.to_string(), imported);
        }
        UseTree::Rename(rename) => {
            let imported = join_path(prefix, &rename.ident.to_string());
            aliases.paths.insert(rename.rename.to_string(), imported);
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, prefix, aliases);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn join_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_owned()
    } else {
        format!("{prefix}::{segment}")
    }
}

fn is_public(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

struct PublicTypeGate<'a> {
    aliases: &'a ImportAliases,
    context: String,
    violations: Vec<String>,
    allowed_port_traits: &'static [&'static str],
}

impl PublicTypeGate<'_> {
    fn inspect_type(&mut self, ty: &Type) {
        self.visit_type(ty);
    }

    fn inspect_signature(&mut self, signature: &Signature) {
        if signature
            .generics
            .params
            .iter()
            .any(|parameter| !matches!(parameter, syn::GenericParam::Lifetime(_)))
        {
            self.reject("public type or const generic parameters are not a closed projection");
        }
        for input in &signature.inputs {
            if let syn::FnArg::Typed(input) = input {
                self.inspect_type(&input.ty);
            }
        }
        if let syn::ReturnType::Type(_, output) = &signature.output {
            self.inspect_type(output);
        }
        if let Some(clause) = &signature.generics.where_clause {
            for predicate in &clause.predicates {
                self.visit_where_predicate(predicate);
            }
        }
    }

    fn reject(&mut self, detail: &str) {
        self.violations.push(format!("{}: {detail}", self.context));
    }
}

impl<'ast> Visit<'ast> for PublicTypeGate<'_> {
    fn visit_type_path(&mut self, path: &'ast TypePath) {
        let resolved = self.aliases.resolve(&path.path);
        let forbidden = resolved.starts_with("pale_application::")
            || resolved == "pale_application"
            || resolved.starts_with("pale_adapter_")
            || resolved.starts_with("pale_core::")
            || resolved == "pale_core"
            || resolved.starts_with("pale_ports::")
            || resolved == "pale_ports"
            || resolved == "serde_json::Value"
            || resolved == "std::path::Path"
            || resolved == "std::path::PathBuf"
            || resolved.ends_with("::MaterializationPlanV1")
            || resolved == "MaterializationPlanV1"
            || resolved.ends_with("::CompiledGeneration")
            || resolved == "CompiledGeneration";
        if forbidden {
            self.reject(&format!("forbidden public type `{resolved}`"));
        }
        visit::visit_type_path(self, path);
    }

    fn visit_type_trait_object(&mut self, ty: &'ast syn::TypeTraitObject) {
        if self.context == "resident provisioning port `AuthenticatedResidentCarrierSource::open`" {
            visit::visit_type_trait_object(self, ty);
            return;
        }
        let admitted = !self.allowed_port_traits.is_empty()
            && ty.bounds.iter().all(|bound| {
                let syn::TypeParamBound::Trait(bound) = bound else {
                    return matches!(bound, syn::TypeParamBound::Lifetime(_));
                };
                let resolved = self.aliases.resolve(&bound.path);
                self.allowed_port_traits
                    .contains(&resolved.strip_prefix("crate::").unwrap_or(&resolved))
            });
        if !admitted {
            self.reject("public `dyn Trait` erases the closed façade contract");
        }
        visit::visit_type_trait_object(self, ty);
    }

    fn visit_type_impl_trait(&mut self, ty: &'ast syn::TypeImplTrait) {
        self.reject("public `impl Trait` erases the closed façade contract");
        visit::visit_type_impl_trait(self, ty);
    }
}

fn inspect_fields(gate: &mut PublicTypeGate<'_>, fields: &Fields, inherited_public: bool) {
    for field in fields {
        if inherited_public || is_public(&field.vis) {
            gate.inspect_type(&field.ty);
        }
    }
}

fn inspect_impl(gate: &mut PublicTypeGate<'_>, item_impl: &ItemImpl) {
    let resident_attachment = matches!(item_impl.self_ty.as_ref(), Type::Path(path) if path.path.is_ident("ResidentAttachment"));
    for item in &item_impl.items {
        match item {
            ImplItem::Fn(function) if is_public(&function.vis) => {
                gate.context = format!("method `{}`", function.sig.ident);
                let name = function.sig.ident.to_string();
                let pale_api = matches!(item_impl.self_ty.as_ref(), Type::Path(path) if path.path.is_ident("PaleApi"));
                gate.allowed_port_traits = if resident_attachment
                    && matches!(
                        name.as_str(),
                        "provision_observation"
                            | "provision_operations"
                            | "provision_hooks"
                            | "provision_admitted_hooks"
                    ) {
                    &[
                        "AuthenticatedResidentCarrierSource",
                        "ResidentResponseAdmission",
                    ]
                } else if pale_api && name == "sign_resident_native_tool_request" {
                    &["ResidentNativeRequestSigner"]
                } else {
                    &[]
                };
                // Native seams accept only the named SDK ports. Their other
                // arguments still pass the same internal-type checks.
                gate.inspect_signature(&function.sig);
                gate.allowed_port_traits = &[];
            }
            ImplItem::Const(item_const) if is_public(&item_const.vis) => {
                gate.context = format!("associated const `{}`", item_const.ident);
                gate.inspect_type(&item_const.ty);
            }
            ImplItem::Type(item_type) if is_public(&item_type.vis) => {
                gate.context = format!("associated type `{}`", item_type.ident);
                gate.reject("public associated type is not a closed façade projection");
            }
            _ => {}
        }
    }
}

fn inspect_items(items: &[Item], gate: &mut PublicTypeGate<'_>) {
    for item in items {
        match item {
            Item::Fn(function) if is_public(&function.vis) => {
                gate.context = format!("function `{}`", function.sig.ident);
                gate.inspect_signature(&function.sig);
            }
            Item::Struct(item_struct) if is_public(&item_struct.vis) => {
                gate.context = format!("struct `{}`", item_struct.ident);
                if item_struct
                    .generics
                    .params
                    .iter()
                    .any(|parameter| !matches!(parameter, syn::GenericParam::Lifetime(_)))
                {
                    gate.reject(
                        "public type or const generic parameters are not a closed projection",
                    );
                }
                inspect_fields(gate, &item_struct.fields, false);
            }
            Item::Enum(item_enum) if is_public(&item_enum.vis) => {
                gate.context = format!("enum `{}`", item_enum.ident);
                if item_enum
                    .generics
                    .params
                    .iter()
                    .any(|parameter| !matches!(parameter, syn::GenericParam::Lifetime(_)))
                {
                    gate.reject(
                        "public type or const generic parameters are not a closed projection",
                    );
                }
                for variant in &item_enum.variants {
                    gate.context = format!("enum `{}::{}`", item_enum.ident, variant.ident);
                    inspect_fields(gate, &variant.fields, true);
                }
            }
            Item::Impl(item_impl) => inspect_impl(gate, item_impl),
            Item::Const(item_const) if is_public(&item_const.vis) => {
                gate.context = format!("const `{}`", item_const.ident);
                gate.inspect_type(&item_const.ty);
            }
            Item::Static(item_static) if is_public(&item_static.vis) => {
                gate.context = format!("static `{}`", item_static.ident);
                gate.inspect_type(&item_static.ty);
            }
            Item::Trait(item_trait)
                if is_public(&item_trait.vis)
                    && matches!(
                        item_trait.ident.to_string().as_str(),
                        "AuthenticatedResidentCarrier"
                            | "AuthenticatedResidentCarrierSource"
                            | "ResidentResponseAdmission"
                            | "ResidentNativeRequestSigner"
                    ) =>
            {
                for item in &item_trait.items {
                    if let TraitItem::Fn(function) = item {
                        gate.context = format!(
                            "resident provisioning port `{}::{}`",
                            item_trait.ident, function.sig.ident
                        );
                        gate.inspect_signature(&function.sig);
                    }
                }
            }
            Item::Trait(item_trait) if is_public(&item_trait.vis) => {
                gate.context = format!("trait `{}`", item_trait.ident);
                gate.reject("public trait is outside the closed façade contract");
            }
            Item::Type(item_type) if is_public(&item_type.vis) => {
                gate.context = format!("type alias `{}`", item_type.ident);
                gate.reject("public type alias is outside the closed façade contract");
            }
            Item::Use(item_use) if is_public(&item_use.vis) => {
                let mut reexports = ImportAliases::default();
                collect_use_tree(&item_use.tree, "", &mut reexports);
                for resolved in reexports.paths.values() {
                    if resolved.starts_with("pale_application::")
                        || resolved.starts_with("pale_adapter_")
                        || resolved.starts_with("pale_core::")
                        || resolved.starts_with("pale_ports::")
                    {
                        gate.context = "public re-export".to_owned();
                        gate.reject(&format!("forbidden public re-export `{resolved}`"));
                    }
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, nested)) = &item_mod.content {
                    inspect_items(nested, gate);
                }
            }
            _ => {}
        }
    }
}

fn public_surface_violations(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).expect("API Rust source parses");
    let aliases = ImportAliases::collect(&file);
    let mut gate = PublicTypeGate {
        aliases: &aliases,
        context: "public item".to_owned(),
        violations: Vec::new(),
        allowed_port_traits: &[],
    };
    inspect_items(&file.items, &mut gate);
    gate.violations
}

#[test]
fn public_signatures_do_not_leak_internal_or_host_capability_types() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let violations = rust_sources(&root)
        .into_iter()
        .flat_map(|path| {
            let source = fs::read_to_string(&path).expect("API Rust source is UTF-8");
            public_surface_violations(&source)
                .into_iter()
                .map(move |violation| format!("{}: {violation}", path.display()))
        })
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "wrong-layer public API signatures:\n{}",
        violations.join("\n")
    );
}

#[test]
fn imported_aliases_cannot_hide_internal_or_host_types() {
    let fixture = r#"
        use pale_application::compile::Compilation as AppCompilation;
        use pale_adapter_fs::ConfinedRoot as AdapterRoot;
        use std::path::Path as HostPath;
        use pale_ports::LocalIpcFrameLimits as FrameLimits;

        pub fn application_leak(value: &AppCompilation) {}
        pub fn adapter_leak(value: AdapterRoot) {}
        pub struct HostLeak {
            pub path: HostPath,
        }
        pub struct Borrowed<'a> { bytes: &'a [u8] }
        pub struct Generic<T> { value: T }
        pub struct ConstGeneric<const N: usize> { bytes: [u8; N] }
        impl ResidentAttachment {
            pub fn provision_hooks(value: AdapterRoot, limits: FrameLimits, port: Box<dyn UnownedPort>) {}
        }
    "#;

    let violations = public_surface_violations(fixture);
    assert!(violations
        .iter()
        .any(|item| item.contains("pale_application::compile::Compilation")));
    assert!(violations
        .iter()
        .any(|item| item.contains("pale_adapter_fs::ConfinedRoot")));
    assert!(violations
        .iter()
        .any(|item| item.contains("std::path::Path")));
    assert!(violations
        .iter()
        .any(|item| item.contains("struct `Generic`")));
    assert!(violations
        .iter()
        .any(|item| item.contains("struct `ConstGeneric`")));
    assert!(violations
        .iter()
        .any(|item| item.contains("method `provision_hooks`")
            && item.contains("pale_ports::LocalIpcFrameLimits")));
    assert!(!violations.iter().any(|item| item.contains("Borrowed")));
    assert!(violations
        .iter()
        .any(|item| item.contains("method `provision_hooks`") && item.contains("pale_adapter_fs")));
    assert!(violations
        .iter()
        .any(|item| item.contains("method `provision_hooks`") && item.contains("dyn Trait")));
}

#[test]
fn resident_attachment_has_explicit_read_only_and_operation_provisioning_surfaces() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/resident_attachment.rs");
    let source = fs::read_to_string(path).expect("resident attachment source is UTF-8");
    let file = syn::parse_file(&source).expect("resident attachment source parses");
    let attachment = file
        .items
        .iter()
        .find_map(|item| match item {
            Item::Struct(item_struct) if item_struct.ident == "ResidentAttachment" => {
                Some(item_struct)
            }
            _ => None,
        })
        .expect("the sealed resident attachment remains declared");
    let fields = attachment.fields.iter().collect::<Vec<_>>();
    assert!(
        !fields.is_empty(),
        "attachment is not a public unit constructor"
    );
    assert!(
        fields.iter().all(|field| !is_public(&field.vis)),
        "arbitrary callers cannot initialize attachment fields"
    );

    let public_associated_functions = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Impl(item_impl) => Some(item_impl),
            _ => None,
        })
        .filter(|item_impl| match item_impl.self_ty.as_ref() {
            Type::Path(path) => path.path.is_ident("ResidentAttachment"),
            _ => false,
        })
        .flat_map(|item_impl| &item_impl.items)
        .filter_map(|item| match item {
            ImplItem::Fn(function)
                if is_public(&function.vis) && function.sig.receiver().is_none() =>
            {
                Some(function.sig.ident.to_string())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        public_associated_functions,
        [
            "provision_admitted_hooks",
            "provision_hooks",
            "provision_observation",
            "provision_operations"
        ]
    );
    let constructing_trait_impls = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Impl(item_impl) => Some(item_impl),
            _ => None,
        })
        .filter(|item_impl| match item_impl.self_ty.as_ref() {
            Type::Path(path) => path.path.is_ident("ResidentAttachment"),
            _ => false,
        })
        .filter_map(|item_impl| item_impl.trait_.as_ref().map(|(_, path, _)| path))
        .filter_map(|path| path.segments.last())
        .map(|segment| segment.ident.to_string())
        .filter(|name| {
            ["Default", "Deserialize", "From", "FromStr", "TryFrom"].contains(&name.as_str())
        })
        .collect::<Vec<_>>();
    assert!(
        constructing_trait_impls.is_empty(),
        "ResidentAttachment unexpectedly gained constructing trait impls: {constructing_trait_impls:?}"
    );
}
