use super::{Error, Outcome, Store, rules, touch};
pub(super) fn run(store: &Store, key: &[u8], now: i64) -> Result<Outcome, Error> {
    store.write(|tx| {
        touch(tx, key, now)?;
        Ok(Outcome::Online(
            tx.seats().since(now - rules::PRESENCE_SECONDS)?,
        ))
    })
}
