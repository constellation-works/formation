use super::{Error, Outcome, Store, refused, touch};

pub(super) fn run(store: &Store, key: &[u8], now: i64, receipt: i64) -> Result<Outcome, Error> {
    store.write(|tx| {
        let seat = touch(tx, key, now)?;
        if tx.messages().ack(&seat, receipt)? {
            Ok(Outcome::Ack(receipt))
        } else {
            Err(refused(
                "unknown_receipt",
                "receipt not found in this inbox",
            ))
        }
    })
}
