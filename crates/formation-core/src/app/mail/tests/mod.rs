use super::{Outcome, SeatRef, ack, join, send, who};
use formation_store::Store;

#[test]
fn absence_holds_new_mail_and_preserves_replays() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    join::run(&store, None, "chief".into(), None)?;
    join::run(&store, None, "captain".into(), None)?;
    let chief = SeatRef::Name("chief");
    let captain = SeatRef::Name("captain");
    let send = |now, id: &str, body| {
        let draft = send::Draft {
            to: "captain",
            id: id.into(),
            body,
            reply_to: None,
        };
        send::commit(&store, chief, now, draft)
    };

    let (
        Outcome::Sent {
            receipt,
            held: false,
            ..
        },
        true,
    ) = send(100, "first", "hello")?
    else {
        panic!("send outcome")
    };
    who::run(&store, captain, 100)?;
    let near = send(150, "second", "more")?;
    assert!(matches!(near, (Outcome::Sent { held: true, .. }, true)));
    let late = send(190, "third", "again")?;
    assert!(matches!(late, (Outcome::Sent { held: false, .. }, true)));

    let replay = send(190, "first", "hello")?;
    assert!(matches!(replay, (Outcome::Sent { receipt: id, .. }, false) if id == receipt));

    ack::run(&store, captain, 190, receipt)?;
    assert_eq!(store.read()?.messages().unread("captain")?.len(), 2);
    Ok(())
}
