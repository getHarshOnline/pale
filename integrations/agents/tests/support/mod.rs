//! Native test connections use isolated keys; application authority is modeled.

use ed25519_dalek::{Signer as _, SigningKey};
use pale_agent_host::LocalAgentCarrierSource;
use pale_ports::{
    CryptographicRandomSourceError, CryptographicRandomSourcePort, LocalConnectionSignerPort,
    LocalConnectionSigningError, LocalIpcFrameLimits,
};
use std::{
    fs,
    num::NonZeroU32,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, Instant},
};

pub(super) struct Fixture {
    _directory: tempfile::TempDir,
    pub(super) directory: PathBuf,
    pub(super) configuration: PathBuf,
    pub(super) endpoint: PathBuf,
    credential: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().canonicalize().unwrap().join("runtime");
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let configuration = directory.join("connection.json");
        let endpoint = directory.join("resident.sock");
        fs::write(
            &configuration,
            serde_json::to_vec(&serde_json::json!({"schema_version":1,"endpoint":endpoint}))
                .unwrap(),
        )
        .unwrap();
        fs::set_permissions(&configuration, fs::Permissions::from_mode(0o600)).unwrap();
        let credential = directory.join("caller.key");
        fs::write(&credential, [17; 32]).unwrap();
        fs::set_permissions(&credential, fs::Permissions::from_mode(0o600)).unwrap();
        Self {
            credential,
            _directory: temporary,
            directory,
            configuration,
            endpoint,
        }
    }

    pub(super) fn source(&self, resident_key: [u8; 32]) -> LocalAgentCarrierSource {
        LocalAgentCarrierSource::load(
            self.configuration.to_str().unwrap(),
            self.credential.to_str().unwrap(),
            FixtureSigner::caller().public_key(),
            resident_key,
        )
        .unwrap()
    }
}

pub(super) fn limits() -> LocalIpcFrameLimits {
    LocalIpcFrameLimits::new(NonZeroU32::new(1 << 20).unwrap())
}
pub(super) fn later() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

pub(super) struct FixtureSigner(SigningKey);
impl FixtureSigner {
    pub(super) fn caller() -> Self {
        Self(SigningKey::from_bytes(&[17; 32]))
    }
    pub(super) fn resident() -> Self {
        Self(SigningKey::from_bytes(&[29; 32]))
    }
}
impl LocalConnectionSignerPort for FixtureSigner {
    fn public_key(&self) -> [u8; 32] {
        self.0.verifying_key().to_bytes()
    }
    fn sign(
        &mut self,
        message: &[u8],
        deadline: Instant,
    ) -> Result<[u8; 64], LocalConnectionSigningError> {
        if Instant::now() >= deadline {
            return Err(LocalConnectionSigningError);
        }
        Ok(self.0.sign(message).to_bytes())
    }
}
pub(super) struct OsEntropy;
impl CryptographicRandomSourcePort for OsEntropy {
    fn fill_bytes(&self, bytes: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        getrandom::fill(bytes).map_err(|_| CryptographicRandomSourceError::unavailable())
    }
}
