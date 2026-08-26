> **Status: Superseded — do not execute.**
>
> This historical plan invented a custom HMAC/nonce/sequence IPC protocol,
> coupled the fixture to the product CLI, and attempted to accept its own
> effect RFC. Those choices conflict with the program-wide protocol and RFC
> governance rules. Use the
> [Synthetic Owner-UV and Exact Local-Outbox Fixture v2 plan](2026-08-14-synthetic-owner-exact-local-outbox-v2-implementation.md)
> instead; the body below is retained unchanged as design history.

# Synthetic Owner Session and Exact Local-Outbox Fixture v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a synthetic-data-only proof of the Program 1C0 WebAuthn/session/one-use-approval mechanism and Program 2 exact local `.eml` state machine without creating, importing, mutating, marking, or promoting a product workspace.

**Architecture:** One authority-owner broker process owns the only writable redb handle, all owner/session state, Capability V2 reservation, immutable synthetic recipient/RFC 5322 bytes, effect state, and the local-outbox write. The CLI and real subprocess test clients use authenticated local IPC and receive only opaque IDs, deliberately synthetic preview fields, and value-free outcomes; no raw payload accessor or callback crosses a crate boundary. Empty-registry WebAuthn enrollment is explicitly an unqualified fixture mechanism, not independent owner admission, and the product path remains blocked on Program 1B1, Program 1C1, Program 1D `ActiveV2`, and a later reviewed protected-payload persistence design.

**Tech Stack:** Rust 1.97; existing RFC 0003 COSE/Ed25519 and Capability V2 primitives; `webauthn-rs = =0.5.5` safe `Passkey` API; `redb = =4.1.0` behind one immediate/two-phase helper; HMAC-SHA-256 via `hmac = { version = "=0.12.1", default-features = false }`; same-image `sovereign` broker re-exec over bounded stdin/stdout bootstrap; `tiny_http`; zero-dependency browser assets; deterministic synthetic RFC 5322 composer; release-excluded fault injection.

## Global Constraints

- RFC 0003, RFC 0004, and RFC 0005 are normative. Task 1 must accept RFC 0006 as a fixture-proof contract before implementation begins; a conflict blocks later tasks.
- This whole slice is synthetic/fixture-only. The only recipient is `fixture-recipient@example.test`, and sender/subject/body are compile-time canaries named by RFC 0006. No venture/customer/document schema exists in the fixture. No route, command, IPC message, import, form, or library API accepts real business data or an existing product/Vault root.
- No new approval/authority/audit/IPC private key is persisted before Program 1D/1C1. Approval keys are one-object ephemeral; authority/audit keys and the parent-generated IPC key are per-broker-launch synthetic keys invalidated on restart. Persisted records contain only public keys/identities and `non_product=true`; no continuity claim is made.
- There is no product workspace-format marker, enrollment/setup CTA, migration, activation, support promotion, or “1C0/Program 2 complete” state in this plan. The dedicated directory marker is `synthetic-owner-effect-fixture-v1`; the distinct authority-root record tag is `synthetic_fixture_root_v1`. Both are synthetic-only and structurally rejected by product open/import/export code.
- Product use remains blocked until Program 1B1, Program 1C1, and Program 1D are complete with the workspace in `ActiveV2`, and a later RFC/review defines protected exact-payload persistence. Real-platform WebAuthn evidence can qualify only an authenticator mechanism matrix entry; it cannot remove any product gate.
- Empty-registry registration is first-valid-user-verification wins. It does **not** identify the intended owner against a hostile same-account native process. This plan invents no bootstrap anchor and makes no independent-admission, native-bootstrap-defeat, owner-continuity, recovery, or product-enrollment claim.
- `ProductOwnerAdmission` has no constructor or route in this slice. `UnqualifiedFixtureBootstrap` is available only behind the non-default `owner-effect-fixture` feature and explicit `security-fixture --synthetic-only` command. Its type cannot satisfy future product admission APIs.
- Every fixture constructor/module/IPC command is placed behind a crate-local non-default `owner-effect-fixture` feature from the task that first introduces it; dependency features are forwarded explicitly and only by the same named feature. Every Rust RED/GREEN command names either `--no-default-features` or the exact comma-separated feature set. The checked manifest `scripts/owner-effect-tests.tsv` plus `scripts/run-owner-effect-tests.sh` rejects an unknown/required-but-disabled feature, a missing target/name, a skipped target, `running 0 tests`, or a GREEN run that did not execute every registered fixture test. There is no interim commit where a default/release binary can call fixture registration, root, payload, reservation, broker entrypoint, or dispatch. Task 9 performs CLI/business/legacy isolation in the same checkpoint that first exposes the fixture CLI; Tasks 10-11 remove the old authority consumer/raw effect APIs; Task 14 only audits and consolidates barriers already active.
- The exact origin is the compile-time constant `http://localhost:7787`; RP ID is `localhost`; the socket binds only `127.0.0.1:7787`. The origin record is bound to the random fixture generation in the broker database and must equal the compiled constant on every reopen. Port occupation or mismatch returns `FixtureOriginUnavailable`; there is no random port, fallback, redirect, alternate hostname, or port-change API.
- A mechanism matrix entry is frozen before owner code: exact OS build, browser build, authenticator attachment/transport and resident/discoverable behavior observed, cookie behavior, and origin. Each entry must prove WebAuthn create/get plus acceptance and subsequent transmission of the exact Secure cookie at the exact origin. It must also characterize a malicious second-port discoverable assertion and same-user-handle credential creation where the platform exposes those behaviors. Failing Safari/WebKit or any build that cannot satisfy or characterize the required behavior is excluded; `Secure` is never removed.
- Cookies are host-wide and not port-isolated. Another localhost port may receive/overwrite/bomb the fixture cookie; the cookie is not claimed secret from other localhost services. Authorization still requires the exact origin and an independent in-memory CSRF value, so the accepted cookie residual is denial of service/session theft at the cookie layer without authorization. RP ID `localhost` is also port-insensitive: after user interaction a malicious second-port service may discover the fixture user handle, confuse credential selection, or create a same-RP/same-user-handle credential that replaces/locks out the fixture's only credential. Exact-origin verification prevents that credential from authorizing port 7787, but destructive credential DoS is accepted and explicitly not called owner admission, session authorization, or platform locality.
- The cookie is `__Host-sfo_fixture_session` with `Secure; HttpOnly; SameSite=Strict; Path=/` and no `Domain`. Session and CSRF values are independent random 256-bit values retained server-side only as SHA-256 digests. Sessions have 15-minute absolute and 5-minute idle lifetimes.
- WebAuthn browser option timeout and server ceremony expiry are both exactly 300 seconds. Before registration the broker creates and stores one random non-PII user UUID bound to the random fixture generation; the fixed synthetic name/display strings are `sfo-fixture` / `Sovereign Synthetic Fixture`. Registration uses that UUID, and login always supplies the one stored credential ID in `allowCredentials` and validates any returned user handle against the stored UUID; it never performs username-less fixture login. The fixture requests a platform attachment and requires UV, but its label is only `WebAuthnUvPlatformFixture(<frozen-matrix-entry>)`; it does not claim hardware backing, non-exportability, attestation, locality, or unsynced credentials. Roaming/hybrid classes are excluded unless separately frozen and qualified.
- The fixture admits exactly one credential. There is no add-second-credential, rotation, or recovery path. Loss is destructive: delete the synthetic fixture root and begin another unqualified fixture. This is forbidden product UX.
- `POST /api/fixture/auth/logout` requires exact Host/Origin/Fetch Metadata, live cookie, and live CSRF. It atomically revokes the server session, outstanding WebAuthn/approval challenges, unconsumed owner-approved objects, and narrowed/native handoff grants, then returns the exact cookie deletion and no-store headers.
- One broker process owns the sole writable `authority.redb` for its lifetime. Clients never open it. A second broker that loses the fixture process lock returns `BrokerAlreadyRunning`; a test-only direct second `redb::Database::open` that bypasses that lock must independently return the exact `DatabaseAlreadyOpen`. Neither path retries, waits, opens read-only, or creates a fresh store. Broker death closes IPC, invalidates memory-only sessions/grants, and fails pending work closed.
- There is no sibling broker artifact. A clean `sovereign-cli` build with `--no-default-features --features owner-effect-fixture` re-execs its own `current_exe()` in cfg-gated hidden mode `__owner-effect-broker`; the authority library exports only the narrow feature-gated broker main used by that match arm. The parent creates the random 256-bit launch key, writes one bounded bootstrap frame containing the key, launch nonce, canonical fixture root, and protocol version to the child's piped stdin, then closes stdin. The child writes only a bounded non-secret address/nonce/version frame to piped stdout, closes stdout, and must prove key possession in the first HMAC-authenticated IPC `BrokerReady` response before any other command. Stderr carries only fixed value-free diagnostics. Missing, malformed, oversized, or prematurely closed bootstrap input exits before binding, opening redb, or reading fixture state. A same-account native process can invoke the hidden mode and supply its own syntactically valid frame; hidden-mode naming and a caller-chosen pipe key are **not** an admission anchor. That process can win/deny the unqualified synthetic fixture but still cannot create a product admission claim; this is covered by the hostile-native fixture residual rather than contradicted by an impossible direct-invocation test. The default/release image has neither enum variant, match arm, authority broker entrypoint, nor broker symbols.
- Broker startup has one non-extendable supervisor-establishment deadline: `Instant::now() + Duration::from_secs(5)`, captured immediately before the non-secret address frame is emitted. Root classification and loopback bind may precede it, but acquiring `.owner-effect-broker.lock`, opening redb, reading fixture database state, or starting any command is forbidden until a complete launch-key-authenticated supervisor hello is accepted before that same monotonic deadline. Accepts, partial frames, failed authentication, wall-clock changes, or retries never reset/extend it. On deadline/EOF/auth failure, close listener/sockets, zeroize the launch key, and exit without ever taking the lock or opening redb. Once the hello authenticates, that exact socket becomes the supervisor liveness lease before lock/open begins; EOF during store startup or while emitting `BrokerReady` unwinds any acquired lock/redb immediately, and EOF after ready does the same through the normal lease path.
- Broker IPC is exactly a loopback-only `TcpListener` on an OS-assigned ephemeral `127.0.0.1` port, distinct from the immutable browser origin port. The resolved address is the only bootstrap stdout value and the launch key never enters argv/environment/stdout/stderr/disk. The first HMAC-authenticated connection is the sole supervisor control connection. Only it may send `RegisterConnectionV1`/`RevokeConnectionV1`; registration creates a random connection ID, independent random 256-bit connection key, exact closed command scope, absolute expiry bounded by broker lifetime, and sequence zero in broker memory. The supervisor sends that ID/key only through the corresponding child's bounded piped stdin; clients never receive the launch/control key or another client's credential. Each bounded canonical request carries its registered connection ID, strictly increasing per-connection sequence, random nonce, and HMAC-SHA-256. Unknown/revoked/expired ID, scope violation, cross-connection ID/key substitution, replay, gap, bad MAC, peer mismatch, oversize, timeout, or broker death fails closed. Supervisor control EOF—whether clean close or parent death—makes the broker revoke every connection, stop accepting work, close redb, release the process lock, and exit; it never survives as an orphan. The reviewed manifest profile is exactly `hmac 0.12.1` with default features disabled; RFC evidence records and the dependency contract freezes the resolved `sha2 0.10.9 -> digest 0.10.7 -> crypto-common 0.1.7` graph and fails on version/feature drift. MAC verification uses only `hmac::Mac::verify_slice`.
- `AuthorityStore::begin_immediate_two_phase()` is the only crate-private redb write-transaction constructor. It calls `Database::begin_write`, then `set_durability(redb::Durability::Immediate)` and `set_two_phase_commit(true)` before returning the transaction; the database field is private to `store.rs`, and source/instrumentation tests reject any other `begin_write` or modifying command/table path. Synthetic legacy-claim migration is initially admitted only on exact `x86_64-unknown-linux-gnu`: after the temporary database's final immediate/two-phase commit, all transaction/database handles are dropped, the Linux helper verifies same parent/regular files and uses `std::fs::hard_link(temp, absent_final)` as the exact atomic no-replace publication, fail-closed `File::open(parent).sync_all()`, temp unlink, and a second parent sync. The portable harness uses `Child::kill`/wait and claims process-crash atomicity only, not power-loss durability. Other OS/ABI/filesystem profiles return `FixtureMigrationUnavailable`; fresh synthetic fixtures remain separate.
- `effect_intent_id` is 128 random bits allocated by the broker before reading even synthetic recipient/body constants and before composition. It is never derived from a value, timestamp, path, or digest. The file name is exactly `<effect_intent_id>.eml`.
- The broker seals one exact normalized recipient, exact RFC 5322 bytes, operation `local_outbox.write_rfc5322`, policy/invocation binding, fixture root/generation, prepared expiry, and retention. Any change creates a new random intent and fresh preview/approval.
- A prepared intent expires 15 minutes after creation, additionally bounded by the fixture root. RFC 0003 evidence remains at most 600 seconds and is bounded by the live session/prepared intent/root/policy. The exact fixture Capability V2 lifetime is 60 seconds (within its existing 300-second maximum); reservation rechecks all clocks immediately.
- An unreserved expired `Prepared` payload is deleted transactionally and leaves a value-free non-dispatchable tombstone. Redb page reuse is not secure erasure, so prior synthetic bytes may remain in private transactional pages. Once reserved, exact bytes are retained for reconciliation until the entire synthetic fixture directory is destructively removed while the broker is stopped; no automatic purge, UI cleanup, revoke, or per-intent delete exists.
- RFC 0003/Capability V2 primary resource and canonical input bind the random intent ID, random fixture generation, fixed operation, and immutable coordinator reference only. They never contain recipient/content or an unkeyed digest of either, so approval/token/evidence cannot become a low-entropy dictionary oracle.
- Protected payload and local-outbox implementation are private modules in the coordinator crate. No public payload getter, closure, iterator, serialization, trait, permit constructor, or raw writer crosses a crate boundary. A high-level authenticated fixture-preview IPC response may contain only the fixed synthetic display projection. Runtime plaintext is expected only in: logical redb table `fixture_protected_payload_v1` and its private redb transactional/freed pages inside `authority.redb`; at most one owner-only same-directory `<effect_intent_id>.eml.tmp` after durable `Dispatching`; the ephemeral authenticated preview; and exact final `<effect_intent_id>.eml`. The table-aware/filesystem-aware scanner must prove expected copies exist for each state and that every other table, file, log, error, IPC/HTTP field, evidence record, and export is canary- and deterministic-digest-free; source/build artifacts containing compile-time fixture constants are outside the fresh runtime-root scan and are checked separately for absence of generic value paths.
- The existing authority store becomes the one transaction coordinator. Approval ID/its own expiry, token ID/expiry, idempotency key bound to random intent ID, fixture authority node/use, reservation, and `Prepared -> AuthorityReserved` commit together through `begin_immediate_two_phase()`.
- Reservation is not sufficient authority for later exposure. Before starting/restarting the harmless pure fixture and again inside the same immediate/two-phase transaction that would commit `AuthorityReserved -> Dispatching`, the broker revalidates trusted current time against prepared expiry, policy snapshot/epoch/expiry, root and every ancestor's lifetime/revocation, fixture/origin generation, a currently live session for the same registered fixture credential, and equality of the current logout epoch with the epoch captured at reservation. The original approval/session binding remains immutable evidence, but a broker restart may use a freshly logged-in session if no logout epoch advanced; root use is not decremented twice. A losing expiry/revocation/logout race atomically records the logical `Dispatching -> FailedBeforeDispatch` closure with the live no-I/O proof and never invokes the writer. Once `Dispatching` is durably committed, later expiry/logout cannot cancel, retry, or relabel the already-started effect; terminal reconciliation remains observation-only.
- Required states are `Prepared -> AuthorityReserved -> Dispatching -> {Succeeded, FailedBeforeDispatch, Indeterminate}`. `FailedBeforeDispatch` is allowed only in the live broker when it proves no payload byte was written/published/exposed and durably commits that result. After a process death, `Dispatching + absent` is always `Indeterminate`; absence is never proof of non-exposure.
- Entering `Dispatching` never causes automatic retry, failover, rewrite, deletion, or a new-send suggestion. Recovery maps exact identical final bytes to `Succeeded`; absent, different, wrong-type, unreadable, or uncertain durability to `Indeterminate`.
- Public signed effect evidence contains only version/type, random event ID, random intent ID, closed outcome, previous-event hash, synthetic signer public identity/public key, event hash, and signature. It contains no recipient/content or deterministic digest, business ID, path, byte count, time, account, reason, or policy value.
- `sovereign-effects` is an opaque façade/client only. The actual local-outbox write is coordinator-private. The sandbox exposes only feature-gated parameter-free `run_synthetic_exact_effect_fixture()`, which owns the fixed admitted artifact/input internally, calls its existing crate-private verified Wasmtime primitive, accepts no token/capability/payload/path/bytes, and returns one closed result; authority orders it after reservation without a second capability consumption. The only socket permission is the authority broker/client's bootstrapped-address loopback IPC; local-outbox/effect code has no arbitrary connect/listen API and neither crate gains DNS, SMTP, HTTP-client, OAuth, provider, credential, or non-loopback egress.
- Current legacy behavior remains runnable only under non-default `legacy-experimental` plus an explicit runtime flag and its existing warnings. Legacy cannot open a synthetic fixture root; the fixture cannot open/import a legacy/product root.
- Legacy compatibility is not an implementation or consumer of this new slice; it retains only the repository's pre-existing behavior for honesty/regression access. No owner, authority, payload, session, evidence, or qualification result from the fixture can authorize or upgrade legacy data.
- `fault-injection` is non-default, test-only, and forbidden from default/release dependency graphs and binaries. Named barriers may exist only under that feature. Every semantic fault test must first capture a genuine failing RED before the barrier/behavior is implemented.
- No Program 1C1 custody, Program 1D activation, Vault migration, backup/recovery, real business mutation, provider credential, SMTP, or network email work is in scope.
- Execute strict RED -> minimal GREEN -> focused gate -> full task gate -> review -> commit -> remote checkpoint. Each task records RED output in `.superpowers/task-<N>-red.txt`; no commit or push occurs with an unresolved Critical/High review finding.

