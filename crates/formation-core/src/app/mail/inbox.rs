use super::{Ctx, Duration, Error, Outcome, Store, touch};
use std::time::Instant;

const POLL_INTERVAL: Duration = Duration::from_millis(500);

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

pub(super) fn run(
    ctx: &Ctx<'_>,
    store: &Store,
    key: &[u8],
    timeout: Duration,
) -> Result<Outcome, Error> {
    let start = Instant::now();
    let heartbeat = || store.write(|tx| touch(tx, key, (ctx.clock)().unix_timestamp()));
    let seat = heartbeat()?;
    let mut refresh = start;

    loop {
        let mail = store.read()?.messages().unread(&seat)?;
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
