//! An Experimental provider that talks to a local Ollama daemon over
//! loopback HTTP. It is deliberately narrow:
//!
//! - loopback only — `ProviderTrust::Local` is only honest when the daemon
//!   is on this machine, and the gateway's Red-data guard relies on it;
//! - raw HTTP/1.1 over `std::net::TcpStream` with read/write timeouts, so a
//!   hung daemon cannot hang the product and no HTTP client crate is added;
//! - non-streaming `POST /api/generate`; the gateway still enforces the
//!   output ceiling and records the disclosure.
//!
//! The product does not confine, sandbox, or audit the Ollama process; it
//! only routes to it. That is stated to the founder, never hidden.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::{Health, LocalVouch, ModelProvider, ModelRequest, ProviderError, ProviderTrust};

/// Why a provider could not be constructed from its configuration.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OllamaConfigError {
    #[error("base_url must be http://127.0.0.1:<port> or http://localhost:<port>")]
    NotLoopback,
    #[error("model name is empty or contains control characters")]
    InvalidModel,
}

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    id: String,
    host: String,
    port: u16,
    model: String,
    timeout: Duration,
    /// Health probes wait far less than completions: a hung daemon must not
    /// stall the gateway's provider classification.
    health_timeout: Duration,
}

const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

impl OllamaProvider {
    /// `base_url` must name a loopback host; anything else is refused at
    /// construction so a remote endpoint can never masquerade as local.
    pub fn new(
        id: impl Into<String>,
        base_url: &str,
        model: &str,
    ) -> Result<Self, OllamaConfigError> {
        let (host, port) = parse_loopback_base_url(base_url)?;
        let model = model.trim();
        if model.is_empty() || model.chars().any(char::is_control) || model.len() > 128 {
            return Err(OllamaConfigError::InvalidModel);
        }
        Ok(Self {
            id: id.into(),
            host,
            port,
            model: model.to_owned(),
            timeout: Duration::from_secs(60),
            health_timeout: Duration::from_secs(3),
        })
    }

    /// Completion timeout; the health-probe timeout becomes the smaller of
    /// this and three seconds.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self.health_timeout = timeout.min(Duration::from_secs(3));
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// One HTTP/1.1 exchange. Returns the status code and body.
    fn exchange(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
        timeout: Duration,
    ) -> Result<(u16, Vec<u8>), String> {
        let address = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|error| format!("resolve: {error}"))?
            .next()
            .ok_or_else(|| "resolve: no address".to_owned())?;
        let mut stream = TcpStream::connect_timeout(&address, timeout)
            .map_err(|error| format!("connect: {error}"))?;
        stream
            .set_read_timeout(Some(timeout))
            .and_then(|_| stream.set_write_timeout(Some(timeout)))
            .map_err(|error| format!("timeout: {error}"))?;
        let head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nAccept: application/json\r\n\r\n",
            self.host,
            self.port,
            body.len()
        );
        stream
            .write_all(head.as_bytes())
            .and_then(|_| stream.write_all(body))
            .and_then(|_| stream.flush())
            .map_err(|error| format!("write: {error}"))?;
        let mut raw = Vec::new();
        stream
            .take((MAX_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut raw)
            .map_err(|error| format!("read: {error}"))?;
        if raw.len() > MAX_RESPONSE_BYTES {
            return Err("response too large".into());
        }
        parse_response(&raw)
    }
}

/// Accept only `http://127.0.0.1[:port]` and `http://localhost[:port]`.
fn parse_loopback_base_url(base_url: &str) -> Result<(String, u16), OllamaConfigError> {
    let rest = base_url
        .trim()
        .strip_prefix("http://")
        .ok_or(OllamaConfigError::NotLoopback)?;
    let rest = rest.trim_end_matches('/');
    let (host, port) = match rest.split_once(':') {
        Some((host, port)) => (
            host,
            port.parse::<u16>()
                .map_err(|_| OllamaConfigError::NotLoopback)?,
        ),
        None => (rest, 11434),
    };
    if host != "127.0.0.1" && host != "localhost" {
        return Err(OllamaConfigError::NotLoopback);
    }
    Ok((host.to_owned(), port))
}

/// Minimal HTTP/1.1 response parser: status line, headers, then a body framed
/// by `Content-Length`, by chunked transfer, or by connection close.
///
/// Chunked is not optional. Ollama's Go server sets a content length only
/// when the whole body fits its write buffer; a non-streaming `/api/generate`
/// answer of more than a few hundred bytes — every real crew draft — comes
/// back `Transfer-Encoding: chunked`. Refusing it made the provider fail on
/// exactly the responses that mattered, and the gateway failed over to the
/// template drafter with the health probe (a short `/api/tags`) still green.
fn parse_response(raw: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "malformed response: no header terminator".to_owned())?;
    let head = std::str::from_utf8(&raw[..split]).map_err(|_| "malformed response head")?;
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| format!("malformed status line: {status_line}"))?;
    let mut content_length: Option<usize> = None;
    let mut chunked = false;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();
            if name == "transfer-encoding" && value.to_ascii_lowercase().contains("chunked") {
                chunked = true;
            }
            if name == "content-length" {
                content_length = Some(value.parse().map_err(|_| "bad content-length")?);
            }
        }
    }
    let body = &raw[split + 4..];
    let body = if chunked {
        dechunk(body)?
    } else {
        match content_length {
            Some(length) if length <= body.len() => body[..length].to_vec(),
            Some(_) => return Err("truncated response body".into()),
            None => body.to_vec(),
        }
    };
    Ok((status, body))
}

