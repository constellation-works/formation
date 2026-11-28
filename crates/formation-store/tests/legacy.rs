//! Exercise the shipped unversioned SQLite format at the public store boundary.

use cw_sqlite::rusqlite::Connection;
use formation_store::contract::{HookProvider, NewWatch, WatchHandover};
use formation_store::{SessionId, Store};
use std::os::unix::fs::PermissionsExt;

fn drop_hook_state(connection: &Connection) -> cw_sqlite::rusqlite::Result<()> {
    connection.execute_batch(
        "DROP TABLE room_request;
         DROP TABLE hook_waiter;
         ALTER TABLE watch_handover DROP COLUMN absent_reads;
         DROP TABLE watch_notice;
         ALTER TABLE messages DROP COLUMN watch_name;
         ALTER TABLE messages DROP COLUMN watch_added_by;
         ALTER TABLE seats DROP COLUMN wake_requested;
         ALTER TABLE seats DROP COLUMN hook_busy;
         ALTER TABLE seats DROP COLUMN hook_provider;",
    )
}

/// Undo the request-mail step, so the file matches release 22's schema.
fn drop_request_mail(connection: &Connection) -> cw_sqlite::rusqlite::Result<()> {
    connection.execute_batch("ALTER TABLE messages DROP COLUMN request_id;")
}

/// Undo the house grants step, so the file matches release 21's schema.
fn drop_house(connection: &Connection) -> cw_sqlite::rusqlite::Result<()> {
    drop_request_mail(connection)?;
    connection.execute_batch(
        "DROP TABLE room_house_catalog;
         ALTER TABLE room_grant DROP COLUMN origin;",
    )
}

/// Undo the admiral steps, so the file matches release 19's schema.
fn drop_admiral(connection: &Connection) -> cw_sqlite::rusqlite::Result<()> {
    drop_house(connection)?;
    connection.execute_batch(
        "ALTER TABLE messages DROP COLUMN terminal;
         DELETE FROM seats WHERE name = 'admiral';",
    )
}

fn drop_roster_details(connection: &Connection) -> cw_sqlite::rusqlite::Result<()> {
    drop_admiral(connection)?;
    connection.execute_batch(
        "DROP TABLE room_close;
         DROP TABLE roster_application;
         ALTER TABLE roster_seat DROP COLUMN model;
         ALTER TABLE roster_seat DROP COLUMN effort;
         ALTER TABLE roster_seat DROP COLUMN record;",
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
    assert_eq!(
        store.read()?.seats().names()?,
        ["admiral", "captain", "chief"]
    );

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
    drop_roster_details(&connection)?;
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
    drop_roster_details(&connection)?;
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
    drop_roster_details(&connection)?;
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

#[test]
fn upgrade_to_watch_absence_retains_handovers_with_zero_missed_reads()
-> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|tx| {
        tx.watch().put(&NewWatch {
            recipient: "captain",
            name: "ci",
            command: "true",
            cwd: "/",
            every: 120,
            added_by: "captain",
            added_at: 100,
        })?;
        tx.watch().mark_handed("captain", "ci", "fp-1", 100)?;
        Ok(())
    })?;

    let connection = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_roster_details(&connection)?;
    connection.execute_batch(
        "DROP TABLE room_request;
         DROP TABLE hook_waiter;
         ALTER TABLE watch_handover DROP COLUMN absent_reads;
         PRAGMA user_version = 11;",
    )?;
    let snapshot = store.read()?;
    assert_eq!(snapshot.watch().list()?[0].remembered, 1);
    assert_eq!(
        connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))?,
        11
    );
    drop(snapshot);
    drop(connection);

    let remembered = store.write(|tx| tx.watch().remembered("captain", "ci"))?;
    assert_eq!(
        remembered,
        vec![WatchHandover {
            fingerprint: "fp-1".to_owned(),
            absent_reads: 0,
        }]
    );
    Ok(())
}

