# Security Architecture Program

**Status:** Approved sequencing record; each implementation slice requires its own detailed TDD plan, independent review, commit, and remote push.
**Date:** 2026-08-13

## Goal

Build the smallest trustworthy foundation that lets Sovereign Founder OS run complete founder workflows locally, then safely use purpose-limited external compute without turning the Trust Layer into a second product.

## Hard dependency graph

```text
Vault format/custody engine (1A) ───► backup mechanics (1B0) ─────┐
Owner authenticator/session (1C0) ─► 1B1/1D owner ceremonies ─────┤
Owner role-key custody/handoff (1C1) ─────────────────────────────┼──► 1D PendingV2
Workspace secrecy closure ────────────────────────────────────────┘        │
                                 exact real clean restore (1B1) ◄──────────┘
                                                   │
                                                   └──► 1D ActiveV2

Owner authenticator/session (1C0) ─► Exact Effect (local outbox first)

Credential Broker + Exact Effect (local outbox first) ───────────┐
Privacy pure boundary ─► verified local-model worker ─────────────┼──► integrated founder workflows
1B1 + 1C0 + 1C1 + ActiveV2 + Credential + Exact Effect + Privacy ──┴──► real network/provider enablement
```

The pure privacy types, compiler, authority bug fixes, and local-model worker
protocol can be developed without product Vault activation. Persistent privacy
authority, freshness state, queue ciphertext, protected founder data, and real
dispatch cannot be integrated until 1D. A real local model does not need the
effect protocol because it has no network capability, but a founder workflow
may give it protected data only after the complete workspace persistence
inventory passes 1D. Every real public provider, owned node, or named-recipient
action additionally needs the privacy boundary appropriate to its data and the
Exact Effect Protocol appropriate to its side effect.

## Program-wide rules

- Keep the current independent-consultant walking skeleton runnable after every
  slice: capture a customer need, produce an outreach/proposal draft, and review
  it. Legacy mode may prepare its honestly labelled local outbox artifact until
  migration. Activated v2 mode disables outbox preparation unless the Exact
  Effect slice has passed; preserving a known post-approval composition gap is
  never a compatibility requirement. The synthetic consultant Playground is
  additive and standalone: it must not replace the current Experimental UI,
  export, disclosure, integrity, or Workspace paths before an authenticated
  product router exists. Preserving those paths is continuity, not a security
  endorsement. Security setup remains an advanced readiness view until product
  activation is safe.
- Use Current, Target, and Research exactly as defined in the design documents. Never promote a feature because an RFC or mock exists.
- Follow strict red-green-refactor. Capture the failing test before production code, then run focused and workspace gates after the minimal fix.
- Give each slice a fresh implementer, specification review, security/code-quality review, and fix loop until no known Critical or High finding remains in scope.
- One commit per independently reviewable slice. Push immediately after fresh verification. Do not add assistant attribution.
- Add readers/version parsers before writers, migrate callers before closing old entry points, and preserve an explicit rollback path until the new format is verified.
- No real public egress, owned-node dispatch, or named-recipient network effect
  is enabled until 1B1 recovery qualification, 1C0, 1C1, Program 1D
  `ActiveV2`, Exact Effect, Credential Broker where applicable, and Privacy all
  pass.
- Never invent cryptographic primitives, protocol patterns, ratchets, KEM hybrids, suites, key derivation, nonce strategies, or authentication-failure fallbacks.

### Chosen Target reference-slice design

- `broker` names a logical coordinator and authority boundary. It does not
  require a child process, internal listener, or private transport protocol.
  The Target synthetic owner/exact-outbox reference slice will use one process
  with one public loopback listener; that process will own WebAuthn/session
  state, the coordinator database, and the local outbox. New process isolation
  requires a concrete threat, an established transport, and a separate RFC.
- The Target reference executable will be `publish = false`, excluded from
  product releases, and absent from `sovereign-cli`'s dependency graph. It will
  exercise only fixed `.example.test` fixture mechanisms. It will not create a
  product owner, open a product Workspace, satisfy 1C0, complete Program 2,
  send email, or establish E2EE.
