//! Forward-failure and concurrent-edit recovery journeys.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fs;

use pale_core::records::{PlanKind, SurfaceOwnership, MATERIALIZATION_STATE_PATH};

use super::{
    apply, confirmed_coordination_source, contract_identity, repository_id, roots, state, surface,
    transition_plan, FilesystemEffects, MutationEffects,
};
use crate::confine::ConfinedLocation;
use crate::errors::FsError;
use crate::file_transaction::commit;
use crate::materialization_state::read_materialization_state;

struct FailOnWrite {
    fail_at: usize,
    writes: Cell<usize>,
}

struct EditThenFail {
    fail_at: usize,
    writes: Cell<usize>,
    first_write: RefCell<Option<ConfinedLocation>>,
}

struct PublishThenCleanupFailure {
    failed: Cell<bool>,
}

impl MutationEffects for PublishThenCleanupFailure {
    fn write(&self, location: &ConfinedLocation, bytes: &[u8], mode: u32) -> Result<(), FsError> {
        FilesystemEffects.write(location, bytes, mode)?;
        if !self.failed.replace(true) {
            return Err(FsError::CleanupFailed {
                operation: "inject staged publication cleanup",
                path: location.display_path(),
                source: Box::new(crate::errors::invariant(
                    "inject publication failure",
                    location.display_path(),
                    "test fault",
                )),
                cleanup: Box::new(crate::errors::invariant(
                    "inject staging cleanup failure",
                    location.display_path(),
                    "test fault",
                )),
            });
        }
        Ok(())
    }

    fn remove(&self, location: &ConfinedLocation) -> Result<(), FsError> {
        FilesystemEffects.remove(location)
    }
}

impl MutationEffects for EditThenFail {
    fn write(&self, location: &ConfinedLocation, bytes: &[u8], mode: u32) -> Result<(), FsError> {
        let attempt = self.writes.get() + 1;
        self.writes.set(attempt);
        if attempt == self.fail_at {
            let first = self.first_write.borrow();
            FilesystemEffects.write(
                first.as_ref().expect("an earlier write was recorded"),
                b"concurrent user edit\n",
                0o640,
            )?;
            return Err(crate::errors::invariant(
                "inject write failure",
                location.display_path(),
                "test fault",
            ));
        }
        FilesystemEffects.write(location, bytes, mode)?;
        if self.first_write.borrow().is_none() {
            self.first_write.replace(Some(location.clone()));
        }
        Ok(())
    }

    fn remove(&self, location: &ConfinedLocation) -> Result<(), FsError> {
        FilesystemEffects.remove(location)
    }
}

impl MutationEffects for FailOnWrite {
    fn write(&self, location: &ConfinedLocation, bytes: &[u8], mode: u32) -> Result<(), FsError> {
        let attempt = self.writes.get() + 1;
        self.writes.set(attempt);
        if attempt == self.fail_at {
            return Err(crate::errors::invariant(
                "inject write failure",
                location.display_path(),
                "test fault",
            ));
        }
        FilesystemEffects.write(location, bytes, mode)
    }

    fn remove(&self, location: &ConfinedLocation) -> Result<(), FsError> {
        FilesystemEffects.remove(location)
    }
}

#[test]
fn lineage_write_failure_rolls_back_surfaces_and_target_state() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let desired = state(
        vec![
            surface(
                ".claude/a.txt",
                "artifact",
                SurfaceOwnership::WholeFile,
                "a\n",
            ),
            surface(
                ".claude/b.txt",
                "artifact",
                SurfaceOwnership::WholeFile,
                "b\n",
            ),
        ],
        '3',
    );
    let plan = transition_plan(PlanKind::Apply, None, Some(desired));
    let store = FailOnWrite {
        fail_at: 4,
        writes: Cell::new(0),
    };
    let error = match apply(
        &store,
        &target,
        &lineage,
        &plan,
        &BTreeMap::from([
            (".claude/a.txt".into(), "a\n".to_owned()),
            (".claude/b.txt".into(), "b\n".to_owned()),
        ]),
    ) {
        Ok(txn) => {
            commit(txn);
            panic!("the injected lineage write failure was not observed");
        }
        Err(error) => error,
    };
    assert!(matches!(
        confirmed_coordination_source(error),
        FsError::Invariant { .. }
    ));
    assert!(!target.join(".claude/a.txt").exists());
    assert!(!target.join(".claude/b.txt").exists());
    assert!(!target.join(MATERIALIZATION_STATE_PATH).exists());
    assert!(
        read_materialization_state(&target, &lineage, &repository_id(), &contract_identity())
            .unwrap()
            .is_none()
    );
}

#[test]
fn uncertain_publication_is_preserved_when_ownership_was_not_bound() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let path = ".claude/surface.txt";
    let desired = state(
        vec![surface(
            path,
            "artifact",
            SurfaceOwnership::WholeFile,
            "generated\n",
        )],
        '3',
    );
    let plan = transition_plan(PlanKind::Apply, None, Some(desired));
    let store = PublishThenCleanupFailure {
        failed: Cell::new(false),
    };

    let error = apply(
        &store,
        &target,
        &lineage,
        &plan,
        &BTreeMap::from([(path.into(), "generated\n".to_owned())]),
    )
    .unwrap_err();

    assert!(matches!(
        confirmed_coordination_source(error),
        FsError::RestoreFailed {
            source,
            ..
        } if matches!(*source, FsError::CleanupFailed { .. })
    ));
    assert_eq!(fs::read(target.join(path)).unwrap(), b"generated\n");
    assert!(!target.join(MATERIALIZATION_STATE_PATH).exists());
}

#[test]
fn rollback_preserves_a_concurrent_edit_and_restores_unaffected_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let desired = state(
        vec![
            surface(
                ".claude/a.txt",
                "artifact",
                SurfaceOwnership::WholeFile,
                "a\n",
            ),
            surface(
                ".claude/b.txt",
                "artifact",
                SurfaceOwnership::WholeFile,
                "b\n",
            ),
        ],
        '3',
    );
    let plan = transition_plan(PlanKind::Apply, None, Some(desired));
    let store = EditThenFail {
        fail_at: 4,
        writes: Cell::new(0),
        first_write: RefCell::new(None),
    };

    let error = match apply(
        &store,
        &target,
        &lineage,
        &plan,
        &BTreeMap::from([
            (".claude/a.txt".into(), "a\n".to_owned()),
            (".claude/b.txt".into(), "b\n".to_owned()),
        ]),
    ) {
        Ok(txn) => {
            commit(txn);
            panic!("the final write must fail");
        }
        Err(error) => error,
    };

    assert!(matches!(
        confirmed_coordination_source(error),
        FsError::RestoreFailed { .. }
    ));
    assert_eq!(
        fs::read(target.join(".claude/a.txt")).unwrap(),
        b"concurrent user edit\n"
    );
    assert!(!target.join(".claude/b.txt").exists());
    assert!(!target.join(MATERIALIZATION_STATE_PATH).exists());
}
