use super::*;
use tempfile::tempdir;

const NOW: i64 = 1_800_000_000;

#[test]
fn one_use_consumption_survives_reopen() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let token = Uuid::new_v4();
    store.consume_token(token, NOW, NOW + 60).unwrap();
    assert_eq!(
        store.consume_token(token, NOW + 1, NOW + 60),
        Err(AuthorityError::AlreadyConsumed)
    );

    // "Restart": a fresh instance over the same directory still refuses.
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    assert_eq!(
        reopened.consume_token(token, NOW + 2, NOW + 60),
        Err(AuthorityError::AlreadyConsumed)
    );
    assert_eq!(
        reopened.consume_approval(token, NOW, NOW + 60),
        Ok(()),
        "token and approval namespaces are separate"
    );
}

#[test]
fn concurrent_racers_get_exactly_one_win() {
    let dir = tempdir().unwrap();
    let token = Uuid::new_v4();
    let root = dir.path().to_path_buf();
    let winners: usize = std::thread::scope(|scope| {
        (0..16)
            .map(|_| {
                let root = root.clone();
                scope.spawn(move || {
                    let store = AuthorityStore::open(&root).unwrap();
                    store.consume_token(token, NOW, NOW + 60).is_ok() as usize
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .sum()
    });
    assert_eq!(winners, 1);
}

#[test]
fn idempotency_distinguishes_replay_from_conflict() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let key = Uuid::new_v4();
    let fingerprint_a = [0xAA_u8; 32];
    let fingerprint_b = [0xBB_u8; 32];

    store
        .bind_idempotency(key, &fingerprint_a, NOW, NOW + 60)
        .unwrap();
    assert_eq!(
        store.bind_idempotency(key, &fingerprint_a, NOW, NOW + 60),
        Err(AuthorityError::IdempotencyReplay)
    );
    // Across a "restart" a different fingerprint is a conflict.
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    assert_eq!(
        reopened.bind_idempotency(key, &fingerprint_b, NOW, NOW + 60),
        Err(AuthorityError::IdempotencyConflict)
    );
}

#[test]
fn unavailable_store_fails_closed() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("not-a-directory");
    std::fs::write(&file_path, b"occupied").unwrap();
    assert!(matches!(
        AuthorityStore::open(&file_path),
        Err(AuthorityError::Unavailable(_))
    ));
}

#[test]
fn purge_removes_expired_and_keeps_live_records() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let expired = Uuid::new_v4();
    let live = Uuid::new_v4();
    store.consume_token(expired, NOW, NOW + 10).unwrap();
    store.consume_token(live, NOW, NOW + 1_000).unwrap();
    // Orphan temp file from a simulated crash is collected, never trusted.
    std::fs::write(dir.path().join("tokens").join("tmp-orphan"), b"junk").unwrap();

    let removed = store.purge_expired(NOW + 100).unwrap();
    assert_eq!(removed, 1);
    assert_eq!(
        store.consume_token(live, NOW + 101, NOW + 1_000),
        Err(AuthorityError::AlreadyConsumed),
        "live record must survive the purge"
    );
    assert!(
        store.consume_token(expired, NOW + 101, NOW + 200).is_ok(),
        "purged ids are reclaimable; expiry checks upstream deny stale authorities"
    );
}

#[test]
fn purge_uses_each_claim_kind_expiry() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let token = Uuid::new_v4();
    let approval = Uuid::new_v4();
    let idempotency = Uuid::new_v4();

    store.consume_token(token, NOW, NOW + 30).unwrap();
    store.consume_approval(approval, NOW, NOW + 120).unwrap();
    store
        .bind_idempotency(idempotency, &[0x01; 32], NOW, NOW + 30)
        .unwrap();

    assert_eq!(store.purge_expired(NOW + 31).unwrap(), 2);
    assert_eq!(
        store.consume_approval(approval, NOW + 31, NOW + 120),
        Err(AuthorityError::AlreadyConsumed),
        "the approval must retain its own later expiry"
    );
    assert_eq!(store.purge_expired(NOW + 120).unwrap(), 1);
}

