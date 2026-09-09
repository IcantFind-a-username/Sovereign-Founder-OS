use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use crate::assets;
use crate::http::{HandlerOutcome, HttpRequest, PlaygroundHttpHandler};

const MAX_HEAD_BYTES: usize = 8192;
const MAX_HEADERS: usize = 32;
const MAX_BODY_READ_BYTES: usize = 257;
const READ_BUDGET: Duration = Duration::from_secs(5);
const WRITE_BUDGET: Duration = Duration::from_secs(5);
const TRANSPORT_HEADERS: &[(&str, &str)] = &[
    ("Cache-Control", "no-store"),
    ("X-Content-Type-Options", "nosniff"),
];

enum ReadFailure {
    BadRequest,
    HeadersTooLarge,
    Timeout,
    Io(io::Error),
}

pub(crate) fn run(port: u16) -> io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let address = listener.local_addr()?;
    let bound_port = address.port();
    if address.ip() != Ipv4Addr::LOCALHOST || bound_port == 0 {
        return Err(io::Error::other("invalid loopback listener"));
    }
    writeln!(
        io::stdout().lock(),
        "Playground: http://127.0.0.1:{bound_port}"
    )?;
    let handler = PlaygroundHttpHandler::new();
    loop {
        let (mut stream, _) = match listener.accept() {
            Ok(connection) => connection,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let _ = serve_connection(&mut stream, &handler, bound_port);
        let _ = stream.shutdown(Shutdown::Both);
    }
}

fn serve_connection(
    stream: &mut TcpStream,
    handler: &PlaygroundHttpHandler,
    bound_port: u16,
) -> io::Result<()> {
    let deadline = Instant::now() + READ_BUDGET;
    let mut head = [0; MAX_HEAD_BYTES];
    let result = read_head(stream, deadline, &mut head).and_then(|length| {
        respond_to_request(stream, handler, bound_port, &head[..length], deadline)
    });
    let status = match result {
        Ok(()) => return Ok(()),
        Err(ReadFailure::BadRequest) => 400,
        Err(ReadFailure::HeadersTooLarge) => 431,
        Err(ReadFailure::Timeout) => 408,
        Err(ReadFailure::Io(error)) => return Err(error),
    };
    write_response(stream, status, TRANSPORT_HEADERS, &[], false)
}