- Before its reservation transaction, the Target slice will separate
  side-effect-free Capability V2/RFC 0003 verification from current
  `AuthorityStore` consumption. An upper fixture coordinator will consume
  opaque verified proofs and will be the only constructor of the writer's
  private, non-cloneable, non-serializable, move-only reservation handle. The
  legacy pure-compute wrapper will preserve current behavior; dependency
  inversion remains separate future work.
- After taking its OS lock, the Target fixture will create an ephemeral
  approval-role signer and random signer epoch. A closed approval bridge will
  consume one-use, exact-bound fresh-UV grants; only public historical trust
  records will persist. Restarted pre-dispatch work will not be re-signed, and
  restarted `Dispatching` work will reconcile only to `Succeeded` or
  `Indeterminate`.
- Existing approval claims now retain the verified signed approval expiry.
  Tests cover token-expiry purge, store reopen and replay denial while the
  approval remains valid, and purge at approval expiry. The token,
  idempotency, and approval claims remain ordered filesystem operations rather
  than one transaction; revocation, a full real-subprocess validator race, and
  independently admitted owner presence remain unfinished.
- Product continuity is an invariant for every slice: the current UI and
  consultant walking skeleton remain runnable. The Target synthetic teaching
  and security fixtures will use separate commands, routes, state, and
  maturity labels.
- Target race qualification will use concurrent requests/threads inside the
  one lock-holding process plus real process kill/reopen. A second live process
  will test only lock denial before redb open; a full cross-process validator
  race remains unfinished.

## Program 1A: Vault v2 format, custody, and migration engine

**Normative design:** RFC 0005.

Deliver a versioned dual-unlock Vault engine with a closed device-key
protector, independent Argon2id recovery path, a pinned SQLCipher transactional
store keyed by a random DBK, typed XChaCha20-Poly1305 wrappers for that DBK and
Recovery KEK, fail-closed unversioned-v1 migration engine, and explicit
rotation. Reusing SQLCipher/SQLite's reviewed page encryption, HMAC,
transactions, locking, and crash recovery keeps custom cryptography and
multi-file atomicity out of the business-data store. This is a cryptographic,
database-profile, platform-adapter, and migration foundation. It is not yet a
product enrollment, backup, recovery, or workspace-protection claim.

Program 1A lives in the separate `publish = false`
`sovereign-vault-v2-engine` workspace crate. The shipped CLI and legacy Vault
have no dependency edge to it; the exact SQLCipher 4.14/OpenSSL graph is absent
from product release trees and binaries. It is a binary-first dedicated
process: a protocol-only library exposes no DBK/connection API, `main` creates
the private OpenSSL process-owner before dispatch, and every database open
requires that owner. Task 1 qualifies only exact native
`x86_64-unknown-linux-gnu` through the sanitized uncached wrapper; Task 5 must
separately name and qualify any additional triple. Program 1D may add a narrow
broker/dependency edge only after the newer exact SQLCipher profile and
process boundary receive RFC review.

Exit gate:

- no v2 API, writer, protector, migration output, or release-test seam creates
  or accepts a co-located raw `vault.key`, environment/CLI secret, machine-id
  key, or silent software fallback;
- a copied Vault directory without its external protector or recovery ceremony is not decryptable;
- missing/corrupt protection never creates a replacement key for existing ciphertext;
- an absent `vault.format` plus the exact current root-level v1 layout is
  recognized read-only; the engine imports it into a side-by-side encrypted
  SQLCipher database transaction and verifies/reopens it without moving legacy
  files. Program 1A has no selector writer, including in product-callable
  fixtures. Only a later Program 1D transaction may create `vault.format`,
  after 1C has handed off every legacy role key and 1D has classified the
  complete workspace;
- the importer rejects identity, admission, approval, authority, audit-signing,
  credential, session, or unknown entries as business DB rows. In particular,
  current legacy owner/approval/runtime-authority entries are a 1C dependency,
  not `BackupEligible` data;
- the existing product remains explicitly Experimental and no enrollment or
  migration mutation is exposed before 1B0, 1C0, 1C1, and 1D;
- device and recovery routes each unlock the DBK without deriving the other.
  Wrapper rotations preserve and verify both routes; a recovery-record change
  requires an available DeviceKEK to update the recovery-slot commitment and
  device wrapper atomically, rather than silently destroying recovery;
- headless Linux without an admitted TPM2 or available system credential store fails closed;
- whole-device rollback remains explicitly unclaimed.

## Program 1B: Encrypted backup and clean restore

