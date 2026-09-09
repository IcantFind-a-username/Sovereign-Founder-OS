# Playground loopback transport contract

**Revision 1 · Independently reviewed and Frozen by controller; implementation
requires the dependencies below.** Design baseline c6b5b8a; S1-G05 acceptance is recorded in
[its report](reports/S1-G05-supervised.md). S1-05 is in progress; S1-G06/S1-06
are dependencies, not completed evidence. This contract defines the next
bounded increment of [standalone-v2](../../superpowers/plans/2026-08-14-consultant-playground-standalone-v2-implementation.md).
[HTTP Rev2](playground-http-contract.md) and [assets Rev1](playground-assets-contract.md)
retain their interfaces and business behavior. RFC 0004/0005 do not authorize
real-data/model/storage integration; OwnerSession and ActiveV2 remain absent.

## Accepted transport-only amendment and evidence

Replace standalone-v2's assumed `tiny_http` transport with literal IPv4
loopback `std::net::TcpListener`/`TcpStream` and
`httparse = { version = "=1.10.1", default-features = false }` in the leaf.
No runtime dependency of httparse is declared in its installed manifest.
It is cached locally but **not currently in Cargo.lock**; S1-07 adds its one
registry package and the leaf dependency edge. No root workspace dependency or
CLI dependency change is needed here. Its build script is a dependency build
detail, not permission for a leaf build script or runtime environment access.

Inspected local registry sources, not inferred from the legacy CLI pattern:

| Installed source | Observed behavior / implication |
| --- | --- |
| tiny_http-0.12.0/src/client.rs:80–149, 285 | Unbounded line/header buffers; trims request/header lines; request-line parsing does not check for extra parts. Raw malformed input cannot be reconstructed downstream. |
| tiny_http-0.12.0/src/request.rs:143–246 | First Content-Length selected; any Transfer-Encoding supersedes it and selects chunked decoding; lengths <=1024 may be fully read before delivery. A handler `.take(257)` cannot enforce the approved total body-read bound. |
| tiny_http-0.12.0/src/util/equal_reader.rs:69 | Dropping an unread fixed-length body allocates/drains its remainder. Dropping a request is not proof of bounded close. |
| tiny_http-0.12.0/src/lib.rs:288–340, 388 | Per-client tasks are internally managed; recv_timeout times the request queue, not socket/header/body reads. No public per-client deadline/forced-close control was found in these sources. |
| tiny_http-0.12.0/src/response.rs:73, 351–360; request.rs:451–470 | Default response adds clock-derived Date and Server; respond suppresses several disconnect errors. This cannot prove exact deterministic headers or successful delivery. |
| httparse-1.10.1/src/lib.rs:210–218, 458–553, 719 | Caller supplies bytes and bounded header slots. Default request parser preserves method/path and duplicate headers; no IO/body reader. No permissive ParserConfig option is enabled. |
| httparse-1.10.1/src/lib.rs:558, 1119–1249 | Parser permits leading empty lines/bare LF; request name whitespace/folding is rejected, values trim SP/HTAB. The narrow CRLF/ASCII policy below rejects unwanted tolerance before parsing. |

Also amend the plan's no-clock statement **only for server liveness**:
`Instant` and `Duration` may enforce a total read deadline and a total write
deadline. No SystemTime, timestamp, Date header, business clock, timed action,
state expiry, telemetry, or timing input reaches domain/handler/assets. Permit
one fixed startup URL line to stdout. Controller independently accepted these narrowly scoped exceptions.
Implementation still requires the predecessor cards and G07 fixture review.
No tiny_http fork, proxy, raw-socket escape, custom HTTP grammar parser, or
dependency-wide IO allowance is proposed.

## Exact interfaces and lifecycle

`src/lib.rs` retains all accepted declarations and adds private `mod server;`
plus this sole public entry (wrapper, not public module or re-export):

```rust
pub fn run(port: u16) -> std::io::Result<()> { server::run(port) }
```

New `src/server.rs` uses these concrete interfaces; other production helpers
require declaration in G07's fixture review, not invention during S1-07:

```rust
const MAX_HEAD_BYTES: usize = 8192;
const MAX_HEADERS: usize = 32;
const MAX_BODY_READ_BYTES: usize = 257;
const READ_BUDGET: Duration = Duration::from_secs(5);
const WRITE_BUDGET: Duration = Duration::from_secs(5);
enum ReadFailure { BadRequest, HeadersTooLarge, Timeout, Io(std::io::Error) }
pub(crate) fn run(port: u16) -> std::io::Result<()>;
fn serve_connection(stream: &mut TcpStream, handler: &PlaygroundHttpHandler,
                    bound_port: u16) -> std::io::Result<()>;
fn read_head(stream: &mut TcpStream, deadline: Instant,
             head: &mut [u8; MAX_HEAD_BYTES]) -> Result<usize, ReadFailure>;
fn content_length(headers: &[httparse::Header<'_>]) -> Result<usize, ReadFailure>;
fn read_body(stream: &mut TcpStream, deadline: Instant, length: usize,
             body: &mut [u8; MAX_BODY_READ_BYTES]) -> Result<usize, ReadFailure>;
fn write_response(stream: &mut TcpStream, status: u16, headers: &[(&str, &str)],
                  body: &[u8], head_only: bool) -> std::io::Result<()>;
```

