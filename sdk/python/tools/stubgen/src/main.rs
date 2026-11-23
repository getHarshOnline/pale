//! Render the Python type declarations for the `_native` module of `pale-python`
//! from its compiled PyO3 metadata.
//!
//! The tool augments the collected metadata before the pinned renderer runs.
//! Missing constructors on these native classes mean runtime construction is
//! unavailable, not Python's implicit zero-argument constructor. No source or
//! generated-stub text is parsed to determine that policy.

use pyo3_introspection::model::{Argument, Arguments, Class, Expr, Function, Module};
use std::error::Error;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let library = PathBuf::from(arguments.next().ok_or("expected native library path")?);
    let output = PathBuf::from(arguments.next().ok_or("expected output stub path")?);
    if arguments.next().is_some() {
        return Err("expected exactly a native library and an output stub path".into());
    }
    let mut module = pyo3_introspection::introspect_cdylib(library, "_native")?;
    if module.classes.is_empty() {
        return Err("native module has no collected PyO3 classes".into());
    }
    restrict_factory_constructors(&mut module)?;
    let mut files = pyo3_introspection::module_stub_files(&module);
    let stub = files
        .remove(Path::new("__init__.pyi"))
        .ok_or("renderer did not emit the expected native module stub")?;
    if !files.is_empty() {
        return Err("native submodule layout requires explicit packaging support".into());
    }
    std::fs::write(output, stub)?;
    Ok(())
}

fn restrict_factory_constructors(module: &mut Module) -> Result<(), String> {
    // PyO3 collects class method fragments before returning this model. Module
    // incompleteness describes other unknown exports (such as create_exception!),
    // not missing method fragments on the concrete Class records visited here.
    // Preserve that flag and leave unknown exports outside this augmentation.
    for class in &mut module.classes {
        restrict_class(class)?;
    }
    for child in &mut module.modules {
        restrict_factory_constructors(child)?;
    }
    Ok(())
}

fn restrict_class(class: &mut Class) -> Result<(), String> {
    let has_constructor = class
        .methods
        .iter()
        .any(|method| matches!(method.name.as_str(), "__new__" | "__init__"));
    if !has_constructor {
        // Inherited constructors need their own qualification. Never guess that
        // an arbitrary base class is factory-only from missing local metadata.
        if !class.bases.is_empty() {
            return Err(format!(
                "class {} has unqualified inherited construction",
                class.name
            ));
        }
        class.methods.push(forbidden_constructor());
    }
    for child in &mut class.inner_classes {
        restrict_class(child)?;
    }
    Ok(())
}

fn forbidden_constructor() -> Function {
    let no_return = Expr::Attribute {
        value: Box::new(Expr::Name {
            id: "typing".into(),
        }),
        attr: "NoReturn".into(),
    };
    Function {
        name: "__new__".into(),
        decorators: vec![],
        arguments: Arguments {
            positional_only_arguments: vec![
                Argument {
                    name: "cls".into(),
                    default_value: None,
                    annotation: None,
                },
                Argument {
                    name: "_factory_only".into(),
                    default_value: None,
                    annotation: Some(no_return.clone()),
                },
            ],
            arguments: vec![],
            vararg: None,
            keyword_only_arguments: vec![],
            kwarg: None,
        },
        returns: Some(no_return),
        is_async: false,
        docstring: Some(
            "Created by SDK results or named factories, not direct construction.".into(),
        ),
    }
}

#[cfg(test)]
#[path = "../tests/internal/model.rs"]
mod tests;
