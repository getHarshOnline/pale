//! Rendered-surface values grouped by the contract they carry.

mod error;
mod machine;
mod operation;
mod support;
mod surface;

pub use error::RenderError;
pub(crate) use machine::MachineAdmission;
pub(in crate::render) use operation::{
    HarnessSurfaceRefs, HostOperationProjection, OperationSurfaceRefs,
};
pub(crate) use support::ExecutionPathSupportSeal;
pub use support::{ExecutionPathSupport, ProjectRender};
pub use surface::GeneratedSurface;
pub(in crate::render) use surface::SurfaceKind;
