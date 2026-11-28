use super::{
    Ctx, Duration, Error, HEARTBEAT_INTERVAL, Outcome, POLL_INTERVAL, SeatRef, Store, touch,
};
use std::time::Instant;

pub(super) fn run(
    ctx: &Ctx<'_>,
    store: &Store,
    caller: SeatRef<'_>,
    timeout: Duration,
) -> Result<Outcome, Error> {
    let start = Instant::now();
    let heartbeat = || store.write(|tx| touch(tx, caller, (ctx.clock)().unix_timestamp()));
    let seat = heartbeat()?;
    let mut refresh = start;

    loop {
        let mail = read_snapshot(&store.read()?, &seat)?;
        if !mail.is_empty() || start.elapsed() >= timeout {
            return Ok(Outcome::Inbox(mail));
        }

        let remaining = timeout.saturating_sub(start.elapsed());
        std::thread::sleep(POLL_INTERVAL.min(remaining));

        if refresh.elapsed() >= HEARTBEAT_INTERVAL {
            heartbeat()?;
            refresh = Instant::now();
        }
    }
}

// Read unread mail without renewing presence.
pub(crate) fn read_snapshot(
    snapshot: &formation_store::Snapshot,
    seat: &str,
) -> Result<Vec<formation_store::contract::Message>, Error> {
    snapshot.messages().unread(seat)
}