---

## Honest security boundary

This plan proves mechanisms against deterministic synthetic values after an unqualified fixture credential exists. It does not prove who won empty-registry enrollment. A native process can synthesize loopback headers/client data, drive its own authenticator, and win the empty fixture registry; the test suite preserves this as an explicit non-claim. Exact Host/Origin/Fetch Metadata and CSRF protect browser requests after session establishment, not first-owner admission.

`CoreProtectedFixture` means payload bytes are unnameable outside private coordinator modules and omitted from logs/evidence/export. Redb is ACID/crash-safe, not encrypted or cryptographically authenticated, so even synthetic plaintext persistence cannot be generalized to founder data. Same-account arbitrary modification of the trusted process image, process-memory/FD capture, browser-profile compromise, root compromise, and disk tamper remain outside this mechanism proof.

The localhost cookie is host-wide. A malicious service on another port can receive or overwrite it if the browser visits that service. Exact-origin CSRF prevents that cookie alone from authorizing the broker; cross-port theft/overwrite remains a documented confidentiality/availability residual and blocks stronger session-secrecy language. The shared RP namespace separately permits user-prompt confusion and same-user-handle credential replacement/lockout; an exact-origin mismatch prevents authorization but cannot restore the single credential.

## Final fixture crate/process dependency model

Here `A -> B` means “A depends on B” for the dependency edges added or directionally changed by this fixture slice. Existing unrelated workspace edges used by offline verification or the explicitly retained legacy feature remain unless a task names their removal; this diagram does not pretend the whole current CLI manifest contains only two dependencies.

```text
sovereign-owner      -> identity + artifact + policy
sovereign-capability -> sovereign-owner + identity + artifact + policy
sovereign-authority  -> sovereign-owner + sovereign-capability + sovereign-sandbox + audit-ledger
sovereign-effects    -> sovereign-authority
sovereign-cli        -> sovereign-effects + sovereign-authority
```

Task 10 removes the current `sovereign-capability -> sovereign-authority` edge before adding `sovereign-authority -> sovereign-capability`; both directions never coexist. `sovereign-owner` defines a registry-backend trait implemented inside the authority broker, so owner never points back to authority. No stored RFC byte vector, payload handle, or generic byte-bearing interface leaves authority; only the independently public compile-time synthetic canaries may appear in the high-level authenticated preview. The CLI is not a database owner and never constructs authority/session/approval roots.

The mutually exclusive legacy build does not reverse this edge. Its CLI-only `LegacyAuthorityOrchestrator` calls capability's pure legacy verifier and then authority's typed `LegacyClaimCoordinator`; capability never owns/opens a store, authority accepts no raw claim IDs, and neither legacy type exists in default or fixture metadata/symbols.

## File responsibility map

| File | Responsibility |
| --- | --- |
| `rfcs/0006-synthetic-owner-session-exact-effect-fixture.md` | Normative fixture-only protocol, blocked product gates, origin/matrix, bootstrap non-claim, state encodings |
| `docs/security/owner-auth-mechanism-matrix.md` | Frozen exact real/virtual mechanism entries; never a product-support matrix |
| `crates/owner/` | WebAuthn ceremonies, memory sessions/CSRF/logout, opaque RFC 0003 approval, backend trait |
| `crates/authority/src/broker/` | Sole writable database owner, authenticated IPC, session/authority composition, fail-closed lifecycle |
| `crates/authority/src/broker/bootstrap.rs` | Same-`sovereign` stdin/stdout framing, fixed monotonic supervisor deadline, pre-auth no-store ordering, and first authenticated IPC readiness |
| `crates/authority/src/broker/connections.rs` | Supervisor-authenticated per-client key/scope/expiry/sequence registration and revocation |
| `crates/authority/src/broker/process_lock.rs` | Rust 1.97 OS-held exclusive fixture broker lock; crash releases the retained file handle |
| `crates/authority/src/broker/platform_publish.rs` | Exact gated same-parent no-replace migration publication and directory sync |
| `crates/authority/src/store.rs` | Redb schema and sole `begin_immediate_two_phase()` write helper |
| `crates/authority/src/forest.rs` | Synthetic-only root lifecycle, narrowing, revocation, generation binding |
| `crates/authority/src/exact_fixture.rs` | Pre-content ID, fixed synthetic composition, private immutable payload, safe preview projection |
| `crates/authority/src/fixture_policy.rs` | Broker-private fixed policy snapshot/epoch/expiry used by reservation and dispatch revalidation |
| `crates/authority/src/local_outbox.rs` | Private exact file publication and conservative reconciliation |
| `crates/capability/src/v2.rs` | Pure RFC 0003/Capability V2 validation that yields broker-consumable opaque proof |
| `crates/sandbox/src/synthetic_fixture.rs` | Parameter-free fixed admitted Wasm/input execution over the existing private verified primitive |
| `crates/effects/src/lib.rs` | Value-free IPC façade; no payload or raw write API |
| `crates/audit-ledger/src/effect_v1.rs` | Value-free signed projection format and verification |
| `apps/cli/src/fixture_http/` | Exact-origin HTTP security, auth/logout, synthetic preview/effect routes |
| `apps/cli/src/broker_client.rs` | Authenticated bounded IPC client; no database open |
| `apps/cli/src/workspace/legacy.rs` | Feature-gated unchanged legacy compatibility and warnings |
| `scripts/owner-auth-origin-preflight.sh` | Exact browser/origin/WebAuthn/Secure-cookie mechanism preflight |
| `scripts/owner-effect-tests.tsv` | Exact package/target/feature/test manifest used to reject skipped or zero-test gates |
| `scripts/run-owner-effect-tests.sh` | RED/GREEN runner that enforces the manifest, exact features, expected diagnostic, and nonzero execution |
| `scripts/run-owner-effect-regression.sh` | Supplemental Cargo-test wrapper that rejects missing, skipped, filtered, or zero-test runs |
| `scripts/run-authority-subprocess-claims.sh` | Exact 25-iteration Task 3 legacy subprocess characterization |
| `scripts/check-owner-effect-broker-build.sh` | Clean same-image fixture bootstrap plus default/release broker-exclusion gate |
| `scripts/check-owner-effect-profile-builds.sh` | Per-checkpoint clean default/fixture/legacy command and symbol separation |
| `scripts/check-owner-effect-authority-plane.sh` | Metadata/tree/public-symbol contract rejecting the removed direct claim plane |
| `scripts/check-owner-effect-crypto-profile.sh` | Exact HMAC/direct-transitive version, checksum, and feature-tree contract |
| `scripts/check-owner-effect-canaries.sh` | State-aware redb-table/filesystem/capture canary allowlist verifier |
| `scripts/exact-effect-kill-matrix.sh` | Broker/transaction/publication/evidence crash matrix |
| `scripts/check-owner-effect-boundary.sh` | Fixture/product/dependency/feature/raw-API/no-network contract |

## Per-task execution protocol

For every task: register only the named tests and their exact package/target/features in `scripts/owner-effect-tests.tsv`; run them through `scripts/run-owner-effect-tests.sh` with the literal Cargo command shown and retain the RED. The runner rejects disabled/unknown features, absent targets/names, zero/skipped/fully-filtered tests, and a RED whose output does not contain the task's expected diagnostic; GREEN must list and execute every manifest row for that task. No bare package/workspace Cargo invocation counts as fixture evidence: those commands are supplemental regression gates after the checked runner, while the runner must execute every registered test nonzero. Browser, subprocess, canary, boundary, and kill-matrix scripts likewise enumerate their required cases and fail on an empty/skipped case set. Implement the named minimum, run focused GREEN plus full task gate, run `git diff --check` and `./scripts/check-file-size.sh`, review the diff for RFC/security conformance, commit with the exact message, and push `feature/owner-session-exact-effect`. Record the remote commit in `.superpowers/task-<N>-report.md` before continuing.

## Task 1: Accept the fixture-only RFC and freeze the mechanism matrix/origin

**Files:**

- Create: `rfcs/0006-synthetic-owner-session-exact-effect-fixture.md`
- Create: `docs/security/owner-auth-mechanism-matrix.md`
- Create: `scripts/check-owner-effect-rfc.sh`
- Create: `scripts/owner-auth-origin-preflight.sh`
- Create: `scripts/owner-auth-origin-preflight.mjs`
- Create: `scripts/owner-effect-tests.tsv`
- Create: `scripts/run-owner-effect-tests.sh`
- Create: `scripts/run-owner-effect-regression.sh`
- Create: `scripts/tests/run-owner-effect-tests.sh`
- Create: `scripts/tests/run-owner-effect-regression.sh`
- Modify: `docs/INDEX.md`
- Modify: `ROADMAP.md`
- Modify: `THREAT_MODEL.md`

**Produces:** The immutable fixture contract, exact `FixtureOriginV1(http://localhost:7787, generation_id)`, frozen mechanism-entry schema, 300-second WebAuthn timeout, user-handle/cross-port residual, exact HTTP ceremony schemas, runtime-plaintext allowlist, unqualified-bootstrap statement, checked test-manifest contract, and blocked product gates consumed by all later tasks.

- [ ] **RED:** Create `scripts/check-owner-effect-rfc.sh` first. It must fail unless RFC 0006 states every Global Constraint, contains no product activation transition, and names 1B1 + 1C1 + 1D `ActiveV2` + later protected-payload review as conjunctive future gates. Run:

  ```bash
  ./scripts/check-owner-effect-rfc.sh
  ```

  Expected RED: `missing rfcs/0006-synthetic-owner-session-exact-effect-fixture.md`.

- [ ] Write RFC 0006 with canonical bounded records for fixture origin/generation, random user UUID/fixed synthetic WebAuthn names/stored credential ID, the four pre-session registration/login route schemas, credential/challenge/session/logout, same-image broker bootstrap, the exact five-second non-extendable monotonic supervisor-establishment deadline and pre-supervisor no-lock/no-redb startup state, supervisor-only authenticated `RegisterConnectionV1`/`RevokeConnectionV1`, per-client connection ID/key/scope/expiry/sequence state, supervisor-EOF shutdown before/during/after `BrokerReady`, RFC 0003 bridge, fixture authority root, intent/payload, reservation/revalidation, effect transitions, IPC envelope, exact redb helper/migration platform gate, runtime-plaintext allowlist, and value-free evidence. State explicitly that a hostile native valid-UV client may win empty-registry fixture enrollment, a same-account native process can invoke the hidden broker mode with its own valid bootstrap frame (the pipe key is connection authentication, not owner admission), another localhost service may cause user-handle credential replacement/lockout after user interaction, and no `ProductOwnerAdmission` constructor exists.
- [ ] Define the fixed synthetic corpus in the RFC, including `fixture-recipient@example.test`, fixed ASCII sender, fixed subject/body canaries, and the narrow ASCII addr-spec grammar. Reject every other recipient/content/business field before persistence.
- [ ] Implement the shell wrapper plus a zero-dependency Node `.mjs` HTTP/WebDriver harness. It binds only `127.0.0.1:7787`, serves embedded same-origin preflight JavaScript, sets the exact Secure cookie, verifies its later transmission, executes WebAuthn create/get, parses clientData challenge/origin, returned user handle, credential ID, and authenticator-data UV flag, and emits canonical value-free JSON with exact OS/browser/authenticator/origin identifiers plus a report digest labelled non-product. It must also start a malicious server on a second localhost port, demonstrate host-wide cookie receipt/overwrite/bombing, attempt a discoverable assertion, then attempt same-RP/same-user-handle credential creation where supported, record whether the original credential is replaced/confused, prove any resulting assertion is rejected at port 7787 for wrong origin, and prove cookie-alone requests fail without the origin-bound CSRF value. Virtual runs delete the virtual authenticator/cookies/profile automatically; attended real runs delete browser cookies/profile and require explicit operator-recorded platform credential-manager cleanup because WebAuthn has no RP credential-delete API. No production owner/session code or remote script is used by this preflight.
- [ ] Populate the matrix only with exact observed passing rows. Record resident/discoverable status, stored-credential allow-list behavior, second-port discovery/replacement result, and accepted destructive-DoS residual. Mark virtual rows `protocol_fixture_only`; mark real rows `mechanism_qualified_only`. Exclude failed or uncharacterized builds (including affected WebKit) without weakening `Secure`. An empty real matrix is allowed and leaves the mechanism unqualified.
- [ ] Implement `scripts/owner-effect-tests.tsv` with exact columns `task,package,target,profile,test_name` and the checked runner. `profile` is exactly `no-default-features`, `owner-effect-fixture`, `owner-effect-fixture,fault-injection`, `fault-injection`, or `legacy-experimental`. The runner owns no inferred defaults: it verifies the literal Cargo command contains the matching `--no-default-features`/`--features`, checks target/name presence before execution, rejects feature/target/package diagnostics and zero/skipped/fully-filtered output, requires the task-specific RED diagnostic on nonzero exit, and on GREEN first lists then executes every row. Repeated `--require-profile <exact-profile>` arguments make an `--all` run fail if the manifest omits or adds a profile, so the final command declares its complete feature set rather than inheriting one. Its shell self-test feeds success, zero-test, skipped-required-feature, wrong-diagnostic, filtered-output, and incomplete-profile-set transcripts and requires only the real nonzero run to pass.
- [ ] Implement `run-owner-effect-regression.sh -- <literal cargo test ...>` for every supplemental package/workspace test shown below. It rejects a missing/non-`cargo test` command, absence of `--no-default-features`, Cargo feature/target diagnostics, skipped required-feature targets, fully filtered output, or an aggregate zero executed tests; it preserves the underlying exit status and fails unless at least one named test ran. Its shell test feeds nonzero, zero, filtered, skipped-target, missing-feature-flag, and real-success transcripts and requires only the real nonzero run to pass. Thus no raw Cargo test command in a RED, GREEN, checkpoint, or final block can report a vacuous success.
- [ ] Update ROADMAP/THREAT_MODEL/INDEX to link the fixture RFC and matrix while leaving 1C0, Program 2, product owner admission, and protected product effects incomplete.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/check-owner-effect-rfc.sh
  ./scripts/owner-auth-origin-preflight.sh --virtual
  ./scripts/tests/run-owner-effect-tests.sh
  ./scripts/tests/run-owner-effect-regression.sh
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `docs(rfc): constrain owner exact effect to synthetic fixtures`
- [ ] Push checkpoint 1. No production implementation task starts before RFC/security acceptance.

## Task 2: Fix RFC 0003 approval retention before coordinator work

> **VERIFIED ALREADY LANDED (2026-08-26).** This task's behavior and all three
> exact test names shipped ahead of this plan (see the vault missing-key round,
> backlog run log 2026-08-15). No code change was needed. The `run-owner-effect`
> runners named below do not exist until Task 1 lands, so verification used
> plain `cargo test`; the runner-wrapped commands become the standing gate once
> Task 1 is done. Task 4 (redb migration of these legacy claims) still consumes
> this. Checkboxes are ticked to reflect the verified state, not a fresh
> implementation.

**Files:**

- Modify: `crates/capability/src/v2.rs`
- Modify: `crates/authority/src/lib.rs`
- Modify: `crates/capability/tests/approval_v2.rs`
- Modify: `scripts/owner-effect-tests.tsv`

**Consumes:** RFC 0003's independent approval expiry. **Produces:** Correct legacy claim semantics that Task 4 migrates.

- [x] **RED:** Add `durable_approval_survives_token_expiry_purge_until_approval_expiry` and `expired_approval_purges_at_approval_expiry`. Use approval expiry `t+120`, token expiry `t+30`, purge/reopen at `t+31`, and attempt reuse with a second token. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 2 --phase red \
    --expected-diagnostic 'approval replay incorrectly reopened after token-expiry purge' -- \
    cargo test -p sovereign-capability --test approval_v2 \
      durable_approval_survives_token_expiry_purge_until_approval_expiry \
      --no-default-features --locked -- --exact
  ```

  Expected RED: reuse succeeds because current storage purges the approval at token expiry.

- [x] Carry `(approval_id, approval_expires_at_unix)` from full RFC 0003 verification into `AuthorityStore::claim_approval`; keep token/idempotency expiries independent. Add authority test `purge_uses_each_claim_kind_expiry`.
- [x] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 2 --phase green -- \
    cargo test -p sovereign-capability --test approval_v2 --no-default-features --locked
  ./scripts/run-owner-effect-tests.sh --task 2 --phase green -- \
    cargo test -p sovereign-authority --lib --no-default-features --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [x] Commit: `fix(authority): retain approvals through their own expiry`
- [x] Push checkpoint 2.

## Task 3: Characterize real subprocess claims with exact release-excluded barriers

**Files:**

- Modify: `crates/authority/Cargo.toml`
- Modify: `crates/authority/src/lib.rs`
- Create: `crates/authority/src/fault_injection.rs`
- Create: `crates/authority/tests/subprocess_claims.rs`
- Create: `scripts/check-fault-injection-excluded.sh`
- Create: `scripts/run-authority-subprocess-claims.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** A real-process parity oracle for current filesystem claims and a named fault mechanism that cannot enter default/release artifacts. This task intentionally precedes redb/broker replacement.

- [ ] **RED 1 — harness:** Add parent tests `real_subprocess_token_claim_has_one_winner`, `real_subprocess_approval_survives_restart`, `real_subprocess_idempotency_distinguishes_replay_and_conflict`, and `real_subprocess_mixed_claims_record_current_partial_consumption` before implementing child mode. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 3 --phase red \
    --expected-diagnostic 'unknown child protocol' -- \
    cargo test -p sovereign-authority --test subprocess_claims \
      --no-default-features --features fault-injection --locked -- --test-threads=1
  ```

  Expected genuine RED: re-executed child rejects the unknown child protocol and the parent receives no valid framed result.

- [ ] Implement only the test-binary re-exec protocol, bounded barrier wait, child result framing, and cleanup. Run the focused suite; the mixed case records the current token-only partial state after a kill and passes as characterization.
- [ ] **RED 2 — semantic crash:** Add `kill_after_legacy_temp_sync_before_publish_exposes_no_partial_record`, requesting named barrier `LegacyAfterTempSyncBeforePublish`, before adding the library barrier. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 3 --phase red \
    --expected-diagnostic 'unknown fault barrier LegacyAfterTempSyncBeforePublish' -- \
    cargo test -p sovereign-authority --test subprocess_claims \
      kill_after_legacy_temp_sync_before_publish_exposes_no_partial_record \
      --no-default-features --features fault-injection --locked -- --exact
  ```

  Expected genuine RED: child reports `unknown fault barrier LegacyAfterTempSyncBeforePublish` and never reaches the parent release point.

