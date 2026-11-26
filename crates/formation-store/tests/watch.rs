//! Exercise the watch set at the public store boundary.

use formation_store::Store;
use formation_store::contract::{NewWatch, WatchRole, WatchState};

fn new<'a>(recipient: &'a str, name: &'a str, command: &'a str, added_by: &'a str) -> NewWatch<'a> {
    NewWatch {
        recipient,
        name,
        command,
        cwd: "/work",
        every: 120,
        added_by,
        added_at: 100,
    }
}

#[test]
fn watches_are_keyed_by_recipient_and_name() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|tx| {
        assert!(!tx.watch().put(&new("beta", "ci", "one", "alpha"))?);
        assert!(!tx.watch().put(&new("alpha", "ci", "two", "alpha"))?);
        assert!(
            !tx.watch()
                .put(&new("alpha", "attention", "three", "beta"))?
        );
        Ok(())
    })?;

    let watches = store.read()?.watch().list()?;
    let keys: Vec<_> = watches
        .iter()
        .map(|watch| (watch.name.as_str(), watch.recipient.as_str()))
        .collect();
    assert_eq!(
        keys,
        [("attention", "alpha"), ("ci", "alpha"), ("ci", "beta")]
    );
    assert_eq!(watches[0].added_by, "beta");
    assert_eq!(watches[0].cwd, "/work");
    assert_eq!(watches[0].every, 120);
    assert_eq!(watches[0].state, WatchState::On);
    assert_eq!(
        (watches[0].last_read, watches[0].last_result.as_deref()),
        (None, None)
    );
    Ok(())
}

#[test]
fn replacing_keeps_fingerprints_and_removal_forgets_them() -> Result<(), Box<dyn std::error::Error>>
{
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    store.write(|tx| {
        tx.watch().put(&new("alpha", "ci", "one", "alpha"))?;
        tx.watch().put(&new("beta", "ci", "other", "beta"))?;
        tx.watch().mark_handed("alpha", "ci", "fp-1", 100)?;
        tx.watch().mark_handed("alpha", "ci", "fp-2", 100)?;
        tx.watch().mark_handed("beta", "ci", "fp-1", 100)?;
        tx.watch().record_failure("alpha", "ci", "broke", 100)?;
        assert_eq!(
            tx.watch().set_state("alpha", "ci", WatchState::Paused)?,
            Some(true)
        );
        assert_eq!(
            tx.watch().set_state("alpha", "ci", WatchState::Paused)?,
            Some(false)
        );
        assert_eq!(tx.watch().set_state("alpha", "gone", WatchState::On)?, None);
        Ok(())
    })?;

    store.write(|tx| {
        let mut replacement = new("alpha", "ci", "two", "beta");
        replacement.every = 30;
        replacement.added_at = 200;
        assert!(tx.watch().put(&replacement)?);
        Ok(())
    })?;
    let watches = store.read()?.watch().list()?;
    let alpha = &watches[0];
    assert_eq!(
        (
            alpha.command.as_str(),
            alpha.every,
            alpha.state,
            alpha.added_by.as_str()
        ),
        ("two", 30, WatchState::On, "beta")
    );
    assert_eq!((alpha.added_at, alpha.remembered), (200, 2));
    assert_eq!(watches[1].remembered, 1);

    store.write(|tx| {
        assert_eq!(tx.watch().remove("alpha", "ci")?, Some(2));
        assert_eq!(tx.watch().remove("alpha", "ci")?, None);
        assert!(!tx.watch().is_handed("alpha", "ci", "fp-1")?);
        assert!(tx.watch().is_handed("beta", "ci", "fp-1")?);
        assert!(tx.watch().record_failure("alpha", "ci", "again", 300)?);
        Ok(())
    })?;
    assert_eq!(store.read()?.watch().list()?.len(), 1);
    Ok(())
}

#[test]
fn lease_expiry_and_stop_fence_previous_invocations() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let (first, standby) = store.write(|tx| {
        let lease = tx.watch_reader();
        let (first, role) = lease.register("alpha", 10, 0, 45)?;
        assert!(matches!(role, WatchRole::Reader));
        let (standby, role) = lease.register("beta", 11, 15, 60)?;
        assert!(matches!(role, WatchRole::Standby));
        Ok((first, standby))
    })?;
    store.write(|tx| {
        let lease = tx.watch_reader();
        assert!(lease.holds_lease(first, 44)?);
        assert!(matches!(
            lease.pulse(standby, 44, None)?,
            Some(WatchRole::Standby)
        ));
        assert!(matches!(
            lease.pulse(standby, 45, Some(90))?,
            Some(WatchRole::Reader)
        ));
        assert!(!lease.holds_lease(first, 45)?);
        assert!(lease.pulse(first, 45, Some(90))?.is_none());

        lease.stop(46)?;
        let (restart, role) = lease.register("beta", 11, 47, 92)?;
        assert_ne!(standby, restart);
        assert!(matches!(role, WatchRole::Reader));
        assert!(lease.pulse(standby, 47, Some(92))?.is_none());
        lease.leave(standby, 47)?;
        assert!(lease.holds_lease(restart, 47)?);
        Ok(())
    })?;

    let status = store.read()?.watch().readers(47)?;
    assert!(status.stopped_at.is_none());
    assert!(status.standbys.is_empty());
    assert_eq!(
        status.reader.map(|reader| (reader.pid, reader.heartbeat)),
        Some((11, 47)),
    );
    Ok(())
}

#[test]
fn in_flight_reads_cannot_update_edited_watches() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let definition = new("alpha", "ci", "true", "alpha");
    store.write(|tx| tx.watch().put(&definition))?;
    let mut watches = store.read()?.watch().list()?;
    let original = watches.remove(0);
    store.write(|tx| {
        let watch = tx.watch();
        watch.set_state("alpha", "ci", WatchState::Paused)?;
        watch.set_state("alpha", "ci", WatchState::On)?;
        assert!(!watch.record_read(&original, 100, "stale")?);
        watch.remove("alpha", "ci")?;
        watch.put(&definition)?;
        assert!(!watch.record_read(&original, 100, "stale")?);
        let current = watch.list()?.remove(0);
        assert!(watch.record_read(&current, 101, "ok, 0 items")?);
        Ok(())
    })?;

    let current = store.read()?.watch().list()?.remove(0);
    assert_eq!(current.last_read, Some(101));
    assert_eq!(current.last_result.as_deref(), Some("ok, 0 items"));
    Ok(())
}