run binds **once** with `TcpListener::bind(("127.0.0.1", port))`. Immediately
derive bound_port from this listener's `local_addr()?.port()`; reject zero or a
non-127.0.0.1 actual address as an internal configuration error. Port 0 requests
an ephemeral port and never reaches the handler as zero. Nothing derives the
expected authority from request Host/Origin, environment, CLI root, or URL.
Write exactly `Playground: http://127.0.0.1:<bound_port>\n` to stdout using
fallible std IO; no request/error payloads or additional logging.

Construct exactly one `PlaygroundHttpHandler::new()` per run; synchronously
accept one connection, serve it once, then close/drop it and continue. There
are no production threads, task pools, queues, public listener injection,
shutdown flag, endpoint, env switch, or signal handler. Bind/local-address/
startup-output/fatal accept failures return Err; retry Interrupted accept.
Request parse/timeout/socket/encoding failures terminate only that connection.
The OS terminates the process on the usual external stop; restart constructs
the fixed fixture. One slow accepted client can delay others for a read/write
budget; this is a bounded small demo, not a general availability guarantee.

## Input framing and limits

read_head uses the fixed 8192-byte array and **one-byte socket reads** until
CRLFCRLF, so headers cannot prefetch an oversized body. Do not introduce a
BufReader, read_to_end, unbounded line reader, or larger staging buffer.
Reject a leading CR/LF, non-ASCII head bytes, bare LF, or CR not followed by LF;
this is framing policy, not a second method/header/URI grammar. The head must
finish including its delimiter within 8192 bytes. Parse it once with
`httparse::Request::new(&mut [httparse::EMPTY_HEADER; MAX_HEADERS]).parse(...)`;
require Complete equal to the head length, populated method/path, and version
0 or 1. TooManyHeaders or unfinished full head yields transport 431; other
parse/framing/version errors yield transport 400. Default parser options only.

Retain every parsed header in order as borrowed name/value pairs; conversion
to &str is fallible and no header is discarded. Names/method/target are not
case-folded, trimmed, decoded, normalized or reconstructed. httparse's value
OWS trimming is compatible with the handler's SP/HTAB trim. In particular,
duplicate Host/Origin/Content-Type reach the handler unchanged; malformed name
whitespace/folding/control characters are rejected at transport first.

Before body reads, content_length enforces this deliberately small framing
policy using parsed headers, ASCII-insensitive field names, and checked usize
decimal conversion:

- Any Transfer-Encoding, Expect, Upgrade, or a comma-separated Connection token
  equal to `upgrade` yields transport 400. No chunked body, trailers,
  100-continue, protocol upgrade, TE/CL precedence, or fallback decoder.
- Content-Length may occur zero or one times, never a comma list. If present,
  after SP/HTAB OWS it is `0` or a nonzero ASCII digit followed by ASCII digits;
  reject signs, leading zeroes, empty/overflow values and duplicates, including
  equal duplicates. Missing length means a zero-length request body.
- Both HTTP/1.0 and HTTP/1.1 are accepted, with the handler's Host requirement
  on both. There is one request per connection, no keepalive or pipelining.
  Bytes beyond the declared length are not a second request and are not read.

Start one read deadline on accept, shared by head and body. Before **each**
read set the socket timeout to checked positive time remaining until that
Instant. Expiry/TimedOut/WouldBlock yields 408; Interrupted retries within the
same deadline. A per-read five-second reset is forbidden. EOF before complete
head/body is incomplete and never a successful empty request; best-effort
transport 400 if still writable, otherwise close. Other IO failures close.

Read exactly min(Content-Length,257) actual body bytes into the fixed array.
Lengths <=256 must finish completely; failure never invokes handle. Once 257
bytes have been read, invoke handle with that actual slice and immediately
close after its rejection; never allocate/read/drain the remaining declared
body. Content-Length alone is not a fabricated 257-byte body. If fewer than
257 of an oversized declared body arrive before EOF/deadline, return the
incomplete/timeout transport result. Accepted handler validation precedence
is preserved: with valid Host/Origin/target/method this produces typed 413;
earlier handler errors still win. Framing policy errors precede the handler
and never mutate its session.

## Handler and response integration

For a framed request call the existing handle **once**, passing raw parsed
method/target/duplicate header slice/actual bounded body and trusted bound_port.
Do not reimplement route, Host, Origin, JSON, action, or state logic.

