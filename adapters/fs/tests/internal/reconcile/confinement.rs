//! Adversarial ancestor and retained-root replacement journeys.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use pale_core::records::{PlanKind, SurfaceOwnership, MATERIALIZATION_STATE_PATH};

use super::{
    apply, confirmed_coordination_source, roots, state, surface, transition_plan,
    FilesystemEffects, MutationEffects,
};
use crate::confine::ConfinedLocation;
use crate::errors::FsError;

struct SwapRootBeforeWrite {
    root: PathBuf,
    moved: PathBuf,
    swapped: Cell<bool>,
}

impl MutationEffects for SwapRootBeforeWrite {
    fn write(&self, location: &ConfinedLocation, bytes: &[u8], mode: u32) -> Result<(), FsError> {
        if !self.swapped.get() && location.display_path().starts_with(&self.root) {
            fs::rename(&self.root, &self.moved).unwrap();
            fs::create_dir(&self.root).unwrap();
            self.swapped.set(true);
        }
        FilesystemEffects.write(location, bytes, mode)
    }

    fn remove(&self, location: &ConfinedLocation) -> Result<(), FsError> {
        FilesystemEffects.remove(location)
    }
}

struct SwapAncestorBeforeWrite {
    target: PathBuf,
    parked: PathBuf,
    outside: PathBuf,
    swapped: Cell<bool>,
}

impl MutationEffects for SwapAncestorBeforeWrite {
    fn write(&self, location: &ConfinedLocation, bytes: &[u8], mode: u32) -> Result<(), FsError> {
        use std::os::unix::fs::symlink;

        if !self.swapped.replace(true) {
            fs::rename(self.target.join(".claude"), &self.parked).unwrap();
            symlink(&self.outside, self.target.join(".claude")).unwrap();
        }
        FilesystemEffects.write(location, bytes, mode)
    }

    fn remove(&self, location: &ConfinedLocation) -> Result<(), FsError> {
        FilesystemEffects.remove(location)
    }
}

#[test]
fn symlinked_surface_ancestor_is_rejected_before_any_owned_write() {
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let outside = tmp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, target.join(".claude")).unwrap();
    let path = ".claude/skills/pale-review/SKILL.md";
    let desired = state(
        vec![surface(
            path,
            "operation-skill",
            SurfaceOwnership::WholeFile,
            "review\n",
        )],
        '3',
    );
    let plan = transition_plan(PlanKind::Apply, None, Some(desired));
    assert!(apply(
        &FilesystemEffects,
        &target,
        &lineage,
        &plan,
        &BTreeMap::from([(path.into(), "review\n".to_owned())]),
    )
    .is_err());
    assert!(fs::read_dir(outside).unwrap().next().is_none());
}

#[test]
fn substituted_ancestor_cannot_redirect_a_staged_write() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let outside = tmp.path().join("outside");
    let parked = tmp.path().join("parked-claude");
    fs::create_dir(&outside).unwrap();
    fs::create_dir(target.join(".claude")).unwrap();
    let path = ".claude/skills/pale-review/SKILL.md";
    let desired = state(
        vec![surface(
            path,
            "operation-skill",
            SurfaceOwnership::WholeFile,
            "review\n",
        )],
        '3',
    );
    let plan = transition_plan(PlanKind::Apply, None, Some(desired));
    let store = SwapAncestorBeforeWrite {
        target: target.clone(),
        parked,
        outside: outside.clone(),
        swapped: Cell::new(false),
    };

    assert!(apply(
        &store,
        &target,
        &lineage,
        &plan,
        &BTreeMap::from([(path.into(), "review\n".to_owned())]),
    )
    .is_err());
    assert!(
        fs::read_dir(outside).unwrap().next().is_none(),
        "a replaced path prefix must never redirect an owned write"
    );
}

#[test]
fn replaced_target_root_is_detected_and_detached_effects_are_recovered() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let moved = tmp.path().join("moved-target");
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
    let canonical_target = fs::canonicalize(&target).unwrap();
    let store = SwapRootBeforeWrite {
        root: canonical_target.clone(),
        moved: canonical_target
            .parent()
            .expect("target has a parent")
            .join("moved-target"),
        swapped: Cell::new(false),
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
        FsError::Collision(_, _)
    ));
    assert!(
        !target.join(path).exists(),
        "replacement root stays untouched"
    );
    assert!(!moved.join(path).exists(), "detached write is recovered");
    assert!(!moved.join(MATERIALIZATION_STATE_PATH).exists());
}

#[test]
fn replaced_lineage_root_is_detected_and_the_whole_transition_is_recovered() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let moved = tmp.path().join("moved-lineage");
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
    let canonical_lineage = fs::canonicalize(&lineage).unwrap();
    let store = SwapRootBeforeWrite {
        root: canonical_lineage.clone(),
        moved: canonical_lineage
            .parent()
            .expect("lineage has a parent")
            .join("moved-lineage"),
        swapped: Cell::new(false),
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
        FsError::Collision(_, _)
    ));
    assert!(!target.join(path).exists());
    assert!(!target.join(MATERIALIZATION_STATE_PATH).exists());
    assert!(fs::read_dir(&lineage).unwrap().next().is_none());
    assert!(fs::read_dir(&moved).unwrap().next().is_none());
}