In a separate `sovereign-backup` plan, serialize a filtered, authenticated,
backup-eligible business snapshot—not an unfiltered Vault directory—pad it,
and encrypt it with standard age v1 X25519 recipient mode. Excluded identity,
authority, audit-signing, session, and unfinished-effect state stays outside the
business Vault or is omitted by a separately verified recovery manifest.

Program 1B0 mechanics and every product activation remain blocked on the Program 1A
SQLCipher 4.14.0 dependency. They may begin only after an RFC amendment pins
one exact reviewed released Rust binding and exact SQLCipher release no older
than 4.17.0, and that profile passes the dependency, runtime-version,
authorizer, interoperability, and platform gates; or after a separate owner-
approved exact-source profile passes equivalent
supply-chain and integration review. The known-vulnerable API remains
unreachable even after upgrade.

The v1 backup is an explicit owner-present ceremony: creating its independent
snapshot DBK wrapper consumes the recovery password locally because the online
device route cannot unwrap RecoveryKEK. Unattended backup is not part of this
profile and needs a separately reviewed online backup authority/key design.
The builder also consumes a one-use, exact-bound `BackupAuthorization`.
Program 1B is split so tests cannot qualify themselves:

- **1B0 — mechanics:** the filtered builder, parser, age interoperability,
  loss matrix, and clean-machine harness may use a crate-internal issuer and a
  fixture staging binding. This proves only the mechanism.
- **1B1 — real-workspace qualification:** during Program 1D, the owner session
  exclusively freezes the last legacy generation, rebuilds the candidate from
  that retained read snapshot, and consumes the same-transaction
  `VerifiedMigration` before admitting externally authenticated `PendingV2`.
  An earlier staging candidate whose source differs is never admitted. The
  real product backup route consumes
  the recovery password and a 1C0 authorization, restores that exact candidate
  on a clean environment, and emits a `RecoveryQualification` bound to its
  workspace/database ID, activation epoch, schema/registry, platform profile,
  source-head commitment, and backup commitment. Only a matching 1B1 result may
  advance the external binding to `ActiveV2`; failure removes or safely resumes
  `PendingV2` while legacy remains the sole authority.

Neither synthetic fixtures nor an already `ActiveV2` workspace are needed to
break this dependency: 1B0 precedes activation, and 1B1 is a required step
inside the PendingV2→ActiveV2 transaction.

Minimum recovery gate (required before any real network effect):

- encrypted business backup, offline age identity, recovery-password path, and
  separately retained public trust-continuity/freshness material all pass a
  clean-machine drill;
- restore creates a new device identity and transport state and never revives
  pending authority/session secrets;
- history signatures are verified; authority continuity is claimed only with
  an admitted surviving authority, otherwise the result is data rescue;
- stale/rollback rejection is claimed only relative to a surviving external
  freshness anchor; whole-device rollback remains Research;
- an interrupted backup/restore never publishes partial state and the minimum
  headless restore drill is automated on every enabled platform class.

Alpha recovery gate (v0.5):

- a non-technical test user can create, verify, rotate, and restore a backup
  through progressive onboarding without handling raw key bytes;
- cancellation, wrong-factor, missing-factor, stale-backup, and data-rescue-only
  states have tested plain-language UX and accessibility.

Complete recovery UX gate (v0.6):

- scheduled verification, retention, device-loss/revocation, migration, and
  disaster-recovery drills have documented SLOs and support boundaries;
- recovery completeness remains relative to a separately retained freshness
  anchor; whole-device rollback without such an anchor stays Research.

## Program 1C: Owner identity, session, and ceremony authority

Vault enrollment is a security-sensitive state transition. Same-origin
loopback HTTP, an unlocked OS account, and knowledge of a newly chosen recovery
password are not owner authentication. Define a separate identity-domain RFC
and plan before product activation. It must not derive signing keys from the
DBK or store them as ordinary Vault business objects.

Program 1C has two independently reviewable slices but exactly one owner-
authority plane:

- **1C0 — owner authenticator/session:** admitted owner authenticator,
  expiry-bound/CSRF-resistant session, and exact one-use approval issuer. It is
  a v0.1 and Program 2 prerequisite, and later issues Program 1B1/1D ceremonies.
  Program 2, Vault, CLI, and plugins may consume this authority but MUST NOT
  create an app-local owner signer, parallel session, or alternate ceremony.
