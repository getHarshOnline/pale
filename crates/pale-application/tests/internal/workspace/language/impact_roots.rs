//! Contributing-root queries borrow an already-verified comparison relation.

use super::*;
use crate::workspace::{PortableWorkspaceContributingRoots, PortableWorkspaceValueComparison};

#[test]
fn contributing_roots_preserve_borrowed_membership_order_and_offset_laws() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let prior = load_portable_fixture(&fixture);
    let temporary = tempfile::tempdir().unwrap();
    for entry in fs::read_dir(&fixture).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), temporary.path().join(entry.file_name())).unwrap();
    }
    for label in [
        "pale-workspace.toml",
        "resolution.lock.toml",
        "universe.lock.toml",
    ] {
        let path = temporary.path().join(label);
        let mut bytes = fs::read(&path).unwrap();
        bytes.extend_from_slice(b"\n# changed captured bytes, unchanged selected value\n");
        fs::write(path, bytes).unwrap();
    }
    let current = load_portable_fixture(temporary.path());
    let comparison = current.compare_with_predecessor(&prior).unwrap();
    assert_eq!(comparison.root_count(), 3);
    for impact in &comparison.receipt().impacts {
        match &impact.state {
            WorkspaceDerivationImpactStateV1::Unaffected => {
                assert!(matches!(
                    comparison
                        .contributing_roots(&impact.subject, 0, 1)
                        .unwrap(),
                    PortableWorkspaceContributingRoots::Unaffected
                ));
                assert!(comparison
                    .contributing_roots(&impact.subject, 1, 1)
                    .is_err());
            }
            WorkspaceDerivationImpactStateV1::Affected { root_digests } => {
                for limit in [0, 1, 2, 64, 256, u16::MAX] {
                    let mut collected = Vec::new();
                    let mut offset = 0;
                    loop {
                        let PortableWorkspaceContributingRoots::Affected(page) = comparison
                            .contributing_roots(&impact.subject, offset, limit)
                            .unwrap()
                        else {
                            panic!("affected receipt member lost its disposition")
                        };
                        assert!(
                            page.roots().len()
                                <= usize::from(if limit == 0 { 64 } else { limit.min(256) })
                        );
                        for root in page.roots() {
                            let retained = comparison
                                .receipt()
                                .roots
                                .iter()
                                .find(|candidate| candidate.root_digest == root.root_digest)
                                .unwrap();
                            assert!(std::ptr::eq(root, retained));
                            collected.push(root.root_digest.clone());
                        }
                        let Some(next) = page.next_offset() else {
                            break;
                        };
                        assert!(next > offset);
                        offset = next;
                    }
                    assert_eq!(&collected, root_digests);
                }
                let PortableWorkspaceContributingRoots::Affected(end) = comparison
                    .contributing_roots(&impact.subject, root_digests.len(), 1)
                    .unwrap()
                else {
                    panic!("exact end must retain affected disposition")
                };
                assert_eq!(end.roots().len(), 0);
                assert!(end.next_offset().is_none());
                assert!(comparison
                    .contributing_roots(&impact.subject, root_digests.len() + 1, 1)
                    .is_err());
            }
        }
    }
    let missing = WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: WorkspaceId::parse("absent").unwrap(),
    };
    assert!(matches!(
        comparison.contributing_roots(&missing, 0, 1).unwrap(),
        PortableWorkspaceContributingRoots::NotPresent
    ));
    assert!(comparison.contributing_roots(&missing, 1, 1).is_err());
    let root = WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: WorkspaceId::parse(current.workspace_id()).unwrap(),
    };
    assert!(matches!(
        current.compare_subject(&prior, &root),
        crate::workspace::PortableWorkspaceSubjectComparison::Shared {
            comparison: PortableWorkspaceValueComparison::Equal,
            ..
        }
    ));
    let PortableWorkspaceContributingRoots::Affected(page) =
        comparison.contributing_roots(&root, 0, 256).unwrap()
    else {
        panic!("workspace must retain source impact despite equal value")
    };
    assert_eq!(
        page.roots().len(),
        1,
        "lock validation roots do not affect the semantic workspace root"
    );
}
