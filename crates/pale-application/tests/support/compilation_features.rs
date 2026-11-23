//! Compiler-test extensions for the shared source-backed fixture.

use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::{
    MachineBindingFile, ParticipantProviderManifestV1, ParticipantRefV1,
    WorkspaceCompositionSourceV1, WorkspaceId, WorkspaceManifestFile,
};

use super::compilation_fixtures::SyntheticWorkspace;
use super::{compile_portable_inputs, Compilation, CompileError};

impl SyntheticWorkspace {
    pub(super) fn retarget_workspace_identity(&self, workspace_id: &str) {
        self.update_manifest(|manifest| {
            manifest.workspace.workspace_id =
                WorkspaceId::parse(workspace_id).expect("fixture workspace identity is portable");
        });
        super::workspace_fixtures::write_empty_universe(
            &self.root.path().join("workspace"),
            workspace_id,
        );
    }

    pub(super) fn select_composition_source(&self) {
        let workspace = self.root.path().join("workspace");
        let source = std::fs::read_to_string(workspace.join("composition.toml"))
            .expect("read selected composition source");
        let mut source: WorkspaceCompositionSourceV1 =
            toml::from_str(&source).expect("parse selected composition source");
        source
            .declarations
            .participant_provider_manifests
            .push(ParticipantProviderManifestV1 {
                id: "manifest.reviewable-capability".into(),
                participants: vec![ParticipantRefV1::Capability {
                    capability_id: "capability.reviewable".into(),
                }],
                providers: Vec::new(),
            });
        std::fs::write(
            workspace.join("composition.toml"),
            toml::to_string(&source).expect("serialize typed composition source"),
        )
        .expect("write composition source");
        self.update_manifest(|manifest| {
            manifest.workspace.experience_compositions =
                vec![OwnedConfinedRelativePath::parse("composition.toml")
                    .expect("fixture composition path is confined")];
        });
    }

    pub(super) fn select_responsibility_derivation(&self) {
        let mut source = super::responsibility_fixtures::responsibility_derivation(
            "compilation-bound",
            1,
            false,
        );
        source.publisher_module =
            super::responsibility_fixtures::omitted_answer("compilation-bound-publisher");
        source.adopter_module =
            super::responsibility_fixtures::omitted_answer("compilation-bound-adopter");
        let workspace = self.root.path().join("workspace");
        std::fs::write(
            workspace.join("responsibility.toml"),
            toml::to_string(&source).expect("serialize typed responsibility source"),
        )
        .expect("write responsibility source");
        self.update_manifest(|manifest| {
            manifest.workspace.responsibility_derivations =
                vec![OwnedConfinedRelativePath::parse("responsibility.toml")
                    .expect("fixture responsibility path is confined")];
        });
    }

    pub(super) fn select_organizational_translation(&self) {
        let source = super::organization_fixtures::organizational_translation();
        let workspace = self.root.path().join("workspace");
        std::fs::write(
            workspace.join("organization.toml"),
            toml::to_string(&source).expect("serialize typed organization source"),
        )
        .expect("write organization source");
        self.update_manifest(|manifest| {
            manifest.workspace.organizational_translations =
                vec![OwnedConfinedRelativePath::parse("organization.toml")
                    .expect("fixture organizational path is confined")];
        });
    }

    pub(super) fn retarget_machine_repository(&self) {
        let alternate = self.root.path().join("repository-alternate");
        std::fs::create_dir_all(&alternate).expect("create alternate repository");
        let source = std::fs::read_to_string(&self.machine_binding).expect("read machine binding");
        let mut machine: MachineBindingFile =
            toml::from_str(&source).expect("parse typed machine binding");
        machine
            .repository_checkouts
            .iter_mut()
            .find(|checkout| checkout.repository_id.as_str() == "engine")
            .expect("engine checkout binding")
            .checkout_root = alternate.to_string_lossy().into_owned();
        std::fs::write(
            &self.machine_binding,
            toml::to_string(&machine).expect("serialize typed machine binding"),
        )
        .expect("rewrite machine binding");
    }

    fn update_manifest(&self, update: impl FnOnce(&mut WorkspaceManifestFile)) {
        let source = std::fs::read_to_string(&self.manifest).expect("read workspace manifest");
        let mut manifest: WorkspaceManifestFile =
            toml::from_str(&source).expect("parse typed workspace manifest");
        update(&mut manifest);
        std::fs::write(
            &self.manifest,
            toml::to_string(&manifest).expect("serialize typed workspace manifest"),
        )
        .expect("rewrite workspace manifest");
    }

    pub(super) fn compile(&self) -> Result<Compilation, CompileError> {
        compile_portable_inputs(
            self.capture_inputs()?,
            &pale_adapter_fs::LocalSourceFilesystem,
        )
    }
}
