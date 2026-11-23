//! Actual owner-private files and sockets for explicit endpoint discovery.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::{
    fs,
    num::NonZeroU32,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use pale_adapter_uds::{
    EndpointConfigurationError, LocalEndpointConfiguration, UnixSocketListener,
};
use pale_ports::{LocalIpcFrameLimits, LocalIpcFramePort};

fn private_fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let file = root.join("machine.json");
    let endpoint = root.join("resident.sock");
    write(
        &file,
        &serde_json::json!({"schema_version":1,"endpoint":endpoint}),
    );
    (temp, file, endpoint)
}

fn write(file: &Path, value: &serde_json::Value) {
    fs::write(file, serde_json::to_vec(value).unwrap()).unwrap();
    fs::set_permissions(file, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn explicit_configuration_connects_to_the_exact_native_endpoint() {
    let (_temp, file, endpoint) = private_fixture();
    let limits = LocalIpcFrameLimits::new(NonZeroU32::new(1024).unwrap());
    let configuration = LocalEndpointConfiguration::load(&file).unwrap();
    assert_eq!(configuration.verified_path().unwrap(), endpoint);
    let listener = UnixSocketListener::bind(&endpoint, limits).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut client = configuration.connect(limits, deadline).unwrap();
    let mut server = listener.accept(deadline).unwrap();
    client.send_frame(b"exact endpoint", deadline).unwrap();
    assert_eq!(&*server.receive_frame(deadline).unwrap(), b"exact endpoint");
}

#[test]
fn unsafe_files_and_document_shapes_are_refused_without_repair() {
    let (_temp, file, endpoint) = private_fixture();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        LocalEndpointConfiguration::load(&file),
        Err(EndpointConfigurationError::Protection)
    ));
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o644
    );
    for value in [
        serde_json::json!({"schema_version":2,"endpoint":endpoint}),
        serde_json::json!({"schema_version":1,"endpoint":"relative.sock"}),
        serde_json::json!({"schema_version":1,"endpoint":endpoint,"fallback":"/tmp/guess.sock"}),
    ] {
        write(&file, &value);
        assert!(LocalEndpointConfiguration::load(&file).is_err());
    }
    fs::write(&file, vec![b' '; 16 * 1024 + 1]).unwrap();
    assert!(matches!(
        LocalEndpointConfiguration::load(&file),
        Err(EndpointConfigurationError::Document)
    ));
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    assert!(matches!(
        LocalEndpointConfiguration::load(&file),
        Err(EndpointConfigurationError::Protection)
    ));
}

#[test]
fn links_and_public_parent_directories_cannot_supply_configuration() {
    let (_temp, file, endpoint) = private_fixture();
    let parent = file.parent().unwrap();
    let linked = parent.join("linked.json");
    symlink(&file, &linked).unwrap();
    assert!(LocalEndpointConfiguration::load(&linked).is_err());
    let hard = parent.join("hard.json");
    fs::hard_link(&file, &hard).unwrap();
    assert!(matches!(
        LocalEndpointConfiguration::load(&file),
        Err(EndpointConfigurationError::Protection)
    ));
    fs::remove_file(hard).unwrap();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        LocalEndpointConfiguration::load(&file),
        Err(EndpointConfigurationError::Protection)
    ));
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).unwrap();
    let alias = parent.join("alias");
    symlink(parent, &alias).unwrap();
    write(
        &file,
        &serde_json::json!({"schema_version":1,"endpoint":alias.join(endpoint.file_name().unwrap())}),
    );
    assert!(LocalEndpointConfiguration::load(&file).is_err());
}

#[test]
fn a_replaced_endpoint_parent_does_not_rebind_the_captured_selection() {
    let (_temp, file, _) = private_fixture();
    let root = file.parent().unwrap();
    let directory = root.join("runtime");
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    write(
        &file,
        &serde_json::json!({"schema_version":1,"endpoint":directory.join("resident.sock")}),
    );
    let configuration = LocalEndpointConfiguration::load(&file).unwrap();
    fs::rename(&directory, root.join("retained")).unwrap();
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(
        configuration.verified_path(),
        Err(EndpointConfigurationError::Path)
    ));
}