fn respond_to_request(
    stream: &mut TcpStream,
    handler: &PlaygroundHttpHandler,
    bound_port: u16,
    head: &[u8],
    deadline: Instant,
) -> Result<(), ReadFailure> {
    let mut slots = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut parsed = httparse::Request::new(&mut slots);
    match parsed.parse(head) {
        Ok(httparse::Status::Complete(length)) if length == head.len() => {}
        Err(httparse::Error::TooManyHeaders) => return Err(ReadFailure::HeadersTooLarge),
        _ => return Err(ReadFailure::BadRequest),
    }
    if !matches!(parsed.version, Some(0 | 1)) {
        return Err(ReadFailure::BadRequest);
    }
    let method = parsed.method.ok_or(ReadFailure::BadRequest)?;
    let target = parsed.path.ok_or(ReadFailure::BadRequest)?;
    let length = content_length(parsed.headers)?;
    let headers = parsed
        .headers
        .iter()
        .map(|header| {
            std::str::from_utf8(header.value)
                .map(|value| (header.name, value))
                .map_err(|_| ReadFailure::BadRequest)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut body = [0; MAX_BODY_READ_BYTES];
    let length = read_body(stream, deadline, length, &mut body)?;
    let request = HttpRequest {
        method,
        target,
        headers: &headers,
        body: &body[..length],
    };
    match handler.handle(request, bound_port) {
        HandlerOutcome::Json(response) => {
            let body = serde_json::to_vec(&response.body)
                .map_err(|error| ReadFailure::Io(io::Error::other(error)))?;
            write_response(
                stream,
                response.status,
                response.headers,
                &body,
                method == "HEAD",
            )
            .map_err(ReadFailure::Io)
        }
        HandlerOutcome::Asset(route) => {
            let asset = assets::asset(route);
            write_response(
                stream,
                200,
                &[
                    ("Content-Type", asset.content_type),
                    ("Cache-Control", "no-store"),
                    ("X-Content-Type-Options", "nosniff"),
                ],
                asset.bytes,
                method == "HEAD",
            )
            .map_err(ReadFailure::Io)
        }
    }
}

fn read_head(
    stream: &mut TcpStream,
    deadline: Instant,
    head: &mut [u8; MAX_HEAD_BYTES],
) -> Result<usize, ReadFailure> {
    let mut length = 0;
    while length < head.len() {
        read_timeout(stream, deadline)?;
        match stream.read(&mut head[length..length + 1]) {
            Ok(0) => return Err(ReadFailure::BadRequest),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(read_failure(error)),
        }
        let byte = head[length];
        if !byte.is_ascii()
            || (byte.is_ascii_control() && !matches!(byte, b'\r' | b'\n' | b'\t'))
            || (length == 0 && matches!(byte, b'\r' | b'\n'))
            || (byte == b'\n' && (length == 0 || head[length - 1] != b'\r'))
            || (length > 0 && head[length - 1] == b'\r' && byte != b'\n')
        {
            return Err(ReadFailure::BadRequest);
        }
        length += 1;
        if head[..length].ends_with(b"\r\n\r\n") {
            return Ok(length);
        }
    }
    Err(ReadFailure::HeadersTooLarge)
}

fn content_length(headers: &[httparse::Header<'_>]) -> Result<usize, ReadFailure> {
    let mut length = None;
    for header in headers {
        if ["Transfer-Encoding", "Expect", "Upgrade"]
            .iter()
            .any(|name| header.name.eq_ignore_ascii_case(name))
        {
            return Err(ReadFailure::BadRequest);
        }
        let value = std::str::from_utf8(header.value).map_err(|_| ReadFailure::BadRequest)?;
        if header.name.eq_ignore_ascii_case("Connection")
            && value.split(',').any(|token| {
                token
                    .trim_matches([' ', '\t'])
                    .eq_ignore_ascii_case("upgrade")
            })
        {
            return Err(ReadFailure::BadRequest);
        }
        if header.name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(ReadFailure::BadRequest);
            }
            let value = value.trim_matches([' ', '\t']).as_bytes();
            if value.is_empty() || (value.len() > 1 && value[0] == b'0') {
                return Err(ReadFailure::BadRequest);
            }
            let mut number = 0_usize;
            for &digit in value {
                if !digit.is_ascii_digit() {
                    return Err(ReadFailure::BadRequest);
                }
                number = number
                    .checked_mul(10)
                    .and_then(|number| number.checked_add(usize::from(digit - b'0')))
                    .ok_or(ReadFailure::BadRequest)?;
            }
            length = Some(number);
        }
    }
    Ok(length.unwrap_or(0))
}

fn read_body(
    stream: &mut TcpStream,
    deadline: Instant,
    length: usize,
    body: &mut [u8; MAX_BODY_READ_BYTES],
) -> Result<usize, ReadFailure> {
    let limit = length.min(MAX_BODY_READ_BYTES);
    let mut received = 0;
    while received < limit {
        read_timeout(stream, deadline)?;
        match stream.read(&mut body[received..limit]) {
            Ok(0) => return Err(ReadFailure::BadRequest),
            Ok(length) => received += length,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(read_failure(error)),
        }
    }
    Ok(received)
}

fn read_timeout(stream: &TcpStream, deadline: Instant) -> Result<(), ReadFailure> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or(ReadFailure::Timeout)?;
    stream
        .set_read_timeout(Some(remaining))
        .map_err(ReadFailure::Io)
}

fn read_failure(error: io::Error) -> ReadFailure {
    match error.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => ReadFailure::Timeout,
        _ => ReadFailure::Io(error),
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    headers: &[(&str, &str)],
    body: &[u8],
    head_only: bool,
) -> io::Result<()> {
    let deadline = Instant::now() + WRITE_BUDGET;
    let mut head = format!("HTTP/1.1 {status} {}\r\n", reason_phrase(status)?);
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    write_bytes(stream, deadline, head.as_bytes())?;
    if !head_only {
        write_bytes(stream, deadline, body)?;
    }
    Ok(())
}

