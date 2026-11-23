//! Command-line entry point for explicit contract-bundle generation and verification.

use std::path::Path;

use pale_contract_codegen::{
    generate_bundle, read_descriptor_manifest, verify_bundle, write_bundle_atomically,
};

fn main() {
    if let Err(error) = run(std::env::args_os()) {
        eprintln!("pale-contract-codegen: {error}");
        std::process::exit(1);
    }
}

fn run(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<(), String> {
    let mut args = args.into_iter();
    let _program = args.next();
    let mode = args
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(usage)?;
    let manifest_path = args.next().ok_or_else(usage)?;
    let bundle_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage());
    }

    let manifest =
        read_descriptor_manifest(Path::new(&manifest_path)).map_err(|error| error.to_string())?;
    let bundle = generate_bundle(manifest).map_err(|error| error.to_string())?;
    match mode.as_str() {
        "generate" => write_bundle_atomically(&bundle, Path::new(&bundle_path))
            .map_err(|error| error.to_string()),
        "verify" => {
            verify_bundle(&bundle, Path::new(&bundle_path)).map_err(|error| error.to_string())
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: pale-contract-codegen <generate|verify> <descriptor-manifest.json> <bundle-directory>"
        .into()
}
