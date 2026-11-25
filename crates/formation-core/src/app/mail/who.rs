use super::{Error, Outcome, SeatRef, Store, rule, touch};

pub(super) fn run(store: &Store, caller: SeatRef<'_>, now: i64) -> Result<Outcome, Error> {
    store.write(|tx| {
        touch(tx, caller, now)?;
        Ok(Outcome::Online(
            tx.seats().since(now - rule::PRESENCE_SECONDS)?,
        ))
    })
}
