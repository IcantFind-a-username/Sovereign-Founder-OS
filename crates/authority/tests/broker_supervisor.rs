//! Authenticating the one connection allowed to control the broker.
//!
//! The listener admits every process on the machine. What separates the
//! parent from a stranger that found an open port is possession of the launch
//! key, so these tests are about exactly that: a hello proves the key or it
//! proves nothing, and a hello that was valid somewhere else is not valid
//! here.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::protocol::{Diagnostic, KEY_LEN, NONCE_LEN};
use sovereign_authority::broker::supervisor::{authenticate, encode_hello, HELLO_LEN, MAC_LEN};

const KEY: [u8; KEY_LEN] = [11; KEY_LEN];
const NONCE: [u8; NONCE_LEN] = [22; NONCE_LEN];

fn read_from(bytes: &[u8]) -> std::io::Cursor<Vec<u8>> {
    std::io::Cursor::new(bytes.to_vec())
}

#[test]
fn a_hello_under_the_launch_key_authenticates() {
    let hello = encode_hello(&KEY, &NONCE);
    assert_eq!(hello.len(), HELLO_LEN);
    assert!(authenticate(&mut read_from(&hello), &KEY, &NONCE).is_ok());
}

/// A stranger that found the open port has everything except the key.
#[test]
fn a_hello_under_the_wrong_key_is_refused() {
    let hello = encode_hello(&[99; KEY_LEN], &NONCE);
    assert_eq!(
        authenticate(&mut read_from(&hello), &KEY, &NONCE).err(),
        Some(Diagnostic::SupervisorUnauthenticated)
    );
}

/// The nonce binds a hello to the broker that published it. Without this, a
/// hello captured from an earlier run — same key, same machine — would open
/// the next broker too.
#[test]
fn a_hello_for_another_brokers_nonce_is_refused() {
    let hello = encode_hello(&KEY, &[33; NONCE_LEN]);
    assert_eq!(
        authenticate(&mut read_from(&hello), &KEY, &NONCE).err(),
        Some(Diagnostic::SupervisorUnauthenticated)
    );
}

/// Every single-bit change to the tag must fail. A verification that got the
/// length right but the comparison wrong would still pass the tests above.
#[test]
fn every_flipped_tag_bit_is_refused() {
    let hello = encode_hello(&KEY, &NONCE);
    let tag_at = HELLO_LEN - MAC_LEN;
    for byte in 0..MAC_LEN {
        for bit in 0..8 {
            let mut tampered = hello.clone();
            tampered[tag_at + byte] ^= 1 << bit;
            assert_eq!(
                authenticate(&mut read_from(&tampered), &KEY, &NONCE).err(),
                Some(Diagnostic::SupervisorUnauthenticated),
                "flipping bit {bit} of tag byte {byte} was accepted"
            );
        }
    }
}

/// Tampering anywhere in the signed region must fail too — the MAC covers the
/// magic and the version, not only the nonce.
#[test]
fn tampering_with_the_signed_region_is_refused() {
    let hello = encode_hello(&KEY, &NONCE);
    for index in 0..(HELLO_LEN - MAC_LEN) {
        let mut tampered = hello.clone();
        tampered[index] ^= 0x01;
        assert_eq!(
            authenticate(&mut read_from(&tampered), &KEY, &NONCE).err(),
            Some(Diagnostic::SupervisorUnauthenticated),
            "tampering with byte {index} of the signed region was accepted"
        );
    }
}

/// A hello is fixed size. A short one is a caller that went away, and must
/// fail closed rather than be padded or partially accepted.
#[test]
fn a_short_hello_fails_closed() {
    let hello = encode_hello(&KEY, &NONCE);
    for cut in [0, 1, HELLO_LEN / 2, HELLO_LEN - 1] {
        assert!(
            authenticate(&mut read_from(&hello[..cut]), &KEY, &NONCE).is_err(),
            "a hello cut to {cut} bytes was accepted"
        );
    }
}

/// Extra bytes after the hello are not read. Nothing is negotiated on this
/// connection, so a caller cannot make the broker consume more than one
/// fixed-size frame before it has authenticated.
#[test]
fn only_the_hello_is_consumed() {
    let mut bytes = encode_hello(&KEY, &NONCE);
    bytes.extend_from_slice(&[0xAB; 4096]);
    let mut reader = read_from(&bytes);
    assert!(authenticate(&mut reader, &KEY, &NONCE).is_ok());
    assert_eq!(
        reader.position(),
        HELLO_LEN as u64,
        "authentication consumed more than the hello"
    );
}

/// Over a real socket, not a cursor: the same hello that authenticates from
/// memory must authenticate from the wire, and a stranger's must not.
#[test]
fn the_handshake_works_over_a_real_loopback_socket() {
    use sovereign_authority::broker::listener::{accept_by_deadline, bind_and_publish};
    use std::io::Write;

    for (key, expected_ok) in [(KEY, true), ([44; KEY_LEN], false)] {
        let mut published = Vec::new();
        let established = bind_and_publish(&NONCE, &mut published).unwrap();
        let port = established.port();

        let sender = std::thread::spawn(move || {
            let mut client = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            client.write_all(&encode_hello(&key, &NONCE)).unwrap();
            client.flush().unwrap();
            // Hold the connection until the broker side has read.
            std::thread::sleep(std::time::Duration::from_millis(200));
        });

        let mut supervisor = accept_by_deadline(&established).unwrap();
        let outcome = authenticate(&mut supervisor, &KEY, &NONCE);
        sender.join().unwrap();

        assert_eq!(
            outcome.is_ok(),
            expected_ok,
            "over the wire, a hello under {} key gave {outcome:?}",
            if expected_ok {
                "the right"
            } else {
                "the wrong"
            }
        );
    }
}