- [ ] Add the smallest `#[cfg(feature = "fault-injection")]` internal hook exactly after temp `sync_all` and before publication. No polling approximation and no default code path reads an environment variable. The test kills at the barrier and proves restart sees either no record or one complete record, never truncated/partial JSON.
- [ ] Make `fault-injection` non-default and non-forwarded by CLI/owner/effects features. The exclusion script builds default release artifacts, inspects `cargo tree -e features` and binary symbols, and fails on the feature/barrier names.
- [ ] Implement `scripts/run-authority-subprocess-claims.sh` as the exact bounded repetition gate. `--iterations` must be an integer `1..100`; each iteration invokes the manifest-checked `subprocess_claims` target with `--no-default-features --features fault-injection -- --test-threads=1`, requires every Task 3 test once, and exits on the first zero/skipped/failing iteration. **GREEN:** Run exactly:

  ```bash
  ./scripts/run-authority-subprocess-claims.sh --iterations 25 --features fault-injection
  ./scripts/check-fault-injection-excluded.sh
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `test(authority): characterize subprocess claims with exact faults`
- [ ] Push checkpoint 3. Task 4 rewrites contention tests to broker IPC; it does not preserve direct multi-open assumptions.

## Task 4: Make one broker the sole writable redb owner

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/authority/Cargo.toml`
- Modify: `crates/authority/src/lib.rs`
- Create: `crates/authority/src/store.rs`
- Create: `crates/authority/src/format.rs`
- Create: `crates/authority/src/claims.rs`
- Create: `crates/authority/src/broker/mod.rs`
- Create: `crates/authority/src/broker/bootstrap.rs`
- Create: `crates/authority/src/broker/connections.rs`
- Create: `crates/authority/src/broker/ipc.rs`
- Create: `crates/authority/src/broker/process_lock.rs`
- Create: `crates/authority/src/broker/platform_publish.rs`
- Create: `crates/authority/src/broker/protocol.rs`
- Create: `crates/authority/tests/broker_ownership.rs`
- Create: `crates/authority/tests/migration.rs`
- Create: `apps/cli/src/broker_client.rs`
- Modify: `apps/cli/Cargo.toml`
- Modify: `apps/cli/src/main.rs`
- Create: `apps/cli/tests/broker_bootstrap.rs`
- Create: `scripts/check-owner-effect-broker-build.sh`
- Create: `scripts/check-owner-effect-crypto-profile.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** `AuthorityOwnerBroker` as the only writable `authority.redb` owner, same-image feature-gated broker bootstrap with a fixed monotonic pre-supervisor deadline, `BrokerClient` as the only client path, sole `begin_immediate_two_phase()` write helper, and an initially Linux-x86_64-only synthetic migration gate. No public client opens a database and no sibling executable is required or packaged.

- [ ] Add test/build scaffolding before the RED without broker behavior: `default = []` and non-default `owner-effect-fixture` in authority and CLI, explicit `sovereign-cli/owner-effect-fixture -> sovereign-authority/owner-effect-fixture`, cfg-gated hidden enum/match declarations that still call a missing broker main, and matching `required-features = ["owner-effect-fixture"]` on fixture integration targets. Register every test in the manifest; default compilation must not see the hidden variant.
- [ ] Pin `redb = "=4.1.0"` and `hmac = { version = "=0.12.1", default-features = false }`; reuse locked `sha2 0.10.9`. `scripts/check-owner-effect-crypto-profile.sh` fails unless the manifest, lock checksums, and `cargo tree -p sovereign-authority --no-default-features --features owner-effect-fixture -e features` match RFC 0006's reviewed `hmac 0.12.1 / sha2 0.10.9 / digest 0.10.7 / crypto-common 0.1.7` graph exactly. Redb is ACID, not authenticity/encryption.
- [ ] **RED:** Add exact tests `clean_fixture_binary_reexecs_broker_from_same_image`, `default_binary_has_no_hidden_broker_mode_or_symbols`, `direct_hidden_broker_without_valid_stdin_exits_before_open`, `direct_valid_bootstrap_is_unqualified_native_fixture_control_not_product_admission`, `direct_valid_bootstrap_rejects_product_unmarked_or_legacy_root_before_open`, `bootstrap_key_uses_only_bounded_stdin_and_never_stdout_argv_env_or_disk`, `first_ipc_ready_requires_launch_key`, `supervisor_establishment_deadline_is_fixed_monotonic_five_seconds_and_never_extended`, `parent_death_after_address_before_supervisor_auth_exits_without_lock_or_redb_open`, `parent_death_during_broker_ready_unwinds_any_lock_and_redb`, `parent_death_after_broker_ready_exits_and_releases_lock`, `only_supervisor_can_register_or_revoke_connection`, `each_client_gets_independent_random_256_bit_key_id_scope_expiry_and_sequence`, `child_receives_only_its_own_connection_credential_on_stdin`, `cross_connection_id_key_or_scope_use_fails_closed`, `expired_revoked_replayed_or_gapped_connection_fails_closed`, `supervisor_eof_or_parent_death_exits_broker_and_releases_lock`, `second_broker_returns_broker_already_running_without_retry`, `broker_process_lock_releases_on_death_without_stale_claim`, `direct_second_redb_open_returns_database_already_open`, `client_cannot_open_database`, `real_subprocesses_race_authenticated_ipc_not_redb`, `bad_mac_nonce_sequence_or_peer_fails_closed`, `broker_death_fails_pending_request_closed`, `broker_restart_invalidates_connections_and_connection_credentials`, `every_modifying_command_uses_immediate_two_phase_helper`, `corrupt_or_unknown_database_fails_without_fresh_create`, `temporary_migration_handle_is_closed_before_publish`, `linux_migration_kills_reopen_one_complete_generation`, and `unqualified_platform_migration_fails_unavailable`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 4 --phase red \
    --expected-diagnostic 'AuthorityOwnerBroker' -- \
    cargo test -p sovereign-authority --test broker_ownership \
      --no-default-features --features owner-effect-fixture --locked -- --test-threads=1
  ./scripts/run-owner-effect-tests.sh --task 4 --phase red \
    --expected-diagnostic 'begin_immediate_two_phase' -- \
    cargo test -p sovereign-authority --test migration \
      --no-default-features --features owner-effect-fixture,fault-injection \
      --locked -- --test-threads=1
  ./scripts/run-owner-effect-tests.sh --task 4 --phase red \
    --expected-diagnostic '__owner-effect-broker' -- \
    cargo test -p sovereign-cli --test broker_bootstrap \
      --no-default-features --features owner-effect-fixture --locked -- --test-threads=1
  ```

  Expected RED: the feature graph and test targets exist, but the same-image broker main, bootstrap, redb helper, and IPC protocol do not.

- [ ] In CLI, re-exec `std::env::current_exe()` with hidden cfg-gated `__owner-effect-broker` using exactly `.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())`, mirroring the repository's existing compile-worker composition. Parent generates key/nonce, writes only stdin, concurrently drains a bounded stream of fixed value-free diagnostic codes from stderr, and treats overflow/malformed text as failure. Authority's feature-only `run_owner_effect_fixture_broker(reader, writer, diagnostics)` accepts one bounded canonical bootstrap frame. After safe root classification and loopback bind, it captures `supervisor_establishment_deadline = Instant::now().checked_add(Duration::from_secs(5))` immediately before writing the bounded non-secret address/nonce/version frame, then closes bootstrap stdin/stdout. The same absolute monotonic deadline bounds accept plus every partial read of the launch-key-authenticated supervisor hello; no attempt resets it. Missing/malformed/oversized/early-EOF bootstrap input exits before root validation, bind, lock, or redb open; supervisor timeout/EOF/bad MAC exits after bind but before lock/redb. A syntactically valid frame supplied by a direct same-account caller remains deliberately indistinguishable from the parent and is tested/documented as unqualified fixture control/DoS, never product admission. Make no executable-provenance claim and use no extra inherited FD/HANDLE or sibling binary.
- [ ] Before bind or supervisor authentication, the broker itself canonicalizes the root and independently requires the dedicated `synthetic-owner-effect-fixture-v1` marker/generation plus the RFC-frozen private-directory/ancestor allowlist; it rejects symlinks, unknown files, nesting within or containing any Workspace/Vault/product/legacy/ActiveV2 marker, and any existing non-fixture database. This validation is not delegated to the CLI, so a direct valid hidden-mode frame cannot point the broker at business data. Bind one ephemeral `127.0.0.1` IPC listener (never `0.0.0.0`/`::`) without opening `.owner-effect-broker.lock` or redb, publish the address, and establish the authenticated supervisor lease by the fixed deadline. Only then may `process_lock.rs` open the fixed owner-only regular lock file with read+write access and call Rust 1.97's exact nonblocking `std::fs::File::try_lock()`; only after that succeeds may `AuthorityStore` open redb. `TryLockError::WouldBlock` maps only to `BrokerAlreadyRunning`, every other open/lock error fails closed, and the broker retains the sole un-cloned/uninherited lock `File`. Check the retained supervisor socket for EOF/error before and after lock acquisition, after redb open, and while writing the authenticated `BrokerReady`; any loss drops whatever was acquired and exits. Send `BrokerReady` only once the store is ready. A deliberately process-lock-bypassing direct redb open in the test harness must separately observe `redb::DatabaseError::DatabaseAlreadyOpen`. Use no lock/open sleep, retry, read-only, or fresh-store fallback. The IPC port is not an origin/config fallback and is never used by the browser.
- [ ] From the moment its hello authenticates—before process lock/redb open—retain that exact socket as the unique supervisor control/liveness connection. Do not accept any business command until authenticated `BrokerReady` completes. Then accept canonical `RegisterConnectionV1 { request_id, scope, expires_at_unix }` only under the launch key; broker generates—not caller supplies—an independent 256-bit key and random connection ID, clips expiry to broker lifetime, stores `{key, exact closed scope, expiry, next_sequence=0, revoked=false}` only in `connections.rs`, and returns the credential once. `RevokeConnectionV1` is likewise supervisor-only and zeroizes/removes the entry. The supervisor writes exactly one returned credential to exactly the intended child's bounded piped stdin, closes it, and never puts any connection credential in argv/environment/stdout/stderr/disk or another child's frame.
- [ ] Implement client frames `{version, connection_id, sequence, nonce, command}` with distinct request/response HMAC domains, strict per-connection sequence, nonce replay set, loopback peer check, exact registered scope, expiry, deadlines, and generic value-free errors. Verify MACs only with `hmac::Mac::verify_slice`; never hand-roll comparison or reuse launch/browser/session/other-client keys. A credential cannot register another connection, widen itself, act under another ID, or survive revoke/restart. Real subprocess children receive only their distinct credential through parent-written stdin and race actual IPC commands.
- [ ] Treat the startup deadline followed by the authenticated supervisor socket as one continuous broker liveness rule: before supervisor authentication, expiry at the fixed five-second `Instant` closes the listener/partial socket and proves lock/redb open counters stayed zero; from authentication onward, supervisor EOF/error—including while lock/redb are being opened or `BrokerReady` is being written—unwinds startup or atomically stops new commands, revokes/zeroizes all in-memory connections, drops sessions/grants, closes redb and the retained process-lock handle, and exits nonzero. No child connection or repeated partial hello can keep it alive. Tests kill the parent (a) after address publication but before authentication, (b) at barriers during authenticated `BrokerReady`, and (c) after ready; each waits at most the frozen five-second deadline plus one second of scheduler allowance, then starts a new same-image broker and proves lock acquisition succeeds and every old credential, if any, is rejected.
- [ ] Make `AuthorityStore::begin_immediate_two_phase()` the sole constructor of a write transaction and keep `Database` private in `store.rs`. The helper calls `begin_write`, `set_durability(Durability::Immediate)`, and `set_two_phase_commit(true)` before mutation. A cfg-test observer records helper ID plus touched table IDs; broker command coverage asserts every schema/claim/session/root/reservation/effect mutation used it. The boundary script rejects `begin_write(` outside that helper.
- [ ] Migrate only synthetic test-generated legacy claim fixtures and only when `SyntheticMigrationPlatformV1::current()` is exact `cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))`. Freeze old claim writers, validate all input, import through the sole helper, commit, drop write transaction and every temporary `Database`, reopen/validate then drop. `platform_publish.rs` verifies the temp/final share one canonical parent and the temp is a regular non-symlink, requires final absence, calls `std::fs::hard_link(temp, final)` (an `AlreadyExists` race fails), opens and `sync_all()`s the parent with errors propagated, unlinks the temp alias, and syncs the parent again. Add named test-only barriers after import commit, handle drop, hard-link publish, and each parent sync. Use `Child::kill` plus wait in tests. Reopen must see old complete or new complete generation; this qualifies process-crash behavior only. Other targets return `FixtureMigrationUnavailable`. Never import a product/Vault root; unknown legacy approval expiry becomes `NeverPurge`.
- [ ] Broker death closes all channels and invalidates connection credentials. Restart may reopen committed claim state but has no automatic resume/dispatch command; after Task 5, explicit inspect/reconcile additionally requires a newly authenticated fixture login.
- [ ] `scripts/check-owner-effect-broker-build.sh` allocates a new temporary `CARGO_TARGET_DIR`, runs `cargo build -p sovereign-cli --bin sovereign --no-default-features --features owner-effect-fixture --locked`, asserts the exact resulting `<target>/debug/sovereign[.exe]` exists and no sibling broker artifact exists, then runs `CARGO_TARGET_DIR=<same-target> cargo test -p sovereign-cli --test broker_bootstrap --no-default-features --features owner-effect-fixture --locked -- --test-threads=1`. The integration target uses Cargo's compile-time `env!("CARGO_BIN_EXE_sovereign")`, asserts that path canonicalizes to the just-built same-target image, executes it, authenticates same-image re-exec, and exercises parent death before/during/after `BrokerReady`; pre-auth death must hit the fixed deadline with zero lock/redb opens, and later deaths must permit immediate clean restart. The script rejects zero/skipped tests. It separately clean-builds `--no-default-features --release`, checks help/symbols for absence of the hidden mode/broker entrypoint, and never relies on a prior workspace/all-target build. Task 9 extends this gate through the user-facing synthetic fixture command once that command exists.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 4 --phase green -- \
    cargo test -p sovereign-authority --test broker_ownership \
      --no-default-features --features owner-effect-fixture --locked -- --test-threads=1
  ./scripts/run-owner-effect-tests.sh --task 4 --phase green -- \
    cargo test -p sovereign-authority --test migration \
      --no-default-features --features owner-effect-fixture,fault-injection \
      --locked -- --test-threads=1
  ./scripts/run-owner-effect-tests.sh --task 4 --phase green -- \
    cargo test -p sovereign-cli --test broker_bootstrap \
      --no-default-features --features owner-effect-fixture --locked -- --test-threads=1
  ./scripts/check-owner-effect-broker-build.sh
  ./scripts/check-owner-effect-crypto-profile.sh
  ./scripts/check-fault-injection-excluded.sh
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  cargo audit
  ```

- [ ] Commit: `refactor(authority): own redb through one authenticated broker`
- [ ] Push checkpoint 4.

## Task 5: Add the unqualified WebAuthn fixture session and complete logout

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `crates/owner/Cargo.toml`
- Create: `crates/owner/src/lib.rs`
- Create: `crates/owner/src/config.rs`
- Create: `crates/owner/src/registry.rs`
- Create: `crates/owner/src/ceremony.rs`
- Create: `crates/owner/src/session.rs`
- Create: `crates/owner/tests/fixture_session.rs`
- Modify: `crates/authority/Cargo.toml`
- Modify: `crates/authority/src/broker/mod.rs`
- Modify: `crates/authority/src/broker/protocol.rs`
- Modify: `crates/authority/src/store.rs`
- Create: `crates/authority/src/broker/owner_backend.rs`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** `OwnerService<B: OwnerRegistryBackend>`, opaque in-broker `OwnerSessionGrant`, 300-second ceremonies, fixture-only bootstrap type, and atomic logout. Owner has no authority/capability/effects/CLI dependency; authority implements its backend and hosts it in the sole broker.

- [ ] Create `sovereign-owner` with `default = []` and non-default `owner-effect-fixture`; gate every fixture type/module and set its fixture integration target's `required-features` accordingly. Extend only `sovereign-authority/owner-effect-fixture` to forward the owner feature; no default or CLI direct owner dependency is added. Pin `webauthn-rs = "=0.5.5"`; use safe `Passkey` APIs only. Reject direct `webauthn-rs-core`, `danger-credential-internals`, and `danger-user-presence-only-security-keys`. Constructor requires compiled origin `http://localhost:7787`, RP ID `localhost`, a frozen mechanism-entry ID (or explicit `virtual_fixture_only`), and exact 300-second timeout/expiry.
- [ ] Put the deterministic authenticator adapter behind `cfg(test)` as `VirtualWebAuthnUvPlatformFixture`; its registry/matrix tag is rejected by every real-mechanism constructor and its reports can never become a real matrix row.
- [ ] **RED:** Add exact tests `product_owner_admission_has_no_constructor`, `product_registration_api_is_unavailable`, `fixture_bootstrap_is_typed_unqualified`, `hostile_native_valid_uv_can_win_empty_fixture_registry`, `fixture_report_never_calls_winner_intended_owner`, `registration_and_login_require_uv`, `ceremony_expires_at_300_seconds_and_is_one_use`, `user_uuid_is_random_non_pii_and_generation_bound`, `registration_uses_fixed_synthetic_names`, `second_registration_start_and_finish_are_rejected_after_first_credential`, `login_allow_credentials_is_exactly_the_stored_credential_id`, `returned_user_handle_must_equal_stored_uuid`, `second_port_same_handle_replacement_is_destructive_dos_not_authorization`, `wrong_origin_rp_credential_or_counter_fails`, `session_rotates_random_cookie_and_csrf`, `session_absolute_and_idle_expiry_are_enforced`, `restart_invalidates_memory_sessions`, `one_credential_loss_requires_fixture_deletion`, and `roaming_or_hybrid_is_rejected_without_matrix_entry`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 5 --phase red \
    --expected-diagnostic 'OwnerService' -- \
    cargo test -p sovereign-owner --test fixture_session \
      --no-default-features --features owner-effect-fixture --locked
  ```

  Expected RED: owner crate, fixture bootstrap type, and session service do not exist.

- [ ] Broker initialization allocates one `Uuid::new_v4()` user handle before registration and persists it with fixture generation only in private logical table `fixture_owner_registry_v1` through the sole write helper; it is never sent by an application API except as required inside WebAuthn options/assertions and never logged. `start_fixture_registration(UnqualifiedFixtureBootstrap)` is compiled only with `owner-effect-fixture`; under the same backend serialization used by finish it first proves the registry is empty, otherwise returns a closed `FixtureCredentialAlreadyRegistered` without creating a challenge. It passes that UUID plus exact names `sfo-fixture` / `Sovereign Synthetic Fixture` to the safe library API. Finish rechecks emptiness, validates exact library ceremony state/UV/origin/RP/user-handle/matrix entry, and atomically stores the first passkey and exact credential ID through the broker backend; every losing concurrent finish is consumed/rejected and can never replace the stored credential. In a synchronized hostile-vs-intended fixture race, the first valid assertion wins and the result is permanently labelled `unqualified_first_writer`; do not turn this expected result into a security pass.
- [ ] Login start uses only the registered `Passkey` and requires generated `allowCredentials` to contain exactly its stored credential ID—never empty/discoverable username-less login. Login finish rejects a returned user handle different from the stored UUID. Simulated/real malicious-port same-handle replacement may make the original credential unusable, but a wrong-origin assertion can never update the registry or issue a session; report that result only as destructive DoS.
- [ ] Login finish transactionally updates passkey counter/backup flags and issues independent 256-bit cookie/CSRF values; retain only constant-time-comparable SHA-256 digests with issued/last-seen/absolute/idle expiry and random session generation. Raw values are returned once over the authenticated broker channel and never logged/persisted.
- [ ] Every persistent registration, passkey-counter/backup-state, session tombstone, logout epoch, and challenge/grant cleanup mutation uses the broker backend's typed method and `begin_immediate_two_phase()`; add each command/table to Task 4's instrumentation coverage. Memory-session insertion occurs only after the persistent portion commits.
- [ ] Implement `OwnerService::logout(SessionPresentation) -> LogoutReceipt`. It validates live cookie plus CSRF, then through `begin_immediate_two_phase()` increments the fixture's monotonic logout epoch and commits the session-generation revocation tombstone that every challenge/approval/handoff and later pre-Dispatching check revalidates; the same command removes persisted unconsumed objects/grants and drops memory challenges/session before acknowledgement. Already reserved effects are retained for evidence but their captured epoch mismatch forces live no-I/O `FailedBeforeDispatch`; an already durable `Dispatching` effect is never canceled. The tombstone/epoch make logical revocation atomic even if cleanup is interrupted. Concurrent logout/fixture authorization has one transaction-order winner.
- [ ] Add exact logout tests `logout_requires_live_cookie_and_csrf`, `old_cookie_and_csrf_fail_after_logout`, `concurrent_logout_and_fixture_action_have_one_winner`, `logout_revokes_pending_approval_and_native_grant`, `logout_cookie_deletion_is_exact`, and `csrf_is_reusable_only_within_same_live_session`. Cookie deletion is `__Host-sfo_fixture_session=; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; Path=/; Secure; HttpOnly; SameSite=Strict`, no Domain, and no-store headers.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 5 --phase green -- \
    cargo test -p sovereign-owner --test fixture_session \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-owner --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --features owner-effect-fixture --locked
  cargo tree -p sovereign-owner --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  cargo audit
  ```

