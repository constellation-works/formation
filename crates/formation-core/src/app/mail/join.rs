mod claim;

use super::{Error, Outcome, SessionId, Store};
use claim::Claim;
use std::path::Path;

/// The session-to-room pointer a join from a provider session records.
#[derive(Clone, Copy)]
pub(super) struct Route<'a> {
    pub(super) sessions_dir: &'a Path,
    pub(super) room: &'a str,
}

/// Join all-or-nothing.
///
/// The seat folder comes first, so a refused folder leaves the room untouched. A join from a
/// session then claims that session's route, and the binding and the pointer share one
/// transaction: a pointer that cannot be written rolls the binding back, and a binding that
/// cannot commit puts the previous pointer back.
pub(super) fn run(
    store: &Store,
    session: Option<&SessionId>,
    name: String,
    route: Option<Route<'_>>,
) -> Result<Outcome, Error> {
    let seat_dir = store.seat_dir(&name)?;
    let mut claimed = match session.zip(route) {
        Some((session, route)) => Some((Claim::take(route.sessions_dir, session)?, route.room)),
        None => None,
    };
    let bound = store.write(|tx| {
        let replaced = tx.seats().bind(&name, session)?;
        if let Some((claim, room)) = &mut claimed {
            claim.record(room)?;
        }
        Ok(replaced)
    });
    let replaced_session = match bound {
        Ok(replaced) => replaced,
        Err(error) => {
            return Err(match claimed {
                Some((claim, _)) => claim.abandon(error),
                None => error,
            });
        }
    };
    if let Some((claim, _)) = claimed {
        claim.keep();
    }
    Ok(Outcome::Joined {
        seat: name,
        replaced_session,
        seat_dir,
    })
}

#[cfg(test)]
mod tests;
