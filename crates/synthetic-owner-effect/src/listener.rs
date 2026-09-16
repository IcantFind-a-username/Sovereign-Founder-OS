//! The one public listener this fixture will own.
//!
//! v01-D02 records the compiled origin and the sole bind entry. HTTP routes,
//! WebAuthn, and owner sessions are later tickets. There is no internal
//! listener, no second port, and no HMAC transport.

use std::net::{Ipv4Addr, SocketAddr, TcpListener};

/// Frozen by the v2 plan. Not configurable: a port the caller can choose is
/// not a compiled origin.
pub const LISTEN_PORT: u16 = 7787;

/// Exact origin the later UV ceremony must present. Host name, not the
/// loopback address: `http://127.0.0.1:7787` is a different origin.
pub const ORIGIN: &str = "http://localhost:7787";

/// WebAuthn RP ID for the later ceremony. Port-insensitive by the platform
/// rule; exact-origin checks are what stop a second-port credential.
pub const RP_ID: &str = "localhost";

/// The sole production bind. Loopback only; never an unspecified IPv4 or
/// IPv6 bind address.
///
/// D02 does not call this from the process-boundary path: a rejected root
/// must fail before listener or database state, and a second process must
/// fail on the OS lock before this bind could occupy the port.
pub fn bind_public_origin() -> std::io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, LISTEN_PORT)))
}