- **1C1 — role custody/handoff:** device/privacy/audit and legacy approval/
  admission/runtime-authority key protection, rotation, continuity/reset, and
  invalidation. It is a Program 1D prerequisite but need not block the earlier
  exact local-outbox proof once 1C0 passes.

1C0 exit gate:

- a supported platform provides an admitted owner-authenticator or
  administrator-provisioned equivalent with an honest capability label;
- the loopback UI has an authenticated, expiry-bound owner session and
  request-forgery protection; local native callers cannot synthesize it;
- every business-value read, decrypt, list, preview, copy, backup, and export
  path—including `/api/workspace` and `/api/export` replacements—requires that
  live session or a narrower one-use broker authorization. GET/status/open
  paths are non-mutating and never enroll, create a key, or disclose protected
  state merely because the OS account or loopback origin is accessible;
- the Exact Effect approval issuer is the same 1C0 authority, and compile/API
  tests prevent a second app-created owner key or same-account bypass;

1C1 exit gate:

- enrollment/migration consumes one exact one-use authorization bound to
  workspace, operation, policy, format/suite, wrapper commitments, and expiry;
- device identity, privacy authority, and audit signing have distinct custody,
  rotation, loss, and recovery semantics; they are not DBK descendants and are
  not included in an ordinary business backup;
- legacy `owner_admission_key`, `owner_approval_key`,
  `runtime_authority_key`, and `device.json` material is never bulk-imported
  into the business DB. A role-specific handoff provisions protected signer
  handles, updates trust, invalidates the old key, verifies the transition, and
  only then permits legacy-key retirement;
- unavailable owner presence fails closed. There is no same-account, terminal,
  environment, command-line, or recovery-password-only bypass.

## Program 1D: Product storage activation and legacy closure

Inventory every persistent file produced by the founder workflow. Move
business values into typed Vault objects, make durable evidence value-free or
store its sensitive display material separately under Vault protection, and
declare narrow owner-approved effect artifacts such as the local outbox.

Exit gate:

- supported new workspaces enroll v2 before their first persistent business
  write; existing unversioned workspaces are read-only until owner-authorized
  side-by-side migration succeeds;
- no supported product writer creates or accepts `vault.key`, and no v1/v2
  split-brain version is ever published;
- an owner-authenticated external registry—not `vault.format`, a path, or the
  sidecar—binds expected workspace, format, activation epoch, and database ID.
  Deleting/rolling back the marker or retaining legacy after ActiveV2 is
  terminal and never a downgrade path; activation uses a crash-recoverable
  PendingV2→ActiveV2 protocol with no dual writer;
- entering `PendingV2` first exclusively freezes the last legacy generation,
  rebuilds the candidate from that retained read snapshot, and consumes an
  unforgeable same-transaction `VerifiedMigration` binding frozen source and
  candidate content commitments. Any change since an earlier staging build
  invalidates and rebuilds it. Before `ActiveV2`, the product executes Program
  1B1 against that exact pending database and accepts only a matching
  `RecoveryQualification`; synthetic 1B0 evidence cannot satisfy this gate;
- a closed core registry, not a caller flag, assigns object type and backup
  disposition; unknown/unregistered types are excluded and fail closed;
- whole-workspace canary tests cover the Vault, ledger, journals, checkpoints,
  exports, outbox, temporary files, caches, logs, and crash artifacts. Each
  allowed plaintext artifact has an exact purpose, owner authorization, and
  retention rule;
- ledger/evidence tests prove no low-entropy business value, action/resource
  label, exact timestamp/count, or deterministic unkeyed digest creates an
  offline dictionary or linkability oracle. V1 persistent correlation uses
  only pre-content opaque random IDs. Keyed/blinded commitments are blocked
  Research until a separate accepted RFC fixes the maintained primitive,
  domain/encoding/blinding, audit-domain NDS key lifecycle, vectors, and
  dictionary/linkability tests;
- all business reads/decrypts/previews/copies/backups/exports require the live
  owner session or a narrower one-use grant. Sensitive responses are `no-store`;
  export is an authorized one-use POST/download, and clipboard/download/
  autocomplete/browser-cache behavior is inventoried as a plaintext effect;
