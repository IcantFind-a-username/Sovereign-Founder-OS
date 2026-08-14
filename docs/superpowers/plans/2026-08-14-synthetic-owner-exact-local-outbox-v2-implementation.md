# Synthetic Owner-UV and Exact Local-Outbox Fixture v2

**Status:** Target — blocked until the closed RFC 0002 profile below is
accepted through normal governance.

**Goal:** Build a fixed-value fixture that can demonstrate a fresh
user-verification ceremony authorizing one exact local `.eml` publication
through the existing RFC 0003 and Capability V2 primitives, one durable
reservation transaction, and conservative crash recovery.

This is a security fixture, not product onboarding. It is not 1C0, Program 2
completion, product authority, product-safe persistence, an email send, or
E2EE.

## Sources and prerequisites

- [Security Architecture Program](2026-08-13-security-architecture-program.md)
- [RFC 0002](../../../rfcs/0002-wasm-sandbox-and-plugin-capabilities.md)
- [RFC 0003](../../../rfcs/0003-signed-approval-evidence.md)
- [Threat model](../../../THREAT_MODEL.md)
- [Contributing and RFC process](../../../CONTRIBUTING.md)

Before implementation, a **separate PR** must amend RFC 0002 with the closed
profile in this plan. That RFC stays Draft during discussion, receives at
least the repository's seven-day substantial-change discussion period, and
requires explicit maintainer acceptance with rationale. An implementation PR
cannot accept its own RFC. Any accepted change to the profile updates this
plan before code begins.

The approval-retention fix is already Current at commit `6c0259b`. Do not
reimplement it: durable approval claims retain the verified signed approval
expiry, and token-expiry purge, store reopen, replay denial, and
approval-expiry purge are tested. The larger reservation is still missing.

## Chosen architecture

```text
browser at fixed loopback origin
        |
        v
one release-excluded fixture process
  - HTTP origin and closed routes
  - WebAuthn UV, session and CSRF state
  - RFC 0003 approval ceremony
  - sole redb coordinator
  - fixed Core Wasm invocation
  - exact local-outbox publisher
        |
        v
value-free synthetic evidence
```

The independent `publish = false` fixture executable owns the entire path.
It has one public listener bound to `127.0.0.1:7787`, with origin
`http://localhost:7787` and RP ID `localhost`. It acquires and retains an OS
file lock **before** opening redb. There is no internal listener or private
transport: all owner, session, coordinator, and outbox calls are ordinary
typed calls inside the same process.

The fixture package may be a workspace member for tests, but it is excluded
from product packaging and is not a dependency of `sovereign-cli`.
`sovereign-cli`, `apps/cli/**`, and the current product UI are untouched.
Clean product release graphs and symbols must exclude the fixture package,
redb, and WebAuthn dependencies.

### Rejected design

Do not implement the superseded HMAC transport, launch key/nonce, sequence
protocol, connection registration/revocation, hidden child mode, broker child
process, or second/ephemeral TCP listener. Process separation can return only
after a concrete threat and a separately reviewed standard transport justify
it.

## Capability verification and authority seam

Current `CapabilityValidatorV2` mixes COSE/context verification with
side-effecting `AuthorityStore` consumption. Before the fixture implements any
transaction, split that path into a side-effect-free cryptographic and context
verifier. It returns an opaque `VerifiedCapabilityV2` and, when approval is
required, an opaque `VerifiedApprovalV1`. Both proof types have private fields,
no public constructor, and no `Clone`, `Serialize`, or `Debug` implementation.
Narrow read-only accessors may expose already-verified bindings needed by the
coordinator, but the coordinator must consume each proof by value.

Pure verification must not mutate process-local replay state, open or write an
`AuthorityStore`, or reserve any claim. The existing pure-compute
`authorize_and_consume*` wrappers temporarily call this verifier and then
perform the current `AuthorityStore` token, idempotency, and approval
operations with unchanged ordering, error behavior, and regression results.
That compatibility path remains non-transactional. This slice does not invert
or remove the current capability-to-authority dependency; dependency inversion
is a separately reviewed follow-up plan, not hidden fixture work.

