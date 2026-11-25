use super::{Error, Outcome, SessionId, Store};

pub(super) fn run(
    store: &Store,
    session: Option<&SessionId>,
    name: String,
) -> Result<Outcome, Error> {
    let replaced_session = store.write(|tx| tx.seats().bind(&name, session))?;
    Ok(Outcome::Joined {
        seat: name,
        replaced_session,
    })
}
