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
    stdout_reader: Option<JoinHandle<io::Result<Vec<u8>>>>,
    stderr_reader: Option<JoinHandle<io::Result<Vec<u8>>>>,
    stdout: Option<Vec<u8>>,
    stderr: Option<Vec<u8>>,
    status: Option<ExitStatus>,
    failure: Option<String>,
    pub port: u16,
}

impl ChildServer {
    // Used by the S1-07 fixture today; retained for the future CLI consumer.
    #[allow(dead_code)]
    pub fn start() -> io::Result<Self> {
        Self::start_mode("1")
    }

    // Used by the S1-07 fixture today; retained for the future CLI consumer.
    #[allow(dead_code)]
    pub fn start_mode(mode: &str) -> io::Result<Self> {
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args(["--exact", "child_server", "--nocapture"])
            .env("S1_07_CHILD_SERVER", mode);
        Self::start_command(command)
    }

    pub fn start_command(mut command: Command) -> io::Result<Self> {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let child = command.spawn()?;
        // Install cleanup immediately, before any fallible startup operation.
        let mut guard = Self {
            child,
            stdout_reader: None,
            stderr_reader: None,
            stdout: None,
            stderr: None,
            status: None,
            failure: None,
            port: 0,
        };
        let stdout = guard
            .child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("child stdout"))?;
        let stderr = guard
            .child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("child stderr"))?;
        let (sender, receiver) = mpsc::channel();
        guard.stdout_reader = Some(thread::spawn(move || capture_pipe(stdout, Some(sender))));
        guard.stderr_reader = Some(thread::spawn(move || capture_pipe(stderr, None)));
        let startup = receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|error| io::Error::new(io::ErrorKind::TimedOut, error))?;
        let port = match startup {
            Ok(port) => port,
            Err(error) => {
                let _ = guard.finish();
                return Err(error);
            }
        };
        guard.port = port;
        Ok(guard)
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn stop(&mut self) -> io::Result<ExitStatus> {
        self.finish()
    }

    fn finish(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.status {
            return self
                .failure
                .as_ref()
                .map_or(Ok(status), |error| Err(io::Error::other(error.clone())));
        }
        let mut failure = self.failure.take();
        match self.child.try_wait() {
            Ok(None) => {
                if let Err(error) = self.child.kill() {
                    failure.get_or_insert_with(|| error.to_string());
                }
            }
            Ok(Some(_)) => {}
            Err(error) => {
                failure.get_or_insert_with(|| error.to_string());
                // A failed status probe does not establish that the child is
                // gone. Continue with the same kill/wait cleanup path.
                if let Err(kill_error) = self.child.kill() {
                    failure.get_or_insert_with(|| kill_error.to_string());
                }
            }
        }
        let status = match self.child.wait() {
            Ok(status) => Some(status),
            Err(error) => {
                failure.get_or_insert_with(|| error.to_string());
                None
            }
        };
        let stdout = self
            .stdout_reader
            .take()
            .and_then(|reader| match reader.join() {
                Ok(result) => match result {
                    Ok(bytes) => Some(bytes),
                    Err(error) => {
                        failure.get_or_insert_with(|| error.to_string());
                        None
                    }
                },
                Err(_) => {
                    failure.get_or_insert_with(|| "stdout reader panic".to_owned());
                    None
                }
            });
        let stderr = self
            .stderr_reader
            .take()
            .and_then(|reader| match reader.join() {
                Ok(result) => match result {
                    Ok(bytes) => Some(bytes),
                    Err(error) => {
                        failure.get_or_insert_with(|| error.to_string());
                        None
                    }
                },
                Err(_) => {
                    failure.get_or_insert_with(|| "stderr reader panic".to_owned());
                    None
                }
            });
        self.status = status;
        self.stdout = stdout;
        self.stderr = stderr;
        self.failure = failure;
        if let Some(error) = self.failure.clone() {
            return Err(io::Error::other(error));
        }
        status.ok_or_else(|| io::Error::other("child wait failed"))
    }

    pub fn captured_output(&self) -> io::Result<(&[u8], &[u8])> {
        if self.status.is_none() || self.failure.is_some() {
            return Err(io::Error::other("complete capture unavailable"));
        }
        Ok((
            self.stdout
                .as_deref()
                .ok_or_else(|| io::Error::other("stdout capture"))?,
            self.stderr
                .as_deref()
                .ok_or_else(|| io::Error::other("stderr capture"))?,
        ))
    }
}