#[test]
fn upgrade_to_hook_waiter_retains_mail_and_seat_links() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let session = SessionId::try_new("claude-receiver")?;
    let receipt = store.write(|tx| {
        tx.seats().bind("captain", Some(&session))?;
        tx.seats().bind("chief", None)?;
        tx.seats()
            .mark_hook(&session, HookProvider::Claude, false)?;
        tx.messages()
            .insert("chief", "captain", "first", "retained", 100)
    })?;

    let connection = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_roster_details(&connection)?;
    connection.execute_batch(
        "DROP TABLE room_request;
         DROP TABLE hook_waiter;
         PRAGMA user_version = 12;",
    )?;
    let snapshot = store.read()?;
    assert_eq!(
        snapshot.seats().by_session(&session)?.as_deref(),
        Some("captain")
    );
    assert!(
        !snapshot
            .hook_waiter()
            .holds_lease("captain", 1, &session, 100)?
    );
    assert_eq!(snapshot.messages().unread("captain")?[0].receipt, receipt);
    assert_eq!(
        connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))?,
        12
    );
    drop(snapshot);
    drop(connection);

    let token = store.write(|tx| tx.hook_waiter().claim("captain", &session, 42, 101, 146))?;
    assert_eq!(token, Some(1));
    let snapshot = store.read()?;
    assert_eq!(
        snapshot.seats().by_session(&session)?.as_deref(),
        Some("captain")
    );
    assert!(
        snapshot
            .hook_waiter()
            .holds_lease("captain", 1, &session, 145)?
    );
    let unread = snapshot.messages().unread("captain")?;
    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].receipt, receipt);
    assert_eq!(unread[0].body, "retained");
    Ok(())
}

#[test]
fn upgrade_to_roster_preserves_room_records() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let session = SessionId::try_new("session")?;
    store.write(|tx| {
        tx.seats().bind("chief", Some(&session))?;
        tx.messages().insert("chief", "chief", "id", "body", 100)?;
        Ok(())
    })?;
    let db = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_roster_details(&db)?;
    db.execute_batch(
        "INSERT INTO watch_set (recipient,name,command,cwd,every,state,added_by,added_at)
         VALUES ('chief','probe','true','/',10,'on','chief',100);
         DROP TABLE room_request;
         DROP TABLE roster_seat; PRAGMA user_version = 13;",
    )?;
    drop(db);
    assert!(store.read()?.roster().seats()?.is_empty());
    let roster = formation_store::decode(
        br#"{"version":2,"seats":[{"name":"chief","role":"chief","host":"headless"}],"routes":[]}"#,
    )?;
    store.write(|tx| tx.roster().replace(&roster))?;
    let snapshot = store.read()?;
    assert_eq!(snapshot.roster().seats()?.len(), 1);
    assert_eq!(
        snapshot.seats().by_session(&session)?.as_deref(),
        Some("chief")
    );
    assert_eq!(snapshot.messages().unread("chief")?[0].body, "body");
    assert_eq!(snapshot.watch().list()?.len(), 1);
    Ok(())
}

#[test]
fn upgrade_to_grant_preserves_room_records() -> Result<(), Box<dyn std::error::Error>> {
    use formation_store::contract::{
        GrantEffect, GrantLifetime, GrantOrigin, GrantTargetKind, NewGrant,
    };

    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let session = SessionId::try_new("grant-session")?;
    let roster = formation_store::decode(
        br#"{"version":2,"seats":[{"name":"chief","role":"chief","host":"headless"}],"routes":[]}"#,
    )?;
    store.write(|tx| {
        tx.seats().bind("chief", Some(&session))?;
        tx.roster().replace(&roster)?;
        tx.messages().insert("chief", "chief", "id", "body", 100)?;
        Ok(())
    })?;
    let db = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_roster_details(&db)?;
    db.execute_batch(
        "DROP TABLE room_request;
         DROP TABLE room_grant; PRAGMA user_version = 14;",
    )?;
    drop(db);

    assert!(store.read()?.grants().active()?.is_empty());
    let inserted = store.write(|tx| {
        tx.grants().insert(&NewGrant {
            effect: GrantEffect::Grant,
            target_kind: GrantTargetKind::Rank,
            target: "chief",
            class: "direction",
            workspace: "all",
            limit: Some("reversible settings"),
            lifetime: GrantLifetime::Day,
            origin: GrantOrigin::Room,
            source: "advisor chat 12:00",
            entered_by: None,
            entered_at: 200,
            entered_at_ns: 0,
        })
    })?;
    assert_eq!(inserted.id, 1);

    let snapshot = store.read()?;
    assert_eq!(snapshot.grants().active()?.len(), 1);
    assert_eq!(snapshot.roster().seats()?.len(), 1);
    assert_eq!(
        snapshot.seats().by_session(&session)?.as_deref(),
        Some("chief")
    );
    assert_eq!(snapshot.messages().unread("chief")?[0].body, "body");
    Ok(())
}

