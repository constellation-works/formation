//! Exercise per-seat hook waiter leases and fencing at the public store boundary.

use formation_store::{SessionId, Store};

#[test]
fn waiter_lease_fences_stale_owners_and_isolates_seats() -> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let alpha = SessionId::try_new("session-alpha")?;
    let beta = SessionId::try_new("session-beta")?;

    let (alpha_first, beta_first) = store.write(|tx| {
        tx.seats().bind("captain", Some(&alpha))?;
        tx.seats().bind("sergeant", Some(&beta))?;

        // Claiming for a mismatched seat-session pair is refused.
        assert!(
            tx.hook_waiter()
                .claim("captain", &beta, 100, 10, 55)?
                .is_none()
        );

        let alpha_token = tx.hook_waiter().claim("captain", &alpha, 101, 10, 55)?;
        assert_eq!(alpha_token, Some(1));
        let alpha_token = alpha_token.unwrap_or_default();

        // Concurrent or later claim while live is refused without replacing the owner.
        assert!(
            tx.hook_waiter()
                .claim("captain", &alpha, 102, 20, 65)?
                .is_none()
        );
        assert!(
            tx.hook_waiter()
                .holds_lease("captain", alpha_token, &alpha, 54)?
        );
        assert!(
            !tx.hook_waiter()
                .holds_lease("captain", alpha_token, &alpha, 55)?
        );

        // Independent seats hold independent leases.
        let beta_token = tx.hook_waiter().claim("sergeant", &beta, 201, 20, 65)?;
        assert_eq!(beta_token, Some(1));
        let beta_token = beta_token.unwrap_or_default();
        Ok((alpha_token, beta_token))
    })?;

    assert!(
        store
            .read()?
            .hook_waiter()
            .holds_lease("captain", alpha_first, &alpha, 54)?
    );
    assert!(
        store
            .read()?
            .hook_waiter()
            .holds_lease("sergeant", beta_first, &beta, 64)?
    );

    store.write(|tx| {
        let waiter = tx.hook_waiter();
        // Renewal works while unexpired, and fails once expired.
        assert!(waiter.renew("captain", alpha_first, &alpha, 30, 75)?);
        assert!(waiter.holds_lease("captain", alpha_first, &alpha, 74)?);
        assert!(!waiter.renew("captain", alpha_first, &alpha, 75, 120)?);

        // Takeover after expiry bumps the token and fences the stale owner.
        let alpha_second = waiter.claim("captain", &alpha, 103, 75, 120)?;
        assert_eq!(alpha_second, Some(2));
        let alpha_second = alpha_second.unwrap_or_default();
        assert!(!waiter.holds_lease("captain", alpha_first, &alpha, 75)?);
        assert!(!waiter.renew("captain", alpha_first, &alpha, 76, 121)?);
        assert!(!waiter.release("captain", alpha_first)?);
        assert!(waiter.holds_lease("captain", alpha_second, &alpha, 76)?);

        // Ordinary release clears ownership while preserving monotonicity for the next claim.
        assert!(waiter.release("captain", alpha_second)?);
        assert!(!waiter.release("captain", alpha_second)?);
        let alpha_third = waiter.claim("captain", &alpha, 104, 77, 122)?;
        assert_eq!(alpha_third, Some(3));
        assert!(waiter.holds_lease("sergeant", beta_first, &beta, 64)?);
        Ok(())
    })?;
    Ok(())
}

#[test]
fn seat_rebinding_invalidates_prior_waiters_and_prevents_aba_reuse()
-> Result<(), Box<dyn std::error::Error>> {
    let root = cw_testkit::TempRoot::new("FORMATION")?;
    let store = Store::create(root.path())?;
    let original = SessionId::try_new("session-original")?;
    let replacement = SessionId::try_new("session-replacement")?;

    store.write(|tx| {
        tx.seats().bind("captain", Some(&original))?;
        let first = tx.hook_waiter().claim("captain", &original, 10, 100, 145)?;
        assert_eq!(first, Some(1));
        let first = first.unwrap_or_default();

        // Idempotent re-join by the same session keeps the live waiter intact.
        tx.seats().bind("captain", Some(&original))?;
        assert!(
            tx.hook_waiter()
                .holds_lease("captain", first, &original, 110)?
        );

        // Rebinding captain to a new session immediately clears the prior waiter lease.
        tx.seats().bind("captain", Some(&replacement))?;
        assert!(
            !tx.hook_waiter()
                .holds_lease("captain", first, &original, 110)?
        );
        assert!(
            !tx.hook_waiter()
                .renew("captain", first, &original, 110, 155)?
        );

        let second = tx
            .hook_waiter()
            .claim("captain", &replacement, 20, 110, 155)?;
        assert_eq!(second, Some(2));
        let second = second.unwrap_or_default();
        assert!(!tx.hook_waiter().release("captain", first)?);
        assert!(
            tx.hook_waiter()
                .holds_lease("captain", second, &replacement, 110)?
        );

        // ABA: rebinding back to the original session still fences token 1.
        tx.seats().bind("captain", Some(&original))?;
        assert!(
            !tx.hook_waiter()
                .holds_lease("captain", first, &original, 115)?
        );
        assert!(
            !tx.hook_waiter()
                .holds_lease("captain", second, &replacement, 115)?
        );
        let third = tx.hook_waiter().claim("captain", &original, 30, 115, 160)?;
        assert_eq!(third, Some(3));
        let third = third.unwrap_or_default();

        // Moving the session to another seat in the same room clears the old seat's lease.
        tx.seats().bind("chief", Some(&original))?;
        assert!(
            !tx.hook_waiter()
                .holds_lease("captain", third, &original, 120)?
        );
        assert!(
            !tx.hook_waiter()
                .renew("captain", third, &original, 120, 165)?
        );

        // Unbinding a seat also clears its live lease.
        let chief_token = tx.hook_waiter().claim("chief", &original, 31, 120, 165)?;
        assert_eq!(chief_token, Some(1));
        let chief_token = chief_token.unwrap_or_default();
        tx.seats().bind("chief", None)?;
        assert!(
            !tx.hook_waiter()
                .holds_lease("chief", chief_token, &original, 125)?
        );
        Ok(())
    })?;
    Ok(())
}
