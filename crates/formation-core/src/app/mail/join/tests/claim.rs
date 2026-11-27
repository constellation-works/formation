use crate::app::mail::join::claim::Claim;
use cw_base::{Error, Kind};
use formation_store::{SessionId, Store};

#[test]
fn a_dropped_claim_puts_the_route_back() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let sessions = root.path().join("sessions");
    let session = SessionId::try_new("sess")?;
    let mut first = Claim::take(&sessions, &session)?;
    first.record("prior")?;
    first.keep();

    let mut dropped = Claim::take(&sessions, &session)?;
    dropped.record("next")?;
    drop(dropped);

    let recorded = Store::session_room(&sessions, &session)?;
    assert_eq!(recorded.as_deref(), Some("prior"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn an_unrestorable_route_is_reported_with_the_join_failure()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let sessions = root.path().join("sessions");
    let session = SessionId::try_new("sess")?;
    let mut claim = Claim::take(&sessions, &session)?;
    claim.record("next")?;
    // A directory that takes no change: the new pointer cannot be removed.
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o500))?;
    let refused = std::io::Error::other("commit refused");

    let error = claim.abandon(Error::new(Kind::Io, "mail_database", refused));

    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o700))?;
    assert_eq!(error.code(), "session_route_unrestored");
    let message = error.to_string();
    assert!(message.contains("commit refused"), "{message}");
    let pointer = sessions.join("sess");
    assert!(
        message.contains(&pointer.display().to_string()),
        "this test needs an unprivileged user: {message}"
    );
    Ok(())
}
