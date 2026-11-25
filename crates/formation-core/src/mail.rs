//! Local seat operations and bounded sentry waits.
pub use formation_store::mailbox::{Error, Message, Room};
use std::time::{Duration, Instant};

/// Check a seat or caller-generated message identifier.
pub fn check_name(name: &str) -> Result<(), Error> {
    if name.len() <= 96 && super::identifier(name) {
        Ok(())
    } else {
        Err(Error::Rejected(
            "invalid identifier (use 1-96 letters, digits, hyphens or underscores)",
        ))
    }
}

/// Submit a bounded text message to an online seat.
pub fn send(room: &mut Room, sender: &str, to: &str, id: &str, body: &str) -> Result<i64, Error> {
    check_name(to)?;
    check_name(id)?;
    if body.trim().is_empty() || body.len() > 8192 {
        return Err(Error::Rejected(
            "body must contain text and fit within 8192 bytes",
        ));
    }
    room.send(sender, to, id, body)
}

/// Wait for unread mail, renewing presence and releasing the database between checks.
pub fn wait(room: &Room, key: &[u8], timeout: Duration) -> Result<Vec<Message>, Error> {
    let start = Instant::now();
    let mut refresh = start;
    let mut seat = room.heartbeat(key)?;
    loop {
        let messages = room.inbox(&seat)?;
        if !messages.is_empty() || start.elapsed() >= timeout {
            return Ok(messages);
        }
        std::thread::sleep(Duration::from_millis(500).min(timeout.saturating_sub(start.elapsed())));
        if refresh.elapsed() >= Duration::from_secs(15) {
            seat = room.heartbeat(key)?;
            refresh = Instant::now();
        }
    }
}
