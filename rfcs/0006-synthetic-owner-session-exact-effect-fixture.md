# RFC 0006: Synthetic Owner Session and Exact Local-Outbox Fixture

**Status:** Draft; fixture-proof contract (no product claim)
**Stage:** Program 1C0 mechanism proof / Program 2 exact-effect proof
**Security impact:** Critical
**Normative dependencies:** RFC 0003, RFC 0004, RFC 0005

## Summary

This RFC freezes the contract for a **synthetic-data-only** proof of two
mechanisms: the Program 1C0 WebAuthn / session / one-use-approval flow, and the
Program 2 exact local-outbox (`.eml`) effect state machine. It exists so the
mechanisms can be built and adversarially tested against deterministic
synthetic values **before** any product owner admission, workspace migration,
or protected-payload persistence is designed.

It admits **no product path**. It creates, imports, mutates, marks, or promotes
no product or Vault workspace. Every constraint below is a freeze: a later
implementation task may satisfy it, never relax it. `scripts/check-owner-effect-rfc.sh`
gates this document against the constraint set.

This is a mechanism proof, not owner admission. Concretely, and stated up front
as the honest boundary: a hostile same-account native process can win the
empty-registry enrollment, and that is **not owner admission**. The fixture
proves that *given* an established fixture credential the downstream session,
approval, reservation, effect, and evidence mechanics hold — nothing about who
established it.

## Normative vocabulary and maturity

The keywords MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are requirements for
the fixture boundary. Every capability here is `Fixture` maturity: synthetic,
unqualified, and structurally rejected by product code. Nothing in this RFC is
`Current` product protection.

## Global constraints (frozen)

### G1 — Synthetic scope

This whole slice is **synthetic** / fixture-only. The one and only recipient is
the compile-time constant `fixture-recipient@example.test`; sender, subject, and
body are compile-time canaries. No venture, customer, or document schema exists
in the fixture. No route, command, IPC message, import, form, or library API
accepts real business data or an existing product/Vault root.

### G2 — Conjunctive future gates, no activation transition

Product use remains blocked until **all** of the following are complete —
these are **conjunctive** gates, not alternatives:

- Program 1B1 clean-machine recovery qualification;
- Program 1C1 identity/role-key custody and handoff;
- Program 1D `ActiveV2` activation; and
- a later reviewed **protected-payload** persistence design.

This RFC defines no product-activation transition, no enrollment or setup
call-to-action, no migration, and no support-promotion state. Real-platform
WebAuthn evidence can qualify only a mechanism-matrix entry; it removes no
product gate.

### G3 — No persisted long-lived owner key

No new approval, authority, audit, or IPC private key is persisted before
Program 1D / 1C1. Approval keys are one-object ephemeral. Authority, audit, and
the parent-generated IPC key are per-broker-launch synthetic keys invalidated
on restart. Persisted records carry only public keys/identities plus
`non_product=true`; no continuity claim is made.

### G4 — Fixture markers, structurally rejected by product

The dedicated directory marker is `synthetic-owner-effect-fixture-v1`; the
distinct authority-root record tag is `synthetic_fixture_root_v1`. Both are
synthetic-only and MUST be rejected by product open/import/export code. There
is no product workspace-format marker in this plan.

### G5 — Feature-gated, no default/release exposure

Every fixture constructor, module, IPC command, route, and CLI entry point
lives behind a crate-local non-default `owner-effect-fixture` feature,
forwarded explicitly by that same named feature. No default or release binary
can call fixture registration, root, payload, reservation, broker entrypoint,
or dispatch. `ProductOwnerAdmission` has no constructor or route in this slice;
`UnqualifiedFixtureBootstrap` is reachable only behind the feature and an
explicit `security-fixture --synthetic-only` command, and its type cannot
satisfy a future product admission API.

### G6 — Empty-registry enrollment is not owner admission

Empty-registry registration is *first valid user-verification wins*. It does
**not** identify the intended owner against a hostile same-account native
process. This RFC invents no bootstrap anchor and makes no independent-admission,
native-bootstrap-defeat, owner-continuity, recovery, or product-enrollment
claim. Loopback origin, the same OS account, possession of a password, and an
unlocked credential store are **not owner admission**.