#[test]
fn purged_idempotency_key_can_be_rebound_without_false_conflict() {
    // An idempotency key that has expired and been purged must not haunt a
    // later, unrelated invocation as a phantom conflict — otherwise expired
    // keys would wedge new work forever. After purge, the same key rebinds
    // to a different fingerprint cleanly.
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let key = Uuid::new_v4();
    let first = [1u8; 32];
    let second = [2u8; 32];

    store.bind_idempotency(key, &first, NOW, NOW + 10).unwrap();
    // While live, a different fingerprint is a real conflict.
    assert_eq!(
        store.bind_idempotency(key, &second, NOW + 1, NOW + 10),
        Err(AuthorityError::IdempotencyConflict)
    );

    // After expiry + purge the record is gone, and the key is free again.
    assert_eq!(store.purge_expired(NOW + 100).unwrap(), 1);
    assert_eq!(
        store.bind_idempotency(key, &second, NOW + 101, NOW + 200),
        Ok(())
    );
    // And it is once more a live one-use binding.
    assert_eq!(
        store.bind_idempotency(key, &second, NOW + 102, NOW + 200),
        Err(AuthorityError::IdempotencyReplay)
    );
}

#[test]
fn corrupt_records_fail_closed() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let key = Uuid::new_v4();
    std::fs::write(
        dir.path().join("idempotency").join(key.to_string()),
        b"garbage",
    )
    .unwrap();
    assert_eq!(
        store.bind_idempotency(key, &[0x01; 32], NOW, NOW + 60),
        Err(AuthorityError::CorruptRecord)
    );
}

fn part(expires_at_unix: i64) -> BundlePart {
    BundlePart {
        id: Uuid::new_v4(),
        expires_at_unix,
    }
}

#[test]
fn a_retried_bundle_after_any_interruption_completes_without_burning_claims() {
    for stop_after_step in 0..=5 {
        let dir = tempdir().unwrap();
        let store = AuthorityStore::open(dir.path()).unwrap();
        let token = part(NOW + 60);
        let approval = part(NOW + 60);
        let idempotency = part(NOW + 60);
        let fingerprint = [0x42_u8; 32];
        let bundle_hex = compute_bundle_hex(token.id, approval.id, idempotency.id, &fingerprint);

        if stop_after_step >= 1 {
            store
                .bundle_publish_intent(&bundle_hex, token, approval, idempotency, &fingerprint, NOW)
                .unwrap();
        }
        if stop_after_step >= 2 {
            store
                .bundle_check_revocation(token.id, approval.id)
                .unwrap();
        }
        if stop_after_step >= 3 {
            store.bundle_claim_token(&bundle_hex, token, NOW).unwrap();
        }
        if stop_after_step >= 4 {
            store
                .bundle_bind_idempotency(&bundle_hex, idempotency, &fingerprint, NOW)
                .unwrap();
        }
        if stop_after_step >= 5 {
            store
                .bundle_claim_approval(&bundle_hex, approval, NOW)
                .unwrap();
        }

        assert_eq!(
            store.consume_bundle(token, approval, idempotency, &fingerprint, NOW + 1),
            Ok(()),
            "retry after stopping at step {stop_after_step} must complete, not burn"
        );
    }
}