| Outcome | Transport action |
| --- | --- |
| Json(response) | `serde_json::to_vec(&response.body)` with Result handling; status and every application header come directly from response. |
| Asset(route) | Call `assets::asset(route)` once; status 200, exact returned content_type/bytes, Cache-Control:no-store and X-Content-Type-Options:nosniff. |

The six route variants and MIME/file mappings remain exactly assets Rev1.
There is no fallback filesystem lookup, asset placeholder JSON, extra route,
redirect, index fallback, browser opening, or mutable asset cache.

write_response emits HTTP/1.1, a closed status reason phrase, application
headers in their supplied order, Content-Length of the entire encoded body,
and Connection: close, then CRLF and body. Status phrases are standard fixed
strings for 200/400/403/404/405/408/413/415/431/500/503 only. No Date, Server,
Set-Cookie, Access-Control-*, Location, dynamic message or request echo. For
HEAD, preserve the handler's 405/404/etc and headers/encoded Content-Length,
but suppress body bytes as required by HEAD framing; it never becomes 200.

Transport 400/408/431 responses have an empty body and **only** no-store,
nosniff, Content-Length:0 and Connection:close. They are framing failures, not
new typed business errors. The eleven existing typed codes remain unchanged.
No success response is emitted on serialization failure; close with Err.
Construct a separate five-second total write deadline for the complete head
and body, updating positive socket write timeout before every bounded write;
retry Interrupted within the same deadline, error on zero write. No unbounded
write_all retry loop. Close/drop after every result; never drain or retry the
request. A best-effort shutdown error is harmless to already completed work.

A valid POST may apply before a serialization/write/disconnect failure.
Do not roll it back, reset, apply twice, or claim delivery. The process-local
state remains available to a later GET; browser failure/reload behavior remains
assets Rev1. No test may infer nonmutation merely because response delivery
failed. No transaction or exactly-once guarantee is claimed.

## Gate migration and implementation ownership

[S1-G07](cards/S1-G07.md) is an Astra architecture task followed by independent
review; [S1-07](cards/S1-07.md) is the later Luna implementation. Before G07,
S1-06 and G06's extraction to `tests/support/source_closure.rs` must actually
be accepted, and previous writers stopped. New source closure is exactly
assets/catalog/domain/http/lib/server; all previous closures remain valid.
Only the new lib shape includes public run and server. The server source is
an independently written, compiled, rustfmt-stable full token fixture, pinned
before Luna starts; do not refresh expected source from Luna's candidate.
Reuse G06's small asset mapping fixture and placeholder six-file tree, never
duplicate the HTML/CSS/JS website. Do not grow physical_boundary_source.rs.

Extend existing manifest validation once: admit old empty and exact serde +
serde_json dependencies unchanged; admit the new exact three-dependency set
only when server.rs exists, and require that set when server.rs exists.
All registry/kind/rename/target/path/optional/features/default-feature checks
remain exact; httparse requires `=1.10.1`, no features, defaults false. Source
closure separately pins the server/lib pairing. No TLS, tiny_http, feature
switch, dev/build dependency, leaf build script, alternate target or backend.

## Required evidence and acceptance

G07 and S1-07 contain the exact named tests, RED/GREEN and write paths. Actual
raw TCP tests must prove bytes on a socket, rejection nonmutation, timeout
liveness and one-request close; a fabricated HttpRequest is insufficient.
Test-only subprocess launch uses the integration test executable's exact
named child fixture, calling **public run(0)**. The fixture marker/environment,
port discovery, watchdog, process kill/wait and temporary state belong solely
to tests; production has no test switch. Guarded teardown kills and reaps on
both success and panic, including a process with an incomplete connected body.
The fixture test is not counted as behavioral acceptance. Restart must restore
the exact initial GET state. Real CLI/two-root canary isolation remains S1-09
and later; no test binary is shipped as a CLI shortcut.

Controller accepted the concrete amendments, 8192/32/257 and 5s/5s constants,
strict non-chunked framing, HEAD suppression and transport-error distinction
after reviewing the independent architect candidate. G07 still requires an
independently reviewed private helper layout/fixture; future CLI/browser cards
remain separate prerequisites. Real 375px/desktop accessibility, full browser
action/error flow and network interception remain S1-08; this design claims
none of those checks have run.

Tool inventory: legacy apps/cli/src/ui.rs run/bind pattern (read only, never
imported); RustLexer/source_boundary/shared wrapper/path; G06 source_closure;
source_root/ManifestFixture/manifest_boundary; accepted HTTP and asset fixtures;
serde_json/test_changed. New declared capability: httparse-owned HTTP grammar,
bounded transport framing/deadline helpers above, and test-only raw TCP/process
fixture. No replacement JSON/router/asset/source scanner tools.
Reuse these; do not reimplement equivalents. Declare new helpers first.
