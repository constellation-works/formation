//! Exercise the shipped unversioned SQLite format at the public store boundary.

use formation_store::Store;
use std::os::unix::fs::PermissionsExt;

#[test]
fn upgrade_retains_seats_and_pending_receipts() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let path = root.path().join("inbox.sqlite3");
    let old = cw_sqlite::rusqlite::Connection::open(&path)?;
    old.execute_batch(
        "CREATE TABLE seats(
            name TEXT PRIMARY KEY,
            identity BLOB UNIQUE NOT NULL,
            seen INTEGER NOT NULL
        );
        CREATE TABLE messages(
            receipt INTEGER PRIMARY KEY,
            sender TEXT NOT NULL,
            recipient TEXT NOT NULL,
            id TEXT NOT NULL,
            body TEXT NOT NULL,
            ack INTEGER NOT NULL DEFAULT 0,
            UNIQUE(sender, id)
        );
        CREATE INDEX unread ON messages(recipient,ack,receipt);
        INSERT INTO seats VALUES('captain',x'01',100);
        INSERT INTO seats VALUES('chief',x'02',100);
        INSERT INTO messages VALUES(17,'chief','captain','first','retained',0);",
    )?;
    drop(old);

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
    assert!(Store::open(root.path()).is_err());

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let store = Store::open(root.path())?;
    assert!(store.write(|tx| tx.seats().touch("captain", 102))?);
    assert_eq!(store.read()?.seats().names()?, ["captain", "chief"]);

    let pending = store.read()?.messages().unread("captain")?;
    assert_eq!(pending.len(), 1);
    assert_eq!(
        (
            pending[0].receipt,
            pending[0].sender.as_str(),
            pending[0].id.as_str(),
            pending[0].body.as_str(),
        ),
        (17, "chief", "first", "retained")
    );
    Ok(())
}

#[test]
fn new_room_tables_have_singular_names() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|_| Ok(()))?;

    let connection = cw_sqlite::rusqlite::Connection::open(root.path().join("inbox.sqlite3"))?;
    let mut tables = connection.prepare(
        "SELECT name FROM sqlite_schema
          WHERE type = 'table' AND name NOT GLOB 'sqlite_*'",
    )?;

    for name in tables.query_map([], |row| row.get::<_, String>(0))? {
        let name = name?;
        let legacy_name = matches!(name.as_str(), "seats" | "messages");
        assert!(
            legacy_name || !name.ends_with('s'),
            "plural room table: {name}"
        );
    }
    Ok(())
}
