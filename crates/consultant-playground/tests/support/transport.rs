use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub raw: Vec<u8>,
}

pub fn connect(port: u16) -> io::Result<TcpStream> {
    let stream = TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_secs(2),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(8)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    Ok(stream)
}

pub fn raw_request(port: u16, method: &str, target: &str, headers: &str, body: &[u8]) -> Vec<u8> {
    let mut raw = format!("{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{headers}\r\n")
        .into_bytes();
    raw.extend_from_slice(body);
    raw
}

pub fn request(port: u16, bytes: &[u8]) -> io::Result<Response> {
    let mut stream = connect(port)?;
    stream.write_all(bytes)?;
    stream.shutdown(Shutdown::Write)?;
    read_response(&mut stream)
}

pub fn read_response(stream: &mut TcpStream) -> io::Result<Response> {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut raw = Vec::new();
    let mut buf = [0; 4096];
    // Whole-second socket waits avoid the fractional-timeout EINVAL observed
    // on the test host. The absolute deadline bounds all retries to <9 seconds.
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    loop {
        if Instant::now() >= deadline {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&buf[..n]);
                if raw.len() > 1024 * 1024 {
                    return Err(io::Error::other("response exceeds fixture bound"));
                }
            }
            // Closing without draining pipelined bytes can reset TCP. Preserve
            // every received byte; callers still assert exact complete framing.
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset && !raw.is_empty() => break,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::Interrupted
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                ) =>
            {
                continue
            }
            Err(e) => return Err(e),
        }
    }
    let mut slots = [httparse::EMPTY_HEADER; 32];
    let mut parsed = httparse::Response::new(&mut slots);
    let httparse::Status::Complete(length) = parsed
        .parse(&raw)
        .map_err(|error| io::Error::other(error.to_string()))?
    else {
        return Err(io::Error::other("incomplete response head"));
    };
    if parsed.version != Some(1) {
        return Err(io::Error::other("response must be HTTP/1.1"));
    }
    let status = parsed
        .code
        .ok_or_else(|| io::Error::other("missing status"))?;
    let headers = parsed
        .headers
        .iter()
        .map(|header| {
            Ok((
                header.name.to_owned(),
                std::str::from_utf8(header.value)
                    .map_err(io::Error::other)?
                    .to_owned(),
            ))
        })
        .collect::<io::Result<Vec<_>>>()?;
    Ok(Response {
        status,
        headers,
        body: raw[length..].to_vec(),
        raw,
    })
}

pub struct ChildServer {
    child: Child,
    reader: Option<JoinHandle<()>>,
    pub port: u16,
}

impl ChildServer {
    pub fn start() -> io::Result<Self> {
        Self::start_mode("1")
    }

    pub fn start_mode(mode: &str) -> io::Result<Self> {
        let child = Command::new(std::env::current_exe()?)
            .args(["--exact", "child_server", "--nocapture"])
            .env("S1_07_CHILD_SERVER", mode)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        // Install cleanup immediately, before any fallible startup operation.
        let mut guard = Self {
            child,
            reader: None,
            port: 0,
        };
        let mut stdout = guard
            .child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("child stdout"))?;
        let (sender, receiver) = mpsc::channel();
        guard.reader = Some(thread::spawn(move || {
            let result = (|| -> io::Result<String> {
                let mut line = Vec::new();
                for _ in 0..4096 {
                    let mut byte = [0];
                    stdout.read_exact(&mut byte)?;
                    line.push(byte[0]);
                    if byte[0] == b'\n' {
                        if line.starts_with(b"Playground:") {
                            return String::from_utf8(line).map_err(io::Error::other);
                        }
                        line.clear();
                    }
                }
                Err(io::Error::other("startup output exceeds fixture bound"))
            })();
            let _ = sender.send(result);
        }));
        let line = receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|error| io::Error::new(io::ErrorKind::TimedOut, error))??;
        let port: u16 = line
            .strip_prefix("Playground: http://127.0.0.1:")
            .and_then(|text| text.strip_suffix('\n'))
            .and_then(|text| text.parse().ok())
            .filter(|port| *port != 0)
            .ok_or_else(|| io::Error::other("invalid exact startup line"))?;
        if line != format!("Playground: http://127.0.0.1:{port}\n") {
            return Err(io::Error::other("noncanonical startup line"));
        }
        guard.port = port;
        guard
            .reader
            .take()
            .unwrap()
            .join()
            .map_err(|_| io::Error::other("startup reader panic"))?;
        Ok(guard)
    }

    pub fn stop(&mut self) -> io::Result<ExitStatus> {
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
        }
        let status = self.child.wait()?;
        if let Some(reader) = self.reader.take() {
            reader
                .join()
                .map_err(|_| io::Error::other("startup reader panic"))?;
        }
        Ok(status)
    }
}

impl Drop for ChildServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