- product copy names separate measured states. `PendingV2` makes no protection
  claim. After 1B1 and `ActiveV2`, it may say `Vault v2 active; recovery
  qualification passed` and separately report the latest backup artifact as
  `absent`, `stale`, or `verified`; it never collapses mechanism qualification
  and current-backup freshness into one success state. Independent
  identity/audit/freshness limitations remain visible;
- only after 1B1, 1C0, 1C1, and this gate pass may the existing v1 writer be
  retired and v2 become the default on an enabled platform.
- the local outbox path is enabled only after Program 2 binds the exact RFC 5322
  recipient and content bytes before owner approval and durable authority
  reservation. Until then, v2 product mode disables outbox preparation rather
  than carrying forward the current post-approval composition gap.

## Program 2: Authority + Exact Effect Protocol v1

Deliver a versioned `EffectIntentV1` and minimal single-parent authority forest. A trusted broker canonicalizes exact payload and target into opaque handles. One durable coordinator atomically reserves approval, capability/token, idempotency, authority node, and effect state.

Program 2 depends on Program 1C0 and consumes its one-use owner approval. It
does not issue, persist, or infer owner presence itself, and it cannot accept
the current application-generated signer as independent human evidence.

The first local-outbox slice needs no provider credential. Before any real
network adapter is admitted, a separate closed Credential Broker slice owns
every OAuth refresh token, API credential, bank/payment credential, and similar
`NonDisclosableSecret`. It stores them in a role-specific OS/hardware-protected
domain—not the business SQLCipher DB, model context, logs, ordinary backup, or
effect payload—and exposes only operation-scoped opaque handles to an admitted
broker. Enrollment and reauthorization consume owner presence; each handle is
bound to provider/account, audience, scopes, operation, device/workspace,
expiry, and revocation state. Refresh, narrowing, rotation, revocation, provider
disconnect, device loss, and reconnect have explicit state transitions and
audit commitments. Recovery creates no credential continuity: the owner must
reauthorize on the new device. A model, plugin, caller, or provider adapter
cannot create a trusted handle or read credential bytes.

Required state machine:

```text
Prepared ───────► AuthorityReserved ───────► Dispatching
  │                       │                    ├─► Succeeded
  └─► FailedBeforeDispatch└─► FailedBeforeDispatch
                                               └─► Indeterminate
```

Exit gate:

- real provider dispatch uses a closed `ProviderTargetV1` tuple containing the
  exact provider identity, adapter artifact digest, endpoint origin/audience,
  account/tenant, credential-handle ID, model ID and immutable model/version
  descriptor. Canonical encoding and authenticated coordinator storage are fixed by the future
  real-egress RFC; arbitrary strings cannot construct the tuple. A protected
  coordinator assigns a pre-content random `provider_target_id`, seals the
  tuple immutably under authenticated state, and never reassigns that ID;
- preview, `PendingPublicJob`, Program 1C0 approval, Capability V2, and durable
  reservation bind one exact ordered `provider_target_id[]` plus its immutable
  coordinator state, exact bytes, broker operation, resource, policy, runtime,
  expiry, and idempotency. Each dispatch attempt resolves exactly one unchanged
  tuple from that list.
  A tuple change or identity outside the list needs a new preview and approval.
  Before entering `Dispatching`, the broker may advance within the list only
  after it proves `FailedBeforeDispatch` with zero exposed request bytes. Once
  `Dispatching` is durable, observation can close only as `Succeeded` or
  `Indeterminate`; exposure or uncertain exposure makes
  timeout/error/drop/crash terminal `Indeterminate`, with no retry or failover;
- signed/value-free evidence projects only the random target ID, outcome, and
  provider/model fields independently classified approved-public by the
  registry. Account, tenant, credential-handle, private endpoint/audience, and
  every sensitive/linkable tuple field remain in protected coordinator state;
  no plaintext or deterministic digest of them enters the ledger;
- for the first outbox slice, canonical RFC 5322 bytes and the exact recipient
  are composed before approval in protected coordinator state. The coordinator
  first assigns a pre-content random `effect_intent_id`, then seals those exact
  bytes, recipient, operation, policy and retention under that immutable ID.
  Approval, Capability V2, reservation, and write bind the ID plus the same
  protected state; document ID or later composition is insufficient. Signed
  value-free evidence carries only the random intent ID, closed outcome, and
  independently approved-public fields—not recipient/content plaintext or a
  deterministic digest. Offline public proof of exact content is blocked until
  the separate commitment RFC exists;