### G7 — Exact origin

The exact origin is the compile-time constant `http://localhost:7787`; the RP
ID is `localhost`; the socket binds only `127.0.0.1:7787`. The origin record is
bound to the random fixture generation and MUST equal the compiled constant on
every reopen. Port occupation or mismatch returns `FixtureOriginUnavailable`;
there is no random port, fallback, redirect, alternate hostname, or
port-change API.

### G8 — Mechanism matrix frozen before owner code

A mechanism-matrix entry (exact OS build, browser build, authenticator
attachment/transport, resident/discoverable behavior, cookie behavior, origin)
is frozen before owner code. Each entry MUST prove WebAuthn create/get plus
acceptance and later transmission of the exact `Secure` cookie at the exact
origin, and characterize a malicious second-port discoverable assertion and
same-user-handle credential creation where the platform exposes them. Builds
that cannot satisfy or characterize the required behavior are excluded; `Secure`
is never removed. An empty real matrix is allowed and leaves the mechanism
unqualified.

### G9 — Cookie and CSRF

The cookie is `__Host-sfo_fixture_session` with `Secure; HttpOnly;
SameSite=Strict; Path=/` and no `Domain`. Session and CSRF values are
independent random 256-bit values retained server-side only as SHA-256 digests.
Sessions have a 15-minute absolute and 5-minute idle lifetime. Cookies are
host-wide and **not** port-isolated: another localhost port may receive,
overwrite, or bomb the fixture cookie. Authorization still requires the exact
origin and the independent in-memory CSRF value, so the accepted residual is
denial of service / session theft at the cookie layer without authorization —
a **cross-port** residual, explicitly not called locality.

### G10 — WebAuthn ceremony

Browser option timeout and server ceremony expiry are both exactly `300`
seconds. Registration and login require **user verification** (UV). Before
registration the broker stores one random non-PII user UUID bound to the random
fixture generation; the fixed synthetic strings are `sfo-fixture` /
`Sovereign Synthetic Fixture`. Login always supplies the one stored credential
ID in `allowCredentials` and validates any returned user handle against the
stored UUID; it never performs username-less login. The label is only
`WebAuthnUvPlatformFixture(<frozen-matrix-entry>)` — no hardware-backing,
non-exportability, attestation, or locality claim. The fixture admits exactly
one credential; there is no add-second-credential, rotation, or recovery path,
and credential loss is destructive.

### G11 — One-use approval issuer

There is one **one-use** approval issuer. Each authorization is per-operation
bound to the fixture workspace/root, generation, operation, policy/invocation
commitment, expiry, and a **fresh challenge**. Its consumers are Exact Effect,
Program 1B1 backup, and Program 1D activation. The signed approval object stays
opaque; no payload getter crosses a crate boundary.

### G12 — One broker owns the sole writable store

One broker process owns the sole writable `authority.redb` for its lifetime;
clients never open it. A second broker that loses the fixture process lock
returns `BrokerAlreadyRunning`; a direct second `redb` open returns
`DatabaseAlreadyOpen`. Neither retries, waits, opens read-only, or creates a
fresh store. Broker death closes IPC, invalidates memory-only sessions/grants,
and fails pending work closed.

### G13 — Same-image bootstrap, no sibling artifact

A clean `sovereign-cli` build with the feature re-execs its own
`current_exe()` in a cfg-gated hidden mode; there is no sibling broker
artifact, and the default/release image has neither the enum variant, the match
arm, the broker entrypoint, nor broker symbols. The parent writes a bounded
bootstrap frame (random 256-bit launch key, launch nonce, canonical fixture
root, protocol version) to the child's piped stdin and closes it; the child
emits only a bounded non-secret address frame to stdout and must prove key
possession in the first authenticated IPC response. The launch key never enters
argv, environment, stdout, stderr, or disk. A same-account native process can
invoke the hidden mode with its own valid frame; the hidden-mode name and a
caller-chosen pipe key are connection authentication, **not owner admission**.

