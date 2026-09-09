//! Session lifetime: every way a session ends, and the one way it does not.
//!
//! A ceremony is a moment and a session is a span. These tests are about
//! keeping the span short and its end unambiguous — which means checking the
//! two expiries answer different questions, that a cookie alone is not a
//! session, and that a restart really is a revocation rather than a hope.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_owner::session::{
    SessionError, SessionTokens, Sessions, ABSOLUTE_LIFETIME, IDLE_TIMEOUT, TOKEN_LEN,
};
use std::time::Instant;
use uuid::Uuid;

/// Deterministic values, one per call, so a test can name what it presents.
fn counter() -> impl FnMut() -> [u8; TOKEN_LEN] {
    let mut next = 0u8;
    move || {
        next = next.wrapping_add(1);
        [next; TOKEN_LEN]
    }
}

#[test]
fn a_fresh_session_authenticates() {
    let mut sessions = Sessions::new();
    let subject = Uuid::new_v4();
    let now = Instant::now();
    let tokens = sessions.issue(subject, now, counter());

    assert_eq!(sessions.authenticate(&tokens, now), Ok(subject));
}

/// The cookie and the CSRF value are independent. Deriving one from the other
/// would mean whoever learned either — a cookie is sent on every request, a
/// CSRF token is rendered into a page — could compute the other, and the pair
/// exists precisely because those exposures differ.
#[test]
fn the_cookie_and_csrf_are_different_values() {
    let mut sessions = Sessions::new();
    let tokens = sessions.issue(Uuid::new_v4(), Instant::now(), counter());
    assert_ne!(tokens.cookie, tokens.csrf);
}

/// Both values rotate on every issue. A reused cookie would let an old
/// holder's value name a new session.
#[test]
fn every_issue_rotates_both_values() {
    let mut sessions = Sessions::new();
    let mut random = counter();
    let now = Instant::now();
    let first = sessions.issue(Uuid::new_v4(), now, &mut random);
    let second = sessions.issue(Uuid::new_v4(), now, &mut random);

    assert_ne!(first.cookie, second.cookie);
    assert_ne!(first.csrf, second.csrf);
}

/// A cookie alone is not a session. This is the whole reason for the second
/// value: a cross-origin page can cause a browser to send the cookie, and
/// cannot make it send something it never learned.
#[test]
fn a_cookie_without_the_matching_csrf_is_refused() {
    let mut sessions = Sessions::new();
    let now = Instant::now();
    let tokens = sessions.issue(Uuid::new_v4(), now, counter());

    let cookie_only = SessionTokens {
        cookie: tokens.cookie,
        csrf: [0xFF; TOKEN_LEN],
    };
    assert_eq!(
        sessions.authenticate(&cookie_only, now),
        Err(SessionError::CsrfMismatch)
    );
}

/// The absolute expiry bounds how long a session may exist however busy it
/// is. A session that could be kept alive by using it is one an attacker
/// keeps alive by using it.
#[test]
fn the_absolute_lifetime_cannot_be_extended_by_use() {
    let mut sessions = Sessions::new();
    let start = Instant::now();
    let tokens = sessions.issue(Uuid::new_v4(), start, counter());

    // Used steadily, well inside the idle timeout, right up to the boundary.
    let step = IDLE_TIMEOUT / 2;
    let mut now = start;
    while now.duration_since(start) + step < ABSOLUTE_LIFETIME {
        now += step;
        assert!(
            sessions.authenticate(&tokens, now).is_ok(),
            "an active session expired early at {:?}",
            now.duration_since(start)
        );
    }

    let past = start + ABSOLUTE_LIFETIME;
    assert_eq!(
        sessions.authenticate(&tokens, past),
        Err(SessionError::Expired),
        "constant use extended the absolute lifetime"
    );
}

/// The idle expiry bounds how long a session may sit unused, which is a
/// different question from how long it may exist.
#[test]
fn an_unused_session_expires_on_the_idle_timeout() {
    let mut sessions = Sessions::new();
    let start = Instant::now();
    let tokens = sessions.issue(Uuid::new_v4(), start, counter());

    assert_eq!(
        sessions.authenticate(&tokens, start + IDLE_TIMEOUT),
        Err(SessionError::Idle)
    );
    // And the idle clock is well inside the absolute one, so this really is
    // the idle rule firing rather than the other.
    assert!(IDLE_TIMEOUT < ABSOLUTE_LIFETIME);
}

