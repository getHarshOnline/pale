//! Authenticated administration submissions and exact shared head coordinates.

use super::*;
use crate::records::*;

mod command;
mod heads;
mod receipt;

pub use command::*;
pub use heads::*;
pub use receipt::*;