### G14 — Supervisor establishment deadline

Broker startup has one non-extendable **five-second** monotonic
supervisor-establishment deadline, captured immediately before the non-secret
address frame is emitted. Acquiring the process lock, opening redb, reading
fixture state, or starting any command is forbidden until a complete
launch-key-authenticated supervisor hello is accepted before that deadline.
Accepts, partial frames, failed authentication, wall-clock changes, or retries
never reset it. On deadline/EOF/auth failure the broker closes sockets,
zeroizes the launch key, and exits without ever taking the lock or opening redb.

### G15 — Authenticated IPC

Broker IPC is a loopback-only `TcpListener` on an OS-assigned ephemeral
`127.0.0.1` port, distinct from the browser origin port. The first
authenticated connection is the sole **supervisor** control connection; only it
may `RegisterConnectionV1` / `RevokeConnectionV1`. Each client receives an
independent random 256-bit connection key, closed command scope, absolute
expiry bounded by broker lifetime, and a per-connection sequence. Each request
carries its connection ID, strictly increasing sequence, random nonce, and an
**HMAC-SHA-256** tag. Unknown, revoked, expired, out-of-scope, replayed,
gapped, bad-MAC, peer-mismatched, oversize, or timed-out messages fail closed.
Supervisor-control EOF — clean close or parent death — revokes every
connection, stops accepting work, closes redb, releases the lock, and exits.
MAC verification uses only constant-time verify; the reviewed profile is
`hmac 0.12.1` with default features disabled, with the resolved
`sha2 -> digest -> crypto-common` graph frozen by a dependency contract.

### G16 — Two-phase redb writes and gated migration

`begin_immediate_two_phase()` is the only crate-private redb write-transaction
constructor: immediate durability plus two-phase commit, database field private
to the store module, with source/instrumentation tests rejecting any other
write path. Synthetic legacy-claim migration is admitted only on exact
`x86_64-unknown-linux-gnu`, publishes via `hard_link`-to-absent with fail-closed
parent fsyncs, and claims **process-crash atomicity only, not power-loss
durability**. Other OS/ABI/filesystem profiles return
`FixtureMigrationUnavailable`.

### G17 — Random intent id and immutable sealed payload

`effect_intent_id` is 128 random bits allocated before any synthetic value is
read and never derived from a value, timestamp, path, or digest; the file name
is exactly `<effect_intent_id>.eml`. The broker seals one exact normalized
recipient, exact RFC 5322 bytes, operation `local_outbox.write_rfc5322`,
policy/invocation binding, fixture root/generation, prepared expiry, and
retention. Any change creates a new random intent and a fresh preview/approval.
RFC 0003 / Capability V2 primary resource and canonical input bind only the
random intent id, generation, fixed operation, and immutable coordinator
reference — never recipient/content or an unkeyed digest of either — so
approval, token, and evidence cannot become a low-entropy dictionary oracle.

### G18 — Reservation is one transaction; revalidation before dispatch

The existing authority store is the one transaction coordinator: approval id
and its own expiry, token id and expiry, idempotency key bound to the random
intent id, fixture authority node/use, reservation, and the
`Prepared -> AuthorityReserved` commit happen together through the two-phase
helper. Reservation is not sufficient authority for exposure: before starting
the pure fixture and again inside the transaction that would commit
`AuthorityReserved -> Dispatching`, the broker revalidates trusted time against
prepared expiry, policy snapshot/epoch/expiry, root and every ancestor's
lifetime/revocation, origin generation, a live session for the same registered
credential, and equality of the current logout epoch with the epoch captured at
reservation. A losing race atomically records
`Dispatching -> FailedBeforeDispatch` with the no-I/O proof and never invokes
the writer. Once `Dispatching` is durably committed, later expiry/logout cannot
cancel, retry, or relabel it.

### G19 — Effect state machine

