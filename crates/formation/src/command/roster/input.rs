//! The bounded roster input shared by check and apply.

use crate::output;
use formation_core::Env;
use formation_core::app::roster::{self, Command};
use std::io::{self, Read};
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
    let mut bytes = Vec::new();
    io::stdin()
        .lock()
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::other("roster exceeds 256 KiB"));
    }
    Ok(bytes)
}
