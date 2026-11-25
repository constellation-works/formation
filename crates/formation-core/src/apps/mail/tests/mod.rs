use super::{Outcome, ack, join, send, who};
use formation_store::Store;

#[test]
fn expiry_blocks_new_mail_and_preserves_replays() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::open(root.path())?;
    join::run(&store, b"a", 100, "chief".into())?;
    join::run(&store, b"b", 100, "captain".into())?;
    let Outcome::Sent(_, receipt) =
        send::run(&store, b"a", 100, "captain", "first".into(), "hello")?
    else {
        panic!("send outcome")
    };
    assert!(matches!(who::run(&store,b"a",190)?,Outcome::Online(seats) if seats==["chief"]));
    assert!(send::run(&store, b"a", 190, "captain", "second".into(), "more").is_err());
    assert!(
        matches!(send::run(&store,b"a",190,"captain","first".into(),"hello")?,Outcome::Sent(_,id) if id==receipt)
    );
    ack::run(&store, b"b", 190, receipt)?;
    assert!(store.read()?.messages().unread("captain")?.is_empty());
    Ok(())
}