- child authority only narrows rights, scope, lifetime, and uses; parent revocation invalidates descendants;
- crash injection at every state boundary has a deterministic recovery result;
- concurrent requests have one reservation winner;
- entering `Dispatching` never triggers an unsafe automatic retry;
- the local outbox is the first and only migrated effect; raw production write APIs are closed afterward;
- real-network tests prove credential substitution, audience/scope widening,
  refresh replay, revoked handles, cross-workspace/account reuse, backup/export
  inclusion, and device-loss restore all fail before dispatch;
- the audit ledger is a signed evidence projection, not the transaction coordinator;
- the existing pure-compute execution journal remains version-compatible.

The approval-retention/purge bug was fixed by commit `6c0259b`: durable
approval claims retain the verified signed approval expiry, with token-expiry
purge/store-reopen and approval-expiry regressions. This does not make the
three ordered claim writes transactional, add revocation, prove a full
real-subprocess validator race, establish owner presence, or justify storing
new secrets before 1D. Exact Effect may be built against deterministic
fixtures before 1D, but product activation and real dispatch remain blocked.

## Program 3: Data Sovereignty Boundary v1

Keep `sovereign-privacy` small and foundational: provenance-bound values, visibility algebra, immutable policy inputs, deterministic projection compiler, value-free manifest, strict response parser, and opaque compiler products. Authority, key lifecycle, ledger append, and durable freshness orchestration stay in their existing owner crates or a thin kernel adapter.

Exit gate:

- raw protected model requests are local-only and no public/provider API accepts them;
- arbitrary dynamic text cannot acquire trusted source provenance or a public scope;
- one fixed-slot no-network demonstration produces a byte-stable projection and local rehydration under strict topology;
- evidence and preview semantics are value-free where persisted and exact only in transient local preview;
- `AutoProtect` and `LocalOnly` are the only selectable presets; Owned Mesh stays non-executable Research;
- local failure cannot create a public request;
- persistent runtime/freshness/queue integration occurs only after 1D and
  sensitive transitions use live authority checks;
- real egress remains disabled until Exact Effect Protocol also passes.

The earlier monolithic privacy plan is superseded by a revised subsystem plan before implementation. Its useful test matrices are retained; privacy-owned identity, authority, Vault, ledger, and effect transaction responsibilities are removed.

## Program 4: Verified real local model

Define a closed admitted local-model worker protocol with fixed executable/model/config digests, length-bounded framed IPC, empty ambient environment, no network, no ambient filesystem, read-only model handles, isolated working directory, resource ceilings, and process-tree termination. Add one real backend only on platforms where the confinement tests actually run.

Exit gate:

- the prompt is sent only after admission and every confinement control succeeds;
- unsupported platforms return `LocalModelUnavailable` and never fall back to a native/in-process/cloud provider;
- model output remains untrusted and cannot mutate authoritative company state;
- artifact mismatch, environment/home/file access, network, child-process, output-flood, hang, and crash fixtures all fail safely;
- one founder drafting workflow uses the real local model without opening the professional/security UI;
- the deterministic stand-in remains clearly labelled as a demonstration and a safe fallback.

## Later research gates

- **Noise Phase 1 Research candidate:** only after a validated synchronous
  one-to-one owned-node use case may the fixed candidate
  `Noise_XX_25519_ChaChaPoly_BLAKE2s` profile enter a separate RFC and become
  Target; implementation then needs two-node replay/revoke/re-handshake/recovery
  drills and honest endpoint visibility.
- **MLS Research:** considered only for three-or-more-member, formal Add/Remove, or asynchronous-delivery requirements. It is not a transparent Noise upgrade.
- **Whole-device rollback Research:** requires a state anchor outside the restored snapshot. An internally valid full-device snapshot cannot prove its own freshness.
- **PQC Research:** no custom hybrid. Adopt only a standardized profile with a maintained implementation and migration evidence.

## Reporting after each program

Report the founder-visible outcome first, then the trust boundary it proves, tests and attack reviews, exact commit and remote branch, known limitations, and the next independently testable slice. Never call a Target or Research feature complete, secure, production-ready, or enterprise-grade.
