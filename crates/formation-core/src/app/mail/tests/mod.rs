use super::{Outcome, SeatRef, ack, join, send, who};
use formation_store::Store;

#[test]
fn expiry_blocks_new_mail_and_preserves_replays() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    join::run(&store, None, 100, "chief".into())?;
    join::run(&store, None, 100, "captain".into())?;
    let chief = SeatRef::Name("chief");
    let captain = SeatRef::Name("captain");

    let Outcome::Sent(_, receipt) =
        send::run(&store, chief, 100, "captain", "first".into(), "hello")?
    else {
        panic!("send outcome")
    };
    let online = who::run(&store, chief, 190)?;
    assert!(matches!(online, Outcome::Online(seats) if seats == ["chief"]));
    assert!(send::run(&store, chief, 190, "captain", "second".into(), "more").is_err());

    let replay = send::run(&store, chief, 190, "captain", "first".into(), "hello")?;
    assert!(matches!(replay, Outcome::Sent(_, id) if id == receipt));

    ack::run(&store, captain, 190, receipt)?;
    assert!(store.read()?.messages().unread("captain")?.is_empty());
    Ok(())
}