impl Drop for ChildServer {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn capture_pipe<R: Read>(
    mut reader: R,
    startup: Option<mpsc::Sender<io::Result<u16>>>,
) -> io::Result<Vec<u8>> {
    capture_bytes_with_startup(&mut reader, startup)
}

fn capture_bytes<R: Read>(mut reader: R) -> io::Result<Vec<u8>> {
    capture_bytes_with_startup(&mut reader, None)
}

fn capture_bytes_with_startup<R: Read>(
    reader: &mut R,
    startup: Option<mpsc::Sender<io::Result<u16>>>,
) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    let mut startup = startup;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                if let Some(sender) = startup.take() {
                    let _ = sender.send(Err(io::Error::other("missing startup line")));
                }
                return Ok(output);
            }
            Ok(size) => {
                if output.len() + size > 65_536 {
                    if let Some(sender) = startup.take() {
                        let _ = sender.send(Err(io::Error::other(
                            "capture output exceeds fixture bound",
                        )));
                    }
                    return Err(io::Error::other("capture output exceeds fixture bound"));
                }
                output.extend_from_slice(&buffer[..size]);
                if startup.is_some() {
                    // Only inspect complete lines. A line may be split across
                    // reads; its final fragment is not a malformed line yet.
                    if let Some(line) = output[..output.len().min(4096)]
                        .split_inclusive(|byte| *byte == b'\n')
                        .filter(|line| line.ends_with(b"\n"))
                        .find(|line| line.starts_with(b"Playground:"))
                    {
                        let port = line
                            .strip_prefix(b"Playground: http://127.0.0.1:")
                            .and_then(|text| text.strip_suffix(b"\n"))
                            .and_then(|text| std::str::from_utf8(text).ok())
                            .and_then(|text| text.parse::<u16>().ok())
                            .filter(|port| *port != 0);
                        let port = port.filter(|port| {
                            line == format!("Playground: http://127.0.0.1:{port}\n").as_bytes()
                        });
                        let result =
                            port.ok_or_else(|| io::Error::other("invalid exact startup line"));
                        let sender = startup.take().expect("startup sender");
                        let _ = sender.send(result);
                        if port.is_none() {
                            return Err(io::Error::other("invalid exact startup line"));
                        }
                    }
                    if startup.is_some() && output.len() >= 4096 {
                        let sender = startup.take().expect("startup sender");
                        let _ = sender.send(Err(io::Error::other(
                            "startup output exceeds fixture bound",
                        )));
                        return Err(io::Error::other("startup output exceeds fixture bound"));
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                if let Some(sender) = startup.take() {
                    let _ = sender.send(Err(io::Error::other(error.to_string())));
                }
                return Err(error);
            }
        }
    }
}

