//! Bounded standard-input reads for commands that take a JSON document.

use std::io::{self, Read};

/// Read all of standard input, refusing a document larger than `max` bytes.
pub(crate) fn read_bounded(max: u64, label: &str) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    io::stdin().lock().take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(io::Error::other(format!(
            "{label} exceeds {} KiB",
            max / 1024
        )));
    }
    Ok(bytes)
}