/// Decode a chunked transfer body: a hex size line (any `;extension` is
/// ignored), that many bytes, CRLF, repeated until the zero-size chunk.
/// Trailers after the last chunk are ignored. Anything malformed or
/// truncated is refused rather than guessed at, and the decoded body can
/// only be smaller than the raw bytes the caller already bounded.
fn dechunk(mut raw: &[u8]) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    loop {
        let line_end = raw
            .windows(2)
            .position(|pair| pair == b"\r\n")
            .ok_or_else(|| "malformed chunk: no size line".to_owned())?;
        let size_text =
            std::str::from_utf8(&raw[..line_end]).map_err(|_| "malformed chunk size".to_owned())?;
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| format!("malformed chunk size: {size_text:?}"))?;
        raw = &raw[line_end + 2..];
        if size == 0 {
            return Ok(body);
        }
        if raw.len() < size + 2 {
            return Err("truncated chunk".into());
        }
        body.extend_from_slice(&raw[..size]);
        if &raw[size..size + 2] != b"\r\n" {
            return Err("malformed chunk terminator".into());
        }
        raw = &raw[size + 2..];
    }
}

impl ModelProvider for OllamaProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn trust(&self) -> ProviderTrust {
        ProviderTrust::Local
    }

    /// Carries the vouch because the constructor already refused every
    /// non-loopback base URL: this adapter cannot have been pointed at
    /// another machine. It still does not confine the daemon itself — the
    /// product routes to that process, it does not sandbox it.
    fn local_vouch(&self) -> Option<LocalVouch> {
        Some(LocalVouch::core_reviewed())
    }

    /// `GET /api/tags` answers 200 when the daemon is up; anything else,
    /// including a hung or absent daemon, is `Down` — never a panic and never
    /// longer than the timeout.
    fn health(&self) -> Health {
        match self.exchange("GET", "/api/tags", b"", self.health_timeout) {
            Ok((200, _)) => Health::Healthy,
            _ => Health::Down,
        }
    }

    fn complete(&self, request: &ModelRequest) -> Result<String, ProviderError> {
        // A generous token hint derived from the caller's character ceiling;
        // the gateway remains the enforcer of the ceiling itself.
        let num_predict = (request.max_output_chars / 2).clamp(64, 4096);
        let body = serde_json::json!({
            "model": self.model,
            "prompt": request.prompt,
            "stream": false,
            "options": { "num_predict": num_predict, "temperature": 0.2 },
        });
        let payload =
            serde_json::to_vec(&body).map_err(|error| ProviderError(error.to_string()))?;
        let (status, response) = self
            .exchange("POST", "/api/generate", &payload, self.timeout)
            .map_err(ProviderError)?;
        if status != 200 {
            return Err(ProviderError(format!("ollama answered HTTP {status}")));
        }
        let parsed: serde_json::Value = serde_json::from_slice(&response)
            .map_err(|error| ProviderError(format!("malformed ollama JSON: {error}")))?;
        parsed
            .get("response")
            .and_then(|value| value.as_str())
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
            .ok_or_else(|| ProviderError("ollama response has no text".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_loopback_base_urls_are_accepted() {
        assert_eq!(
            parse_loopback_base_url("http://127.0.0.1:11434").unwrap(),
            ("127.0.0.1".to_owned(), 11434)
        );
        assert_eq!(
            parse_loopback_base_url("http://localhost/").unwrap(),
            ("localhost".to_owned(), 11434)
        );
        for bad in [
            "https://127.0.0.1:11434",
            "http://10.0.0.5:11434",
            "http://ollama.example.com",
            "http://127.0.0.1:notaport",
            "127.0.0.1:11434",
        ] {
            assert_eq!(
                parse_loopback_base_url(bad).unwrap_err(),
                OllamaConfigError::NotLoopback,
                "{bad}"
            );
        }
        assert_eq!(
            OllamaProvider::new("o", "http://127.0.0.1:11434", "  ").unwrap_err(),
            OllamaConfigError::InvalidModel
        );
    }

    #[test]
    fn responses_are_framed_by_content_length_or_chunked_transfer() {
        let (status, body) =
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhelloEXTRA").unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, b"hello");
        let (status, body) = parse_response(b"HTTP/1.1 404 Not Found\r\n\r\nnope").unwrap();
        assert_eq!((status, body.as_slice()), (404, &b"nope"[..]));
        // Chunked, as Ollama answers any non-trivial non-streaming request:
        // two chunks, an extension on one size line, a trailer after the end.
        let (status, body) = parse_response(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3;ext=1\r\nhel\r\n2\r\nlo\r\n0\r\nX-Trailer: t\r\n\r\n",
        )
        .unwrap();
        assert_eq!((status, body.as_slice()), (200, &b"hello"[..]));
        // Malformed or cut-off chunking is refused, never guessed at.
        for raw in [
            &b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhel"[..],
            &b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\nhello\r\n0\r\n\r\n"[..],
            &b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhelloXX0\r\n\r\n"[..],
            &b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n"[..],
        ] {
            assert!(
                parse_response(raw).is_err(),
                "{:?}",
                String::from_utf8_lossy(raw)
            );
        }
        assert!(parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 50\r\n\r\nshort").is_err());
        assert!(parse_response(b"garbage").is_err());
    }
}
