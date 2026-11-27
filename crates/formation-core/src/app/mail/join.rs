use super::{Error, Outcome, SessionId, Store};

pub(super) fn run(
    store: &Store,
    session: Option<&SessionId>,
    name: String,
) -> Result<Outcome, Error> {
    let replaced_session = store.write(|tx| tx.seats().bind(&name, session))?;
    let seat_dir = store.seat_dir(&name)?;
    Ok(Outcome::Joined {
        seat: name,
        replaced_session,
        seat_dir,
    })
}
