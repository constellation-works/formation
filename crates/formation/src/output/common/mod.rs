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

/// Whether the call is attached to a terminal: stdin, stdout or stderr is one.
///
/// Any of the three counts, so `formation mail inbox | less` and `echo hi | formation mail send`
/// from the operator's shell qualify. A script with every stream redirected, such as one run by
/// cron, has none.
pub(crate) fn attached_to_terminal() -> bool {
    io::stdin().is_terminal() || io::stdout().is_terminal() || io::stderr().is_terminal()
}

pub(in crate::output) fn stdout() -> (Stdout, bool) {
    let stdout = io::stdout();
    let terminal = stdout.is_terminal();
    (stdout, terminal)
}