- [ ] Commit: `feat(owner): prove unqualified webauthn fixture sessions and logout`
- [ ] Push checkpoint 5.

## Task 6: Bridge fresh fixture UV to opaque RFC 0003 evidence

**Files:**

- Modify: `Cargo.lock`
- Modify: `crates/owner/Cargo.toml`
- Create: `crates/owner/src/approval.rs`
- Modify: `crates/owner/src/lib.rs`
- Modify: `crates/owner/src/ceremony.rs`
- Modify: `crates/owner/src/registry.rs`
- Modify: `crates/capability/Cargo.toml`
- Modify: `crates/capability/src/approval.rs`
- Modify: `crates/capability/src/lib.rs`
- Modify: `crates/policy/src/lib.rs`
- Modify: `crates/authority/src/store.rs`
- Modify: `crates/authority/src/broker/owner_backend.rs`
- Modify: `crates/authority/src/broker/protocol.rs`
- Modify: `crates/authority/src/broker/mod.rs`
- Create: `crates/owner/tests/exact_approval.rs`
- Create: `crates/owner/tests/compile_fail.rs`
- Create: `crates/owner/tests/ui/owner_approval_is_opaque.rs`
- Create: `crates/owner/tests/ui/owner_approval_is_opaque.stderr`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** `OwnerApprovedInvocation`, constructible only by a fresh fixture UV ceremony and fully re-verifiable through the broker registry. Canonical RFC 0003 bytes remain unchanged.

- [ ] **RED:** Add `session_alone_cannot_approve`, `fresh_uv_binds_intent_invocation_policy_session_and_fixture_generation`, `changed_binding_fails`, `approval_challenge_expires_at_300_seconds_and_is_one_use`, `concurrent_finish_has_one_winner`, `logout_invalidates_unconsumed_approval`, `broker_restart_invalidates_unconsumed_approval_even_if_signature_verifies_offline`, `ephemeral_private_key_is_not_recoverable`, and compile-fail `owner_approval_is_opaque`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 6 --phase red \
    --expected-diagnostic 'OwnerApprovedInvocation' -- \
    cargo test -p sovereign-owner --test exact_approval \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 6 --phase red \
    --expected-diagnostic 'owner_approval_is_opaque' -- \
    cargo test -p sovereign-owner --test compile_fail \
      --no-default-features --features owner-effect-fixture --locked \
      owner_approval_is_opaque -- --exact
  ```

  Expected RED: no exact approval ceremony/opaque type exists.

- [ ] Move the existing RFC 0003 canonical sign/verify implementation behind owner, preserving golden COSE/JCS bytes and negative vectors. Move the shared policy-decision digest helper to policy. Owner defines the RFC wire tool scope locally and never imports capability.
- [ ] Add `trybuild` as an owner dev-dependency only and lock it. Compile-fail stderr is accepted only when the error proves private construction/fields, not an unrelated unresolved import. It must not enter a default release graph.
- [ ] Start approval only from a live session after a synthetic preview. Generate a unique ephemeral `TypedSigner<ApprovalRole>` inside the one-use challenge. On successful UV, sign RFC 0003 evidence with lifetime `min(600 seconds, session remaining, intent remaining, policy remaining)`, persist only its unique public trust record/evidence digest/session generation through `begin_immediate_two_phase()`, extend helper instrumentation coverage, and destroy the private signer.
- [ ] `OwnerApprovedInvocation` has private fields, no public byte/trust constructor, no Clone/Serialize, and a value-free Debug. Loading it after issuance requires the exact registry record, live originating session generation, matching evidence digest, and unconsumed state. Offline signature verification may succeed after restart, but exact reservation must fail because restart invalidated the owner session/grant.
- [ ] Add `default = []` / `owner-effect-fixture` to capability when the fixture bridge first appears; only that feature enables its optional owner dependency. Authority must not depend on or forward capability yet because capability still depends on authority at this point; Task 10 performs the one-step inversion before adding that forwarding. Capability may re-export inspection types, but its exact API cannot accept a raw `SignedApprovalV1`, arbitrary `RoleTrustStore`, or application signer. Keep legacy signing only behind `legacy-experimental` later.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 6 --phase green -- \
    cargo test -p sovereign-owner --test exact_approval \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 6 --phase green -- \
    cargo test -p sovereign-owner --test compile_fail \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-owner --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-capability --test approval_v2 \
    --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `feat(owner): issue opaque fixture-bound rfc0003 approvals`
- [ ] Push checkpoint 6.

## Task 7: Admit only a synthetic fixture authority root and forest

**Files:**

- Create: `crates/authority/src/forest.rs`
- Modify: `crates/authority/Cargo.toml`
- Create: `crates/authority/src/fixture_root.rs`
- Modify: `crates/authority/src/store.rs`
- Modify: `crates/authority/src/broker/protocol.rs`
- Modify: `crates/authority/src/broker/mod.rs`
- Modify: `crates/authority/src/lib.rs`
- Create: `crates/authority/tests/fixture_root_lifecycle.rs`
- Create: `crates/authority/tests/compile_fail.rs`
- Create: `crates/authority/tests/ui/no_raw_node_writer.rs`
- Create: `crates/authority/tests/ui/no_raw_node_writer.stderr`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** A closed `SyntheticFixtureRootV1` lifecycle. No product root variant or raw node writer exists.

- [ ] **RED:** Add exact tests `unauthorized_caller_cannot_create_root`, `product_root_type_and_command_do_not_exist`, `fixture_root_requires_fresh_fixture_uv_approval`, `root_is_fixture_suite_and_generation_bound`, `root_rights_are_fixed_and_cannot_widen`, `child_only_narrows_scope_lifetime_and_uses`, `ancestor_revocation_blocks_child`, `revocation_requires_fresh_fixture_uv_approval`, `reopen_revalidates_canonical_root`, `fixture_root_is_rejected_by_product_open`, and compile-fail `no_raw_node_writer`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 7 --phase red \
    --expected-diagnostic 'SyntheticFixtureRootV1' -- \
    cargo test -p sovereign-authority --test fixture_root_lifecycle \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 7 --phase red \
    --expected-diagnostic 'no_raw_node_writer' -- \
    cargo test -p sovereign-authority --test compile_fail \
      --no-default-features --features owner-effect-fixture --locked \
      no_raw_node_writer -- --exact
  ```

  Expected RED: there is no forest/root lifecycle and current store has no typed admission boundary.

- [ ] Define the canonical root as tag `synthetic_fixture_root_v1`, exact RFC/fixture-suite digest, fixed semantic issuer `synthetic-fixture-authority-v1` (not a custody key), random fixture generation, fixed origin record, fixed operation `local_outbox.write_rfc5322`, fixed synthetic outbox root handle, maximum 24-hour lifetime, maximum 32 total uses, single-parent rule, and `non_product=true`. Unknown fields/tags fail closed; another issuer/suite/origin/generation cannot reuse it.
- [ ] Add `trybuild` as an authority dev-dependency only and lock it; reuse it for Tasks 8 and 11. Verify it is absent from default release dependencies and inspect each `.stderr` for the intended privacy failure.
- [ ] Broker exposes a fixed high-level synthetic root preview, then creates the root once only after consuming a fresh `OwnerApprovedInvocation` bound to `fixture.authority_root.create` and the complete previewed root. This is fixture-authenticator authorization, not independent owner admission. A second create, generation mismatch, product path, or raw insert is impossible through public IPC/API.
- [ ] Revocation consumes a fresh approval bound to exact root/generation and commits the root revocation plus descendant invalidation marker through `begin_immediate_two_phase()`. Root/child creation uses the same helper; child creation consumes an approved narrowing request, depth is bounded at 16, and cycles/missing ancestors fail. Extend the Task 4 instrumentation expectation set with every root/forest modifying command/table.
- [ ] Expose only value-free root status `{fixture_generation, active|revoked|expired}` to an authenticated synthetic session. CLI/effects compile tests cannot name an insert/update/table handle.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 7 --phase green -- \
    cargo test -p sovereign-authority --test fixture_root_lifecycle \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 7 --phase green -- \
    cargo test -p sovereign-authority --test compile_fail \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `feat(authority): admit only synthetic fixture authority roots`
- [ ] Push checkpoint 7.

## Task 8: Compose and seal the exact synthetic payload inside the coordinator

**Files:**

- Create: `crates/authority/src/exact_fixture.rs`
- Create: `crates/authority/src/fixture_policy.rs`
- Modify: `crates/authority/Cargo.toml`
- Modify: `crates/authority/src/store.rs`
- Modify: `crates/authority/src/broker/protocol.rs`
- Modify: `crates/authority/src/broker/mod.rs`
- Modify: `crates/authority/src/lib.rs`
- Create: `crates/authority/tests/exact_fixture_prepare.rs`
- Create: `crates/authority/tests/ui/protected_payload_is_private.rs`
- Create: `crates/authority/tests/ui/protected_payload_is_private.stderr`
- Modify: `crates/capability/Cargo.toml`
- Create: `crates/capability/tests/protected_payload_boundary.rs`
- Create: `crates/capability/tests/ui/protected_payload_is_private.rs`
- Create: `crates/capability/tests/ui/protected_payload_is_private.stderr`
- Modify: `apps/cli/Cargo.toml`
- Create: `apps/cli/tests/protected_payload_boundary.rs`
- Create: `apps/cli/tests/ui/protected_payload_is_private.rs`
- Create: `apps/cli/tests/ui/protected_payload_is_private.stderr`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** Opaque `EffectIntentId`, private `ProtectedFixturePayload`, exact deterministic RFC 5322 composition, `Prepared` state, and a high-level fixed synthetic preview projection.

- [ ] **RED:** With recording RNG/composer hooks, add `intent_id_is_allocated_before_any_synthetic_value_read`, `intent_id_is_random_not_value_derived`, `only_exact_fixture_recipient_and_corpus_are_accepted`, `same_id_and_fixture_inputs_produce_same_crlf_bytes`, `message_id_uses_random_intent`, `header_injection_is_impossible`, `fixture_policy_snapshot_expiry_and_epoch_are_immutable`, `sealed_recipient_bytes_policy_and_generation_are_immutable`, `changed_binding_requires_new_intent`, `payload_exists_only_in_named_private_table`, `expired_unreserved_payload_deletes_logical_row_but_makes_no_secure_erase_claim`, `reserved_payload_has_no_per_intent_delete_or_purge`, `preview_requires_live_fixture_session`, `preview_is_derived_inside_broker`, and compile-fail `protected_payload_is_private`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 8 --phase red \
    --expected-diagnostic 'ProtectedFixturePayload' -- \
    cargo test -p sovereign-authority --test exact_fixture_prepare \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 8 --phase red \
    --expected-diagnostic 'protected_payload_is_private' -- \
    cargo test -p sovereign-authority --test compile_fail \
      --no-default-features --features owner-effect-fixture --locked \
      protected_payload_is_private -- --exact
  ```

  Expected RED: no private payload/composer/intent state exists.

- [ ] In one broker command, validate only the parameter-free synthetic fixture operation, allocate 128 random intent bits first, then read the compiled corpus and compose exact CRLF bytes. Use fixed Date/header order/transfer encoding and `<intent-id@fixture.invalid>` Message-ID; reject every caller-supplied recipient/body/header/document/customer/value field at IPC decode.
- [ ] Build the RFC 0003/Capability V2 prepared invocation from only the full-random intent ID, random fixture generation, fixed operation, and opaque coordinator reference. Add `approval_and_capability_contain_no_recipient_content_or_digest` and a dictionary scan over approval/token bytes.
- [ ] Define broker-private `FixturePolicySnapshotV1` from the existing opaque `PolicyAuthorizationV2`: exact decision ID/digest, immutable synthetic-policy epoch stored in private table `fixture_policy_epoch_v1`, evaluated time, and RFC 0006 expiry clipped by prepared/root lifetime. There is no public epoch writer; cfg-fault tests may advance it to model invalidation. Seal exact normalized recipient, full RFC bytes, operation, complete invocation/policy snapshot binding, root/generation, prepared expiry, and retention only in logical redb table `fixture_protected_payload_v1`, through `begin_immediate_two_phase()`. Store no unkeyed recipient/content digest. Enumerate every other logical table in tests and prove it has no canary. Public state scans return only random ID, closed state, and expiry class without exact time. Logical deletion removes the row but makes no raw-page secure-erasure claim.
- [ ] Preview is constructed within the broker and returned only after live session validation as `SyntheticPreviewV1` whose fields must exactly equal the compiled canaries and escaped parsed headers/body. It is deliberately synthetic and cannot be used as a generic business preview or payload accessor.
- [ ] **BOUNDARY RED:** After the minimal private payload and public opaque `SyntheticPreviewV1` compile, put privacy cases in the actual consuming package, not in authority pretending to compile another crate. Add `trybuild` as a dev-dependency in capability and CLI, and register `[[test]]` targets named `protected_payload_boundary` with `required-features = ["owner-effect-fixture"]` in each consuming crate's own `Cargo.toml`. Add their harness/UI sources but deliberately omit their `.stderr` goldens. Each UI obtains the real public preview type through that consumer's actual dependency graph and calls forbidden `.payload()`/byte access, so compilation reaches the type and fails specifically with `no method named payload`, never an unresolved crate/import. Register one checked TSV row per package/target with profile `owner-effect-fixture`, then run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 8 --phase red \
    --expected-diagnostic 'no method named `payload`' -- \
    cargo test -p sovereign-capability --test protected_payload_boundary \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 8 --phase red \
    --expected-diagnostic 'no method named `payload`' -- \
    cargo test -p sovereign-cli --test protected_payload_boundary \
      --no-default-features --features owner-effect-fixture --locked
  ```

  Expected genuine RED: trybuild writes `wip/*.stderr` containing only the intended missing-accessor privacy error and fails because no golden was accepted.
- [ ] Inspect those diagnostics, reject any unresolved-import/feature failure, then accept exact `.stderr` goldens. Keep authority's own compile-fail case. Owner is lower and not an authority consumer, so Task 8 adds no bogus owner case. Task 10 removes capability's case when it removes that dependency; Task 11 adds effects' case when effects becomes a real consumer.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 8 --phase green -- \
    cargo test -p sovereign-authority --test exact_fixture_prepare \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 8 --phase green -- \
    cargo test -p sovereign-authority --test compile_fail \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 8 --phase green -- \
    cargo test -p sovereign-capability --test protected_payload_boundary \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 8 --phase green -- \
    cargo test -p sovereign-cli --test protected_payload_boundary \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `feat(authority): seal pre-content synthetic effect payloads`
- [ ] Push checkpoint 8.