The new fixture coordinator lives in an upper, release-excluded crate that
depends on both capability verification and authority primitives. Neither
lower crate gains a reverse dependency, so there is no
authority-to-capability-to-authority cycle. The coordinator must never copy
COSE, canonicalization, trust, policy, approval, or context checks, and it must
never call the legacy consuming validator before opening its own transaction.
It accepts only the opaque verified proofs and commits approval claim,
capability/token claim, idempotency binding, synthetic authority-node use, and
intent transition in one redb transaction.

Only a successful coordinator reservation can construct the private
`AuthorityReservedEffect`. Its fields and constructor are private, it has no
reconstruction accessor, and it implements neither `Clone`, `Serialize`, nor
`Debug`. The sole exact-outbox publish entry consumes that type by value; there
is no raw-ID, raw-redb, boolean, test-only, or alternate constructor that can
reach it.

## Closed RFC 0002 profile to propose

The amendment defines one structured Target profile; it is never parsed from
a dotted or free-form string:

```text
risk_class   = low-risk-effectful
backend      = core-wasm
abi          = sovereign_core_wasm_v2
tool_id      = local_outbox
tool_version = 1.0.0
operation_id = write_rfc5322
```

The profile permits only the fixed synthetic recipient
`fixture-recipient@example.test` and fixed compiled sender, subject, and body.
Every address is under `.example.test`. The Core Wasm guest has no ambient
filesystem, network, environment, clock, randomness, or host-call surface.
It receives authenticated canonical input and returns a closed result; the
trusted coordinator alone owns publication.

The coordinator allocates a random 128-bit `effect_intent_id` before reading
the compiled values, then seals the exact normalized recipient, exact RFC 5322
bytes, exact coordinator-derived outbox-relative path, operation, policy and
invocation binding, expiry, fixture generation, and signer epoch. A changed
byte or field creates a new intent and requires a new preview and approval.

## Owner mechanism and honest label

The fixture uses a maintained WebAuthn library through its safe API. Freeze a
mechanism-matrix row before enabling a real browser/OS combination: exact OS,
browser, authenticator class, UV result, origin, cookie behavior, stored
credential behavior, and localhost cross-port residuals. A deterministic
virtual authenticator is test evidence only.

Empty-registry enrollment is `UnqualifiedFixtureBootstrap`: first valid UV
wins. It does not identify the intended owner against another same-account
native process. The fixture admits one credential, has no recovery or second
credential, and loss requires destroying the synthetic fixture.

After acquiring the retained OS lock and before opening redb, every fixture
process generates a fresh, per-process `TypedSigner<ApprovalRole>` and a random
signer epoch. A closed `ApprovalBridge` owns the signer privately: it exposes
no signer getter, secret export, generic signing method, or serialization path.
The bridge itself is not cloneable, serializable, or debuggable. Keep the
dependency's zeroize-on-drop support enabled, keep the secret key only in
process memory, and exclude it from logs, redb, backups, and crash evidence.
After redb opens, it records only the corresponding approval-role public trust
record, random signer epoch, and explicit `unqualified_fixture` label for
historical verification and reconciliation.

After login, the process issues a memory-only session with a 15-minute
absolute lifetime, five-minute idle lifetime, one Secure/HttpOnly/SameSite
cookie, and an independent memory-only CSRF value. Exact Host, Origin, Fetch
Metadata, JSON media type, and bounded-body checks protect every JSON route.
Logout revokes the session, pending challenges, and unreserved approvals.

Session presence is not approval. After the exact preview, successful fresh UV
creates one opaque `FreshUvGrant`. It has private fields and no public
constructor, and is neither `Clone`, `Serialize`, nor `Debug`. The grant is
one-use and bound to the exact session ID, session generation/logout epoch,
credential ID, WebAuthn challenge, signer epoch, effect intent, invocation,
policy decision, fixture generation, and expiry.

`ApprovalBridge::approve_invocation` is the only signing entry point. It
consumes the grant by value, rechecks every binding and expiry, and then emits
one RFC 0003 COSE/Ed25519 approval over the existing canonical claim. Reuse the
existing role separation and canonical bytes; add no cryptographic primitive,
key derivation, signature envelope, or parallel freshness protocol.