#[test]
fn upgrade_to_request_preserves_room_records() -> Result<(), Box<dyn std::error::Error>> {
    use formation_store::contract::{Request, RequestOption, RequestState};

    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let roster = formation_store::decode(
        br#"{"version":2,"seats":[{"name":"chief","role":"chief","host":"headless"}],"routes":[]}"#,
    )?;
    store.write(|tx| {
        tx.seats().bind("chief", None)?;
        tx.roster().replace(&roster)?;
        tx.messages().insert("chief", "chief", "id", "body", 100)?;
        Ok(())
    })?;

    let db = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_roster_details(&db)?;
    db.execute_batch("DROP TABLE room_request; PRAGMA user_version = 15;")?;
    let snapshot = store.read()?;
    assert!(snapshot.requests().list()?.is_empty());
    assert!(snapshot.requests().get("req-trial-new")?.is_none());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))?,
        15
    );
    drop(snapshot);
    drop(db);

    let now = time::OffsetDateTime::from_unix_timestamp_nanos(120_000_000_001)?;
    let request = Request {
        id: "req-trial-new".to_owned(),
        asker: "chief".to_owned(),
        class: "design".to_owned(),
        workspace: "all".to_owned(),
        record: "change".to_owned(),
        ask: "Make the change?".to_owned(),
        options: vec![
            RequestOption {
                label: "a".to_owned(),
                text: "Proceed".to_owned(),
            },
            RequestOption {
                label: "b".to_owned(),
                text: "Wait".to_owned(),
            },
        ],
        recommendation: "a".to_owned(),
        blocking: false,
        opened_at: now,
        updated_at: now,
        state: RequestState::Open { holder: None },
        advice: Vec::new(),
        overturned: Vec::new(),
    };
    store.write(|tx| tx.requests().insert(&request))?;

    let snapshot = store.read()?;
    let requests = snapshot.requests().list()?;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].opened_at, now);
    assert_eq!(requests[0].id, request.id);
    assert!(matches!(
        requests[0].state,
        RequestState::Open { holder: None }
    ));
    assert_eq!(snapshot.roster().seats()?.len(), 1);
    assert_eq!(snapshot.messages().unread("chief")?[0].body, "body");
    Ok(())
}

#[test]
fn request_history_upgrade_reads_version_sixteen_before_and_after_write()
-> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|_| Ok(()))?;
    let db = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_roster_details(&db)?;
    db.execute_batch("ALTER TABLE room_request DROP COLUMN overturned; PRAGMA user_version = 16;")?;
    drop(db);
    assert!(store.read()?.requests().list()?.is_empty());
    store.write(|_| Ok(()))?;
    assert!(store.read()?.requests().list()?.is_empty());
    Ok(())
}

#[test]
fn room_close_upgrades_version_eighteen_after_reading_it_as_active()
-> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|tx| {
        tx.seats().bind("chief", None)?;
        tx.roster().stamp(200, Some("chief"))
    })?;
    let path = root.path().join("inbox.sqlite3");
    let db = Connection::open(&path)?;
    drop_admiral(&db)?;
    db.execute_batch("DROP TABLE room_close; PRAGMA user_version = 18;")?;
    drop(db);
    // The old binary would have left its WAL companions in place.
    store.read()?;
    let before = std::fs::read(&path)?;
    {
        let snapshot = store.read_strict()?;
        assert_eq!(snapshot.closure()?, None);
        assert_eq!(
            snapshot.roster().application()?,
            Some((200, Some("chief".to_owned())))
        );
    }
    assert_eq!(before, std::fs::read(&path)?);
    store.write(|tx| tx.close_room(300, "chief"))?;
    let snapshot = store.read_strict()?;
    assert_eq!(snapshot.closure()?, Some((300, "chief".to_owned())));
    assert_eq!(snapshot.seats().names()?, ["admiral", "chief"]);
    Ok(())
}

