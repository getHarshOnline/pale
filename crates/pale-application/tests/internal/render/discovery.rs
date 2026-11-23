//! Native-action tool-server contribution boundary tests.

use super::*;
use pale_core::records::ArtifactDigest;

fn artifact(path: &str, bytes: &[u8]) -> SealedArtifactRef {
    SealedArtifactRef {
        rel_path: path.to_owned(),
        sha256: ArtifactDigest::hash_bytes(bytes),
    }
}

#[test]
fn tool_server_contributions_are_harness_scoped_inert_and_catalog_bound() {
    let catalog = artifact(".pale/native-actions.json", b"selected catalog");
    let input_schema = artifact(
        ".pale/native-action-input.schema.json",
        b"selected input schema",
    );

    for (harness, harness_name, native_path, native_entry) in [
        (
            TerminalAdapterDescriptor::claude_code(),
            "claude-code",
            ".mcp.json",
            "mcpServers.pale",
        ),
        (
            TerminalAdapterDescriptor::codex(),
            "codex",
            ".codex/config.toml",
            "mcp_servers.pale",
        ),
    ] {
        let path = tool_server_contribution_path(harness);
        assert_eq!(
            path,
            format!(".pale/tools/{harness_name}/mcp-server-contribution.json")
        );
        assert!(pale_core::paths::SurfaceTarget::parse(&path).is_some());
        assert!(!path.ends_with(native_path));

        let first = tool_server_contribution(harness, &catalog, &input_schema).unwrap();
        let second = tool_server_contribution(harness, &catalog, &input_schema).unwrap();
        assert_eq!(
            first, second,
            "identical inputs must produce identical bytes"
        );
        let contribution = pale_core::canon::parse_strict(first.as_bytes()).unwrap();
        assert_eq!(contribution["attachment_state"], "not-established");
        assert_eq!(contribution["harness"], harness_name);
        assert_eq!(contribution["native_target"]["path"], native_path);
        assert_eq!(contribution["native_target"]["entry"], native_entry);
        assert_eq!(contribution["transport"], "stdio");
        assert_eq!(
            serde_json::from_value::<SealedArtifactRef>(contribution["catalog"].clone()).unwrap(),
            catalog
        );
        assert_eq!(
            serde_json::from_value::<SealedArtifactRef>(contribution["input_schema"].clone())
                .unwrap(),
            input_schema
        );
        assert!(first.contains("pale-mcp"));
        assert!(!first.contains("installed"));
        assert!(!first.contains("running"));
    }
}
