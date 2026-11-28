//! The bounded roster input shared by check and apply.

use crate::command::common::stdin;
use crate::output;
use formation_core::Env;
use formation_core::app::roster::{self, Command};
use std::io;
use std::process::ExitCode;

const MAX_BYTES: u64 = 256 * 1024;

pub(super) fn run(
    env: &Env,
    command: impl FnOnce(Vec<u8>) -> Command,
    mode: output::Mode,
) -> ExitCode {
    match read() {
        Ok(bytes) => roster::exec(env, command(bytes), |result| {
            output::roster_result(result, mode)
        }),
        Err(error) => output::failure(&error),
    }
}

fn read() -> io::Result<Vec<u8>> {
    stdin::read_bounded(MAX_BYTES, "roster")
}
