//! A distinct, application-scoped native XPC helper with no installed service.
#![cfg(target_os = "macos")]

mod support;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use support::{sign, swift, NativeFixture};

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[test]
#[ignore = "requires verified public-fixture core, Swift and owned ad-hoc-signed app/XPC bundles"]
fn separate_native_service_captures_policy_and_signs() {
    let fixture = NativeFixture::capture();
    let source = &fixture.source;
    let identity = format!(
        "io.rtepl.pale.signing-process-qualification.p{}",
        std::process::id()
    );
    let service_id = format!("{identity}.helper");
    let app = fixture.root.join("SigningFixture.app/Contents");
    let service = app.join("XPCServices/SigningHelper.xpc/Contents");
    fs::create_dir_all(app.join("MacOS")).unwrap();
    fs::create_dir_all(service.join("MacOS")).unwrap();
    fs::write(app.join("Info.plist"),format!("<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>{identity}</string><key>CFBundleExecutable</key><string>signing-client</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>")).unwrap();
    let client = app.join("MacOS/signing-client");
    let built = swift()
        .arg(source.join("native/SigningProtocol.swift"))
        .arg(source.join("native/SigningClient.swift"))
        .arg(source.join("tests/process/client/main.swift"))
        .args([
            "-framework",
            "Foundation",
            "-framework",
            "Security",
            "-framework",
            "CryptoKit",
            "-o",
        ])
        .arg(&client)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "client compile: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let policy_path = fixture.root.join("signing-policy.json");
    let public = [
        0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07,
        0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07,
        0x51, 0x1a,
    ];
    let template = fs::read_to_string(source.join("native/service/Info.plist.in")).unwrap();
    let info = template
        .replace("$(PRODUCT_BUNDLE_IDENTIFIER)", &xml(&service_id))
        .replace(
            "$(PALE_SIGNING_POLICY_PATH)",
            &xml(policy_path.to_str().unwrap()),
        );
    fs::write(service.join("Info.plist"), info).unwrap();
    let helper = service.join("MacOS/pale-signing-helper");
    let built = swift()
        .args(["-D", "PALE_SIGNING_QUALIFICATION", "-import-objc-header"])
        .arg(source.join("include/pale_signing.h"))
        .arg(source.join("native/SigningProtocol.swift"))
        .arg(source.join("native/SigningHelper.swift"))
        .arg(source.join("native/SigningPolicy.swift"))
        .arg(source.join("native/service/main.swift"))
        .arg(&fixture.library)
        .args([
            "-framework",
            "Foundation",
            "-framework",
            "Security",
            "-framework",
            "CryptoKit",
            "-o",
        ])
        .arg(&helper)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "helper compile: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let helper_requirement = sign(service.parent().unwrap(), &service_id);
    let client_requirement = sign(app.parent().unwrap(), &identity);
    let policy = json!({"schema_version":1,"client_requirement":client_requirement,"client_uid":rustix::process::geteuid().as_raw(),
        "maximum_connections":2,"slots":[{"alias":"qualified","service":"qualification-only","account":"public-rfc8032-vector-1",
            "public_key":URL_SAFE_NO_PAD.encode(public),"allowed_kinds":["operation_handle"]}]});
    // Exercise all four native ABI blocks with a valid noncanonical config
    // source; policy source bytes are captured, not treated as a signed record.
    let mut policy_bytes = vec![b' '; 14 * 1024];
    policy_bytes.extend(serde_json::to_vec(&policy).unwrap());
    fs::write(&policy_path, policy_bytes).unwrap();
    fs::set_permissions(&policy_path, fs::Permissions::from_mode(0o600)).unwrap();

    let observed = Command::new(&client)
        .arg(&service_id)
        .arg(&helper_requirement)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &fixture.root)
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    let output = String::from_utf8(observed.stdout).unwrap();
    if !observed.status.success() {
        let retained = fixture.owned.keep();
        panic!(
            "separate process: {}\n{output}\nowned fixture retained for lifecycle inspection: {}",
            String::from_utf8_lossy(&observed.stderr),
            retained.display()
        );
    }
    let result: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(result["signature_verified"], true);
    let pid = result["helper_pid"].as_u64().unwrap();
    assert_ne!(pid, result["caller_pid"].as_u64().unwrap());
    let end = Instant::now() + Duration::from_secs(25);
    loop {
        let state = Command::new("/bin/ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .unwrap();
        if !state.status.success() {
            break;
        }
        if Instant::now() >= end {
            let retained = fixture.owned.keep();
            panic!(
                "owned helper did not exit; active fixture retained at {}",
                retained.display()
            );
        }
        thread::sleep(Duration::from_millis(50));
    }
    println!("{output}");
}
