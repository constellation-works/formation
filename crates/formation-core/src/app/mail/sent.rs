use super::{Error, Outcome, SeatRef, Store, refused};

pub(super) fn run(store: &Store, caller: SeatRef<'_>, limit: i64) -> Result<Outcome, Error> {
    if limit < 1 {
        return Err(refused(
            "invalid_limit",
            "--limit must be a positive integer",
        ));
    }
    let snapshot = store.read()?;
    let seat = match caller {
        SeatRef::Name(name) => {
            if !snapshot.seats().contains(name)? {
                return Err(refused(
                    "unknown_seat",
                    "seat has not joined this room; run `formation mail join` first",
                ));
            }
            name.to_owned()
        }
        SeatRef::Session(session) => snapshot.seats().by_session(session)?.ok_or_else(|| {
            refused(
                "seat_required",
                "seat required: pass --as <seat> or join this room from your session",
            )
        })?,
    };
    let (total, messages) = snapshot.messages().sent(&seat, limit)?;
    Ok(Outcome::SentHistory { messages, total })
}
