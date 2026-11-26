//! Formation apps and their shared runtime.

pub mod app;

/// Compatibility path for existing callers.
pub use app as apps;

mod composition;

mod runtime;

pub use app::roster::{Error as RosterError, Summary, check};
pub use composition::Env;
pub use cw_base::{Error, Kind};