## Task 9: Expose only exact-origin synthetic fixture HTTP and complete logout/handoff

**Files:**

- Modify: `apps/cli/Cargo.toml`
- Modify: `apps/cli/src/main.rs`
- Modify: `apps/cli/src/ui.rs`
- Modify: `apps/cli/src/workspace/mod.rs`
- Modify: `apps/cli/src/workspace/store.rs`
- Modify: `apps/cli/src/workspace/kernel_exec.rs`
- Modify: `apps/cli/src/workspace/send_workflow.rs`
- Modify: `apps/cli/src/workspace/ops.rs`
- Create: `apps/cli/src/workspace/legacy.rs`
- Create: `apps/cli/src/fixture_http/mod.rs`
- Create: `apps/cli/src/fixture_http/security.rs`
- Create: `apps/cli/src/fixture_http/auth_routes.rs`
- Create: `apps/cli/src/fixture_http/fixture_routes.rs`
- Create: `apps/cli/src/fixture_http/native_handoff.rs`
- Create: `apps/cli/src/fixture_http/response.rs`
- Modify: `apps/cli/src/broker_client.rs`
- Create: `apps/cli/tests/fixture_loopback.rs`
- Create: `apps/cli/tests/slice_isolation.rs`
- Create: `scripts/check-owner-effect-boundary.sh`
- Create: `scripts/check-owner-effect-profile-builds.sh`
- Modify: `scripts/check-owner-effect-broker-build.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** Feature-gated `security-fixture --synthetic-only`, exact port/origin middleware, CSRF-protected logout, operation-bound native handoff, and immediate CLI/business/legacy separation. From this first fixture CLI checkpoint onward, default, fixture, and legacy are clean-built and command/symbol-audited independently.

- [ ] **RED:** Add exact tests `fixture_command_requires_feature_and_synthetic_flag`, `clean_feature_command_launches_same_image_broker`, `product_or_unmarked_root_is_rejected_before_read`, `fixture_root_cannot_be_inside_product_root`, `no_product_registration_setup_import_or_business_mutation_route`, `route_manifest_is_closed_and_has_no_api_get_alias_or_query_token`, `registration_start_and_finish_work_without_session_or_csrf`, `registration_route_rejects_after_registry_is_populated`, `login_start_and_finish_work_without_session_or_csrf`, `auth_start_does_not_issue_session`, `auth_finish_alone_issues_cookie_and_independent_csrf`, `auth_challenge_is_process_generation_bound_one_use_and_300_seconds`, `auth_routes_require_exact_host_origin_fetch_json_and_size`, `login_options_allow_only_stored_credential_id`, `get_serves_only_task9_synthetic_warning_shell`, `wrong_host_origin_port_fetch_metadata_or_content_type_fails`, `cookie_without_csrf_and_csrf_without_cookie_fail`, `duplicate_cookie_or_security_header_fails`, `occupied_7787_never_falls_back`, `restart_preserves_exact_compiled_origin_and_generation`, `origin_change_api_does_not_exist`, `same_rp_id_cross_port_credential_cannot_authenticate`, `alternate_hostname_or_port_never_authenticates`, `logout_route_revokes_server_state_and_clears_cookie`, `logout_races_fixture_action_with_one_winner`, and `native_handoff_is_operation_process_nonce_and_session_bound`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 9 --phase red \
    --expected-diagnostic 'security-fixture' -- \
    cargo test -p sovereign-cli --test fixture_loopback \
      --no-default-features --features owner-effect-fixture --locked -- --test-threads=1
  ./scripts/run-owner-effect-tests.sh --task 9 --phase red \
    --expected-diagnostic 'default binary still exposes business command' -- \
    cargo test -p sovereign-cli --test slice_isolation \
      --no-default-features --locked -- --test-threads=1
  ```

  Expected RED: fixture command/routes do not exist and current UI has unauthenticated business GETs.

- [ ] Complete the Task 4 non-default `owner-effect-fixture` command as `security-fixture --synthetic-only`. For creation it accepts only an absent leaf under an existing canonical private parent, creates a new owner-only directory, generates the random fixture generation, and atomically writes only the synthetic marker `synthetic-owner-effect-fixture-v1`; for reopen it accepts only that existing marker plus exact generation. In both cases reject a workspace/Vault/product/legacy marker in the directory or any ancestor, and reject ActiveV2, symlink, non-private directory, or unknown file before opening broker state. The broker repeats the same classification before its own bind/lock/open so direct hidden invocation cannot bypass it. There is no caller-chosen generation, import/migrate/adopt path, or product marker write.
- [ ] At this task, `sovereign-cli/owner-effect-fixture` forwards only the already-existing `sovereign-authority/owner-effect-fixture`; authority currently forwards owner only. Task 10 adds capability after the dependency inversion, Task 11 adds sandbox/effects, and Task 12 adds audit—never reference a not-yet-defined feature or create a temporary cycle. None is a dependency default. Add compile-time rejection if fixture and legacy features are both enabled, so Cargo's additive features cannot silently merge the planes.
- [ ] In this same checkpoint, before exposing the fixture command, define non-default `legacy-experimental` and move current business UI/init/status/integrity/import/read/decrypt/list/preview/export/mutations, workspace modules, self-signed approval orchestration, and their tests behind that feature plus explicit runtime `--legacy-experimental`. Default and fixture builds do not compile their module declarations or command enum/match arms. Preserve the current behavior only in the legacy profile with exact warnings `unauthenticated`, `app-approved`, `not exact-bound`, `not at-rest protected`, and `not a product security claim`; legacy and fixture features are compile-time mutually exclusive. Do not wait for Task 14 to create this separation.
- [ ] Bind `127.0.0.1:7787` before spawning the same-image broker, starting registration, or opening a browser. `AddrInUse` returns one value-free diagnostic and exits without broker/database side effects; never select another port. Broker reopen verifies compiled origin, stored fixture generation, HTTP process generation, and matrix entry together. There is no origin-change endpoint; changing the compiled origin requires a new build review, new matrix entry, and destructive new synthetic fixture. Extend the clean-build script to run this exact feature command from a fresh target directory and authenticate its child broker.
- [ ] Split middleware explicitly. All JSON routes—including the four pre-session routes—require exact `Host: localhost:7787`, `Origin: http://localhost:7787`, `Sec-Fetch-Site: same-origin`, `Sec-Fetch-Mode: cors`, `Sec-Fetch-Dest: empty`, one JSON media type, a 16 KiB body ceiling, no query, and the in-process HTTP generation added by the handler to broker IPC. Registration/login start/finish require no existing session or CSRF; they never accept either value in the body/query. Every post-session route additionally requires exactly one valid cookie, exactly one CSRF header, and broker-validated live session. Static GET/navigation is separate and never authorizes an API. All responses use no-store, no-referrer, CORP same-origin, nosniff, frame denial, and restrictive self-only CSP.
- [ ] Implement the exact pre-session schemas from RFC 0006: `POST /api/fixture/auth/register/start` and `POST /api/fixture/auth/login/start` accept only `{}` and return `{challenge_handle, public_key}`; the random 256-bit handle binds HTTP generation, fixture generation, exact origin, ceremony kind, and 300-second one-use state. `POST /api/fixture/auth/register/finish` accepts only `{challenge_handle, credential: RegistrationCredentialV1}`; `POST /api/fixture/auth/login/finish` accepts only `{challenge_handle, credential: AuthenticationCredentialV1}`. The bounded credential records enumerate the exact WebAuthn `id`, `rawId`, `type`, response byte fields, client extensions, attachment/transports allowlist, and nullable user handle; unknown/duplicate/noncanonical base64url fields fail. Start never issues cookie/CSRF. Successful finish alone returns `{csrf_token, mechanism_label}` and the exact session cookie; failure returns neither.
- [ ] Until Task 13, static GET serves only a hardcoded minimal synthetic-warning shell from `fixture_http` with no business assets, script, values, or action controls. This keeps Task 9 independently compilable without embedding the current product UI; Task 13 replaces it with feature-only fixture assets.
- [ ] RFC 0006 freezes the complete final API route manifest—no aliases, wildcard dispatcher, query-token, method override, redirect, OPTIONS success, or API GET: the four auth routes above; `POST /api/fixture/root/preview`, `/root/create/start`, `/root/create/finish`, `/root/revoke/start`, `/root/revoke/finish`; `POST /api/fixture/effect/prepare`, `/effect/preview`, `/effect/approve/start`, `/effect/approve/finish`, `/effect/dispatch`, `/effect/reconcile`; and `POST /api/fixture/auth/logout`. Task 9 installs only routes backed by Tasks 5-8: auth, root, prepare/preview/approve, and logout. The future dispatch/reconcile paths have no enum variant/handler and return the ordinary unknown-route response until Task 11 atomically adds both backend and handlers—never a stub/501. Root/effect preview and prepare accept only `{}` or `{intent_id}`; ceremony starts accept the exact opaque target ID; finishes accept only challenge handle plus bounded assertion. Root create/revoke and effect approval use fresh UV bound to their exact fixed preview. After logout checks, send one broker logout command, wait for acknowledgement, and return exact deletion cookie.
- [ ] Native status/reconcile commands start the same fixed origin and receive an opaque one-use operation/process-nonce grant over an in-process channel only after browser session authorization. No token enters stdout, argv, env, URL, disk, logs, or another process; logout/broker death invalidates it.
- [ ] Raw TCP attacks cover every disallowed method/path, `/api` GET, alias/trailing slash, query strings/tokens, absolute URIs, `127.0.0.1`, alternate port, duplicate/bomb cookies, missing/null/wrong Origin, absent/wrong Fetch Metadata, simple form requests, preflight, oversized/malformed/unknown credential fields, stale/wrong-process/replayed challenge handles, stale/rotated/logged-out CSRF, and concurrent idle expiry. Failures have a fixed value-free class/size.
- [ ] Add `slice_isolation` tests `default_binary_has_no_business_fixture_or_legacy_commands`, `fixture_binary_has_only_fixture_and_offline_public_commands`, `legacy_binary_has_business_commands_only_with_feature_and_runtime_opt_in`, `fixture_and_legacy_features_are_mutually_exclusive`, and `business_modules_are_cfg_absent_from_default_and_fixture`. Create `check-owner-effect-boundary.sh` now with the marker/business-command/module checks available at this checkpoint.
- [ ] Create `check-owner-effect-profile-builds.sh`. For every invocation it allocates three new target directories and clean-builds exactly: default `cargo build -p sovereign-cli --bin sovereign --no-default-features --locked`; fixture with `--features owner-effect-fixture`; and legacy with `--features legacy-experimental`. It checks help and public/link symbols: default has neither fixture/hidden-broker nor business/legacy commands; fixture has only fixture plus offline-public commands and the feature-gated broker symbol, never business/self-signer symbols; legacy has business commands only behind the runtime opt-in and has no fixture/broker symbol. It also proves the combined features fail with the intended compile error, rejects a missing binary/zero test, and starts from no prior target artifacts. Tasks 10-16 invoke this unchanged or extended gate at every GREEN checkpoint.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 9 --phase green -- \
    cargo test -p sovereign-cli --test fixture_loopback \
      --no-default-features --features owner-effect-fixture --locked -- --test-threads=1
  ./scripts/run-owner-effect-tests.sh --task 9 --phase green -- \
    cargo test -p sovereign-cli --test slice_isolation \
      --no-default-features --locked -- --test-threads=1
  ./scripts/check-owner-effect-boundary.sh
  ./scripts/check-owner-effect-profile-builds.sh
  ./scripts/check-owner-effect-broker-build.sh
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-cli --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `feat(cli): expose only exact-origin synthetic fixture routes`
- [ ] Push checkpoint 9.

## Task 10: Make Capability V2 pure and reserve through the broker transaction

**Files:**

- Modify: `crates/capability/Cargo.toml`
- Modify: `crates/capability/src/v2.rs`
- Modify: `crates/capability/src/lib.rs`
- Modify: `crates/capability/tests/capability_v2.rs`
- Modify: `crates/capability/tests/approval_v2.rs`
- Create: `crates/capability/tests/exact_fixture_validation.rs`
- Delete: `crates/capability/tests/protected_payload_boundary.rs`
- Delete: `crates/capability/tests/ui/protected_payload_is_private.rs`
- Delete: `crates/capability/tests/ui/protected_payload_is_private.stderr`
- Modify: `crates/authority/Cargo.toml`
- Modify: `crates/authority/src/lib.rs`
- Modify: `crates/authority/src/claims.rs`
- Create: `crates/authority/src/reservation.rs`
- Modify: `crates/authority/src/store.rs`
- Modify: `crates/authority/src/broker/mod.rs`
- Modify: `crates/authority/src/broker/protocol.rs`
- Create: `crates/authority/tests/reservation_atomicity.rs`
- Create: `crates/authority/tests/approval_v2_integration.rs`
- Modify: `apps/cli/Cargo.toml`
- Modify: `apps/cli/src/workspace/kernel_exec.rs`
- Modify: `apps/cli/src/workspace/legacy.rs`
- Modify: `apps/cli/tests/slice_isolation.rs`
- Create: `apps/cli/tests/authority_plane_boundary.rs`
- Create: `apps/cli/tests/ui/old_direct_authority_plane.rs`
- Create: `apps/cli/tests/ui/old_direct_authority_plane.stderr`
- Create: `apps/cli/tests/legacy_authority_orchestration.rs`
- Create: `scripts/check-owner-effect-authority-plane.sh`
- Modify: `scripts/check-owner-effect-boundary.sh`
- Modify: `scripts/check-owner-effect-profile-builds.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** Final acyclic graph, private `VerifiedFixtureCapability`, and one broker transaction for approval/token/idempotency/root/use/effect reservation.

- [ ] **RED — graph/API:** Add a dependency contract that fails while `sovereign-capability` depends on authority or authority does not depend on capability, and compile tests proving the already-consuming CLI cannot construct `VerifiedFixtureCapability` or `ReservationRequest`. Add `trybuild` as a CLI dev-dependency and a CLI-owned `[[test]] name = "authority_plane_boundary"` target with no required feature so the same target can run under both explicit default and fixture profiles; its `old_direct_authority_plane` UI case must fail until `CapabilityValidatorV2::with_authority_store`, `sovereign_authority::AuthorityStore::open`, and raw `consume_token`/`consume_approval`/`bind_idempotency` cease to be callable. Register distinct default and fixture TSV rows. Add a separate legacy integration target with `required-features = ["legacy-experimental"]` and a matching TSV row; it initially fails because no upper-layer legacy orchestration exists. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 10 --phase red \
    --expected-diagnostic 'sovereign_authority' -- \
    cargo test -p sovereign-capability --test exact_fixture_validation \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 10 --phase red \
    --expected-diagnostic 'Expected test case to fail to compile' -- \
    cargo test -p sovereign-cli --test authority_plane_boundary \
      --no-default-features --locked
  ./scripts/run-owner-effect-tests.sh --task 10 --phase red \
    --expected-diagnostic 'Expected test case to fail to compile' -- \
    cargo test -p sovereign-cli --test authority_plane_boundary \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 10 --phase red \
    --expected-diagnostic 'LegacyAuthorityOrchestrator' -- \
    cargo test -p sovereign-cli --test legacy_authority_orchestration \
      --no-default-features --features legacy-experimental --locked
  ./scripts/run-owner-effect-tests.sh --task 10 --phase red \
    --expected-diagnostic 'ReservationRequest' -- \
    cargo test -p sovereign-authority --test reservation_atomicity \
      --no-default-features --features owner-effect-fixture --locked
  ```

  Expected RED: current validator performs durable claims itself and no broker reservation exists.