A restart creates a new signer epoch and invalidates all earlier sessions and
in-memory grants. Persisted `Prepared` or `AuthorityReserved` work bound to an
older epoch fails closed as `FailedBeforeDispatch`, after proving no writer I/O
occurred, and is never automatically re-signed. Historical public trust
records can verify existing evidence or support reconciliation only; they can
never activate the old signer epoch. Persisted `Dispatching` work does not use
the new signer for authorization and reconciles only to `Succeeded` or
`Indeterminate`. These rules still describe a first-writer
`unqualified_fixture`; they do not provide owner continuity, credential
recovery, role-key custody, or satisfy 1C0/1C1.

The final loopback route manifest is closed and fixed before HTTP code lands:

| Method | Route |
| --- | --- |
| GET | `/` and the exact embedded fixture assets |
| POST | `/api/fixture/auth/register/start` |
| POST | `/api/fixture/auth/register/finish` |
| POST | `/api/fixture/auth/login/start` |
| POST | `/api/fixture/auth/login/finish` |
| POST | `/api/fixture/effect/prepare` |
| POST | `/api/fixture/effect/preview` |
| POST | `/api/fixture/effect/approve/start` |
| POST | `/api/fixture/effect/approve/finish` |
| POST | `/api/fixture/effect/dispatch` |
| POST | `/api/fixture/effect/reconcile` |
| POST | `/api/fixture/auth/logout` |

There are no API GETs, aliases, query actions, wildcard handlers, redirects,
or method overrides. Registration/login are the only pre-session routes; all
effect and logout routes require the live cookie and independent CSRF value.

## Durable coordinator and effect state

The side-effect-free verifier authenticates the capability, approval, policy,
invocation, and exact context before reservation. One redb write transaction
then rechecks the verified bindings against current durable state and commits
all of the following together:

```text
approval claim (retained through signed approval expiry)
capability/token claim
idempotency binding to random effect_intent_id
synthetic authority-node use/decrement
Prepared -> AuthorityReserved
```

The coordinator consumes `VerifiedCapabilityV2` and `VerifiedApprovalV1` by
value. In the transaction it matches their exact structured profile, policy,
intent, fixture generation, signer epoch, session/logout epoch, expiry, and
synthetic authority record. It neither repeats cryptographic verification nor
calls the legacy consuming validator. No public API accepts raw claim IDs, a
raw redb handle, recipient bytes, RFC 5322 bytes, or a generic writer.

Required states are:

```text
Prepared -----------> AuthorityReserved -----------> Dispatching
   |                         |                           |-> Succeeded
   `-> FailedBeforeDispatch  `-> FailedBeforeDispatch  `-> Indeterminate
```

Immediately before committing `Dispatching`, the same transaction rechecks
the live session/logout epoch, policy, prepared intent, capability, authority,
fixture generation, signer epoch, and every expiry/revocation generation. A
losing race may commit `FailedBeforeDispatch` only before `Dispatching` and
with a live proof that no payload byte reached the writer. Once `Dispatching`
has committed, reconciliation has only `Succeeded` or `Indeterminate` outcomes.

The coordinator durably commits `Dispatching` before touching the filesystem.
It publishes `<effect_intent_id>.eml` through an owner-only same-directory
temporary file, flush, no-replace publication, and directory flush. After a
crash, an identical final file may reconcile to `Succeeded`; absence,
difference, wrong type, unreadability, or uncertain durability is
`Indeterminate`. `Dispatching` and `Indeterminate` never retry, rewrite,
delete, fail over, or suggest that a send occurred.

Signed fixture evidence contains only version/type, random event ID, random
intent ID, closed outcome, prior-event hash, synthetic public signer identity,
event hash, and signature. It contains no recipient/content, path, size, exact
time, business identifier, or deterministic digest of a low-entropy value.

## Scope and product gates

The fixture accepts no product root, Workspace, Vault, import, migration,
business value, provider credential, attachment, SMTP setting, or arbitrary
path. Its redb data may contain fixed synthetic plaintext and is not a Vault
or product persistence boundary.

Product enablement remains blocked on all applicable Security Architecture
Program gates, including an independently admitted 1C0 owner, 1C1 custody,
1B1 recovery qualification, Program 1D `ActiveV2`, a reviewed protected
payload design, and the accepted effect profile. A mechanism-matrix pass cannot
promote the fixture into product authority.

## Implementation tasks

Each task uses genuine RED → minimal GREEN → focused gate → workspace
regression → independent review. Capture the RED diagnostic before
production code and make one reviewable commit per task.

### Task 0 — Governance gate (separate PR; blocks every code task)

Modify only RFC 0002 and its direct index/threat-model references. Add the
closed structured profile, exact fixture-only limits, one-transaction
reservation, state machine, value-free evidence, and non-claims above. Follow
the full RFC process; do not mark it Accepted from an implementation branch.

Gate:

```bash
git diff --check
rg -n 'low-risk-effectful|write_rfc5322|Indeterminate' \
  rfcs/0002-wasm-sandbox-and-plugin-capabilities.md
