//! The boundary around the RFC 0006 broker mode, guarded from the first
//! slice rather than after the implementation lands.
//!
//! The broker will eventually be the sole writable owner of the fixture's
//! store, reachable only over authenticated IPC. Long before it does anything,
//! one property has to hold and keep holding: **a shipped build must contain
//! no broker mode at all** — not a disabled one, not one behind a runtime
//! check, not even the string that names it. A hidden subcommand that exists
//! but refuses is still an entry point someone can find and probe.
//!
//! These tests run against the built binary in both configurations, so the
//! property is asserted about the artifact rather than about the source.

use std::process::Command;

const HIDDEN: &str = "__owner-effect-broker";

/// The default build — the one that ships — must not carry the mode.
///
/// Two assertions, because they can fail independently: clap must not accept
/// the subcommand, and the binary must not contain its name. The second
/// catches a variant added without its `cfg`, which would still parse.
#[test]
fn default_binary_has_no_hidden_broker_mode_or_symbols() {
    let binary = env!("CARGO_BIN_EXE_sovereign");

    // Skip when the test binary was itself built with the fixture feature;
    // that configuration is the subject of the next test, not this one.
    if cfg!(feature = "owner-effect-fixture") {
        return;
    }

    let output = Command::new(binary)
        .arg(HIDDEN)
        .output()
        .expect("run the CLI");
    assert!(
        !output.status.success(),
        "a default build accepted {HIDDEN}, so the broker mode is reachable"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand") || stderr.contains("unexpected argument"),
        "expected the argument parser to reject {HIDDEN}, got: {stderr}"
    );

    let bytes = std::fs::read(binary).expect("read the built binary");
    let needle = HIDDEN.as_bytes();
    assert!(
        !bytes.windows(needle.len()).any(|window| window == needle),
        "the default binary contains the string {HIDDEN}; the variant is not cfg-gated"
    );
}

/// Under the fixture feature the mode exists, and — for now — fails closed
/// while saying exactly why.
///
/// A stub that exited successfully would be indistinguishable from a working
/// broker to anything written against it, which is how scaffolding turns into
/// a false claim that a store is owned.
#[cfg(feature = "owner-effect-fixture")]
#[test]
fn the_fixture_build_has_the_mode_and_it_fails_closed() {
    let output = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .arg(HIDDEN)
        .output()
        .expect("run the CLI");
    assert!(
        !output.status.success(),
        "the scaffolding broker must not report success — it owns no store"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("not implemented"),
        "the stub must say what it is, got: {combined}"
    );
}