- [ ] In one commit step, remove every production/dev `sovereign-capability -> sovereign-authority` edge, delete `with_authority_store` and its store field/error/mutation branches, make Capability V2 verification pure, then add `sovereign-authority -> sovereign-capability` and only now forward `sovereign-capability/owner-effect-fixture` from authority's same feature; never leave a cyclic manifest. Remove capability's Task 8 authority-consumer compile target, its now-unused `trybuild` dev-dependency, and its TSV rows because capability is no longer a consumer; the dependency contract replaces it. Move Task 2's durable approval/store integration cases from capability tests into `authority/tests/approval_v2_integration.rs`, leaving wire/pure verifier cases in capability. Exact fixture verification consumes `OwnerApprovedInvocation`, the synthetic fixture authority token/trust, prepared intent binding, policy, invocation, session and generation, and yields a private-field one-use `VerifiedFixtureCapability`.
- [ ] Remove the old public `AuthorityStore`, `AuthorityStore::open`, `consume_token`, `consume_approval`, and `bind_idempotency` API surface rather than leaving it beside the broker. For the already-isolated legacy profile only, capability's `legacy-experimental` pure verifier yields opaque `LegacyValidatedCapability`; authority's mutually exclusive `legacy-experimental` exposes one typed `LegacyClaimCoordinator` that internally owns the old filesystem claim implementation and accepts only that validated object—never raw IDs/trust/signers. Authority's legacy feature forwards capability's same feature, and CLI's already-existing legacy feature forwards exactly both dependency legacy features; none is default or enabled by the fixture feature. `apps/cli/src/workspace/legacy.rs` is the sole upper-layer orchestrator: pure validate, then typed legacy claim, then existing legacy execution. `kernel_exec.rs` must contain no `with_authority_store(AuthorityStore::open(...))`. Default and fixture builds have neither legacy type/module/symbol; fixture reservation still goes only through the broker transaction.
- [ ] After removal, run the default and fixture authority-plane trybuild cases once without a golden, inspect the generated diagnostic, and accept `old_direct_authority_plane.stderr` only if it names the removed method/type/raw-claim members through the real CLI dependency graph. An unresolved crate/import, feature-disabled target, or legacy-only symbol leaking into either profile is a defect. The same accepted case must pass under both explicit profiles shown in GREEN.
- [ ] `check-owner-effect-authority-plane.sh` runs three explicit metadata profiles: `cargo metadata --no-deps --format-version 1 --no-default-features`; the same with `--features sovereign-cli/owner-effect-fixture`; and with `--features sovereign-cli/legacy-experimental`. It pairs them with `cargo tree -p sovereign-cli -e features --no-default-features`, then the same command with respectively `--features owner-effect-fixture` and `--features legacy-experimental`. It runs the two default/fixture compile-fail targets, the legacy orchestration test, source checks, and public/link-symbol inspection. It fails on `capability -> authority`, `with_authority_store`, old `AuthorityStore`/raw claim symbols in any default/fixture artifact, a legacy claim symbol outside the legacy artifact, or simultaneous broker+legacy claim planes. Extend the profile-build/boundary scripts and run them at this checkpoint and every later checkpoint.
- [ ] The fixture Capability authority signer/trust is generated per broker launch, held only in broker memory, invalidated on restart, and tagged `non_product=true`; it is not an owner/device/runtime custody or continuity claim. Persist only its public trust record for verification evidence. Raw approval/authority trust setters/signers remain unavailable in default/fixture app APIs.
- [ ] **RED — transaction:** Add `reservation_reverifies_full_rfc0003_and_capability`, `approval_expiry_not_token_expiry_controls_replay`, `reservation_binds_intent_operation_policy_invocation_session_root_and_generation`, `one_transaction_consumes_approval_token_idempotency_root_use_and_prepared_state`, `failpoint_never_commits_subset`, `concurrent_ipc_clicks_have_one_winner`, `concurrent_real_subprocess_ipc_runners_have_one_winner`, `exact_idempotency_replay_returns_same_receipt`, and `different_intent_conflicts`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 10 --phase red \
    --expected-diagnostic 'reservation was partially committed' -- \
    cargo test -p sovereign-authority --test reservation_atomicity \
      --no-default-features --features owner-effect-fixture,fault-injection \
      --locked -- --test-threads=1
  ```

  Expected RED: validated values are still consumed by separate claim calls.

- [ ] Inside one transaction obtained only from `begin_immediate_two_phase()`, recheck approval/token/prepared/policy/session/root/ancestor expiries and revocation, claim approval until approval expiry, claim token, bind idempotency to random intent ID, decrement one root use, and move `Prepared -> AuthorityReserved`. Persist a private `DispatchRevalidationV1` containing fixture/origin generation, prepared expiry, policy snapshot ID/epoch/expiry, immutable approved-session binding, logout epoch at reservation, and exact root/ancestor IDs plus their expiry/revocation generations; it contains no recipient/content/digest. Validate first, mutate second, commit once. Extend helper instrumentation to every claimed table. Errors/logs carry random IDs and closed codes only.
- [ ] Because validation now runs below/inside authority, keep `ReservationRequest`, `reserve_once`, reservation receipt internals, and all state-transition methods crate-private. Public IPC exposes only high-level fixture commands and value-free results; no Rust friend-crate exception or source-grep-only privacy claim is needed for reservation/dispatch.
- [ ] Replace Task 3's mixed partial characterization with `real_subprocess_mixed_reservation_is_atomic_through_ipc`; children never open redb. Add broker-death-before/after-commit tests: before commit leaves all absent; after acknowledged commit returns/reopens the same opaque receipt without dispatch.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 10 --phase green -- \
    cargo test -p sovereign-capability --test exact_fixture_validation \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 10 --phase green -- \
    cargo test -p sovereign-authority --test reservation_atomicity \
      --no-default-features --features owner-effect-fixture,fault-injection \
      --locked -- --test-threads=1
  ./scripts/run-owner-effect-tests.sh --task 10 --phase green -- \
    cargo test -p sovereign-cli --test authority_plane_boundary \
      --no-default-features --locked
  ./scripts/run-owner-effect-tests.sh --task 10 --phase green -- \
    cargo test -p sovereign-cli --test authority_plane_boundary \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 10 --phase green -- \
    cargo test -p sovereign-cli --test legacy_authority_orchestration \
      --no-default-features --features legacy-experimental --locked
  ./scripts/check-owner-effect-authority-plane.sh
  ./scripts/check-owner-effect-boundary.sh
  ./scripts/check-owner-effect-profile-builds.sh
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-capability --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `feat(authority): reserve verified fixture effects atomically`
- [ ] Push checkpoint 10.

## Task 11: Dispatch the exact file inside the coordinator and recover conservatively

**Files:**

- Create: `crates/authority/src/local_outbox.rs`
- Create: `crates/authority/src/canary_scan.rs`
- Modify: `crates/authority/Cargo.toml`
- Modify: `crates/authority/src/store.rs`
- Modify: `crates/authority/src/broker/mod.rs`
- Modify: `crates/authority/src/broker/protocol.rs`
- Modify: `crates/authority/src/lib.rs`
- Modify: `crates/sandbox/Cargo.toml`
- Modify: `crates/sandbox/src/lib.rs`
- Create: `crates/sandbox/src/synthetic_fixture.rs`
- Create: `crates/sandbox/tests/synthetic_exact_effect_fixture.rs`
- Modify: `crates/effects/Cargo.toml`
- Modify: `crates/effects/src/lib.rs`
- Create: `crates/effects/tests/opaque_facade.rs`
- Create: `crates/effects/tests/protected_payload_boundary.rs`
- Create: `crates/effects/tests/ui/protected_payload_is_private.rs`
- Create: `crates/effects/tests/ui/protected_payload_is_private.stderr`
- Modify: `apps/cli/Cargo.toml`
- Modify: `apps/cli/src/broker_client.rs`
- Modify: `apps/cli/src/fixture_http/fixture_routes.rs`
- Modify: `apps/cli/tests/fixture_loopback.rs`
- Create: `crates/authority/tests/exact_local_outbox.rs`
- Create: `crates/authority/tests/canary_allowlist.rs`
- Create: `crates/authority/tests/ui/no_payload_or_raw_writer.rs`
- Create: `crates/authority/tests/ui/no_payload_or_raw_writer.stderr`
- Create: `scripts/check-owner-effect-canaries.sh`
- Modify: `scripts/check-owner-effect-boundary.sh`
- Modify: `scripts/check-owner-effect-profile-builds.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** Parameter-free sandbox fixture execution without a second capability claim, coordinator-private local publication, post-reservation expiry/revocation revalidation, state-aware canary allowlist, value-free façade/outcome, and exact `Indeterminate` semantics. Effects never sees recipient or bytes.

- [ ] Define `sovereign-sandbox/owner-effect-fixture` with `default = []`, and forward it only from `sovereign-authority/owner-effect-fixture`; define matching non-default effects feature. Add `trybuild` as an effects dev-dependency and register its `[[test]] name = "protected_payload_boundary"` with `required-features = ["owner-effect-fixture"]`, plus the matching TSV row with profile `owner-effect-fixture`. Register all other required-feature targets before RED, without adding the sandbox function or dispatch behavior. Deliberately omit the effects `.stderr` golden for the first boundary run.
- [ ] **RED:** Add sandbox tests `synthetic_fixture_api_is_parameter_free`, `synthetic_fixture_owns_fixed_verified_artifact_and_input`, `synthetic_fixture_uses_verified_runtime_without_capability_consumption`, `synthetic_fixture_is_rerunnable_after_restart`, `synthetic_fixture_has_no_imports_or_host_input`, and `synthetic_fixture_returns_only_closed_pass`; authority/effects/CLI tests `fixed_verified_wasm_runs_after_reservation_before_dispatching`, `reserved_restart_reruns_pure_fixture_without_token_or_approval`, `sandbox_output_cannot_change_recipient_or_bytes`, `sandbox_import_or_failure_closes_failed_before_dispatch_without_io`, `prepared_expiry_after_reservation_closes_failed_before_dispatch_without_io`, `root_or_ancestor_expiry_after_reservation_closes_failed_before_dispatch_without_io`, `policy_expiry_or_epoch_change_after_reservation_closes_failed_before_dispatch_without_io`, `logout_generation_change_racing_dispatch_has_one_winner`, `final_revalidation_and_dispatching_commit_are_one_transaction`, `expiry_after_dispatching_never_cancels_retries_or_relabels`, `coordinator_commits_dispatching_before_filesystem_touch`, `only_authority_reserved_can_dispatch`, `revoked_ancestor_before_dispatch_closes_failed_before_dispatch_without_io`, `filename_is_random_intent_dot_eml`, `publication_temp_is_exact_same_directory_name_and_owner_only`, `published_file_equals_private_sealed_bytes`, `live_failure_before_any_payload_write_commits_failed_before_dispatch`, `crash_after_dispatching_before_failed_before_dispatch_commit_reopens_indeterminate`, `restart_dispatching_absent_is_indeterminate`, `restart_dispatching_identical_is_succeeded`, `different_unreadable_wrong_type_or_uncertain_sync_is_indeterminate`, `no_state_after_dispatching_retries_or_deletes`, `concurrent_dispatch_commands_have_one_winner`, `broker_death_never_auto_respawns_dispatch`, `dispatch_and_reconcile_routes_activate_only_with_exact_backend`, `canary_locations_match_state_allowlist`, and compile-fail `no_payload_or_raw_writer`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 11 --phase red \
    --expected-diagnostic 'run_synthetic_exact_effect_fixture' -- \
    cargo test -p sovereign-sandbox --test synthetic_exact_effect_fixture \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 11 --phase red \
    --expected-diagnostic 'prepared authority expired before dispatch' -- \
    cargo test -p sovereign-authority --test exact_local_outbox \
      --no-default-features --features owner-effect-fixture,fault-injection \
      --locked -- --test-threads=1
  ./scripts/run-owner-effect-tests.sh --task 11 --phase red \
    --expected-diagnostic 'raw effect writer remains public' -- \
    cargo test -p sovereign-effects --test opaque_facade \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 11 --phase red \
    --expected-diagnostic 'no method named `payload`' -- \
    cargo test -p sovereign-effects --test protected_payload_boundary \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 11 --phase red \
    --expected-diagnostic 'dispatch route remains unknown' -- \
    cargo test -p sovereign-cli --test fixture_loopback \
      --no-default-features --features owner-effect-fixture --locked \
      dispatch_and_reconcile_routes_activate_only_with_exact_backend -- --exact
  ```

  Expected RED: current raw effects writer accepts caller bytes, exposes digest/size/path, and current workflow retries/deletes after crashes. Independently, the effects trybuild run writes `wip/protected_payload_is_private.stderr` containing the intended `no method named payload` diagnostic and fails because its golden is absent; an unresolved authority import or disabled target is a test defect, not an acceptable RED.

- [ ] Inspect the effects trybuild diagnostic, reject any dependency/feature/import failure, and accept the exact `.stderr` golden only after it proves the real effects dependency graph cannot access protected bytes. Keep that consuming-crate target and TSV row through the final gate.

- [ ] In `sandbox::synthetic_fixture`, expose only `pub fn run_synthetic_exact_effect_fixture() -> Result<SyntheticExactEffectFixtureResult, SandboxError>` behind its fixture feature. It accepts zero parameters and owns exact embedded Wasm bytes, an offline-signed publisher manifest plus public trust record (no private key), fixed expected component/manifest digests named by RFC 0006, fixed canonical input, operation selector, limits, and expected exit code. It reuses `ArtifactVerifier` to verify the embedded envelope, treats equality with the code/RFC-pinned digests as the fixture-only local admission decision, builds `PreparedInvocation::prepare`, and calls the existing crate-private `WasmSandbox::execute_verified`. It does not use `ArtifactStore` (which would require a path/admission signer), does not call `CapabilityValidatorV2`, accepts/reconstructs no capability, and exposes no generic verified-byte entrypoint. The result is the fieldless closed enum variant `Passed`; this code-admitted synthetic artifact is not a product artifact-admission path. Sandbox gains no authority dependency.
- [ ] Before each initial or restart pure run from `AuthorityReserved`, validate `DispatchRevalidationV1` against trusted time, prepared expiry, policy snapshot/epoch/expiry, complete root/ancestor lifetimes/revocations, fixture/origin generation, a currently live session for the same stored fixture credential, and unchanged logout epoch. Do not require the original in-memory session to survive restart and do not decrement root use again. On failure, use the sole write helper to atomically record the logical `Dispatching -> FailedBeforeDispatch` terminal closure with the live no-I/O sentinel; do not call sandbox or writer. On pass, call only the parameter-free sandbox function. A crash may rerun this harmless function after fresh login because it has no capability or effect, but nothing resumes automatically.
- [ ] After the pure fixture passes, open one `begin_immediate_two_phase()` transaction, repeat the complete revalidation against current store generations and the trusted clock as the final operation before mutation, and commit `AuthorityReserved -> Dispatching` in that same transaction. Root/policy/session updates serialize against it, and fault barriers sit before—not after—the final clock/read set. A losing expiry/revocation/logout race commits `FailedBeforeDispatch` without opening the outbox; a winning Dispatching commit authorizes exactly one subsequent write. Expiry after that commit cannot stop or relabel the in-flight/recovery path.
- [ ] The broker then calls crate-private `local_outbox`. It derives the only outbox as `<validated-synthetic-fixture-root>/outbox` (no caller path), privately resolves payload, creates exactly `<effect_intent_id>.eml.tmp` in that same directory with owner-only/no-follow/exclusive semantics, writes exact bytes, syncs, publishes `<effect_intent_id>.eml` without replacement, and syncs the directory. It never overwrites/removes an existing final target or cleans a crash orphan outside whole-fixture deletion.
- [ ] `FailedBeforeDispatch` requires a live in-process proof flag that no protected bytes were passed to a write syscall or published and a successful terminal-state commit before returning. Any crash, broker/channel loss, first/partial write, publication ambiguity, sync error, or inability to commit that proof becomes/remains `Indeterminate`.
- [ ] Revalidation includes—not merely revocation—prepared expiry, policy epoch/expiry, every root/ancestor expiry/revocation, generation, and logout/session state. Tests hold the pre-dispatch barrier while advancing each clock/generation and use a write-syscall sentinel to prove the losing transaction touches no outbox. A crash before a terminal no-I/O commit remains `AuthorityReserved` if no Dispatching record committed; any crash after durable `Dispatching` reopens conservatively.
- [ ] Recovery never receives/mints a permit and never writes. For stored `Dispatching`: bounded no-follow full-byte identical final file -> `Succeeded`; absent -> `Indeterminate`; every other observation -> `Indeterminate`. It records only that terminal result. There is no path from `Indeterminate` to a new approval/send/retry.
- [ ] Replace effects production API with an opaque authenticated broker-client façade accepting random intent ID/session operation and returning `{intent_id, outcome}` only. Feature-gate old raw writer/revoke/receipt under `legacy-experimental`; extend CLI's legacy feature to forward exactly `sovereign-effects/legacy-experimental`, while the fixture feature forwards only effects' fixture feature. Default and fixture builds contain no symbols for content/path/digest/bytes callbacks. Because effects becomes a real authority consumer here, add its own feature-gated `protected_payload_boundary` trybuild target/UI source/TSV row; it must import through effects' real feature dependency graph and fail on the missing private payload accessor, not an unresolved authority import. Extend the Task 9 profile-symbol gate to reject raw effect symbols in default/fixture and permit them only in the explicit legacy artifact.
- [ ] Extend feature forwarding only now: authority's fixture feature enables sandbox's parameter-free fixture, effects' fixture feature enables authority, and CLI's fixture feature enables effects. In the same change that adds the broker dispatch/reconcile commands, activate exactly `POST /api/fixture/effect/dispatch` and `/effect/reconcile` in the closed router. Each accepts only `{intent_id}`, requires the full post-session middleware, returns only `{intent_id,outcome}`, and has no interim stub, GET, alias, or query form.
- [ ] Implement the canary scanner as an internal state-aware classifier, not a blanket byte grep. It enumerates every redb logical table and requires canaries only in `fixture_protected_payload_v1`; raw occurrences are allowed only inside the single `authority.redb` file's transactional/freed pages. It walks a fresh runtime root and captured stdout/stderr/HTTP/IPC/evidence/export artifacts, allowing the authenticated preview capture, exact final `.eml`, and at most one owner-only same-directory `.eml.tmp` only for a matching durable `Dispatching`/terminal state. Tests for Prepared, AuthorityReserved, pre-I/O Dispatching, partial-temp, Succeeded, FailedBeforeDispatch, and Indeterminate assert both required copies and absence of any additional table/path/capture/deterministic digest. Source and build trees are not passed to this runtime scanner.
- [ ] Add named fault barriers before/after Dispatching commit, before first write, after first write, temp sync, publication, directory sync, and terminal commit. The release-exclusion script must continue to prove their absence from default/release artifacts.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 11 --phase green -- \
    cargo test -p sovereign-sandbox --test synthetic_exact_effect_fixture \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 11 --phase green -- \
    cargo test -p sovereign-authority --test exact_local_outbox \
      --no-default-features --features owner-effect-fixture,fault-injection \
      --locked -- --test-threads=1
  ./scripts/run-owner-effect-tests.sh --task 11 --phase green -- \
    cargo test -p sovereign-authority --test canary_allowlist \
      --no-default-features --features owner-effect-fixture,fault-injection --locked
  ./scripts/run-owner-effect-tests.sh --task 11 --phase green -- \
    cargo test -p sovereign-effects --test opaque_facade \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 11 --phase green -- \
    cargo test -p sovereign-effects --test protected_payload_boundary \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 11 --phase green -- \
    cargo test -p sovereign-cli --test fixture_loopback \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/check-owner-effect-canaries.sh --all-states --features owner-effect-fixture,fault-injection
  ./scripts/check-owner-effect-authority-plane.sh
  ./scripts/check-owner-effect-boundary.sh
  ./scripts/check-owner-effect-profile-builds.sh
  ./scripts/check-fault-injection-excluded.sh
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-sandbox --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-effects --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-cli --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `feat(authority): dispatch exact fixture files inside coordinator`
- [ ] Push checkpoint 11.

## Task 12: Append only value-free synthetic effect evidence

**Files:**

- Create: `crates/audit-ledger/src/effect_v1.rs`
- Modify: `crates/audit-ledger/Cargo.toml`
- Modify: `crates/audit-ledger/src/lib.rs`
- Create: `crates/audit-ledger/tests/effect_v1.rs`
- Modify: `crates/authority/Cargo.toml`
- Create: `crates/authority/src/effect_evidence.rs`
- Modify: `crates/authority/src/broker/mod.rs`
- Modify: `crates/authority/src/store.rs`
- Create: `crates/authority/tests/effect_evidence.rs`
- Modify: `scripts/check-owner-effect-canaries.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** Separate synthetic fixture evidence chain. Coordinator state remains the sole truth.

