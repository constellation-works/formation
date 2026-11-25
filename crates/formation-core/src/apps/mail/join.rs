use super::{Error, Outcome, Store, refused};
pub(super) fn run(store: &Store, key: &[u8], now: i64, name: String) -> Result<Outcome, Error> {
    store.write(|tx| {
        if tx.seats().bind(&name, key, now)? {
            Ok(Outcome::Joined(name))
        } else {
            Err(refused(
                "seat_taken",
                "seat already belongs to another identity",
            ))
        }
    })
}
