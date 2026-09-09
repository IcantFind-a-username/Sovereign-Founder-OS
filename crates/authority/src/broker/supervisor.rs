//! Authenticating the one connection that may control the broker.
//!
//! Binding a loopback port admits every process on the machine, so the port
//! is not the boundary — the launch key is. The parent generated it, sent it
//! only on the broker's stdin, and never wrote it to argv, the environment or
//! disk. A connection that can produce a MAC over the published nonce under
//! that key is the parent; anything else is a stranger that found an open
//! port.
//!
//! Three things this deliberately does not do. It does not compare MACs with
//! `==`, because a byte-by-byte comparison leaks how much of a guess was
//! right through timing; verification goes through `Mac::verify_slice`, which
//! is constant time. It does not accept a hello whose nonce differs from the
//! one just published, so a hello captured from an earlier broker cannot be
//! replayed at this one. And it gives the read no timeout of its own — the
//! caller has already bounded the socket with what remains of the single
//! establishment deadline.

use super::protocol::{Diagnostic, KEY_LEN, NONCE_LEN, VERSION};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::io::Read;

type HmacSha256 = Hmac<Sha256>;

pub const HELLO_MAGIC: &[u8; 12] = b"SFO-HELLO-1\n";
pub const MAC_LEN: usize = 32;
pub const HELLO_LEN: usize = HELLO_MAGIC.len() + 2 + NONCE_LEN + MAC_LEN;

/// The bytes a MAC is taken over: everything in the hello except the MAC.
fn signed_region(nonce: &[u8; NONCE_LEN]) -> Vec<u8> {
    let mut region = Vec::with_capacity(HELLO_MAGIC.len() + 2 + NONCE_LEN);
    region.extend_from_slice(HELLO_MAGIC);
    region.extend_from_slice(&VERSION.to_le_bytes());
    region.extend_from_slice(nonce);
    region
}

/// Build a hello. The parent uses this; so do the tests, so the two cannot
/// drift apart into a passing test and a broken handshake.
pub fn encode_hello(launch_key: &[u8; KEY_LEN], nonce: &[u8; NONCE_LEN]) -> Vec<u8> {
    let region = signed_region(nonce);
    let mut mac = HmacSha256::new_from_slice(launch_key).expect("hmac accepts any key length");
    mac.update(&region);
    let tag = mac.finalize().into_bytes();

    let mut hello = region;
    hello.extend_from_slice(&tag);
    hello
}

/// Read and authenticate one hello.
///
/// Reads exactly `HELLO_LEN` bytes and no more: a hello is fixed size, so
/// there is nothing to negotiate and nothing to buffer. A short read is a
/// caller that went away or a deadline that expired, and both fail closed.
pub fn authenticate(
    stream: &mut impl Read,
    launch_key: &[u8; KEY_LEN],
    expected_nonce: &[u8; NONCE_LEN],
) -> Result<(), Diagnostic> {
    let mut hello = [0u8; HELLO_LEN];
    if let Err(error) = stream.read_exact(&mut hello) {
        return Err(match error.kind() {
            // The socket's read timeout is the establishment deadline, so a
            // timeout here means nobody authenticated in time.
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                Diagnostic::SupervisorTimeout
            }
            _ => Diagnostic::SupervisorLost,
        });
    }

    let mut at = 0;
    if &hello[..HELLO_MAGIC.len()] != HELLO_MAGIC.as_slice() {
        return Err(Diagnostic::SupervisorUnauthenticated);
    }
    at += HELLO_MAGIC.len();
    let version = u16::from_le_bytes(hello[at..at + 2].try_into().expect("2 bytes"));
    at += 2;
    if version != VERSION {
        return Err(Diagnostic::SupervisorUnauthenticated);
    }

    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&hello[at..at + NONCE_LEN]);
    at += NONCE_LEN;
    // A hello for some other broker's nonce is not this broker's parent, even
    // if whoever sent it holds a valid key from an earlier run.
    if &nonce != expected_nonce {
        return Err(Diagnostic::SupervisorUnauthenticated);
    }

    let mut mac = HmacSha256::new_from_slice(launch_key).expect("hmac accepts any key length");
    mac.update(&signed_region(expected_nonce));
    // Constant time, and the only comparison performed on a tag anywhere.
    mac.verify_slice(&hello[at..])
        .map_err(|_| Diagnostic::SupervisorUnauthenticated)
}
