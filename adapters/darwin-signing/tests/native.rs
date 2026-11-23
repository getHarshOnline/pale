//! Ad-hoc-signed native XPC and independent `CryptoKit` verification, no Keychain.
#![cfg(target_os = "macos")]

mod support;
use std::process::Command;
use support::{sign, swift, NativeFixture};

#[test]
#[ignore = "requires explicitly selected public-fixture static library, Swift and ad-hoc codesign"]
fn native_signed_xpc_roundtrip_and_refusals() {
    let fixture = NativeFixture::capture();
    let source = &fixture.source;
    let owned = &fixture.owned;
    let library = &fixture.library;
    let executable = fixture.root.join("signing-native-fixture");
    let client_only = swift()
        .args([
            "-parse-as-library",
            "-emit-module",
            "-module-name",
            "PaleSigningClient",
        ])
        .arg(source.join("native/SigningProtocol.swift"))
        .arg(source.join("native/SigningClient.swift"))
        .arg("-emit-module-path")
        .arg(owned.path().join("PaleSigningClient.swiftmodule"))
        .output()
        .unwrap();
    assert!(
        client_only.status.success(),
        "independent client module: {}",
        String::from_utf8_lossy(&client_only.stderr)
    );
    let compiled = swift()
        .arg("-import-objc-header")
        .arg(source.join("include/pale_signing.h"))
        .arg(source.join("native/SigningProtocol.swift"))
        .arg(source.join("native/SigningHelper.swift"))
        .arg(source.join("native/SigningClient.swift"))
        .arg(source.join("tests/native/main.swift"))
        .arg(library)
        .args([
            "-framework",
            "Foundation",
            "-framework",
            "Security",
            "-framework",
            "CryptoKit",
            "-o",
        ])
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "Swift compilation: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let requirement = sign(&executable, "io.rtepl.pale.signing-qualification");
    let observed = Command::new(&executable)
        .arg(requirement)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", owned.path())
        .current_dir(owned.path())
        .output()
        .unwrap();
    let output = String::from_utf8(observed.stdout).unwrap();
    assert!(
        observed.status.success(),
        "native XPC fixture: {}\n{output}",
        String::from_utf8_lossy(&observed.stderr)
    );
    let result: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(
        result["keychain_access"],
        "none: compiled public RFC fixture only"
    );
    assert_eq!(result["failures"], 0);
    assert_eq!(result["native_cases_passed"], 19);
    println!("{output}");
}