#[test]
fn racing_bundles_over_the_same_token_have_exactly_one_winner() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let token_id = Uuid::new_v4();

    let winners: usize = std::thread::scope(|scope| {
        (0..16u8)
            .map(|i| {
                let root = root.clone();
                scope.spawn(move || {
                    let store = AuthorityStore::open(&root).unwrap();
                    let token = BundlePart {
                        id: token_id,
                        expires_at_unix: NOW + 60,
                    };
                    let approval = part(NOW + 60);
                    let idempotency = part(NOW + 60);
                    let fingerprint = [i; 32];
                    store
                        .consume_bundle(token, approval, idempotency, &fingerprint, NOW)
                        .is_ok() as usize
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .sum()
    });
    assert_eq!(winners, 1);
}

#[test]
fn racing_retries_of_the_same_bundle_authorize_exactly_once() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let token = part(NOW + 60);
    let approval = part(NOW + 60);
    let idempotency = part(NOW + 60);
    let fingerprint = [0x77_u8; 32];

    let authorized: usize = std::thread::scope(|scope| {
        (0..16)
            .map(|_| {
                let root = root.clone();
                scope.spawn(move || {
                    let store = AuthorityStore::open(&root).unwrap();
                    store
                        .consume_bundle(token, approval, idempotency, &fingerprint, NOW)
                        .is_ok() as usize
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .sum()
    });
    assert_eq!(authorized, 1);
}

#[test]
fn a_reopened_store_answers_a_partial_bundle_identically() {
    let dir = tempdir().unwrap();
    let token = part(NOW + 60);
    let approval = part(NOW + 60);
    let idempotency = part(NOW + 60);
    let fingerprint = [0x11_u8; 32];
    let bundle_hex = compute_bundle_hex(token.id, approval.id, idempotency.id, &fingerprint);

    {
        let store = AuthorityStore::open(dir.path()).unwrap();
        store
            .bundle_publish_intent(&bundle_hex, token, approval, idempotency, &fingerprint, NOW)
            .unwrap();
        store.bundle_claim_token(&bundle_hex, token, NOW).unwrap();
        // "Crash" here: the store handle is dropped without committing.
    }

    let reopened = AuthorityStore::open(dir.path()).unwrap();

    // A foreign bundle contending for the same token is denied on the
    // reopened store exactly as it would have been on the original.
    let foreign_approval = part(NOW + 60);
    let foreign_idempotency = part(NOW + 60);
    assert_eq!(
        reopened.consume_bundle(
            token,
            foreign_approval,
            foreign_idempotency,
            &[0x22; 32],
            NOW + 1
        ),
        Err(AuthorityError::AlreadyConsumed)
    );

    // The original consumer's own retry against the reopened store
    // completes the bundle it had already partly claimed.
    assert_eq!(
        reopened.consume_bundle(token, approval, idempotency, &fingerprint, NOW + 2),
        Ok(())
    );
}

#[test]
fn a_foreign_uncommitted_bundle_denies_other_consumers() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let token = part(NOW + 60);
    let approval_a = part(NOW + 60);
    let idempotency_a = part(NOW + 60);
    let fingerprint_a = [0x33_u8; 32];
    let bundle_hex_a =
        compute_bundle_hex(token.id, approval_a.id, idempotency_a.id, &fingerprint_a);

    // Bundle A claims the token but never commits (simulated crash).
    store
        .bundle_publish_intent(
            &bundle_hex_a,
            token,
            approval_a,
            idempotency_a,
            &fingerprint_a,
            NOW,
        )
        .unwrap();
    store.bundle_claim_token(&bundle_hex_a, token, NOW).unwrap();

    // Bundle B, over the same token but otherwise unrelated, is denied
    // even though bundle A never committed.
    let approval_b = part(NOW + 60);
    let idempotency_b = part(NOW + 60);
    assert_eq!(
        store.consume_bundle(token, approval_b, idempotency_b, &[0x44; 32], NOW + 1),
        Err(AuthorityError::AlreadyConsumed)
    );
}

#[test]
fn purge_removes_expired_bundles_and_their_claims() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let token = part(NOW + 30);
    let approval = part(NOW + 30);
    let idempotency = part(NOW + 30);
    let fingerprint = [0x55_u8; 32];

    store
        .consume_bundle(token, approval, idempotency, &fingerprint, NOW)
        .unwrap();

    let removed = store.purge_expired(NOW + 31).unwrap();
    assert_eq!(
        removed, 5,
        "token, approval, and idempotency claims plus the bundle intent and committed marker"
    );

    // Everything purged: the exact same bundle can be authorized again.
    assert_eq!(
        store.consume_bundle(token, approval, idempotency, &fingerprint, NOW + 32),
        Ok(())
    );
}

#[test]
fn a_revoked_token_fails_closed_across_reopen() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let token = Uuid::new_v4();

    assert_eq!(
        store.revoke_token(token, NOW, NOW + 60),
        Ok(RevocationOutcome::Revoked)
    );
    assert_eq!(
        store.consume_token(token, NOW + 1, NOW + 60),
        Err(AuthorityError::Revoked)
    );

    // "Restart": a fresh instance over the same directory still refuses.
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    assert_eq!(
        reopened.consume_token(token, NOW + 2, NOW + 60),
        Err(AuthorityError::Revoked)
    );

    // A revoked token also denies a bundle that tries to claim it.
    let approval = part(NOW + 60);
    let idempotency = part(NOW + 60);
    assert_eq!(
        reopened.consume_bundle(
            BundlePart {
                id: token,
                expires_at_unix: NOW + 60
            },
            approval,
            idempotency,
            &[0x66; 32],
            NOW + 3
        ),
        Err(AuthorityError::Revoked)
    );
}

#[test]
fn revoking_a_consumed_claim_reports_the_distinct_outcome() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();

    // Legacy single-claim consumption.
    let token = Uuid::new_v4();
    store.consume_token(token, NOW, NOW + 60).unwrap();
    assert_eq!(
        store.revoke_token(token, NOW + 1, NOW + 60),
        Ok(RevocationOutcome::RevokedAfterConsumption)
    );
    // A second revoke of the same subject reports the record already exists,
    // not a second "after consumption" claim.
    assert_eq!(
        store.revoke_token(token, NOW + 2, NOW + 60),
        Ok(RevocationOutcome::AlreadyRevoked)
    );

    // Bundle-committed consumption reports the same distinct outcome.
    let bundle_token = part(NOW + 60);
    let approval = part(NOW + 60);
    let idempotency = part(NOW + 60);
    let fingerprint = [0x88_u8; 32];
    store
        .consume_bundle(bundle_token, approval, idempotency, &fingerprint, NOW)
        .unwrap();
    assert_eq!(
        store.revoke_token(bundle_token.id, NOW + 1, bundle_token.expires_at_unix),
        Ok(RevocationOutcome::RevokedAfterConsumption)
    );
    assert_eq!(
        store.revoke_approval(approval.id, NOW + 1, approval.expires_at_unix),
        Ok(RevocationOutcome::RevokedAfterConsumption)
    );

    // A token claimed by a bundle that never committed is NOT yet
    // "authorized" — revoking it is a clean revoke, not after-consumption.
    let uncommitted_token = part(NOW + 60);
    let uncommitted_approval = part(NOW + 60);
    let uncommitted_idempotency = part(NOW + 60);
    let uncommitted_fingerprint = [0x99_u8; 32];
    let uncommitted_bundle_hex = compute_bundle_hex(
        uncommitted_token.id,
        uncommitted_approval.id,
        uncommitted_idempotency.id,
        &uncommitted_fingerprint,
    );
    store
        .bundle_publish_intent(
            &uncommitted_bundle_hex,
            uncommitted_token,
            uncommitted_approval,
            uncommitted_idempotency,
            &uncommitted_fingerprint,
            NOW,
        )
        .unwrap();
    store
        .bundle_claim_token(&uncommitted_bundle_hex, uncommitted_token, NOW)
        .unwrap();
    assert_eq!(
        store.revoke_token(
            uncommitted_token.id,
            NOW + 1,
            uncommitted_token.expires_at_unix
        ),
        Ok(RevocationOutcome::Revoked),
        "an uncommitted bundle's provisional claim is not yet an authorized effect"
    );
}

#[test]
fn a_revoke_vs_consume_race_ends_in_one_durable_outcome() {
    for _ in 0..20 {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let token = part(NOW + 60);
        let approval = part(NOW + 60);
        let idempotency = part(NOW + 60);
        let fingerprint = [0x99_u8; 32];

        let (consume_result, revoke_result) = std::thread::scope(|scope| {
            let root_a = root.clone();
            let consume = scope.spawn(move || {
                AuthorityStore::open(&root_a).unwrap().consume_bundle(
                    token,
                    approval,
                    idempotency,
                    &fingerprint,
                    NOW,
                )
            });
            let root_b = root.clone();
            let revoke = scope.spawn(move || {
                AuthorityStore::open(&root_b).unwrap().revoke_token(
                    token.id,
                    NOW,
                    token.expires_at_unix,
                )
            });
            (consume.join().unwrap(), revoke.join().unwrap())
        });

        let revoke_result = revoke_result.expect("revoking must itself always succeed durably");
        match (consume_result, revoke_result) {
            (Ok(()), RevocationOutcome::RevokedAfterConsumption) => {}
            (Err(AuthorityError::Revoked), RevocationOutcome::Revoked) => {}
            other => panic!("unexpected race outcome: {other:?}"),
        }
    }
}

#[test]
fn a_corrupt_revocation_record_fails_closed() {
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let token = Uuid::new_v4();
    std::fs::write(
        dir.path().join("revoked-tokens").join(token.to_string()),
        b"garbage",
    )
    .unwrap();
    assert_eq!(
        store.consume_token(token, NOW, NOW + 60),
        Err(AuthorityError::CorruptRecord)
    );

    let approval = Uuid::new_v4();
    std::fs::write(
        dir.path()
            .join("revoked-approvals")
            .join(approval.to_string()),
        b"garbage",
    )
    .unwrap();
    assert_eq!(
        store.consume_approval(approval, NOW, NOW + 60),
        Err(AuthorityError::CorruptRecord)
    );
}