/// Expiry is decided before the CSRF value is compared, so an expired session
/// cannot be used as an oracle for guessing one.
#[test]
fn an_expired_session_reports_expiry_not_a_csrf_mismatch() {
    let mut sessions = Sessions::new();
    let start = Instant::now();
    let tokens = sessions.issue(Uuid::new_v4(), start, counter());

    let wrong = SessionTokens {
        cookie: tokens.cookie,
        csrf: [0xFF; TOKEN_LEN],
    };
    assert_eq!(
        sessions.authenticate(&wrong, start + ABSOLUTE_LIFETIME),
        Err(SessionError::Expired),
        "an expired session leaked whether a CSRF guess was right"
    );
}

/// Logout removes the entry rather than flagging it, so there is no second
/// step that could fail and leave the session usable.
#[test]
fn logout_ends_the_session_immediately_and_completely() {
    let mut sessions = Sessions::new();
    let now = Instant::now();
    let tokens = sessions.issue(Uuid::new_v4(), now, counter());
    assert_eq!(sessions.len(), 1);

    sessions.log_out(&tokens.cookie);

    assert!(sessions.is_empty(), "logout left state behind");
    assert_eq!(
        sessions.authenticate(&tokens, now),
        Err(SessionError::Unknown),
        "a logged-out session still authenticated"
    );
}

#[test]
fn logging_out_an_unknown_session_is_harmless() {
    let mut sessions = Sessions::new();
    let now = Instant::now();
    let tokens = sessions.issue(Uuid::new_v4(), now, counter());

    sessions.log_out(&[0x99; TOKEN_LEN]);
    assert!(sessions.authenticate(&tokens, now).is_ok());
}

/// Sessions live in memory and nowhere else, so a restart is a revocation.
/// A session that survived one would be a session whose holder could not be
/// reached by restarting, and the fixture has no way to revoke such a thing.
#[test]
fn restart_invalidates_every_session() {
    let mut sessions = Sessions::new();
    let now = Instant::now();
    let tokens = sessions.issue(Uuid::new_v4(), now, counter());
    assert!(sessions.authenticate(&tokens, now).is_ok());

    // Dropping the store is what a restart does to it.
    drop(sessions);
    let mut restarted = Sessions::new();

    assert_eq!(
        restarted.authenticate(&tokens, now),
        Err(SessionError::Unknown),
        "a session survived a restart"
    );
    // And nothing on disk could bring it back: the module holds no path.
    let source = include_str!("../src/session.rs");
    for persistent in ["File", "fs::", "PathBuf", "serde"] {
        assert!(
            !source.contains(persistent),
            "the session store touches {persistent}, so it may outlive a restart"
        );
    }
}

/// Purging frees memory; it does not decide anything. `authenticate` already
/// refuses an expired session, so a store that was never purged is still
/// correct — just larger.
#[test]
fn purging_removes_dead_sessions_without_being_the_rule() {
    let mut sessions = Sessions::new();
    let start = Instant::now();
    let mut random = counter();
    let old = sessions.issue(Uuid::new_v4(), start, &mut random);
    let fresh_at = start + ABSOLUTE_LIFETIME - IDLE_TIMEOUT / 2;
    let fresh = sessions.issue(Uuid::new_v4(), fresh_at, &mut random);
    assert_eq!(sessions.len(), 2);

    sessions.purge(fresh_at);

    assert_eq!(sessions.len(), 1, "purge removed the wrong number");
    assert_eq!(
        sessions.authenticate(&old, fresh_at),
        Err(SessionError::Unknown)
    );
    assert!(sessions.authenticate(&fresh, fresh_at).is_ok());
}

/// The token pair exists to carry secrets, so its rendering must not.
#[test]
fn session_tokens_never_print_themselves() {
    let mut sessions = Sessions::new();
    let tokens = sessions.issue(Uuid::new_v4(), Instant::now(), counter());
    let rendered = format!("{tokens:?}");
    assert!(rendered.contains("redacted"), "{rendered}");
    assert!(!rendered.contains('['), "the bytes appear: {rendered}");
}