#[test]
fn a_release_nineteen_room_gains_the_admiral_seat_on_its_next_write()
-> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let session = SessionId::try_new("taken-admiral")?;
    store.write(|tx| {
        tx.seats().bind("chief", None)?;
        tx.messages().insert("chief", "chief", "note", "kept", 100)
    })?;
    let db = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_admiral(&db)?;
    // An earlier build let a session join under the name.
    db.execute_batch(
        "INSERT INTO seats (name, session_id, seen) VALUES ('admiral', 'taken-admiral', 150);
         PRAGMA user_version = 19;",
    )?;
    drop(db);
    assert_eq!(
        store.read()?.seats().by_session(&session)?.as_deref(),
        Some("admiral")
    );

    store.write(|_| Ok(()))?;
    let snapshot = store.read()?;
    assert_eq!(snapshot.seats().names()?, ["admiral", "chief"]);
    assert_eq!(snapshot.seats().by_session(&session)?, None);
    let admiral = snapshot.seats().presence()?;
    assert!(admiral.contains(&("admiral".to_owned(), 0)), "{admiral:?}");
    let kept = snapshot.messages().unread("chief")?;
    assert_eq!((kept[0].body.as_str(), kept[0].terminal), ("kept", false));
    Ok(())
}

#[test]
fn a_release_twenty_one_room_keeps_its_grants_as_the_rooms_own()
-> Result<(), Box<dyn std::error::Error>> {
    use formation_store::contract::{
        GrantEffect, GrantLifetime, GrantOrigin, GrantTargetKind, NewGrant,
    };

    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|tx| {
        tx.grants().insert(&NewGrant {
            effect: GrantEffect::Grant,
            target_kind: GrantTargetKind::Rank,
            target: "chief",
            class: "direction",
            workspace: "all",
            limit: None,
            lifetime: GrantLifetime::Day,
            origin: GrantOrigin::Room,
            source: "advisor chat 12:00",
            entered_by: None,
            entered_at: 200,
            entered_at_ns: 0,
        })?;
        Ok(())
    })?;
    let db = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_house(&db)?;
    db.execute_batch("PRAGMA user_version = 21;")?;
    drop(db);

    let snapshot = store.read()?;
    let grants = snapshot.grants().active()?;
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0].origin, GrantOrigin::Room);
    assert!(snapshot.grants().house_classes()?.is_empty());
    drop(snapshot);

    store.write(|tx| {
        tx.grants().replace_house_classes(&[])?;
        Ok(())
    })?;
    let grants = store.read()?.grants().active()?;
    assert_eq!(grants[0].origin, GrantOrigin::Room);
    Ok(())
}

#[test]
fn a_new_room_has_the_admiral_seat_from_the_start() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|_| Ok(()))?;
    let snapshot = store.read()?;
    assert_eq!(snapshot.seats().names()?, ["admiral"]);
    assert_eq!(
        snapshot.seats().by_session(&SessionId::try_new("any")?)?,
        None
    );
    Ok(())
}

#[test]
fn a_release_twenty_two_room_upgrades_for_request_mail_and_keeps_plain_mail()
-> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|tx| {
        tx.seats().bind("chief", None)?;
        tx.messages()
            .insert("chief", "admiral", "plain-1", "Keep this mail", 100)
    })?;
    let db = Connection::open(root.path().join("inbox.sqlite3"))?;
    drop_request_mail(&db)?;
    db.execute_batch("PRAGMA user_version = 22;")?;
    drop(db);

    store.write(|tx| {
        let receipt =
            tx.messages()
                .insert("chief", "admiral", "req-1", "Request req-r-1 passed.", 101)?;
        tx.messages().mark_request(receipt, "req-r-1")?;
        assert_eq!(tx.messages().ack_request("admiral", "req-r-1")?, 1);
        Ok(())
    })?;

    let unread = store.read()?.messages().unread("admiral")?;
    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].id, "plain-1");
    Ok(())
}
