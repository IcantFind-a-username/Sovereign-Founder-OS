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

/// A valid frame naming a valid fixture root passes every check this slice
/// implements and then stops, saying exactly that.
///
/// The stub must not exit successfully: to anything written against it, a
/// successful broker that owns no store is indistinguishable from a working
/// one, which is how scaffolding becomes a false claim.
#[cfg(feature = "owner-effect-fixture")]
#[test]
fn a_valid_bootstrap_and_root_reach_the_not_implemented_boundary() {
    use sovereign_authority::broker::bootstrap::FIXTURE_MARKER;
    use sovereign_authority::broker::protocol::encode;
    use std::io::Write;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("fixture-root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(FIXTURE_MARKER), b"synthetic").unwrap();

    let frame = encode(&[1; 32], &[2; 16], &root);
    let mut child = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .arg(HIDDEN)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the hidden mode");
    child.stdin.take().unwrap().write_all(&frame).unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(
        !output.status.success(),
        "the scaffolding broker must not report success — it owns no store"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not implemented"),
        "a valid bootstrap and root must reach the boundary and say so, got: {stderr}"
    );
    assert!(
        !stderr.contains("E-"),
        "no check should have refused this input: {stderr}"
    );
}

/// The claim this slice exists to make good on, tested through the real
/// binary rather than the library: a same-account caller can reach the hidden
/// mode with a syntactically perfect frame — RFC 0006 says so and calls it
/// unqualified fixture control, never product admission — and it still cannot
/// aim the broker at the owner's data.
#[cfg(feature = "owner-effect-fixture")]
#[test]
fn direct_valid_bootstrap_rejects_a_product_root_before_opening_anything() {
    use sovereign_authority::broker::bootstrap::FIXTURE_MARKER;
    use sovereign_authority::broker::protocol::encode;
    use std::io::Write;

    let dir = tempfile::tempdir().unwrap();

    // A root the caller has marked as a fixture, which also holds product
    // state — the shape of "point the broker at my real vault".
    let root = dir.path().join("looks-like-product");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(FIXTURE_MARKER), b"synthetic").unwrap();
    std::fs::write(root.join("device.json"), b"{}").unwrap();

    let frame = encode(&[3; 32], &[4; 16], &root);
    let mut child = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .arg(HIDDEN)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the hidden mode");
    child.stdin.take().unwrap().write_all(&frame).unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E-ROOT-REJECTED"),
        "a product-looking root must be refused, got: {stderr}"
    );
    // The diagnostic is a fixed code and nothing else: an error channel that
    // echoed its input would be a disclosure channel.
    assert!(
        !stderr.contains(&root.display().to_string()),
        "the diagnostic leaked the path it was given: {stderr}"
    );
}

/// Running the hidden mode with nothing on stdin — what a curious caller does
/// first — ends it before it looks at any root.
#[cfg(feature = "owner-effect-fixture")]
#[test]
fn direct_hidden_broker_without_valid_stdin_exits_before_examining_a_root() {
    let output = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .arg(HIDDEN)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run the hidden mode");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E-BOOTSTRAP-MISSING"),
        "expected the missing-bootstrap code, got: {stderr}"
    );
}
