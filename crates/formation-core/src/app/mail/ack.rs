use super::{Error, Outcome, SeatRef, Store, refused, touch};

pub(super) fn run(
    store: &Store,
    caller: SeatRef<'_>,
    now: i64,
    receipt: i64,
) -> Result<Outcome, Error> {
    store.write(|tx| {
        let seat = touch(tx, caller, now)?;
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
