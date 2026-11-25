use super::{Error, Outcome, Store, rule, touch};

pub(super) fn run(store: &Store, key: &[u8], now: i64) -> Result<Outcome, Error> {
    store.write(|tx| {
        touch(tx, key, now)?;
        Ok(Outcome::Online(
            tx.seats().since(now - rule::PRESENCE_SECONDS)?,
        ))
    })
}
