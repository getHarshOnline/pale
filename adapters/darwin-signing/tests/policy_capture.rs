//! Actual protected files through the native policy ABI, without key lookup.
#![cfg(target_os = "macos")]

use pale_adapter_darwin_signing::{
    pale_darwin_signing_capture_policy_v1, SigningBridgeRequest, SigningPolicyCapture,
};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::Path,
};

fn capture(path: &Path) -> SigningPolicyCapture {
    let bytes = path.to_str().unwrap().as_bytes();
    let mut request = SigningBridgeRequest {
        length: bytes.len() as u32,
        bytes: [0; 4096],
    };
    request.bytes[..bytes.len()].copy_from_slice(bytes);
    pale_darwin_signing_capture_policy_v1(request)
}

#[test]
fn exact_policy_bytes_and_unsafe_or_unclosed_sources_stay_distinct() {
    let owned = tempfile::tempdir().unwrap();
    let root = owned.path().canonicalize().unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.join("policy.json");
    let policy = json!({"schema_version":1,"client_requirement":"fixture-only-rule",
        "client_uid":rustix::process::geteuid().as_raw(),"maximum_connections":1,"slots":[{
            "alias":"fixture","service":"fixture-service","account":"fixture-account",
            "public_key":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","allowed_kinds":["operation_handle"]}]});
    let bytes = serde_json::to_vec_pretty(&policy).unwrap();
    fs::write(&path, &bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let captured = capture(&path);
    assert_eq!(captured.status, 0);
    let captured_bytes = [
        captured.block0.as_slice(),
        captured.block1.as_slice(),
        captured.block2.as_slice(),
        captured.block3.as_slice(),
    ]
    .concat();
    assert_eq!(&captured_bytes[..captured.length as usize], bytes);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(capture(&path).status, 3);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o644
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let linked = root.join("linked.json");
    symlink(&path, &linked).unwrap();
    assert_eq!(capture(&linked).status, 2);
    let hard = root.join("hard.json");
    fs::hard_link(&path, &hard).unwrap();
    assert_eq!(capture(&path).status, 3);
    fs::remove_file(hard).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(capture(&path).status, 3);
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&path, vec![b' '; 16 * 1024 + 1]).unwrap();
    assert_eq!(capture(&path).status, 4);
    let mut invalid = policy;
    invalid["seed"] = json!("forbidden");
    fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let refused = capture(&path);
    assert_eq!(refused.status, 5);
    assert_eq!(refused.length, 0);
    assert!([
        refused.block0,
        refused.block1,
        refused.block2,
        refused.block3
    ]
    .iter()
    .flatten()
    .all(|b| *b == 0));
}
