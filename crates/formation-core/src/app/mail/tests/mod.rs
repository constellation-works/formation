use super::{Outcome, SeatRef, ack, join, send, who};
use formation_store::Store;

#[test]
fn absence_holds_new_mail_and_preserves_replays() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    join::run(&store, None, "chief".into())?;
    join::run(&store, None, "captain".into())?;
    let chief = SeatRef::Name("chief");
    let captain = SeatRef::Name("captain");

    let Outcome::Sent {
        receipt,
        held: false,
        ..
    } = send::run(&store, chief, 100, "captain", "first".into(), "hello")?
    else {
        panic!("send outcome")
    };
    who::run(&store, captain, 100)?;
    let near = send::run(&store, chief, 150, "captain", "second".into(), "more")?;
    assert!(matches!(near, Outcome::Sent { held: true, .. }));
    let late = send::run(&store, chief, 190, "captain", "third".into(), "again")?;
    assert!(matches!(late, Outcome::Sent { held: false, .. }));

    let replay = send::run(&store, chief, 190, "captain", "first".into(), "hello")?;
    assert!(matches!(replay, Outcome::Sent { receipt: id, .. } if id == receipt));

    ack::run(&store, captain, 190, receipt)?;
    assert_eq!(store.read()?.messages().unread("captain")?.len(), 2);
    Ok(())
}
