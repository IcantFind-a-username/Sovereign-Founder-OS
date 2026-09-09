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

/// A valid frame naming a valid fixture root gets past every check that does
/// not need a second party, and then waits for one.
///
/// This is what "the ordering is the contract" looks like from outside: the
/// broker does not refuse, and it does not proceed either. It binds and waits
/// for a supervisor, and when none authenticates within the fixed deadline it
/// exits having claimed nothing.
#[cfg(feature = "owner-effect-fixture")]
#[test]
fn a_valid_bootstrap_with_no_supervisor_times_out_having_claimed_nothing() {
    use sovereign_authority::broker::bootstrap::FIXTURE_MARKER;
    use sovereign_authority::broker::process_lock::LOCK_FILE;
    use sovereign_authority::broker::protocol::encode;
    use sovereign_authority::broker::store::STORE_FILE;
    use std::io::Write;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("fixture-root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(FIXTURE_MARKER), b"synthetic").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .arg(HIDDEN)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the hidden mode");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&encode(&[1; 32], &[2; 16], &root))
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E-SUPERVISOR-TIMEOUT"),
        "expected the deadline to end it, got: {stderr}"
    );
    // Nothing is claimed before a supervisor authenticates. This is the
    // ordering the whole design rests on, seen from the filesystem.
    assert!(
        !root.join(LOCK_FILE).exists(),
        "the lock was taken before any supervisor authenticated"
    );
    assert!(
        !root.join(STORE_FILE).exists(),
        "the store was opened before any supervisor authenticated"
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

/// The whole chain, end to end, through the real binary.
///
/// Everything before this tested one link. This is the only test that shows
/// the links join up, and it is deliberately written against the protocol
/// rather than against the broker's own client: a handshake where both halves
/// share an implementation can agree with itself and still be wrong. Here the
/// parent's half is written out, so the broker is measured against an
/// independent reading of the same protocol.
#[cfg(feature = "owner-effect-fixture")]
#[test]
fn a_parent_starts_a_broker_and_becomes_its_supervisor() {
    use sovereign_authority::broker::bootstrap::FIXTURE_MARKER;
    use sovereign_authority::broker::process_lock::LOCK_FILE;
    use sovereign_authority::broker::protocol::{decode_address, encode, ADDRESS_FRAME_LEN};
    use sovereign_authority::broker::store::STORE_FILE;
    use sovereign_authority::broker::supervisor::encode_hello;
    use std::io::{Read, Write};

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("fixture-root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(FIXTURE_MARKER), b"synthetic").unwrap();

    let launch_key = [0x5Au8; 32];
    let nonce = [0x3Cu8; 16];

    let mut child = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .arg(HIDDEN)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the broker");

    // One frame on stdin, then the pipe closes: the broker gets exactly one.
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(&encode(&launch_key, &nonce, &root))
            .unwrap();
    }

    // Exactly one address frame, and no more.
    let mut published = [0u8; ADDRESS_FRAME_LEN];
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(&mut published)
        .expect("an address frame");
    let (port, echoed) = decode_address(&published).expect("the address must decode");
    assert_eq!(echoed, nonce, "the broker echoed another parent's nonce");

    let mut supervisor = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    supervisor
        .write_all(&encode_hello(&launch_key, &nonce))
        .unwrap();
    supervisor.flush().unwrap();

    // The broker got far enough to claim the root: both artefacts exist. It
    // creates neither before the handshake, so their presence is the proof
    // that authentication succeeded.
    let mut claimed = false;
    for _ in 0..100 {
        if root.join(LOCK_FILE).is_file() && root.join(STORE_FILE).is_file() {
            claimed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        claimed,
        "the broker never took the lock and opened the store, so the handshake did not complete"
    );
}

/// A broker over a product-looking root refuses before it claims anything,
/// even though the parent's half of the handshake is perfectly valid.
#[cfg(feature = "owner-effect-fixture")]
#[test]
fn a_valid_parent_over_a_product_root_claims_nothing() {
    use sovereign_authority::broker::bootstrap::FIXTURE_MARKER;
    use sovereign_authority::broker::process_lock::LOCK_FILE;
    use sovereign_authority::broker::protocol::encode;
    use sovereign_authority::broker::store::STORE_FILE;
    use std::io::Write;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("looks-like-product");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(FIXTURE_MARKER), b"synthetic").unwrap();
    std::fs::write(root.join("device.json"), b"{}").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .arg(HIDDEN)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the broker");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&encode(&[0x11; 32], &[0x22; 16], &root))
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-ROOT-REJECTED"), "got: {stderr}");
    assert!(!root.join(LOCK_FILE).exists(), "a refused root was locked");
    assert!(
        !root.join(STORE_FILE).exists(),
        "a refused root got a store"
    );
}