#[cfg(test)]
mod transport_capture_regressions {
    use super::{capture_bytes, capture_bytes_with_startup, connect, ChildServer};
    use std::io::{self, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::process::Command;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn transport_capture_preserves_complete_stdout_and_stderr() {
        let (mut server, acknowledgment) = captured_child("complete");
        assert!(server.captured_output().is_err());
        assert!(server.id() > 0);
        let status = server.stop().unwrap();
        assert_reaped_and_joined(&mut server);
        let expected = format!(
            "\nrunning 1 test\nPlayground: http://127.0.0.1:{}\nstdout-tail",
            server.port
        );
        let (stdout, stderr) = server.captured_output().unwrap();
        assert_eq!(stdout, expected.as_bytes());
        assert_eq!(stderr, b"stderr-tail");
        assert_eq!(server.stop().unwrap(), status);
        assert_reaped_and_joined(&mut server);
        assert_eq!(
            server.captured_output().unwrap(),
            (expected.as_bytes(), &b"stderr-tail"[..])
        );
        drop(acknowledgment);
    }

    #[test]
    fn transport_capture_rejects_overflow_read_error_and_preserves_cleanup() {
        // Exercise exactly-at-bound and one-over for each pipe's configuration.
        for stdout in [false, true] {
            for size in [65_536, 65_537] {
                let mut bytes = vec![b'x'; size];
                let line = b"Playground: http://127.0.0.1:43125\n";
                if stdout {
                    bytes[..line.len()].copy_from_slice(line);
                }
                let (sender, receiver) = mpsc::channel();
                let result = capture_bytes_with_startup(&mut &bytes[..], stdout.then_some(sender));
                if stdout {
                    assert_eq!(receiver.recv().unwrap().unwrap(), 43125);
                }
                if size == 65_536 {
                    assert_eq!(result.unwrap(), bytes);
                } else {
                    assert_eq!(
                        result.unwrap_err().to_string(),
                        "capture output exceeds fixture bound"
                    );
                }
            }
            let (sender, receiver) = mpsc::channel();
            let error = capture_bytes_with_startup(&mut FailingReader, stdout.then_some(sender))
                .unwrap_err();
            assert_eq!(error.to_string(), "fixture read failure");
            if stdout {
                assert_eq!(
                    receiver.recv().unwrap().unwrap_err().to_string(),
                    "fixture read failure"
                );
            }
        }
        // A read error after startup publication must remain a capture error.
        let (sender, receiver) = mpsc::channel();
        let mut failing_after_startup =
            (&b"Playground: http://127.0.0.1:43125\n"[..]).chain(FailingReader);
        let error =
            capture_bytes_with_startup(&mut failing_after_startup, Some(sender)).unwrap_err();
        assert_eq!(receiver.recv().unwrap().unwrap(), 43125);
        assert_eq!(error.to_string(), "fixture read failure");

        // Interrupted reads must retry; complete lines can span multiple reads.
        let (sender, receiver) = mpsc::channel();
        let mut reader = ChunkedReader {
            chunks: vec![
                b"preamble\nPlayground: http://127.0.0.1:43125".to_vec(),
                b"\ntrailing".to_vec(),
            ],
            interrupt: true,
        };
        let output = capture_bytes_with_startup(&mut reader, Some(sender)).unwrap();
        assert_eq!(receiver.recv().unwrap().unwrap(), 43125);
        assert_eq!(
            output,
            b"preamble\nPlayground: http://127.0.0.1:43125\ntrailing"
        );

        for mode in [
            "overflow-stdout",
            "overflow-stderr",
            "read-error-stdout",
            "read-error-stderr",
            "panic-stdout",
            "panic-stderr",
        ] {
            let (mut server, acknowledgment) = captured_child(mode);
            let stdout = mode.ends_with("stdout");
            let reader = if stdout {
                &mut server.stdout_reader
            } else {
                &mut server.stderr_reader
            };
            if mode.starts_with("overflow") {
                // The child has acknowledged completing its write. Observe the
                // actual overflow reader's completion before stop can kill it.
                let deadline = Instant::now() + Duration::from_secs(3);
                while !reader.as_ref().unwrap().is_finished() {
                    assert!(Instant::now() < deadline, "{mode}: reader did not finish");
                    thread::sleep(Duration::from_millis(1));
                }
            } else {
                // Preserve the real pipe drainer and inject its terminal result
                // through the same join path, after its child pipe reaches EOF.
                let original = reader.take().unwrap();
                *reader = Some(thread::spawn(move || {
                    original.join().unwrap()?;
                    if mode.starts_with("panic") {
                        panic!("fixture reader panic");
                    }
                    capture_bytes(FailingReader)
                }));
            }
            let error = server.stop().unwrap_err().to_string();
            let expected_error = if mode.starts_with("overflow") {
                "capture output exceeds fixture bound"
            } else if mode.starts_with("panic") {
                if stdout {
                    "stdout reader panic"
                } else {
                    "stderr reader panic"
                }
            } else {
                "fixture read failure"
            };
            assert_eq!(error, expected_error, "{mode}");
            assert_reaped_and_joined(&mut server);
            // A failed pipe cannot prevent the other pipe's complete join.
            if stdout {
                assert!(server.stdout.is_none());
                assert_eq!(server.stderr.as_deref().unwrap(), b"stderr-tail");
            } else {
                assert!(server.stderr.is_none());
                let expected = format!(
                    "\nrunning 1 test\nPlayground: http://127.0.0.1:{}\nstdout-tail",
                    server.port
                );
                assert_eq!(server.stdout.as_deref().unwrap(), expected.as_bytes());
            }
            assert!(server.captured_output().is_err());
            assert_eq!(server.stop().unwrap_err().to_string(), error);
            assert_reaped_and_joined(&mut server);
            assert!(server.captured_output().is_err());
            drop(acknowledgment);
        }
    }

    fn assert_reaped_and_joined(server: &mut ChildServer) {
        let status = server.status.expect("wait must have reaped the child");
        assert_eq!(server.child.try_wait().unwrap(), Some(status));
        assert!(
            server.stdout_reader.is_none(),
            "stdout reader must be joined"
        );
        assert!(
            server.stderr_reader.is_none(),
            "stderr reader must be joined"
        );
        // The fixture holds this listener until process termination. A new
        // bind independently proves cleanup released its OS-owned resource.
        let listener = TcpListener::bind(("127.0.0.1", server.port)).unwrap();
        drop(listener);
    }

    fn captured_child(mode: &str) -> (ChildServer, TcpStream) {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            "transport::transport_capture_regressions::capture_child",
            "--nocapture",
            "--test-threads=2",
        ]);
        command.env("S1_10A_CAPTURE", mode);
        let server = ChildServer::start_command(command).unwrap();
        let mut acknowledgment = connect(server.port).unwrap();
        acknowledgment
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut written = [0; 7];
        acknowledgment.read_exact(&mut written).unwrap();
        assert_eq!(&written, b"written");
        // Keep this stream alive until stop: EOF would let the child test
        // return and append libtest's result text to the expected output.
        (server, acknowledgment)
    }

    struct ChunkedReader {
        chunks: Vec<Vec<u8>>,
        interrupt: bool,
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.interrupt {
                self.interrupt = false;
                return Err(io::ErrorKind::Interrupted.into());
            }
            if self.chunks.is_empty() {
                return Ok(0);
            }
            let chunk = self.chunks.remove(0);
            buffer[..chunk.len()].copy_from_slice(&chunk);
            Ok(chunk.len())
        }
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture read failure"))
        }
    }

    #[test]
    fn capture_child() {
        let Ok(mode) = std::env::var("S1_10A_CAPTURE") else {
            return;
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let startup = format!(
            "Playground: http://127.0.0.1:{}\n",
            listener.local_addr().unwrap().port()
        );
        let mut stdout = io::stdout();
        let mut stderr = io::stderr();
        stdout.write_all(startup.as_bytes()).unwrap();
        if mode == "overflow-stdout" {
            // Exactly one byte beyond the total bound: no unsent remainder
            // can race the reader closing its pipe after detecting overflow.
            let remaining = 65_537 - b"\nrunning 1 test\n".len() - startup.len();
            stdout.write_all(&vec![b'x'; remaining]).unwrap();
        } else {
            stdout.write_all(b"stdout-tail").unwrap();
        }
        stdout.flush().unwrap();
        if mode == "overflow-stderr" {
            stderr.write_all(&vec![b'y'; 65_537]).unwrap();
        } else {
            stderr.write_all(b"stderr-tail").unwrap();
        }
        stderr.flush().unwrap();
        let (mut acknowledgment, _) = listener.accept().unwrap();
        acknowledgment
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        acknowledgment
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        acknowledgment.write_all(b"written").unwrap();
        let mut hold = [0];
        let _ = acknowledgment.read(&mut hold);
        drop(listener);
    }
}
