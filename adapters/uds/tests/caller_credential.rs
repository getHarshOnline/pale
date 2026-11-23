//! File credentials sign only the selected caller-to-resident proof.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use ed25519_dalek::{Signature, SigningKey};
use pale_adapter_uds::FileCallerCredential;
use pale_ports::LocalConnectionSignerPort;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};

#[test]
fn signer_rejects_other_domains_peers_shapes_and_expired_deadlines() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap().join("caller.key");
    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&path, [17; 32]).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let caller = SigningKey::from_bytes(&[17; 32]).verifying_key();
    let resident = SigningKey::from_bytes(&[29; 32]).verifying_key().to_bytes();
    let mut signer = FileCallerCredential::load(&path, caller.to_bytes(), resident).unwrap();
    let mut message = b"pale.local-connection.caller-request.v1\0".to_vec();
    message.extend_from_slice(caller.as_bytes());
    message.extend_from_slice(&resident);
    message.extend_from_slice(&[41; 96]);
    let later = || Instant::now() + Duration::from_secs(2);
    let signature = signer.sign(&message, later()).unwrap();
    caller
        .verify_strict(&message, &Signature::from_bytes(&signature))
        .unwrap();
    assert!(signer.sign(&message, Instant::now()).is_err());
    for index in [0, message.len() - 160, message.len() - 128] {
        let mut changed = message.clone();
        changed[index] ^= 1;
        assert!(signer.sign(&changed, later()).is_err());
    }
    assert!(signer.sign(&message[..message.len() - 1], later()).is_err());
    message.push(0);
    assert!(signer.sign(&message, later()).is_err());
    assert_eq!(
        format!("{signer:?}"),
        "FileCallerCredential(<private transport credential>)"
    );
}

#[test]
fn credential_capture_refuses_wrong_key_shape_and_protection() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap().join("caller.key");
    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o700)).unwrap();
    let public = SigningKey::from_bytes(&[17; 32]).verifying_key().to_bytes();
    for length in [0, 31, 33, 1024] {
        fs::write(&path, vec![17; length]).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(FileCallerCredential::load(&path, public, public).is_err());
    }
    fs::write(&path, [17; 32]).unwrap();
    assert!(FileCallerCredential::load(&path, [0; 32], public).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(FileCallerCredential::load(&path, public, public).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let link = path.with_extension("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(FileCallerCredential::load(&link, public, public).is_err());
    fs::remove_file(&link).unwrap();
    fs::hard_link(&path, &link).unwrap();
    assert!(FileCallerCredential::load(&path, public, public).is_err());
}
