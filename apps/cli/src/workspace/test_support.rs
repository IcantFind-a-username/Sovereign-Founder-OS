//! Helpers shared by the in-crate workspace tests.

use std::io::{Read, Write};
use std::net::TcpListener;

/// A fake Ollama daemon: answers each connection with the next canned body.
pub(super) fn fake_ollama(bodies: Vec<String>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for body in bodies {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = vec![0u8; 65_536];
            let mut total = 0;
            loop {
                let read = stream.read(&mut request[total..]).unwrap_or(0);
                if read == 0 {
                    break;
                }
                total += read;
                let head_end = request[..total].windows(4).position(|w| w == b"\r\n\r\n");
                if let Some(end) = head_end {
                    let head = String::from_utf8_lossy(&request[..end]).to_string();
                    let length = head
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.trim()
                                .eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    if total >= end + 4 + length {
                        break;
                    }
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    port
}