Required states are `Prepared -> AuthorityReserved -> Dispatching ->
{Succeeded, FailedBeforeDispatch, Indeterminate}`. `FailedBeforeDispatch` is
allowed only in a live broker that proves no payload byte was written,
published, or exposed and durably commits that result. After a process death,
`Dispatching` plus an absent file is always `Indeterminate`; absence is never
proof of non-exposure. Entering `Dispatching` never causes automatic retry,
failover, rewrite, deletion, or a new-send suggestion. Recovery maps exact
identical final bytes to `Succeeded`; absent, different, wrong-type, unreadable,
or uncertain durability to `Indeterminate`.

### G20 — Value-free effect evidence

Public signed effect evidence contains only version/type, a random event id,
the random intent id, a closed outcome, the previous-event hash, a synthetic
signer public identity/key, the event hash, and the signature. It is
**value-free**: no recipient/content or deterministic digest, business id,
path, byte count, time, account, reason, or policy value. `sovereign-effects`
is an opaque façade; the actual local-outbox write is coordinator-private, and
neither crate gains DNS, SMTP, HTTP-client, provider, credential, or
non-loopback egress.

### G21 — Runtime-plaintext allowlist

Synthetic runtime plaintext is expected only in: the private redb table
`fixture_protected_payload_v1` and its transactional/freed pages inside
`authority.redb`; at most one owner-only same-directory
`<effect_intent_id>.eml.tmp` after durable `Dispatching`; the ephemeral
authenticated preview; and the exact final `<effect_intent_id>.eml`. This is the
**runtime-plaintext allowlist**. A table- and filesystem-aware scanner MUST
prove expected copies exist per state and that every other table, file, log,
error, IPC/HTTP field, evidence record, and export is canary- and
deterministic-digest-free. redb is ACID and crash-safe but **not encrypted** and
not cryptographically authenticated, so even synthetic plaintext persistence
generalizes to nothing about founder data.

### G22 — Fault injection is test-only

`fault-injection` is non-default, test-only, and forbidden from default/release
dependency graphs and binaries. Named crash barriers exist only under that
feature. Every semantic fault test MUST first capture a genuine failing RED
before the barrier/behavior is implemented.

### G23 — Out of scope

No Program 1C1 custody, Program 1D activation, Vault migration,
backup/recovery, real business mutation, provider credential, SMTP, or network
email work is in scope. Legacy behavior remains runnable only under a non-default
`legacy-experimental` feature and its existing warnings; legacy cannot open a
fixture root and the fixture cannot open a legacy/product root.

## Honest security boundary

This RFC proves mechanisms against deterministic synthetic values *after* an
unqualified fixture credential exists. It does not prove who won empty-registry
enrollment. A native process can synthesize loopback headers and client data,
drive its own authenticator, and win the empty fixture registry; the test suite
preserves this as an explicit non-claim. Exact Host/Origin/Fetch-Metadata and
CSRF protect browser requests after session establishment, not first-owner
admission. Same-account modification of the trusted process image,
process-memory/FD capture, browser-profile compromise, root compromise, and
disk tamper are outside this mechanism proof. The shared RP namespace permits
same-user-handle credential replacement/lockout; an exact-origin mismatch
prevents such a credential from authorizing port 7787 but cannot restore the
single credential, so a destructive **cross-port** credential DoS is an accepted,
named residual and is not owner admission, session authorization, or platform
locality.

## Relationship to other RFCs

- **RFC 0003** supplies the one-use approval evidence; this fixture consumes it
  and adds the session/UV bridge and the reservation transaction. RFC 0003
  Amendment 1's transactional consumption bundle applies to the current
  filesystem store; this fixture's authority plane is the broker-owned redb
  store, a separate store on a separate gate.
- **RFC 0004** ordering places 1C0 before backup and activation; this RFC is the
  1C0 mechanism proof and asserts none of RFC 0004's product data-boundary
  protections.
- **RFC 0005** owns the dual-root Vault, recovery, and the conjunctive product
  gates in G2; this RFC neither implements nor claims any of them.

## Change control

Every constraint above is frozen. A change to any of them is an amendment to
this RFC with its own review, never an implementation-task decision. Removing or
weakening a conjunctive gate in G2, the origin in G7, the ceremony bounds in
G10, the IPC authentication in G15, or the value-free evidence rule in G20 is a
security-critical change.
