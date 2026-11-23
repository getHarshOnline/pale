//! Public recovery through a real native pending-journal producer, not forged JSON.

use super::*;
use crate::{
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticRetry, LocalWorkspaceLocator,
};
use std::{fs, os::unix::fs::PermissionsExt};

#[path = "../support/local_workspace.rs"]
mod fixture;

fn request(manifest: &str, host: SourceGuidanceHost) -> LocalSourceGuidancePlanRequest {
    LocalSourceGuidancePlanRequest::new(
        fixture::request(manifest.to_owned()),
        "engine".into(),
        host,
        LocalSourceGuidanceIntent::Apply,
    )
}
fn recovery_request(
    manifest: &str,
    host: SourceGuidanceHost,
    intent: LocalSourceGuidanceRecoveryIntent,
) -> LocalSourceGuidanceRecoveryRequest {
    LocalSourceGuidanceRecoveryRequest::new(
        fixture::request(manifest.to_owned()),
        "engine".into(),
        host,
        intent,
    )
}
fn pending(api: &PaleApi, manifest: &str, host: SourceGuidanceHost) {
    let fresh = api
        .plan_local_source_guidance(request(manifest, host))
        .unwrap();
    let digest = fresh.plan.digest().clone();
    let applied =
        pale_adapter_fs::prepare_source_guidance_write(fresh.plan.admit(&digest).unwrap())
            .unwrap()
            .publish()
            .unwrap();
    let path = std::path::Path::new(manifest)
        .parent()
        .unwrap()
        .join("checkout/.pale-source-guidance-recovery.json");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    assert!(applied.commit().is_err());
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn public_recovery_checks_scope_review_and_restores_both_native_layouts() {
    for (host, entry) in [
        (SourceGuidanceHost::Codex, "AGENTS.md"),
        (SourceGuidanceHost::ClaudeCode, "AGENTS.md"),
    ] {
        let (temporary, manifest) = fixture::source_workspace();
        let root = temporary.path().canonicalize().unwrap();
        let checkout = root.join("checkout");
        let api = PaleApi::new().unwrap();
        fs::write(
            checkout.join(".gitignore"),
            "/local-cache/\n!important.txt\n",
        )
        .unwrap();
        fs::write(
            checkout.join(entry),
            "Private local facts without final newline",
        )
        .unwrap();
        let intent = LocalSourceGuidanceRecoveryIntent::RestoreBefore;
        assert!(api
            .plan_local_source_guidance_recovery(recovery_request(&manifest, host, intent))
            .unwrap()
            .is_none());
        pending(&api, &manifest, host);
        let wrong_host = if host == SourceGuidanceHost::Codex {
            SourceGuidanceHost::ClaudeCode
        } else {
            SourceGuidanceHost::Codex
        };
        let error = api
            .plan_local_source_guidance_recovery(recovery_request(&manifest, wrong_host, intent))
            .unwrap_err();
        assert_eq!(
            error.diagnostic().code(),
            "source_guidance_recovery_scope_rejected"
        );
        let review = api
            .plan_local_source_guidance_recovery(recovery_request(&manifest, host, intent))
            .unwrap()
            .unwrap();
        assert!(!format!("{review:?}").contains(root.to_str().unwrap()));
        assert!(!format!("{review:?}").contains("Private local facts"));
        assert!(serde_json::to_string(review.report())
            .unwrap()
            .contains("Private local facts"));
        let error = api
            .execute_local_source_guidance_recovery(
                LocalSourceGuidanceRecoveryExecutionRequest::new(
                    recovery_request(&manifest, host, intent),
                    "0".repeat(64),
                ),
            )
            .unwrap_err();
        assert_eq!(
            error.diagnostic().disposition(),
            DiagnosticDisposition::Blocked
        );
        assert_eq!(
            error.diagnostic().effect(),
            DiagnosticEffectCertainty::NotStarted
        );
        let result = api
            .execute_local_source_guidance_recovery(
                LocalSourceGuidanceRecoveryExecutionRequest::new(
                    recovery_request(&manifest, host, intent),
                    review.report().digest().into(),
                ),
            )
            .unwrap();
        assert!(result.changed_files() > 0);
        assert_eq!(
            fs::read_to_string(checkout.join(entry)).unwrap(),
            "Private local facts without final newline"
        );
        assert!(api
            .plan_local_source_guidance_recovery(recovery_request(&manifest, host, intent))
            .unwrap()
            .is_none());
        assert!(!checkout
            .join(".pale-source-guidance-recovery.json")
            .exists());
        assert_eq!(
            fs::read_to_string(checkout.join(".gitignore")).unwrap(),
            "/local-cache/\n!important.txt\n"
        );
    }
}

#[test]
fn public_recovery_rebuilds_observations_and_can_finalize_a_complete_after_state() {
    let (temporary, manifest) = fixture::source_workspace();
    let root = temporary.path().canonicalize().unwrap();
    let checkout = root.join("checkout");
    let api = PaleApi::new().unwrap();
    let host = SourceGuidanceHost::Codex;
    pending(&api, &manifest, host);
    let intent = LocalSourceGuidanceRecoveryIntent::FinalizePublished;
    let review = api
        .plan_local_source_guidance_recovery(recovery_request(&manifest, host, intent))
        .unwrap()
        .unwrap();
    let before = fs::read(checkout.join("AGENTS.md")).unwrap();
    fs::write(checkout.join("AGENTS.md"), "independent correction").unwrap();
    assert!(api
        .execute_local_source_guidance_recovery(LocalSourceGuidanceRecoveryExecutionRequest::new(
            recovery_request(&manifest, host, intent),
            review.report().digest().into()
        ))
        .is_err());
    assert_eq!(
        fs::read_to_string(checkout.join("AGENTS.md")).unwrap(),
        "independent correction"
    );
    fs::write(checkout.join("AGENTS.md"), before).unwrap();
    let review = api
        .plan_local_source_guidance_recovery(recovery_request(&manifest, host, intent))
        .unwrap()
        .unwrap();
    let result = api
        .execute_local_source_guidance_recovery(LocalSourceGuidanceRecoveryExecutionRequest::new(
            recovery_request(&manifest, host, intent),
            review.report().digest().into(),
        ))
        .unwrap();
    assert_eq!(result.changed_files(), 0);
    assert!(fs::read_to_string(checkout.join(".gitignore"))
        .unwrap()
        .contains("/.pale-source-guidance-recovery.json"));
    assert!(api
        .plan_local_source_guidance(request(&manifest, host))
        .unwrap()
        .report()
        .publication_order()
        .is_empty());
}

#[test]
fn native_effect_failures_preserve_certainty_and_never_ask_for_blind_retry() {
    use super::effect::{native_failure, NativeBoundary};
    use pale_adapter_fs::{FsError, SourceGuidancePublicationFailureState};
    let api = PaleApi::new().unwrap();
    let private = "private unadmitted review input";
    let write = LocalSourceGuidanceWriteRequest::new(
        request("/private/manifest.toml", SourceGuidanceHost::Codex),
        private.into(),
    );
    assert!(!format!("{write:?}").contains(private));
    let recovery = LocalSourceGuidanceRecoveryExecutionRequest::new(
        recovery_request(
            "/private/manifest.toml",
            SourceGuidanceHost::Codex,
            LocalSourceGuidanceRecoveryIntent::RestoreBefore,
        ),
        private.into(),
    );
    assert!(!format!("{recovery:?}").contains(private));
    for (state, expected) in [
        (
            SourceGuidancePublicationFailureState::NotStarted,
            DiagnosticEffectCertainty::NotStarted,
        ),
        (
            SourceGuidancePublicationFailureState::Restored,
            DiagnosticEffectCertainty::Confirmed,
        ),
        (
            SourceGuidancePublicationFailureState::OutcomeUnknown,
            DiagnosticEffectCertainty::OutcomeUnknown,
        ),
    ] {
        let error = native_failure(
            &api,
            FsError::Collision("AGENTS.md".into(), "fixture refusal".into()),
            NativeBoundary::Publication(state),
            "source_guidance_publication_failed",
        );
        assert_eq!(error.diagnostic().effect(), expected);
        if state == SourceGuidancePublicationFailureState::OutcomeUnknown {
            assert_eq!(error.diagnostic().retry(), DiagnosticRetry::ReconcileFirst);
        }
    }
}
