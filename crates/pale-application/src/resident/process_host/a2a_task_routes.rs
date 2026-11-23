//! Binding-qualified `ProcessHost` routes for remote A2A Task work.
//!
//! This owner prevents concurrent request classes from acting on the same
//! admitted Task binding. A request-class owner retains its own semantic phase.

use std::sync::Arc;

use pale_ports::AuthorityStoreCoordinate;

use super::{
    ResidentDeploymentAuthoritySelector, ResidentProcessHostBuildError,
    ResidentProcessHostSelectionError,
};
use crate::resident::task_owner::ResidentOperationTaskOwnerInstance;
use crate::resident::task_reservation::{ResidentA2aTaskWorkKey, ResidentAggregateTaskLimits};

struct ResidentProcessA2aTaskRoute {
    key: ResidentA2aTaskWorkKey,
    selector: ResidentDeploymentAuthoritySelector,
}

#[derive(Clone)]
struct ResidentProcessA2aTaskRouteInstance(Arc<()>);

impl ResidentProcessA2aTaskRouteInstance {
    fn new() -> Self {
        Self(Arc::new(()))
    }

    fn is_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Checked-out route that keeps one Task binding occupied until disposition.
#[must_use = "release only after durable read-back or exact local disposition"]
pub(super) struct ResidentProcessA2aTaskRouteCustody {
    pub(super) owner_instance: ResidentOperationTaskOwnerInstance,
    route_instance: ResidentProcessA2aTaskRouteInstance,
    pub(super) key: Box<ResidentA2aTaskWorkKey>,
    pub(super) selector: Box<ResidentDeploymentAuthoritySelector>,
}

/// Binding-qualified routes shared by remote A2A Task request classes.
pub(super) struct ResidentProcessA2aTaskRoutes {
    owner_instance: ResidentOperationTaskOwnerInstance,
    route_instance: ResidentProcessA2aTaskRouteInstance,
    maximum_outstanding: usize,
    active: Vec<ResidentProcessA2aTaskRoute>,
    checked_out: Vec<ResidentProcessA2aTaskRoute>,
}

impl ResidentProcessA2aTaskRoutes {
    /// Return retained remote-task routes.
    pub(super) fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Return remote-task routes temporarily checked out for polling.
    pub(super) fn checked_out_count(&self) -> usize {
        self.checked_out.len()
    }

    /// Preallocate the complete route ledger.
    pub(super) fn new(
        owner_instance: ResidentOperationTaskOwnerInstance,
        limits: ResidentAggregateTaskLimits,
    ) -> Result<Self, ResidentProcessHostBuildError> {
        let maximum_outstanding = limits.maximum_outstanding().get();
        let mut active = Vec::new();
        active
            .try_reserve_exact(maximum_outstanding)
            .map_err(|_| ResidentProcessHostBuildError::A2aTaskRouteAllocation)?;
        let mut checked_out = Vec::new();
        checked_out
            .try_reserve_exact(maximum_outstanding)
            .map_err(|_| ResidentProcessHostBuildError::A2aTaskRouteAllocation)?;
        Ok(Self {
            owner_instance,
            route_instance: ResidentProcessA2aTaskRouteInstance::new(),
            maximum_outstanding,
            active,
            checked_out,
        })
    }

    /// Insert one unoccupied binding route before any request work begins.
    pub(super) fn insert(
        &mut self,
        key: ResidentA2aTaskWorkKey,
        selector: ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProcessA2aTaskRouteInsertError> {
        if self.active.iter().any(|route| route.key == key)
            || self.checked_out.iter().any(|route| route.key == key)
        {
            return Err(ResidentProcessA2aTaskRouteInsertError::Conflict);
        }
        if self.active.len() + self.checked_out.len() == self.maximum_outstanding {
            return Err(ResidentProcessA2aTaskRouteInsertError::Saturated);
        }
        self.active
            .push(ResidentProcessA2aTaskRoute { key, selector });
        Ok(())
    }

    /// Return whether an active route has this exact binding and selector.
    pub(super) fn matches(
        &self,
        key: &ResidentA2aTaskWorkKey,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> bool {
        self.active
            .iter()
            .any(|route| &route.key == key && &route.selector == selector)
    }

    /// Require the active route and its physical store association.
    pub(super) fn require_active(
        &self,
        key: &ResidentA2aTaskWorkKey,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        if key.store != *access_store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        if !self.matches(key, selector) {
            return Err(ResidentProcessHostSelectionError::A2aTaskRouteAssociation);
        }
        Ok(())
    }

    /// Remove one exact active route before physical request work exists.
    pub(super) fn remove_active(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> bool {
        let Some(index) = self
            .active
            .iter()
            .position(|route| &route.key == key && &route.selector == selector)
        else {
            return false;
        };
        self.active.remove(index);
        true
    }

    /// Move one active route into linear custody.
    pub(super) fn check_out(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
    ) -> Option<ResidentProcessA2aTaskRouteCustody> {
        let index = self.active.iter().position(|route| &route.key == key)?;
        let route = self.active.remove(index);
        self.checked_out.push(ResidentProcessA2aTaskRoute {
            key: route.key.clone(),
            selector: route.selector.clone(),
        });
        Some(ResidentProcessA2aTaskRouteCustody {
            owner_instance: self.owner_instance.clone(),
            route_instance: self.route_instance.clone(),
            key: Box::new(route.key),
            selector: Box::new(route.selector),
        })
    }

    /// Release exact custody after the request class settles its disposition.
    pub(super) fn release(
        &mut self,
        custody: ResidentProcessA2aTaskRouteCustody,
    ) -> Result<(), ResidentProcessA2aTaskRouteCustody> {
        if !custody.owner_instance.is_same(&self.owner_instance)
            || !custody.route_instance.is_same(&self.route_instance)
        {
            return Err(custody);
        }
        let Some(index) = self
            .checked_out
            .iter()
            .position(|route| route.key == *custody.key && route.selector == *custody.selector)
        else {
            return Err(custody);
        };
        self.checked_out.remove(index);
        Ok(())
    }

    /// Return whether this ledger owns the exact checked-out custody.
    pub(super) fn owns_custody(&self, custody: &ResidentProcessA2aTaskRouteCustody) -> bool {
        custody.owner_instance.is_same(&self.owner_instance)
            && custody.route_instance.is_same(&self.route_instance)
            && self
                .checked_out
                .iter()
                .any(|route| route.key == *custody.key && route.selector == *custody.selector)
    }
}

/// Reason a binding route could not enter the ledger.
pub(super) enum ResidentProcessA2aTaskRouteInsertError {
    /// The binding is already active or checked out.
    Conflict,
    /// The preallocated route capacity is full.
    Saturated,
}