```

Stop until maintainers accept the RFC.

### Task 1 — Separate verification from consumption

Before creating any fixture transaction, refactor `CapabilityValidatorV2` so
its COSE, RFC 0003, trust, canonical-binding, policy, invocation, context, and
expiry checks are one side-effect-free path. That path returns the opaque
`VerifiedCapabilityV2` and `VerifiedApprovalV1` proof types described above.

RED tests and compile-fail fixtures must establish that:

- running pure verification repeatedly changes neither process-local replay
  state nor any attached/on-disk `AuthorityStore`;
- neither verified proof is cloneable, serializable, debuggable, publicly
  constructible, or field-constructible;
- the legacy pure-compute wrappers invoke the pure verifier and then preserve
  their current AuthorityStore operation order, errors, replay behavior,
  approval-retention behavior, and complete regression suite; and
- the dependency graph gains no authority-to-capability edge.

Do not duplicate verification and do not invert the existing
capability-to-authority dependency in this task. Record dependency inversion as
a separate follow-up plan.

Suggested commit: `refactor(capability): separate verification from consumption`.

### Task 2 — Establish the release-excluded single-process boundary

Create the upper `publish = false` fixture package, depending on capability
verification and authority primitives, plus the exact root classifier,
retained OS lock, sole redb open, fixed loopback listener, and boundary script.
Add no owner or effect behavior yet.

RED tests:

- product Cargo tree and release symbols contain no fixture, redb, or WebAuthn;
- a second real fixture process fails on the OS lock before attempting redb
  open; this is the only two-live-process claim in this slice;
- lock acquisition precedes the only production redb open;
- product/unmarked/symlink roots fail before listener or database state;
- source inventory contains one listener and no internal transport module; and
- Cargo metadata confirms the upper dependency seam and no dependency cycle.

Suggested commit: `test(fixture): establish single-process owner boundary`.

### Task 3 — Add unqualified WebAuthn UV, session, CSRF, and signer epoch

Add the exact registration/login route schemas, one-credential registry,
mechanism matrix, memory sessions, request middleware, and complete logout.
After the retained lock and before redb open, generate the ephemeral
`TypedSigner<ApprovalRole>`, random signer epoch, and closed `ApprovalBridge`;
persist only the labelled public trust record after redb opens. Do not add
effect publication.

RED tests cover first-writer-wins non-admission, required UV, exact origin/RP,
stored-credential allowlist, 300-second one-use ceremonies, cookie/CSRF
independence, absolute/idle expiry, cross-port cookie and credential DoS,
logout races, restart invalidation, destructive credential loss, ordering of
lock/signer/redb initialization, signer-epoch rotation, non-exportability, no
secret bytes in redb/backup/log output, and historical-key verify-only status.
The dependency profile must retain secret-key zeroization support.

Suggested commit: `feat(fixture): add unqualified owner uv sessions`.

### Task 4 — Seal the intent and bridge fresh UV to RFC 0003

Allocate the random intent before content access; compose fixed CRLF RFC 5322
bytes inside the coordinator; persist only the private sealed fixture payload;
and render a fixed synthetic preview. Successful fresh UV creates the
non-cloneable, non-serializable, non-debuggable `FreshUvGrant` with every exact
binding above. Only `ApprovalBridge::approve_invocation`, consuming that grant,
may produce RFC 0003 evidence. Validate the accepted RFC 0002 profile and
create exact Capability V2 claims.

RED tests cover byte/recipient substitution, header injection, intent
randomness, approval/capability binding, approval expiry, session-alone denial,
challenge or grant replay, credential/session/logout/signer-epoch mismatch,
restart invalidation, absence of signer getters or generic sign methods, no
automatic re-sign of old pre-dispatch work, no generic value input, and
compile-fail access to private proof, grant, payload, or constructor fields.

Suggested commit: `feat(fixture): bridge fresh uv to exact approval`.

### Task 5 — Reserve every authority fact in one redb transaction

Implement the sole coordinator write-transaction helper. It consumes the
opaque verified capability and approval proofs and atomically reserves their
claims, idempotency binding, synthetic authority node/use, and intent
transition. It returns the privately constructible `AuthorityReservedEffect`.
Never call the legacy consuming validator first and never duplicate its crypto
checks. Carry the already-fixed signed approval expiry into the redb claim; do
not add a second retention implementation.

RED tests cover every named failpoint between logical mutations with full
rollback, real process kill immediately before and after commit followed by
reopen, same-process concurrent HTTP requests/threads racing one intent with
one reservation winner, same-key idempotent replay, different-intent conflict,
expiry/revocation/logout/signer-epoch races, and reopen with either all or none
of the reservation. A second real process is tested only for pre-redb lock
denial. Compile-fail cases reject constructing, cloning, serializing,
debug-formatting, destructuring, or reconstructing `AuthorityReservedEffect`.
A full cross-process validator race remains Target and unqualified.

Suggested commit: `feat(fixture): reserve exact authority atomically`.

### Task 6 — Publish once and reconcile conservatively

Run the fixed import-free Core Wasm step, accept only the private
`AuthorityReservedEffect`, commit `Dispatching`, publish the exact local file,
reconcile without writing, and append value-free signed fixture evidence. Add
release-excluded named failpoints at every state and filesystem boundary.

RED tests cover absence of any alternate writer entry; compile-fail attempts to
reuse the handle after move or manufacture concurrent cloned handles; guest
imports; changed output; first/partial write; temp flush; publication;
directory flush; terminal commit; identical/different/absent reopen; restart
of old pre-dispatch work to `FailedBeforeDispatch`; restart of `Dispatching`
only to `Succeeded` or `Indeterminate`; no retry or automatic re-sign after
ambiguity; no recipient/content/digest in evidence; and a table/path-aware
synthetic canary allowlist.

Suggested commit: `feat(fixture): publish exact local outbox once`.

### Task 7 — Qualification and handoff

Run at least 25 real process-kill/reopen iterations per crash boundary and 100
iterations per same-process concurrent HTTP/thread reservation, logout, and
dispatch race. Run the virtual-browser matrix and each attended real mechanism
row separately. Publish a limitations/evidence note that cannot change product
status and explicitly leaves the full cross-process validator race Target.

Final gate:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
./scripts/check-file-size.sh
./scripts/check-synthetic-owner-effect-boundary.sh
cargo tree -p sovereign-cli --locked
cargo build -p sovereign-cli --release --locked
git diff --check
```

