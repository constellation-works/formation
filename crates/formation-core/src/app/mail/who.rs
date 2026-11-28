use super::{Error, Outcome, SeatRef, SeatStatus, Store, touch};
use crate::runtime;
use formation_store::contract::rfc3339;

pub(super) fn run(store: &Store, caller: SeatRef<'_>, now: i64) -> Result<Outcome, Error> {
    store.write(|tx| {
        touch(tx, caller, now)?;
        let seats = statuses(tx.seats().mailboxes()?, now);
        Ok(Outcome::Seats(seats))
    })
}

pub(crate) fn statuses(
    mailboxes: Vec<formation_store::contract::Mailbox>,
    now: i64,
) -> Vec<SeatStatus> {
    mailboxes
        .into_iter()
        .map(|mailbox| SeatStatus {
            name: mailbox.name,
            undelivered: mailbox.undelivered,
            unacknowledged: mailbox.unacknowledged,
            held: runtime::held(mailbox.seen, now),
            seen: if mailbox.seen > 0 {
                rfc3339(mailbox.seen)
            } else {
                None
            },
        })
        .collect()
}
