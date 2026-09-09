//! Binding the IPC listener, and the one deadline that bounds everything
//! before the store is owned.
//!
//! The window this code governs is the dangerous one. The broker has a
//! classified root and a bound port, and it does not yet hold the process
//! lock or the database. Anything that can stretch that window — a caller
//! that connects and then dribbles bytes, a parent that dies mid-handshake, a
//! retry that starts its own fresh timer — leaves a half-started broker
//! sitting on a port. So there is exactly one absolute deadline, captured
//! before the address is published, and every wait afterwards is measured
//! against it rather than being given its own budget.
//!
//! Monotonic on purpose: `Instant` cannot be moved by a clock adjustment, so
//! a caller that can change the system clock cannot extend the window.

use super::protocol::{self, Diagnostic};
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::time::{Duration, Instant};

/// Frozen by RFC 0006. Not configurable: a deadline a caller can choose is
/// not a bound on that caller.
pub const SUPERVISOR_ESTABLISHMENT: Duration = Duration::from_secs(5);

/// A bound listener and the absolute instant by which a supervisor must have
/// authenticated. The deadline is captured at construction — before the
/// address is published — so publishing, accepting and reading all consume
/// the same budget.
pub struct Established {
    listener: TcpListener,
    deadline: Instant,
    port: u16,
}

impl Established {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    pub fn listener(&self) -> &TcpListener {
        &self.listener
    }

    /// Time left before the deadline, or `None` once it has passed. Callers
    /// use this for every wait, which is what makes the budget shared.
    pub fn remaining(&self) -> Option<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
    }
}

/// Bind an ephemeral loopback listener, capture the deadline, publish the
/// address, and close the bootstrap channel.
///
/// Loopback explicitly and only. Binding `0.0.0.0` or `::` would expose the
/// fixture's IPC to the network, and the difference is one constant.
pub fn bind_and_publish(
    nonce: &[u8; protocol::NONCE_LEN],
    address_out: &mut impl Write,
) -> Result<Established, Diagnostic> {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .map_err(|_| Diagnostic::BindFailed)?;
    let port = listener
        .local_addr()
        .map_err(|_| Diagnostic::BindFailed)?
        .port();

    // Captured immediately before the address is published, so the clock
    // starts when the parent first could connect — not when it did.
    let deadline = Instant::now() + SUPERVISOR_ESTABLISHMENT;

    address_out
        .write_all(&protocol::encode_address(port, nonce))
        .and_then(|()| address_out.flush())
        .map_err(|_| Diagnostic::SupervisorLost)?;

    Ok(Established {
        listener,
        deadline,
        port,
    })
}

/// Wait for a connection, bounded by the shared deadline.
///
/// The listener is put in non-blocking mode and polled, because a blocking
/// `accept` has no timeout and would ignore the deadline entirely. The poll
/// interval is short and the loop's exit condition is the absolute deadline,
/// so a slow caller cannot extend the window by arriving late.
pub fn accept_by_deadline(established: &Established) -> Result<std::net::TcpStream, Diagnostic> {
    established
        .listener
        .set_nonblocking(true)
        .map_err(|_| Diagnostic::BindFailed)?;
    loop {
        match established.listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .map_err(|_| Diagnostic::SupervisorLost)?;
                // Whatever is left of the shared budget bounds every read on
                // this socket. A fresh timeout here would hand a connected
                // caller a second window.
                let left = established
                    .remaining()
                    .ok_or(Diagnostic::SupervisorTimeout)?;
                stream
                    .set_read_timeout(Some(left))
                    .map_err(|_| Diagnostic::SupervisorLost)?;
                return Ok(stream);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if established.remaining().is_none() {
                    return Err(Diagnostic::SupervisorTimeout);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return Err(Diagnostic::SupervisorLost),
        }
    }
}