Also run the fixture-specific browser, crash, race, canary, compile-fail, and
exact-test manifests. Reject zero/skipped tests. Record exact toolchain,
platform, mechanism row, commands, results, and commit; keep all unsupported
rows Target.

## Exit criteria

The fixture is qualified only when:

1. pure verification has no replay/store side effect, its opaque proof objects
   cannot be forged or cloned through public APIs, and every legacy
   pure-compute authorization regression remains unchanged. Repeating
   independent pure verification is allowed; one-use is enforced only by the
   redb reservation transaction;
2. one process and one public listener own the complete synthetic path, while
   a second process is denied on the OS lock before redb open;
3. the ephemeral approval signer is memory-only and bridge-confined, and fresh
   UV yields a one-use exact-bound grant rather than treating session presence
   as approval;
4. one redb transaction consumes verified approval, token, idempotency,
   authority-node, and intent state; every partial failpoint rolls all of them
   back and a same-process concurrent race has one winner;
5. only the coordinator can construct `AuthorityReservedEffect`; it cannot be
   cloned, serialized, debugged, destructured, reconstructed, or reused after
   move, and no other writer entry exists;
6. exact bytes are sealed before approval and published at most once;
7. old pre-dispatch work never gains a new signature, and every ambiguous
   post-`Dispatching` observation is `Indeterminate` with no retry;
8. evidence is value-free and product release graphs exclude the fixture; and
9. documentation still calls it a synthetic, unqualified, plaintext fixture,
   leaves full cross-process validator qualification Target, and makes no
   email, owner-continuity, 1C0/1C1, Vault, or E2EE claim.
