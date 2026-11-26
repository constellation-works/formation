//! Exercise the shipped unversioned SQLite format at the public store boundary.

use cw_sqlite::rusqlite::Connection;
use formation_store::contract::{HookProvider, NewWatch};
use formation_store::{SessionId, Store};
use std::os::unix::fs::PermissionsExt;

fn drop_hook_state(connection: &Connection) -> cw_sqlite::rusqlite::Result<()> {
    connection.execute_batch(
        "ALTER TABLE seats DROP COLUMN wake_requested;
         ALTER TABLE seats DROP COLUMN hook_busy;
         ALTER TABLE seats DROP COLUMN hook_provider;",
    )
}

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
    assert_eq!(pending[0].sent, None);
    assert!(store.read()?.seats().idle_codex("captain")?.is_none());
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

#[test]
fn upgrade_to_the_watch_set_retains_mail_and_handovers() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|tx| {
        tx.seats().bind("captain", None)?;
        tx.seats().bind("chief", None)?;
        tx.messages()
            .insert("chief", "captain", "first", "retained", 100)?;
        tx.watch().mark_handed("captain", "ci", "fp-1", 100)?;
        Ok(())
    })?;

    // Rewind the room to the schema before the watch set, as an earlier build left it.
    let path = root.path().join("inbox.sqlite3");
    let connection = cw_sqlite::rusqlite::Connection::open(&path)?;
    drop_hook_state(&connection)?;
    connection.execute_batch(
        "DROP TABLE watch_reader; DROP TABLE watch_participant;
         DROP TABLE watch_set; PRAGMA user_version = 7;",
    )?;
    drop(connection);

    assert!(store.read()?.watch().list()?.is_empty());
    store.write(|tx| {
        let watch = NewWatch {
            recipient: "captain",
            name: "ci",
            command: "true",
            cwd: "/",
            every: 120,
            added_by: "chief",
            added_at: 200,
        };
        assert!(!tx.watch().put(&watch)?);
        Ok(())
    })?;

    let snapshot = store.read()?;
    let watches = snapshot.watch().list()?;
    assert_eq!(watches.len(), 1);
    assert_eq!((watches[0].name.as_str(), watches[0].remembered), ("ci", 1));
    let pending = snapshot.messages().unread("captain")?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].body, "retained");
    Ok(())
}

#[test]
fn upgrade_to_the_reader_retains_existing_watch_rows() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let watch = NewWatch {
        recipient: "captain",
        name: "ci",
        command: "true",
        cwd: "/",
        every: 120,
        added_by: "captain",
        added_at: 100,
    };
    store.write(|tx| tx.watch().put(&watch))?;

    let connection = cw_sqlite::rusqlite::Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_hook_state(&connection)?;
    connection.execute_batch(
        "DROP TABLE watch_reader; DROP TABLE watch_participant;
         ALTER TABLE watch_set DROP COLUMN revision; PRAGMA user_version = 8;",
    )?;
    let snapshot = store.read()?;
    assert_eq!(snapshot.watch().list()?[0].command, "true");
    assert!(snapshot.watch().readers(100)?.reader.is_none());
    assert_eq!(
        connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))?,
        8
    );
    drop(snapshot);
    drop(connection);

    store.write(|tx| {
        tx.watch_reader().register("captain", 1, 100, 145)?;
        let entry = tx.watch().list()?.remove(0);
        assert!(tx.watch().record_read(&entry, 101, "ok, 0 items")?);
        Ok(())
    })?;
    let snapshot = store.read()?;
    assert_eq!(snapshot.watch().list()?[0].last_read, Some(101));
    assert!(snapshot.watch().readers(101)?.reader.is_some());
    Ok(())
}

#[test]
fn upgrade_keeps_binding_without_granting_a_wake() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let session = SessionId::try_new("receiver")?;
    let receipt = store.write(|tx| {
        tx.seats().bind("captain", Some(&session))?;
        tx.seats().bind("chief", None)?;
        tx.messages()
            .insert("chief", "captain", "first", "retained", 100)
    })?;
    let connection = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_hook_state(&connection)?;
    connection.execute_batch("PRAGMA user_version = 9;")?;
    drop(connection);

    let snapshot = store.read()?;
    assert_eq!(
        snapshot.seats().by_session(&session)?.as_deref(),
        Some("captain")
    );
    assert!(snapshot.seats().idle_codex("captain")?.is_none());
    drop(snapshot);

    assert!(store.write(|tx| tx.seats().touch("captain", 102))?);
    let snapshot = store.read()?;
    assert!(snapshot.seats().idle_codex("captain")?.is_none());
    assert_eq!(snapshot.messages().unread("captain")?[0].receipt, receipt);
    drop(snapshot);

    assert!(store.write(|tx| tx.seats().mark_hook(&session, HookProvider::Codex, false))?);
    assert_eq!(store.read()?.seats().idle_codex("captain")?, Some(session));
    Ok(())
}