- [ ] **RED:** Add `projection_contains_only_allowlisted_fields`, `projection_omits_recipient_content_digest_path_size_time_business_policy_and_reason`, `low_entropy_dictionary_has_no_projection_oracle`, `fixture_signer_is_tagged_non_product`, `signature_and_previous_hash_verify`, `same_intent_outcome_appends_once`, `different_outcome_conflicts`, `crash_after_terminal_before_append_heals_evidence_only`, `evidence_never_advances_effect_state`, and `indeterminate_never_relabels`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 12 --phase red \
    --expected-diagnostic 'SyntheticEffectEvidenceV1' -- \
    cargo test -p sovereign-audit-ledger --test effect_v1 \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 12 --phase red \
    --expected-diagnostic 'terminal evidence is value-bearing' -- \
    cargo test -p sovereign-authority --test effect_evidence \
      --no-default-features --features owner-effect-fixture,fault-injection --locked
  ```

  Expected RED: only legacy value-bearing audit events exist.

- [ ] Add `default = []` and non-default `owner-effect-fixture` to audit-ledger; only authority's fixture feature forwards it. Define a canonical closed projection with only version/type, random event ID, random intent ID, closed outcome, previous-event hash, synthetic fixture signer public identity/public key, event hash, and signature. Use a broker-launch-ephemeral signer tagged `non_product=true`; after restart a new signer may continue the chain and the verifier checks each embedded public identity. This is not Program 1C1 custody, continuity, or product evidence.
- [ ] Append only after terminal coordinator commit, using `begin_immediate_two_phase()` for any broker evidence cursor/index mutation and extending helper instrumentation coverage. On append failure, return `terminal_evidence_pending` and never retry the effect. After fresh fixture login, reconcile a value-free cursor of terminal IDs/outcomes and idempotently append missing evidence.
- [ ] Extend the Task 11 table/path-aware scanner across ledger/evidence/export artifacts. It must positively find the expected private payload-table row and the state's permitted temp/final/preview copies, while proving every non-payload redb table, evidence/ledger record, log/error, non-allowlisted filename/file, value-free HTTP/IPC response, and export fixture contains neither a corpus canary nor its SHA-256. Raw matches inside `authority.redb` remain expected synthetic plaintext because private current/freed pages cannot be distinguished with a file grep; logical table enumeration is decisive. Previous/event hashes are allowed only over the entirely value-free projection preimage and must not equal a canary digest.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 12 --phase green -- \
    cargo test -p sovereign-audit-ledger --test effect_v1 \
      --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-tests.sh --task 12 --phase green -- \
    cargo test -p sovereign-authority --test effect_evidence \
      --no-default-features --features owner-effect-fixture,fault-injection --locked
  ./scripts/check-owner-effect-canaries.sh --all-states --features owner-effect-fixture,fault-injection
  ./scripts/check-owner-effect-authority-plane.sh
  ./scripts/check-owner-effect-boundary.sh
  ./scripts/check-owner-effect-profile-builds.sh
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-audit-ledger --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `feat(audit): project value-free synthetic effect evidence`
- [ ] Push checkpoint 12.

## Task 13: Build the synthetic browser ceremony and cross-port/logout attacks

**Files:**

- Create: `apps/cli/assets/fixture/index.html`
- Create: `apps/cli/assets/fixture/app.js`
- Create: `apps/cli/assets/fixture/styles.css`
- Create: `apps/cli/assets/fixture/i18n.js`
- Create: `apps/cli/assets/fixture/tsconfig.json`
- Modify: `apps/cli/Cargo.toml`
- Modify: `apps/cli/src/fixture_http/mod.rs`
- Create: `apps/cli/tests/fixture_ui_contract.rs`
- Create: `scripts/owner-auth-browser-test.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** A red, unmistakable synthetic-security-fixture UI with no product enrollment/business form and a repeatable exact-origin browser attack suite.

- [ ] **RED:** Add `ui_says_synthetic_unqualified_and_no_real_data`, `ui_has_no_product_setup_import_or_free_text_business_input`, `ui_never_calls_bootstrap_independent_owner_admission`, `ui_uses_exact_four_pre_session_route_schemas`, `ui_auth_start_has_no_cookie_or_csrf_dependency`, `ui_auth_finish_keeps_csrf_only_in_module_memory`, `ui_fetches_post_session_use_exact_origin_cookie_and_csrf`, `ui_never_uses_discoverable_username_less_login`, `ui_requires_synthetic_preview_then_fresh_uv`, `ui_logout_waits_for_server_ack_and_clears_state`, `ui_indeterminate_has_no_retry_or_new_send`, `ui_single_credential_loss_copy_is_destructive`, `ui_warns_cross_port_user_handle_replacement_is_destructive_dos`, and `ui_matrix_label_is_exact_not_generic_webauthnuv`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 13 --phase red \
    --expected-diagnostic 'fixture asset bundle is missing' -- \
    cargo test -p sovereign-cli --test fixture_ui_contract \
      --no-default-features --features owner-effect-fixture --locked
  npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/fixture/tsconfig.json
  ```

  Expected RED: the feature-gated fixture asset bundle does not exist.

- [ ] Use browser WebAuthn only for the frozen entry/virtual label and exact 300000 ms option. Convert base64url locally; load no remote script. Keep CSRF in a module closure only and never store cookie/CSRF/challenge/preview in URL, DOM metadata, local/session storage, service worker, analytics, or logs.
- [ ] Replace Task 9's embedded minimal synthetic warning shell with the new feature-only fixture assets; never reference/embed the existing business UI assets from fixture HTTP, and never embed fixture assets in the default/legacy server.
- [ ] The first screen must state: synthetic fixture, fixed `.example.test` canaries only, empty registry is unqualified first-writer-wins, hostile native bootstrap is not defeated, cookies and RP ID are shared across localhost ports, a malicious second-port WebAuthn ceremony can confuse/replace the single credential after user interaction and destructively lock out the fixture without authorizing port 7787, one credential loss destroys fixture access, deleting the fixture directory does not delete the platform passkey and requires manual credential-manager cleanup, local file is not email delivery, and mechanism evidence cannot enable a product workspace. It has no “set up your workspace,” import, recipient/body editor, or product activation control.
- [ ] UI sequence uses exactly the four Task 9 auth routes: start accepts `{}`, finish sends only challenge handle plus bounded credential, and only finish receives cookie/CSRF. Login always renders the server's one stored-credential `allowCredentials`, never a username-less discoverable flow. Continue with fixed root preview -> fresh UV root creation (if absent) -> fixed synthetic effect prepare -> escaped exact synthetic effect preview -> explicit fresh UV -> one disabled-on-submit approval -> value-free terminal outcome. Root revocation is separate. Refresh discards CSRF and requires login. `Indeterminate` offers only inspect/reconcile after login and forbids retry/new intent guidance.
- [ ] Logout sends the protected POST, waits for acknowledgement, then clears preview/CSRF/UI state and verifies the deletion cookie. If acknowledgement fails, show “server logout unconfirmed; close fixture process,” never claim logged out.
- [ ] Browser script uses the exact origin and runs: all four auth routes and their method/schema negatives; virtual registration/login/preview/approval/file; old cookie+CSRF after logout; concurrent logout/effect; challenge replay/wrong HTTP generation; two approval clicks; idle/absolute/300-second boundary; occupied stable port; malicious second port receiving cookie, overwriting/bombing it, and failing authorization without CSRF; discoverable assertion and same-user-handle create/replacement where supported; resulting credential wrong-origin rejection at port 7787; stored-ID-only login; restart retaining origin but invalidating session. Reports label virtual vs exact real matrix entry and never convert replacement/lockout into a pass.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 13 --phase green -- \
    cargo test -p sovereign-cli --test fixture_ui_contract \
      --no-default-features --features owner-effect-fixture --locked
  npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/fixture/tsconfig.json
  ./scripts/owner-auth-browser-test.sh --virtual --origin http://localhost:7787 \
    --features owner-effect-fixture
  ./scripts/check-owner-effect-authority-plane.sh
  ./scripts/check-owner-effect-boundary.sh
  ./scripts/check-owner-effect-profile-builds.sh
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-cli --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `feat(ui): expose an honest synthetic owner effect fixture`
- [ ] Push checkpoint 13.

## Task 14: Audit and lock the already-structural fixture/product/legacy separation

**Files:**

- Modify: `apps/cli/Cargo.toml`
- Modify: `apps/cli/src/main.rs`
- Modify: `apps/cli/src/ui.rs`
- Modify: `apps/cli/src/workspace/mod.rs`
- Modify: `apps/cli/src/workspace/store.rs`
- Modify: `apps/cli/src/workspace/kernel_exec.rs`
- Modify: `apps/cli/src/workspace/send_workflow.rs`
- Modify: `apps/cli/src/workspace/ops.rs`
- Modify: `apps/cli/src/workspace/legacy.rs`
- Modify: `crates/capability/Cargo.toml`
- Modify: `crates/capability/src/approval.rs`
- Modify: `crates/capability/src/lib.rs`
- Modify: `crates/effects/Cargo.toml`
- Modify: `crates/effects/src/lib.rs`
- Modify: `apps/cli/tests/slice_isolation.rs`
- Modify: `scripts/check-owner-effect-boundary.sh`
- Modify: `scripts/check-owner-effect-profile-builds.sh`
- Modify: `scripts/check-file-size.sh`
- Modify: `scripts/check-owner-effect-broker-build.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** Final adversarial audit of the default/release graph and the two already-separated, non-default, mutually exclusive proof/compatibility modes. This task must not be the first point at which CLI/business isolation becomes true.

- [ ] **RED:** Retain Task 9's isolation regressions and add only the missing final-audit tests `default_binary_cannot_create_import_or_mutate_business_workspace`, `default_binary_cannot_open_read_decrypt_list_preview_or_export_business_workspace`, `default_product_cannot_create_or_accept_core_protected_payload_before_activev2`, `fixture_feature_rejects_product_legacy_activev2_and_unmarked_roots`, `fixture_has_no_product_workspace_marker`, `product_open_rejects_synthetic_fixture_tag`, `legacy_feature_rejects_fixture_root`, `default_and_fixture_have_no_app_approval_signer`, `default_and_fixture_have_no_raw_effect_or_node_writer`, `current_legacy_stage_suite_runs_with_both_opt_ins`, and `final_profile_symbol_manifest_covers_every_public_crate`. Run:

  ```bash
  ./scripts/run-owner-effect-tests.sh --task 14 --phase red \
    --expected-diagnostic 'final profile symbol manifest is incomplete' -- \
    cargo test -p sovereign-cli --test slice_isolation \
      --no-default-features --locked -- --test-threads=1
  ./scripts/check-owner-effect-boundary.sh
  ```

  Expected RED: Task 9's runtime/module isolation already passes, but the final cross-crate metadata/public-symbol manifest has not yet enumerated every crate and legacy-only exception.

- [ ] Verify and tighten Task 9's existing isolation: current business UI/init/status/integrity/mutations, self-signed approval orchestration, placeholder/post-approval composition, and tests remain behind non-default `legacy-experimental` plus explicit `--legacy-experimental`; Task 11 has additionally hidden raw outbox/revoke/digest receipt/delete/retry APIs behind the same profile. Preserve behavior and exact limitations, but do not move any first-line boundary here.
- [ ] Keep `owner-effect-fixture` separate and mutually exclusive at compile time. It contains only exact fixture modules/corpus, directory marker `synthetic-owner-effect-fixture-v1`, and authority-root tag `synthetic_fixture_root_v1`. Neither feature is default. Default/release CLI may verify offline public artifacts and print blocked prerequisites, but cannot initialize/open/decrypt/list/import/mutate/export a business workspace or create a protected payload.
- [ ] Preserve Task 10's typed legacy-only validate-then-claim orchestration without restoring `with_authority_store`, raw claim IDs, or a public trust/store setter. Keep only the pre-existing app signer and raw effects compatibility implementation inside the mutually exclusive legacy module/feature; delete `owner_secret("owner_approval_key")` and signer construction from every default/fixture module. Owner-v1 exact path accepts only `OwnerApprovedInvocation`; local dispatch exists only privately in authority.
- [ ] Boundary script checks Cargo feature graphs and release symbols, and fails on: hidden broker enum/match/entrypoint in the default artifact; any sibling broker binary/packaging dependency; fixture/product marker confusion; fixture code importing Workspace/Vault business types; any free-form recipient/body IPC/HTTP field; `ApprovalRole`/raw trust outside owner or legacy; payload getter/closure/trait; raw node writer; raw effect writer/revoke/receipt; `begin_write(` outside `begin_immediate_two_phase`; direct redb open outside broker/test sentinel; `fault-injection` in release; protected/API GET or unlisted auth route; raw session/CSRF logging or persistence; user-handle logging or persistence outside private `fixture_owner_registry_v1`; generic sandbox verified-byte API; network dependency/socket egress in authority local-outbox/effects; HMAC profile drift; or product-support/activation wording.
- [ ] Compile adversarial external crates against every public workspace crate and prove they cannot construct owner session/approval/root/reservation/dispatch/payload types, call broker-internal commands, or turn a fixture tag/root/evidence into product input.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/check-owner-effect-boundary.sh
  ./scripts/check-owner-effect-authority-plane.sh
  ./scripts/check-owner-effect-profile-builds.sh
  ./scripts/check-owner-effect-broker-build.sh
  ./scripts/check-owner-effect-crypto-profile.sh
  ./scripts/check-fault-injection-excluded.sh
  ./scripts/run-owner-effect-tests.sh --task 14 --phase green -- \
    cargo test -p sovereign-cli --test slice_isolation --no-default-features --locked -- --test-threads=1
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-cli --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-cli --no-default-features --features legacy-experimental --locked
  cargo build -p sovereign-cli --no-default-features --release --locked
  ```

- [ ] Commit: `test(cli): lock synthetic fixture and legacy separation`
- [ ] Push checkpoint 14.

## Task 15: Prove broker crash/race/absence semantics with real processes

**Files:**

- Create: `scripts/exact-effect-kill-matrix.sh`
- Modify: `crates/authority/Cargo.toml`
- Modify: `crates/authority/src/broker/bootstrap.rs`
- Modify: `crates/authority/src/broker/mod.rs`
- Create: `crates/authority/tests/broker_kill_matrix.rs`
- Modify: `apps/cli/Cargo.toml`
- Modify: `apps/cli/src/broker_client.rs`
- Modify: `.github/workflows/ci.yml`
- Create: `.github/workflows/owner-effect-fixture.yml`
- Modify: `scripts/check-owner-effect-canaries.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** Repeated real-subprocess evidence for the selected one-broker model, including the pre-supervisor orphan deadline and parent death before/during/after `BrokerReady`; workload clients never open redb, while one isolated sentinel deliberately attempts and proves direct writable-open denial.

- [ ] **RED:** Make the kill script enumerate and initially fail missing barriers for same-image bootstrap, `after_address_before_supervisor_hello`, `partial_supervisor_hello_before_fixed_deadline`, `after_authenticated_hello_before_lock`, `after_lock_before_redb_open`, `after_redb_open_before_broker_ready`, and `after_authenticated_broker_ready`; supervisor `RegisterConnectionV1`, child credential delivery, cross-connection misuse, supervisor-control EOF/parent death and new-broker lock reacquisition; IPC authentication, each migration/reservation table boundary, pre-pure complete revalidation, pure fixture pre/post execution, final prepared/root/ancestor/policy/session revalidation, `Dispatching` pre/post commit, before first write, partial write, temp sync, publish, directory sync, terminal commit, and evidence append. Add distinct expiry/logout race rows with a write-syscall sentinel and an absent-after-restart assertion expecting `Indeterminate`. Run:

  ```bash
  ./scripts/exact-effect-kill-matrix.sh \
    --iterations 1 --race-iterations 1 \
    --features owner-effect-fixture,fault-injection
  ```

  Expected RED: `missing broker fault barrier/result case` for the first unimplemented matrix row, not a harness parse error.

- [ ] Each iteration creates a new synthetic fixture root, launches the exact feature-built `sovereign` image, and lets it re-exec its broker using bounded piped stdin/non-secret stdout. Release-excluded, value-free barriers let the portable Rust grandparent kill the supervisor parent at each named startup phase without learning the launch key/address. For death after address but before a complete authenticated hello—including a peer dribbling partial frames—the broker must exit by `address_emitted_instant + 5 seconds + 1 second scheduler allowance`; the harness proves `authority.redb` was never created/opened, a fresh lock attempt succeeds, and partial/failed accepts did not move the original deadline. For death after the hello at each pre-/post-lock, pre-/post-redb, and post-`BrokerReady` barrier, the retained supervisor socket must produce EOF, unwind any acquired resource, and permit a new same-image broker within the same bound. The authenticated supervisor then registers an independent scoped/expiring connection and sends it only to the matching child through stdin; real children race IPC and cross-credential attempts. Every prior child credential fails after restart. A second live broker must return `BrokerAlreadyRunning`; a separate lock-bypassing redb sentinel must return `DatabaseAlreadyOpen`; no workload child opens redb.
- [ ] `--iterations` controls every crash row and `--race-iterations` independently controls every concurrent registration/login/approval/root/reservation/dispatch/logout/reconcile row; both reject zero/out-of-range values and the harness rejects an empty/skipped row set. Run crash rows at least 25 times and race rows at least 100 times. Assert one transaction winner, zero partial consumption, zero-or-one exact final file, no automatic broker restart/dispatch retry, and value-free errors. Invoke the state-aware canary scanner after every row: expected private payload/redb pages and state-permitted preview/temp/final copies must be present, while no other table/path/capture or deterministic canary digest is allowed.
- [ ] Exact outcomes: pre-commit reservation absent; committed reservation stable; any post-`Dispatching` crash with absent final file `Indeterminate`; identical file `Succeeded`; all other post-Dispatching observations `Indeterminate`; `FailedBeforeDispatch` appears only from the live no-byte-written proof and never from restart.
- [ ] CI runs default full gates, manifest-enforced nonzero fixture tests for every owner/capability/sandbox/authority/effects/audit/CLI package, separate legacy gates, clean default/fixture/legacy profile command+symbol builds at every checkpoint, metadata/tree/old-authority-API rejection, clean same-image broker build, the fixed monotonic pre-supervisor deadline/no-lock-no-redb tests, real parent-death-before/during/after-`BrokerReady` subprocess rows, the remaining kill matrix, canary/boundary/fault/crypto-profile scripts, virtual browser exact-origin suite, frontend type check, and dependency audit. The migration job admits only `x86_64-unknown-linux-gnu` and proves same-parent no-replace publication, handle-drop order, parent sync, and process termination; macOS/Windows jobs assert `FixtureMigrationUnavailable` and make no power-loss claim. Pin actions/tools; grant no service/network credentials. A real-platform preflight remains separately attended mechanism evidence and never a product release gate.
- [ ] **GREEN:** Run:

  ```bash
  ./scripts/exact-effect-kill-matrix.sh \
    --iterations 25 --race-iterations 100 \
    --features owner-effect-fixture,fault-injection
  ./scripts/run-owner-effect-tests.sh --task 15 --phase green -- \
    cargo test -p sovereign-authority --test broker_kill_matrix \
      --no-default-features --features owner-effect-fixture,fault-injection \
      --locked -- --test-threads=1
  ./scripts/check-owner-effect-canaries.sh --all-states --features owner-effect-fixture,fault-injection
  ./scripts/check-owner-effect-broker-build.sh
  ./scripts/check-owner-effect-authority-plane.sh
  ./scripts/check-owner-effect-boundary.sh
  ./scripts/check-owner-effect-profile-builds.sh
  ./scripts/check-fault-injection-excluded.sh
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ```

