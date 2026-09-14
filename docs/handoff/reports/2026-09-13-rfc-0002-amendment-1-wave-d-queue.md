# RFC 0002 Amendment 1 proposal + frozen Wave D queue

- **Outcome:** planning / RFC draft only. No product code.
- **Date:** 2026-09-13
- **Base scouted:** `main` @ `aa0c6b2` (after #140)
- **Authoritative plan:** `docs/superpowers/plans/2026-08-14-synthetic-owner-exact-local-outbox-v2-implementation.md` (v2). The v1 owner-session plan is obsolete.
- **Product gates:** RFC 0006 G2 remains conjunctive and is not restated as alternatives.
- **Honest-close:** v0.1 = fixture + honest labels; product 1C0 admission = v0.2.

This note freezes the Wave D ticket queue for a later Composer run. It does
**not** claim RFC acceptance. Copy tickets into `docs/backlog.md` only after
a maintainer accepts Amendment 1 with rationale.

---

## 1. What changed tonight

`rfcs/0002-wasm-sandbox-and-plugin-capabilities.md` only (plus this note):

- Status stays **Draft**; Amendment 1 is **proposed, not accepted**.
- Execution-class table gains a fixture row; product “Low-risk constrained
  plugin” stays Denied.
- Authorization paragraph names Amendment 1 as the exact-effect profile
  amendment RFC 0002 already anticipated, and forbids re-planning approval
  retention (already Current).
- Phase D is explicitly not completed by this profile.
- New Amendments section pins the closed profile, wire-token map,
  reservation, `Indeterminate` state machine, value-free evidence, G2
  non-claims, and the seven-day discussion rule.

Task 0 gate (must stay true on this branch):

```bash
rg -n 'low-risk-effectful|write_rfc5322|Indeterminate' \
  rfcs/0002-wasm-sandbox-and-plugin-capabilities.md
```

---

## 2. Maintainer acceptance checklist

`CONTRIBUTING.md` RFC process: draft PR → **minimum 7 days** for substantial
changes → maintainer acceptance or rejection **with rationale** →
implementation PR references the accepted RFC.

**Accepted** means all of the following, recorded by a maintainer (not by
the implementation branch, not by CI green, not by this planning agent):

1. Seven-day discussion window has elapsed, or the maintainer explicitly
   records why a shorter window is allowed. Do not fake this.
2. Written acceptance with rationale is on the RFC PR (comment or status
   line). Status may then move to “Draft; Amendment 1 accepted
   <date>” — still not a product claim.
3. Closed profile identities remain `low-risk-effectful` /
   `write_rfc5322` / `local_outbox` / `1.0.0` / `sovereign_core_wasm_v2`.
4. Wire map is unchanged: hyphenated profile names do not become a second
   accepted serde spelling; product admission of `low_risk_effectful`
   stays denied until a later, separate product RFC.
5. RFC 0006 G2 is untouched (1B1 + 1C1 + 1D `ActiveV2` + protected-payload
   review stay conjunctive).
6. No product 1C0, no app-signer removal, no Vault `ActiveV2`, no Phase D
   “now Current” language snuck into ROADMAP.
7. v1 broker revival (HMAC IPC, hidden child, second listen port,
   `BrokerReady`) is still named as residue, not as the Wave D target.

**Rejected** with rationale also closes Wave D until a revised RFC is
accepted. Discussion comments are not acceptance.

Optional follow-up after acceptance (not tonight): one-line INDEX /
THREAT_MODEL pointers; a separate RFC 0006 amendment if maintainers want
G13–G15 transport text to match the v2 single-process plan. That amendment
must not touch G2.

---

## 3. Inventory — reusable vs forbidden v1 residue

Scouted on `main` @ `aa0c6b2`. Hypotheses that did not survive the tree
are listed in §6.

### Reusable (library / pattern; do not productize)

| Piece | Where | Use in Wave D |
| --- | --- | --- |
| Empty default crate + `FixtureBootstrap` / no `ProductOwnerAdmission` | `crates/owner` (`publish = false`) | Keep the fail-closed empty default. Reuse as a library, not a product path. |
| Session absolute/idle lifetimes, CSRF independence, `__Host-` cookie shape | `crates/owner/src/session.rs`, `http_guard.rs` | Reuse constants and tests; v2 still wants memory-only session + CSRF. |
| One-credential registry, 300s UV ceremony, first-writer-wins non-admission | `crates/owner/src/registry.rs`, `bootstrap.rs` | Reuse ceremony logic. WebAuthn adapter is still absent (measured lock-file cost in the 2026-09-10 handoff). |
| Loopback origin / cross-port residual tests | `crates/owner/tests/fixture_loopback.rs`, `cross_port_attacks.rs` | Keep; they encode G7/G9 residuals. |
| OS file lock before redb | `crates/authority/src/broker/process_lock.rs` | Reuse the lock. v2 still requires lock-before-redb as the only two-live-process claim. |
| Fixture root marker / `synthetic_fixture_root_v1` classifier | `crates/authority/src/broker/fixture_root.rs` | Reuse classification so a fixture cannot open a product/Vault root. |
| One-transaction reservation *pattern* | `crates/authority/src/broker/reservation.rs` | Reuse the “all or nothing in one redb write” idea. Do **not** treat `ReservationRequest` (raw UUIDs) as the public coordinator API. v2 consumes opaque verified proofs by value. |
| `OwnedStore` open-only-with-`&HeldLock` | `crates/authority/src/broker/store.rs` | Reuse constructor discipline. |
| Approval-retention Current behavior | `crates/authority` + capability tests | Do not reimplement. Carry signed approval expiry into the redb claim. |
| Import-free Core Wasm v2 guest | `crates/sandbox` | Guest stays import-free; coordinator publishes. |

### Forbidden v1 residue (do not extend)

| Residue | Where | Why |
| --- | --- | --- |
| Hidden `__owner-effect-broker` + stdin launch frame | `apps/cli/src/main.rs`, `crates/authority/src/broker/bootstrap.rs` | Superseded. v2: no sibling/hidden child, no `sovereign-cli` default/release fixture entry. |
| HMAC supervisor / connection MAC / second ephemeral TCP listener | `broker/supervisor.rs`, `listener.rs`, `protocol.rs`, `connections.rs` | v2 rejected design. RFC 0006 G13–G15 still describe this; implementing it anyway is forbidden. |
| `BrokerExit::NotImplemented` / `AfterRedbOpenBeforeBrokerReady` | `broker/mod.rs`, `fault_injection.rs` | Leave as leftover. Do not “finish” it into `BrokerReady`. |
| Optional `hmac = "=0.12.1"` on `sovereign-authority` | `crates/authority/Cargo.toml` | Reviewed for v1 IPC. v2 has no HMAC transport. Do not add it to a new default graph. |
| `crates/owner` SHA-256 `OwnerApprovedInvocation` | `crates/owner/src/approval.rs` | Session≠approval idea is right; the digest is **not** RFC 0003 COSE. v2 wants `ApprovalBridge` + `TypedSigner<ApprovalRole>`. Do not promote this type. |
| Experimental app-signed workspace outbox | `apps/cli/src/workspace/` | Track B / honest-close. Not Wave D. Do not bind it to 1C0. |

`apps/cli` already forwards `owner-effect-fixture` to authority and compiles
the hidden broker variant. Wave D must not grow that path. The v2 fixture
package is `publish = false`, not a `sovereign-cli` dependency, and is
excluded from product release graphs.

---

## 4. Frozen Wave D tickets (after RFC accept only)

Do not claim these in `docs/backlog.md` until Amendment 1 is accepted.
One ticket per `/iterate` round. Each item must leave the workspace green
alone. No product path 1C0.

### v01-D01 — Separate verification from consumption

- **P1** | `crates/capability/` (and capability tests only)
- **Depends on:** Amendment 1 accepted.
- Split `CapabilityValidatorV2` into a side-effect-free cryptographic /
  context verifier that returns opaque `VerifiedCapabilityV2` and, when
  required, `VerifiedApprovalV1` (private fields; no public constructor;
  no `Clone` / `Serialize` / `Debug`). Legacy `authorize_and_consume*`
  wrappers call the verifier then perform the current AuthorityStore
  operations with unchanged order, errors, replay, and approval-retention.
- **Done when:** named tests prove repeated pure verification mutates
  neither process-local replay state nor any attached store; compile-fail
  fixtures reject forging/cloning the proof types; existing
  `cargo test -p sovereign-capability --locked` regressions stay green;
  no authority→capability dependency edge.
- **Out of scope:** fixture crate, redb coordinator, WebAuthn, CLI, enum
  activation of `low_risk_effectful` on product issuance.

Suggested commit: `refactor(capability): separate verification from consumption`

### v01-D02 — Release-excluded single-process fixture boundary

- **P1** | new `publish = false` fixture package (name in the v2 plan) +
  boundary script
- **Depends on:** v01-D01.
- One public listener story at the compiled origin later; this ticket
  adds the package, root classifier, retained OS lock, sole redb open,
  and release-graph exclusion only. No owner or effect behavior.
- **Done when:** product `cargo tree -p sovereign-cli --locked` and
  release symbols contain no fixture/redb/WebAuthn; a second real
  fixture process fails on the OS lock before redb open; lock precedes
  the only production redb open; product/unmarked/symlink roots fail
  before listener or database state; source inventory has one listener
  and no internal transport module.
- **Out of scope:** HMAC, hidden child, second port, `apps/cli/**`
  product UI, finishing `run_owner_effect_fixture_broker`.

Suggested commit: `test(fixture): establish single-process owner boundary`

### v01-D03 — Unqualified WebAuthn UV, session, CSRF, signer epoch

- **P1** | fixture package + reuse `crates/owner` session/registry
  primitives as a library
- **Depends on:** v01-D02.
- Closed register/login routes, one-credential registry, mechanism
  matrix row (empty real matrix allowed), memory sessions, middleware,
  logout. After lock and before redb: ephemeral
  `TypedSigner<ApprovalRole>`, random signer epoch, closed
  `ApprovalBridge`. Persist only the labelled public trust record.
- **Done when:** named tests cover first-writer-wins non-admission,
  required UV, exact origin/RP, 300s one-use ceremonies, cookie/CSRF
  independence, absolute/idle expiry, cross-port residuals, logout and
  restart invalidation, no secret bytes in redb/log, historical-key
  verify-only. No effect publication.
- **Out of scope:** `/api/workspace/decide`, product WebAuthn, adding a
  second credential, recovery.

Suggested commit: `feat(fixture): add unqualified owner uv sessions`

### v01-D04 — Seal intent and bridge fresh UV to RFC 0003

- **P1** | fixture package + `crates/capability` issuance of the closed
  profile (fixture feature only)
- **Depends on:** v01-D03 + accepted Amendment 1 profile map.
- Random intent before content access; fixed CRLF RFC 5322 bytes inside
  the coordinator; private sealed payload; synthetic preview.
  `FreshUvGrant` (non-cloneable) → only
  `ApprovalBridge::approve_invocation` emits RFC 0003 evidence. Exact
  Capability V2 claims use wire tokens `low_risk_effectful` +
  `core_wasm` + `write_rfc5322`. Product issuance stays `pure_compute`.
- **Done when:** substitution, header injection, session-alone denial,
  grant replay, epoch mismatch, restart invalidation, and compile-fail
  access tests are red-then-green. `signed_shape` / existing
  `pure_compute` tokens stay byte-stable.
- **Out of scope:** dispatch, reservation transaction, product
  risk_class admission.

Suggested commit: `feat(fixture): bridge fresh uv to exact approval`

### v01-D05 — Reserve every authority fact in one redb transaction

- **P1** | fixture coordinator (upper crate)
- **Depends on:** v01-D01 + v01-D04.
- Sole write-transaction helper consumes the opaque proofs and commits
  approval, token, idempotency, synthetic authority-node use, and
  `Prepared -> AuthorityReserved`. Returns privately constructible
  `AuthorityReservedEffect`. Never call the legacy consuming validator
  first. Carry the already-Current signed approval expiry; do not add a
  second retention path.
- **Done when:** failpoint rollback, process-kill before/after commit,
  same-process reservation race (one winner), idempotent replay,
  different-intent conflict, expiry/logout/signer-epoch races, and
  compile-fail construction/clone/serialize tests pass. Full
  cross-process validator race remains Target.
- **Out of scope:** publish/write `.eml`, product AuthorityStore bundle
  rewrite.

Suggested commit: `feat(fixture): reserve exact authority atomically`

### v01-D06 — Publish once and reconcile conservatively

- **P1** | fixture coordinator + value-free evidence
- **Depends on:** v01-D05.
- Import-free Core Wasm step; consume `AuthorityReservedEffect` by
  value; commit `Dispatching`; publish `<effect_intent_id>.eml` once;
  reconcile without writing; append value-free signed evidence.
- **Done when:** no alternate writer entry; compile-fail reuse after
  move; guest import refusal; crash/reopen identical=`Succeeded`,
  absent/different=`Indeterminate`; old pre-dispatch work →
  `FailedBeforeDispatch` with no automatic re-sign; canary allowlist
  scan is green.
- **Out of scope:** SMTP, network email, product outbox, retry UX.

Suggested commit: `feat(fixture): publish exact local outbox once`

### v01-D07 — Qualification and honest handoff

- **P2** | fixture tests + a limitations note
- **Depends on:** v01-D06.
- Soak: ≥25 process-kill/reopen iterations per crash boundary; ≥100
  same-process concurrent reservation/logout/dispatch races.
  Virtual-browser matrix; attended real mechanism rows separately.
  Limitations note cannot change product status and leaves the full
  cross-process validator race Target.
- **Done when:** v2 plan final gate is green and the note makes no
  email, owner-continuity, 1C0/1C1, Vault, or E2EE claim.
- **Out of scope:** preview tag, ROADMAP “1C0 Current”, Wave E.

Suggested commit: `docs(fixture): qualify synthetic exact-effect evidence`

---

## 5. Forbidden list

### Tonight (this PR / this agent)

- Any product decide/auth code (`/api/workspace/decide`, 1C0 admission).
- Removing `kernel_exec` `owner_approval_key` / the Experimental
  app-signed outbox.
- Activating Vault `PendingV2` / `ActiveV2` or any product Vault
  enrollment/migration.
- Starting Wave D implementation (capability split, fixture package,
  WebAuthn pin, redb coordinator).
- Marking RFC 0002 or 0006 Accepted.
- Weakening RFC 0006 G2.
- Editing `docs/backlog.md` so an unattended Composer can claim D01
  before acceptance.
- Amending RFC 0006 G13–G15 in this PR (separate discussion; do not
  smuggle it in).
- “Finishing” the v1 broker (`BrokerReady`, HMAC IPC, hidden child,
  second port).

### Wave D (after acceptance)

- Product 1C0 / WebAuthn on the workspace decide path.
- Treating fixture enrollment as owner admission.
- Product issuance of `low_risk_effectful` or guest host interfaces.
- Accepting both hyphenated and underscored risk_class/backend
  spellings, or parsing a dotted profile string.
- Re-planning approval retention.
- Inverting capability↔authority dependencies as hidden fixture work.
- Adding the fixture package to the `sovereign-cli` default/release
  graph.
- Network email, OAuth, encrypted backup, Mesh / multi-device E2EE.
- A `v0.1` **product** git tag (preview tags are Wave E / owner policy).
- Claiming Phase D, Program 2, or RFC 0004 product boundary complete.

---

## 6. Discarded hypotheses

1. **“Wave D must finish the HMAC hidden broker because RFC 0006 G13–G15
   are frozen.”** Discarded. Honest-close and the v2 plan forbid
   continuing that residue. G2 is the product-enablement freeze; G13–G15
   are stale transport. A later RFC 0006 amendment may retire them. Do
   not implement v1 to paper over the inconsistency.
2. **“`crates/owner` `OwnerApprovedInvocation` is the RFC 0003
   bridge.”** Discarded. It is a SHA-256 binding digest, not
   COSE/Ed25519. Keep the session≠approval invariant; replace the crypto
   with `ApprovalBridge`.
3. **“`ReservationRequest` is the Wave D coordinator API.”** Discarded
   as a public API. The one-transaction idea is reusable; v2 requires
   consume-by-value opaque proofs and a private `AuthorityReservedEffect`.
4. **“`backend = core-wasm` is a new Wasmtime backend.”** Discarded. It
   is the existing `core_wasm` guest. No second engine.
5. **“`RiskClass::LowRiskEffectful` means the profile is already
   admitted.”** Discarded. The variant exists so unknown classes do not
   silently parse as `pure_compute`; admission already fails closed
   (`artifact_admission.rs` pins `json!("low_risk_effectful")` →
   `UnsupportedRiskClass`). Wave D must not flip that on the product path.
6. **“Wave D is required to close v0.1.”** Discarded. Honest-close:
   Wave D is forbidden until G2 and is not required for labelled
   Developer Preview. Prefer Wave E for founder-visible close.

---

## 7. Recommended first Composer ticket

**Tonight, after this PR is open: none.** The RFC edit *is* v01-G2 /
v2 Task 0. Do not start v01-D01.

**After maintainer acceptance (not before):** v01-D01 — separate
capability verification from consumption. It is the only Wave D item
that is a pure refactor of Current code, leaves the workspace green
without a fixture package, and is the prerequisite every later D ticket
needs.

If a Composer session is opened on this PR before acceptance, the only
allowed follow-up is editorial RFC/docs clarification. Zero product or
fixture code.
