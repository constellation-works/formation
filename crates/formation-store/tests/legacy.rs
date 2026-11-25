//! Exercise the shipped unversioned SQLite format at the public store boundary.
use formation_store::Store;
use std::os::unix::fs::PermissionsExt;

#[test]
fn upgrade_retains_seats_and_pending_receipts() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let path = root.path().join("inbox.sqlite3");
    let old = cw_sqlite::rusqlite::Connection::open(&path)?;
    old.execute_batch("CREATE TABLE seats(name TEXT PRIMARY KEY, identity BLOB UNIQUE NOT NULL, seen INTEGER NOT NULL);
        CREATE TABLE messages(receipt INTEGER PRIMARY KEY, sender TEXT NOT NULL, recipient TEXT NOT NULL, id TEXT NOT NULL, body TEXT NOT NULL, ack INTEGER NOT NULL DEFAULT 0, UNIQUE(sender,id));
        CREATE INDEX unread ON messages(recipient,ack,receipt);
        INSERT INTO seats VALUES('captain',x'01',100);
        INSERT INTO messages VALUES(17,'chief','captain','first','retained',0);")?;
    drop(old);
    let store = Store::open(root.path())?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
    assert!(store.write(|tx| tx.seats().touch(&[1], 101)).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    assert_eq!(
        store.write(|tx| tx.seats().touch(&[1], 102))?.as_deref(),
        Some("captain")
    );
    let pending = store.read()?.messages().unread("captain")?;
    assert_eq!(pending.len(), 1);
    assert_eq!(
        (pending[0].receipt, pending[0].body.as_str()),
        (17, "retained")
    );
    Ok(())
}