- [ ] Commit: `test(security): prove broker exact-effect crash semantics`
- [ ] Push checkpoint 15.

## Task 16: Publish qualification evidence without promoting product use

**Files:**

- Create: `docs/security/synthetic-owner-exact-effect-fixture-qualification.md`
- Modify: `docs/security/owner-auth-mechanism-matrix.md`
- Modify: `THREAT_MODEL.md`
- Modify: `ROADMAP.md`
- Modify: `docs/INDEX.md`
- Modify: `README.md`
- Modify: `scripts/check-owner-effect-rfc.sh`
- Modify: `scripts/check-owner-effect-boundary.sh`
- Modify: `scripts/check-owner-effect-broker-build.sh`
- Modify: `scripts/check-owner-effect-crypto-profile.sh`
- Modify: `scripts/check-owner-effect-canaries.sh`
- Modify: `scripts/check-owner-effect-authority-plane.sh`
- Modify: `scripts/check-owner-effect-profile-builds.sh`
- Modify: `scripts/owner-effect-tests.tsv`

**Produces:** Final evidence/limitations document. It has no status transition that can enable a product workspace.

- [ ] **RED:** Extend documentation contracts to fail if any changed document calls the fixture owner admission, says hostile-native bootstrap is defeated, presents hidden-mode obscurity or a caller-chosen bootstrap key as native admission, omits supervisor-only per-client connection registration or parent-EOF shutdown, leaves `with_authority_store`/old raw claim APIs in default/fixture, describes Task 14 rather than Task 9 as the first CLI/business isolation, claims privacy from a non-consuming/unresolved-import compile test, calls the cookie or RP/user-handle port-isolated, omits destructive second-port credential-replacement DoS, broadens an exact matrix entry, claims macOS/Windows migration or power-loss durability, names a sibling/signed-CLI/extra-FD broker bootstrap, omits exact HMAC/profile evidence, labels synthetic plaintext redb/temp persistence product-safe, claims the canary appears only in preview/final, calls local `.eml` delivery/email send, maps restart absence to `FailedBeforeDispatch`, says 1C0/Program 2 complete, or removes any conjunctive future gate. Run:

  ```bash
  ./scripts/check-owner-effect-rfc.sh
  ./scripts/check-owner-effect-boundary.sh
  ```

  Expected RED: qualification document is missing and existing current-state copy lacks the complete fixture limitations.

- [ ] Record exact commands/results for RFC vectors, browser preflight, cookie and WebAuthn user-handle cross-port residuals, owner/session/logout/four auth routes, hostile-native fixture bootstrap race and valid-direct-bootstrap residual/product-root rejection, clean same-image broker build/bootstrap, fixed monotonic five-second supervisor-establishment deadline with zero pre-auth lock/redb opens, parent death before/during/after authenticated `BrokerReady`, supervisor-only per-client connection registration/cross-connection rejection/parent-EOF exit, exact `File::try_lock` process lock versus direct redb-open denial, Task 9 default/fixture/legacy isolation, Task 10 removal of `with_authority_store`/old public claims plus legacy typed orchestration, actual-consumer compile-fail targets, immediate/two-phase helper coverage, exact HMAC dependency graph, Linux-only migration gate, root lifecycle, post-reservation expiry/policy/logout races, parameter-free sandbox fixture, 25-crash/100-race kill matrix, state-aware canary allowlist, no-network boundary, and legacy compatibility.
- [ ] A passing real browser/platform row may change only that row from `candidate` to `mechanism_qualified_only`. It cannot change fixture/product/root status. A failing row is excluded without dropping Secure, changing port, extending scope to roaming/hybrid, or choosing fallback.
- [ ] ROADMAP keeps 1C0 and Program 2 unchecked for product, and explicitly names: external owner bootstrap admission; localhost RP/user-handle isolation or accepted product threat treatment; owner continuity/second credential/recovery; 1B1; 1C1; 1D `ActiveV2`; later protected exact-payload persistence RFC/review; whole-workspace plaintext closure; per-platform storage/migration durability; and product integration tests. README says the command uses fixed synthetic values, stores them in the named private plaintext redb table/pages and possible publication temp, and writes one local fixture `.eml`; it cannot send email or open a product workspace.
- [ ] **GREEN / final gate:** From a clean environment run:

  ```bash
  ./scripts/check-owner-effect-rfc.sh
  ./scripts/check-owner-effect-boundary.sh
  ./scripts/check-owner-effect-authority-plane.sh
  ./scripts/check-owner-effect-profile-builds.sh
  ./scripts/check-owner-effect-broker-build.sh
  ./scripts/check-owner-effect-crypto-profile.sh
  ./scripts/check-owner-effect-canaries.sh --all-states --features owner-effect-fixture,fault-injection
  ./scripts/run-owner-effect-tests.sh --all --phase green \
    --require-profile no-default-features \
    --require-profile owner-effect-fixture \
    --require-profile owner-effect-fixture,fault-injection \
    --require-profile fault-injection \
    --require-profile legacy-experimental
  ./scripts/check-fault-injection-excluded.sh
  ./scripts/check-file-size.sh
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --no-default-features --locked -- -D warnings
  cargo clippy -p sovereign-cli --all-targets --no-default-features --features owner-effect-fixture --locked -- -D warnings
  cargo clippy -p sovereign-cli --all-targets --no-default-features --features legacy-experimental --locked -- -D warnings
  ./scripts/run-owner-effect-regression.sh -- cargo test --workspace --no-default-features --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-owner --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-capability --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-sandbox --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-authority --no-default-features --features owner-effect-fixture,fault-injection --locked -- --test-threads=1
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-effects --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-audit-ledger --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-cli --no-default-features --features owner-effect-fixture --locked
  ./scripts/run-owner-effect-regression.sh -- cargo test -p sovereign-cli --no-default-features --features legacy-experimental --locked
  cargo build -p sovereign-cli --bin sovereign --no-default-features --release --locked
  cargo build -p sovereign-cli --bin sovereign --no-default-features --features owner-effect-fixture --release --locked
  npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/tsconfig.json
  npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/fixture/tsconfig.json
  ./scripts/owner-auth-browser-test.sh --virtual --origin http://localhost:7787 \
    --features owner-effect-fixture
  ./scripts/exact-effect-kill-matrix.sh --iterations 25 --race-iterations 100 \
    --features owner-effect-fixture,fault-injection
  cargo audit
  git diff --check
  ```

- [ ] Perform final review for dependency cycles; crate-local feature forwarding and nonzero test manifest; clean same-image broker composition/default symbol exclusion; fixed monotonic supervisor-establishment deadline, zero pre-auth lock/redb opens, and parent death before/during/after `BrokerReady`; supervisor-only per-client registration/key/scope/expiry/sequence/revoke and parent-EOF exit; Task 9 early business/legacy isolation at every subsequent checkpoint; Task 10 absence of `with_authority_store`/old raw claims and typed legacy-only orchestration; actual-consumer compile-fail targets; process-lock versus direct redb-open distinction; sole immediate/two-phase helper; migration platform/process-crash claim; exact HMAC graph; sandbox API feasibility; all four auth routes; localhost cookie and WebAuthn user-handle replacement residuals; every post-reservation prepared/root/ancestor/policy/session revalidation; impossible cross-crate privacy; empty-registry overclaim; logout; timeout/matrix breadth; single-credential loss; raw root/effect paths; absent-file recovery; exact runtime canary allowlist; product wording; and all prior review resolutions. Resolve all Critical/High findings and rerun the full gate.
- [ ] Commit: `docs(security): qualify only the synthetic owner effect fixture`
- [ ] Push checkpoint 16. A PR may merge mechanism/fixture evidence but cannot promote or enable product use.

## Acceptance trace

| Required property | Decisive tasks/tests |
| --- | --- |
| Entire slice synthetic and product-ineligible | Tasks 1, 8, 9, 14, 16; `default_binary_cannot_create_import_or_mutate_business_workspace`, boundary script |
| No empty-registry owner-admission overclaim | Tasks 1, 4, 5, 13, 16; hostile-first-winner and valid-direct-bootstrap residual tests plus explicit non-claim |
| Frozen exact browser/origin/Secure-cookie/user-handle behavior | Tasks 1, 5, 9, 13; preflight matrix, occupied/cross-port cookie and credential-replacement tests |
| Stable integrity-bound port with no fallback | Tasks 1, 9, 13; `occupied_7787_never_falls_back`, restart/origin tests |
| Clean feature build launches one same-image broker | Tasks 4, 9, 14, 15; `clean_fixture_binary_reexecs_broker_from_same_image`, build script, default symbol exclusion |
| Portable secret bootstrap/authenticated IPC | Tasks 4, 15; bounded stdin key, non-secret stdout, first authenticated ready, malformed-input failure, valid-direct native residual |
| No pre-supervisor orphan/lock window | Tasks 1, 4, 15; fixed non-extendable monotonic five-second deadline, zero pre-auth lock/redb opens, parent-death before/during/after-ready restart tests |
| Per-client IPC least authority and broker liveness | Tasks 1, 4, 15; supervisor-authenticated registration/revoke, independent key/ID/scope/expiry/sequence, cross-connection rejection, supervisor EOF exits/releases lock |
| One writable redb broker and IPC subprocess contention | Tasks 4, 10, 15; exact OS-held `File::try_lock`, distinct `BrokerAlreadyRunning`/`DatabaseAlreadyOpen`, broker death, real IPC races |
| No direct capability-owned authority plane | Tasks 9, 10, 14, 16; no `with_authority_store`/old raw claim API, typed mutually exclusive legacy orchestration, metadata/tree/symbol and compile-fail gates |
| CLI/business isolation precedes fixture exposure | Tasks 9-16; clean default/fixture/legacy profile command+symbol gate at every checkpoint |
| Exact fixture features and nonzero tests | Tasks 1-16; checked TSV manifest/runner and per-package final feature gates |
| Sole immediate/two-phase write helper and gated migration | Tasks 4, 7-12, 15; helper instrumentation, source boundary, Linux process-crash job, unavailable other targets |
| Exact protocol HMAC profile | Tasks 4, 14-16; `=0.12.1`, no defaults, frozen transitive graph contract |
| No protected-byte cross-crate API | Tasks 8, 11, 14; feature-gated trybuild in each actual authority consumer plus external symbol/source boundary |
| Atomic approval/token/idempotency/root/use/effect reservation | Tasks 2, 10, 15; per-claim expiry, failpoint, race tests |
| Every authority-sensitive post-reservation transition revalidates | Tasks 10, 11, 15; prepared/root/ancestor/policy/session expiry and race tests with no-write sentinel |
| Executable parameter-free sandbox fixture | Task 11; existing private verified primitive, no second consumption/raw bytes/authority dependency |
| Fixture-only root lifecycle/no raw writer | Tasks 7, 14; creation/revocation binding and compile-fail tests |
| Post-crash absence is Indeterminate | Tasks 11, 13, 15, 16; `restart_dispatching_absent_is_indeterminate` and kill matrix |
| CSRF-protected complete logout | Tasks 5, 9, 13; stale cookie/CSRF, concurrent request, grant revocation tests |
| Complete registration/login HTTP surface | Tasks 1, 9, 13; exact four routes/schemas, pre-session security, no API GET/alias/query token |
| Two genuine fault-injection REDs/release exclusion | Tasks 3, 11, 15; harness RED, semantic barrier RED, exclusion script |
| Narrow WebAuthn label/300-second timeout/destructive loss | Tasks 1, 5, 13, 16; exact matrix and UI contract |
| Exact expected plaintext and no extra canary/digest oracle | Tasks 8, 11, 12, 15; table/path/state-aware positive and negative scanner |
| No Vault activation or network email | Tasks 14-16 boundary/docs contracts |
| Legacy remains honest/runnable | Tasks 9-16; early separate feature + runtime opt-in and per-checkpoint clean profile suite |

## Self-review checklist

- Search every use of `product`, `support`, `qualified`, `owner`, `admitted`, and `protected` and verify nearby wording preserves fixture/non-claim/future-gate semantics.
- Confirm no task creates `owner-session-exact-effect-v1` or any product marker, accepts a business value/root, offers a product setup/import/mutation, or permits real-platform evidence to flip a product status.
- Confirm the final graph has owner independent of authority, capability independent of authority, authority depending on both, and effects/CLI above authority; verify Task 10 removes the old edge before adding the inverse.
- Confirm a clean feature build needs only the `sovereign` image, its hidden broker mode is cfg-absent from default/release, bootstrap key travels only through bounded piped stdin, stdout is non-secret, and first IPC readiness proves key possession.
- Confirm the broker captures one non-extendable monotonic five-second supervisor deadline before publishing the address; partial/failed peers cannot reset it, lock/redb open counters remain zero until the supervisor hello authenticates, and parent death before/during/after `BrokerReady` always exits and permits a clean restart.
- Confirm the hidden fixture mode and caller-chosen pipe key are not described as a native admission anchor: a direct same-account caller can supply a valid frame and win/deny only the unqualified synthetic fixture, never construct product admission.
- Confirm every workload subprocess after Task 4 races authenticated broker IPC and never opens redb; the isolated direct-open sentinel alone expects `DatabaseAlreadyOpen`, while second-broker process-lock contention is the distinct `BrokerAlreadyRunning` result.
- Confirm only the authenticated supervisor can register/revoke clients; every client has an independent random 256-bit key/ID, exact scope/expiry/sequence, receives only its own stdin credential, cannot cross-use another connection, and supervisor EOF/parent death exits the broker and releases the lock.
- Confirm the broker lock is the retained Rust 1.97 `File::try_lock()` handle, `WouldBlock` is the only `BrokerAlreadyRunning` mapping, and process death releases it without a stale-file claim.
- Confirm every fixture crate/target declares and forwards only `owner-effect-fixture`, every RED/GREEN Cargo command names `--no-default-features` plus exact features, and the manifest runner proves every named test listed and ran nonzero.
- Confirm `begin_immediate_two_phase()` is the only redb write constructor, every later table/command is covered, HMAC is exact-pinned with frozen features/transitives, and migration is unavailable outside the admitted Linux process-crash profile with no power-loss claim.
- Confirm Task 11 uses sandbox's parameter-free fixed fixture over the existing crate-private verified primitive, never the public raw byte API or the capability-consuming executor, and sandbox has no authority dependency.
- Confirm Task 9—not Task 14—is the first checkpoint with business modules/commands behind mutually exclusive legacy feature plus runtime opt-in, and every Task 9-16 GREEN runs clean default/fixture/legacy command+symbol builds.
- Confirm Task 10 removes `with_authority_store`, `AuthorityStore::open`, and raw claim methods from default/fixture/public symbols; only the upper legacy module can compose pure validation with typed mutually exclusive legacy claims, and metadata/tree proves no `capability -> authority` edge.
- Confirm Task 8's capability/CLI and Task 11's effects privacy cases live in those actual consumer crates with feature-gated targets/TSV rows and intended privacy diagnostics; capability's target is deleted when its authority dependency is removed.
- Confirm registration/login start/finish paths and bounded schemas are exhaustive, pre-session routes still require exact origin/fetch/JSON/generation, start issues no session, finish alone issues cookie/CSRF, and there is no API GET/alias/query-token path.
- Confirm payload/composer/local-outbox are coordinator-private and no public closure/trait/callback can receive recipient/RFC bytes; synthetic preview is a fixed high-level projection, not an accessor.
- Confirm no restart path maps missing file to `FailedBeforeDispatch`; only a live no-byte-written proof may commit it, and any uncommitted/crash ambiguity is `Indeterminate` with no retry.
- Confirm fixture root create/narrow/revoke is complete, generation-bound, approval-consuming, non-product, and has no raw writer.
- Confirm logout is an authenticated CSRF POST that revokes session/challenges/approvals/native grants server-side before exact cookie deletion; normal live-session CSRF reuse is not mislabeled replay.
- Confirm fault barriers require `fault-injection`, two distinct REDs are captured, and default/release graphs/symbols exclude hooks.
- Confirm fixed origin `http://localhost:7787`, host-wide cookie residual, random generation-bound non-PII user UUID, stored-ID-only login, second-port discoverable/replacement destructive DoS, exact 300-second timeout, matrix-specific label, roaming/hybrid exclusion, and destructive single-credential loss are consistent in RFC/code/UI/docs tasks.
- Confirm both post-reservation checks cover prepared expiry, policy epoch/expiry, root and every ancestor expiry/revocation, fixture/origin generation, and session/logout generation; races that lose before Dispatching touch no writer, while expiry after Dispatching cannot cancel/retry/relabel.
- Confirm the canary scanner expects the named private payload table/redb pages plus state-permitted authenticated preview/temp/final copies, positively finds them, and rejects every extra table/path/capture or deterministic digest; it never claims private plaintext redb/temp copies are absent or securely erased.
- Confirm README/ROADMAP never claim email delivery, owner identity, product protection, 1C0/Program 2 completion, or support promotion from this fixture.