fn write_bytes(stream: &mut TcpStream, deadline: Instant, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "write deadline expired"))?;
        stream.set_write_timeout(Some(remaining))?;
        match stream.write(bytes) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(length) => bytes = &bytes[length..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn reason_phrase(status: u16) -> io::Result<&'static str> {
    match status {
        200 => Ok("OK"),
        400 => Ok("Bad Request"),
        403 => Ok("Forbidden"),
        404 => Ok("Not Found"),
        405 => Ok("Method Not Allowed"),
        408 => Ok("Request Timeout"),
        413 => Ok("Payload Too Large"),
        415 => Ok("Unsupported Media Type"),
        431 => Ok("Request Header Fields Too Large"),
        500 => Ok("Internal Server Error"),
        503 => Ok("Service Unavailable"),
        _ => Err(io::Error::other("unsupported response status")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn content_length_rejects_ambiguous_and_noncanonical_values() {
        for value in [b"01".as_slice(), b"+1", b"", b"1,1"] {
            let headers = [httparse::Header {
                name: "Content-Length",
                value,
            }];
            assert!(matches!(
                content_length(&headers),
                Err(ReadFailure::BadRequest)
            ));
        }
        let headers = [httparse::Header {
            name: "Content-Length",
            value: b"256",
        }];
        assert!(matches!(content_length(&headers), Ok(256)));
    }

    #[test]
    fn write_response_has_closed_framing_and_head_suppresses_body() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            write_response(
                &mut stream,
                200,
                &[("Content-Type", "text/plain")],
                b"hello",
                true,
            )
            .unwrap();
        });
        let mut peer = TcpStream::connect(address).unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut bytes = Vec::new();
        peer.read_to_end(&mut bytes).unwrap();
        worker.join().unwrap();
        assert!(bytes
            .windows(19)
            .any(|window| window == b"Content-Length: 5\r\n"));
        assert!(bytes.ends_with(b"\r\n\r\n"));
    }

    #[test]
    fn write_response_times_out_against_nonreading_peer() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut stream, _) = listener.accept().unwrap();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let worker = thread::spawn(move || {
            let body = vec![b'x'; 16 * 1024 * 1024];
            let started = Instant::now();
            let result = write_response(&mut stream, 200, &[], &body, false);
            let _ = result_tx.send((result, started.elapsed()));
        });
        // The watchdog is generous on purpose. What this test proves is that
        // the write budget is applied at all — it returns rather than
        // blocking forever, and it does not return before the budget elapsed.
        // A tight upper band would only measure how loaded the machine is:
        // idle this lands near WRITE_BUDGET, but under a concurrent build it
        // drifts, and a scheduling delay is not a defect in the server.
        let observed = result_rx.recv_timeout(WRITE_BUDGET * 12);
        // Always release the blocked syscall and join before any assertion.
        // A watchdog expiry remains a test failure, never a synthetic IO error.
        drop(peer);
        worker.join().unwrap();
        let (result, elapsed) = observed.expect("write_response did not return before watchdog");
        assert!(
            elapsed >= WRITE_BUDGET - Duration::from_millis(200),
            "returned after {elapsed:?}, before the {WRITE_BUDGET:?} write budget could expire"
        );
        assert!(matches!(
            result.unwrap_err().kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        ));
    }

    #[test]
    fn complete_post_applies_before_write_half_failure() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let handler = PlaygroundHttpHandler::new();
        let mut peer = TcpStream::connect(address).unwrap();
        let (mut server_stream, _) = listener.accept().unwrap();
        peer.shutdown(Shutdown::Read).unwrap();
        let body = br#"{"action":"CorrectOfferPrice"}"#;
        let request = format!("POST /api/playground/consultant/action HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", address.port(), body.len());
        peer.write_all(request.as_bytes()).unwrap();
        peer.write_all(body).unwrap();
        server_stream.shutdown(Shutdown::Write).unwrap();
        assert!(serve_connection(&mut server_stream, &handler, address.port()).is_err());
        let host = format!("127.0.0.1:{}", address.port());
        let outcome = handler.handle(
            HttpRequest {
                method: "GET",
                target: "/api/playground/consultant",
                headers: &[("Host", &host)],
                body: &[],
            },
            address.port(),
        );
        let HandlerOutcome::Json(response) = outcome else {
            panic!("expected JSON state")
        };
        let value = serde_json::to_value(response.body).unwrap();
        let mut expected = crate::domain::PlaygroundSession::new();
        expected.apply(crate::domain::PlaygroundAction::CorrectOfferPrice);
        assert_eq!(
            value,
            serde_json::json!({
                "profile": "synthetic_playground", "real_data_enabled": false,
                "persistence": "none", "state": expected.read_model(),
                "catalog": crate::catalog::CATALOG, "teaching": expected.teaching_read_model(),
            })
        );
        // A subsequent real connection through the same handler remains usable.
        drop(server_stream);
        let mut next_peer = TcpStream::connect(address).unwrap();
        let (mut next_server, _) = listener.accept().unwrap();
        next_peer
            .write_all(
                format!("GET /api/playground/consultant HTTP/1.1\r\nHost: {host}\r\n\r\n")
                    .as_bytes(),
            )
            .unwrap();
        serve_connection(&mut next_server, &handler, address.port()).unwrap();
        drop(next_server);
        next_peer
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut raw = Vec::new();
        next_peer.read_to_end(&mut raw).unwrap();
        let body_offset = raw
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap()
            + 4;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&raw[body_offset..]).unwrap(),
            value
        );
    }
}
