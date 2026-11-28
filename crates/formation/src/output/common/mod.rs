//! Shared output primitives used by two or more Formation commands.

pub(crate) mod error;

pub(crate) mod json;

pub(crate) mod logging;

pub(crate) mod mode;

#[cfg(test)]
mod tests;

pub(crate) use error::error;
pub(crate) use error::failure;
pub(crate) use logging::init_logging;
pub(crate) use mode::Mode;

use std::io::{self, IsTerminal, Stdout};

pub(in crate::output) fn stdout() -> (Stdout, bool) {
    let stdout = io::stdout();
    let terminal = stdout.is_terminal();
    (stdout, terminal)
}
