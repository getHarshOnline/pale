//! Verify constructor restrictions without changing other collected metadata.

use super::*;

fn class(name: &str) -> Class {
    Class {
        name: name.into(),
        bases: vec![],
        methods: vec![],
        attributes: vec![],
        decorators: vec![],
        inner_classes: vec![],
        docstring: Some("Original class documentation.".into()),
    }
}

fn module(classes: Vec<Class>) -> Module {
    Module {
        name: "_native".into(),
        modules: vec![],
        classes,
        functions: vec![],
        attributes: vec![],
        incomplete: true,
        docstring: Some("Unknown external exception remains incomplete.".into()),
    }
}

#[test]
fn absent_constructor_gets_required_bottom_argument_before_rendering() {
    let mut value = module(vec![class("Result")]);
    restrict_factory_constructors(&mut value).unwrap();
    assert_eq!(value.classes[0].methods, vec![forbidden_constructor()]);
    let stub = pyo3_introspection::module_stub_files(&value);
    let stub = &stub[Path::new("__init__.pyi")];
    assert!(stub.contains("def __new__(cls, _factory_only: NoReturn, /) -> NoReturn:"));
    assert!(stub.contains("from typing import"));
    let once = value.clone();
    restrict_factory_constructors(&mut value).unwrap();
    assert_eq!(value, once);
}

#[test]
fn explicit_new_and_init_and_inherited_real_constructor_are_unchanged() {
    for name in ["__new__", "__init__"] {
        let mut value = class("Constructible");
        let mut constructor = forbidden_constructor();
        constructor.name = name.into();
        constructor.arguments.positional_only_arguments.truncate(1);
        constructor.returns = None;
        value.methods.push(constructor);
        value.bases.push(Expr::Name { id: "Enum".into() });
        let before = value.clone();
        restrict_class(&mut value).unwrap();
        assert_eq!(value, before);
    }
}

#[test]
fn named_factories_members_and_all_nonconstructor_metadata_are_preserved() {
    let mut value = class("MemberStyleEnum");
    let mut factory = forbidden_constructor();
    factory.name = "from_value".into();
    factory.decorators.push(Expr::Name {
        id: "staticmethod".into(),
    });
    value.methods.push(factory);
    value.attributes.push(pyo3_introspection::model::Attribute {
        name: "Available".into(),
        value: None,
        annotation: None,
        docstring: None,
    });
    value.inner_classes.push(class("NestedResult"));
    let mut value = module(vec![value]);
    let before = value.clone();
    restrict_factory_constructors(&mut value).unwrap();
    assert_eq!(
        value.classes[0].methods.pop(),
        Some(forbidden_constructor())
    );
    assert_eq!(
        value.classes[0].inner_classes[0].methods.pop(),
        Some(forbidden_constructor())
    );
    assert_eq!(value, before);
}

#[test]
fn unqualified_inheritance_is_not_silently_reclassified() {
    let mut value = class("UnknownBase");
    value.bases.push(Expr::Name {
        id: "ExternalBase".into(),
    });
    let before = value.clone();
    assert!(restrict_class(&mut value).is_err());
    assert_eq!(value, before);
}
