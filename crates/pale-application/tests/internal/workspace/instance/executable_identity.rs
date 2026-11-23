//! Verify executable interface claims against the selected machine binding.

use std::path::PathBuf;

use pale_core::records::{
    ExecutableDigest, ExecutableIdentityDigest, InstanceDigest, ProfileDigest,
    ResolvedProjectInstanceV1, SchemaDigest, SchemaId, SurfaceExecutableBinding,
};
use pale_ports::{
    ExecutableIdentityError, ExecutableIdentityPort, ExecutableIdentityRequest,
    ExecutableObservation, IdentityProbeRequest, ProbeObservation, ProcessProbeError,
    ProcessProbePort,
};

use super::verify_instance_executable_binding;

struct SurfaceExecutableProbe {
    output: Vec<u8>,
}

impl ExecutableIdentityPort for SurfaceExecutableProbe {
    fn verify_executable(
        &self,
        request: ExecutableIdentityRequest<'_>,
    ) -> Result<ExecutableObservation, ExecutableIdentityError> {
        Ok(ExecutableObservation {
            canonical_path: PathBuf::from(request.executable),
        })
    }
}

impl ProcessProbePort for SurfaceExecutableProbe {
    fn probe(
        &self,
        request: IdentityProbeRequest<'_>,
    ) -> Result<ProbeObservation, ProcessProbeError> {
        assert_eq!(request.argument, "compiled-identity");
        Ok(ProbeObservation {
            stdout: self.output.clone(),
        })
    }
}

#[test]
fn admitted_executable_identity_returns_its_typed_canonical_digest() {
    let schema = pale_contracts::record_schema_identity("surface-executable-identity.v1")
        .expect("surface executable identity schema");
    let identity = pale_core::records::SurfaceExecutableIdentityV1 {
        schema: schema.schema,
        schema_sha256: schema.schema_sha256,
        schema_version: 1,
        version: "1.0.0".into(),
        protocol: "pale.agent-surface-interface.v1".into(),
        os: pale_core::platform::target_os().into(),
        arch: pale_core::platform::target_arch().into(),
        durable_publication: pale_core::platform::durable_publication(),
    };
    let expected = ExecutableIdentityDigest::hash_canonical(&identity).unwrap();
    let instance = ResolvedProjectInstanceV1 {
        schema: SchemaId::parse("pale/resolved-project-instance.v1").unwrap(),
        schema_sha256: SchemaDigest::hash_bytes(b"resolved instance schema"),
        profile_digest: ProfileDigest::parse("b".repeat(64)).unwrap(),
        machine_id: "machine".into(),
        repositories: Default::default(),
        git_metadata_roots: Default::default(),
        provider_repositories: Vec::new(),
        authority_roots: Default::default(),
        user_mode_defaults: Default::default(),
        surface_executable: Some(SurfaceExecutableBinding {
            exec: "/opt/pale/bin/pale".into(),
            digest: ExecutableDigest::parse("a".repeat(64)).unwrap(),
            version: identity.version.clone(),
            protocol: identity.protocol.clone(),
        }),
        native_harnesses: Default::default(),
        session_state: None,
        durable_evidence: None,
        materialization_lineage: None,
        instance_digest: InstanceDigest::parse("c".repeat(64)).unwrap(),
    };
    let probe = SurfaceExecutableProbe {
        output: serde_json::to_vec(&identity).unwrap(),
    };

    assert_eq!(
        verify_instance_executable_binding(&probe, &instance).unwrap(),
        expected
    );
}
