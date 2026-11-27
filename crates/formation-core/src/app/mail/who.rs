use super::{Error, Outcome, SeatRef, SeatStatus, Store, rule, touch};
use formation_store::contract::rfc3339;

pub(super) fn run(store: &Store, caller: SeatRef<'_>, now: i64) -> Result<Outcome, Error> {
    store.write(|tx| {
        touch(tx, caller, now)?;
        let seats = tx
            .seats()
            .mailboxes()?
            .into_iter()
            .map(|mailbox| SeatStatus {
                name: mailbox.name,
                undelivered: mailbox.undelivered,
                unacknowledged: mailbox.unacknowledged,
                held: rule::held(mailbox.seen, now),
                seen: if mailbox.seen > 0 {
                    rfc3339(mailbox.seen)
                } else {
                    None
                },
            })
            .collect();
        Ok(Outcome::Seats(seats))
    })
}
