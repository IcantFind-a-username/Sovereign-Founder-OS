# Threat Model v0.1

## Scope

This document describes threats against Sovereign Founder OS and Sovereign Runtime during the Alpha phase. It is a living document and will be updated as the implementation matures.

Mitigations are labelled **Current** when enforced by merged repository code and passing tests, **Target** when required before the relevant capability may ship, and **Research** when requirements or implementation have not been accepted. Target and Research controls are not claims of current protection. Older “Alpha target” wording should be read as Target until migrated.

## Assets to Protect

| Asset | Sensitivity |
| --- | --- |
| Device Unlock KEK handles and platform protector state | Critical |
| Recovery KEK, Argon2id wrapper, and offline age identity | Critical |
| Versioned SQLCipher database keys and wrapper keys | Critical |
| Device identity, privacy-authority, and audit-signing keys | Critical |
| Freshness/rollback anchor state | Critical |
| Customer PII and contact lists | Critical |
| Contracts, invoices, financial records | Critical |
| API secrets and credentials | Critical |
| Audit event ledger | Critical |
| Enterprise graph state | High |
| Agent plans and deliverables | High |
| Provider account and billing metadata (excluding API secrets) | High |
| Plugin manifests and signatures | Medium |
| Public market research data | Low |

## Trust Boundaries

### Trusted (with verification)

- **Current:** deterministic Policy Engine
- **Current:** role-separated signing primitives, publisher/Authority trust stores, and Capability V1/V2 validators
- **Current prototype only:** the Vault AES-256-GCM encrypts each payload with a random 96-bit nonce and atomically replaces files, but its Base64 raw master key is stored beside ciphertext, manifest entry names are visible, and there is no KDF, device protector, backup, key rotation, or rollback anchor. It does not protect against copying the whole Vault directory.
- **Current:** device-signed hash-chain prototype behind an append-only API;
  its replaceable local file has no independent rollback anchor and does not
  prevent deletion
- **Current foundation:** RFC 0003 approval-role-signed evidence — approval-required Capability V2 tokens are issued only with exactly invocation-bound evidence and fail closed without it. Validators are process-local by default. The Workspace attaches persistent filesystem claims whose approval records now retain the verified signed approval expiry; token-expiry purge, store reopen, replay denial, and approval-expiry purge are tested. Independent owner presence, one-transaction reservation, revocation, and a full real-subprocess validator race remain Targets.

### Untrusted (always)

- All LLM outputs and suggestions
- External web pages, emails, PDFs, documents
- MCP server responses
- Third-party plugins
- Python agent workers
- Cloud model providers (for confidentiality, not availability)

### Semi-trusted (constrained)

- **Target:** cloud model providers (availability and inference only; data minimized through the compiler boundary)
- **Target:** secondary storage replicas (ciphertext-only and server-blind)
- **Research:** owned compute nodes are plaintext endpoints only after authenticated E2EE; blind recovery replicas remain a distinct ciphertext-only role

## Threat Categories

### T1: Prompt Injection

**Description:** Malicious instructions embedded in external content attempt to override system policy, exfiltrate data, or trigger unauthorized tool use.

**Mitigations:**
- **Target:** Untrusted Content Zone — external content is data, never instructions
- **Target:** Planner/Executor separation
- **Current:** Policy Engine makes implemented authorization decisions deterministically
- **Current foundation:** Capability V2 scopes one publisher-verified pure-compute invocation exactly
- **Current foundation / Target:** adversarial fixtures exist; the full Alpha gauntlet remains incomplete

### T2: Tool Privilege Escalation

**Description:** An agent or plugin attempts to expand its permissions beyond what was granted.

**Mitigations:**
- **Current foundation:** short-lived Capability V2 binds the exact artifact, operation, input commitments, and resource commitments; validators are process-local by default, while the Workspace attaches experimental persistent claims with the transactional, revocation, subprocess, and owner-ceremony limitations above
- **Current:** Authority and Publisher signing roles are distinct, and an AI agent cannot make its own key trusted
- **Current foundation:** strict publisher manifest enforcement and import-free Core Wasm isolation
- **Target:** durable token revocation, container/micro-VM backends, and reviewed effectful host interfaces
- **Target:** 100% of real tool effects require a valid capability and durable evidence

### T3: Credential Exfiltration

**Description:** An agent, plugin, or compromised model path attempts to read and transmit secrets.

**Mitigations:**
- **Target:** `NonDisclosableSecret` is a distinct handle-only type, not a
  `Protected` value. Device Unlock KEK and device/privacy/audit signing keys,
  recovery/session secrets, permanent authentication credentials, bearer
  tokens, API secrets, and bank login credentials can be consumed only by
  closed core brokers; local-model, public-projection, queue, owned-node, and
  generic tool APIs cannot name or accept their bytes
- **Target:** device/privacy/audit signing keys remain non-exportable handles
  only where the selected platform backend actually provides that property;
  an `OsProtected` backend makes no hardware or non-exportability claim
- **Target:** the Recovery KEK and offline age identity are used only in an explicit recovery ceremony, never kept as an online model/tool root or passed as payload
- **Target:** protected business work may reach only this device or an explicitly authorized user/company-owned compute endpoint over authenticated E2EE; it never reaches a public model as raw data
- **Target:** blind recovery replicas and relays store or transport ciphertext only and are not authorized compute endpoints
- **Target:** agents never hold root keys
- **Current:** both Wasmtime paths expose no filesystem, network, environment, WASI, or other host imports; the only host effect is a rooted local outbox file write performed later by trusted host code after the experimental approval/capability path. Exact recipient/content binding and independently authenticated owner presence are not yet enforced.
- **Target:** broker-derived, signed disclosure evidence for every real cloud model call
- **Target:** output scanning for sensitive patterns

Output scanning is defense in depth and does not replace the handle-only type
boundary, closed broker, or exact recipient authorization.

### T4: Model Provider Failure or Revocation

**Description:** Primary AI provider becomes unavailable, changes terms, or revokes API access.

**Mitigations:**
- **Target:** provider redundancy with an exact ordered provider set approved
  before dispatch. A provider outside that set requires a new exact preview and
  authorization; `Indeterminate` is never retried automatically
- **Target:** local model degradation path
- **Design invariant / Target:** no business-critical state stored only at a provider
- **Target:** workflows recoverable from local checkpoints

### T5: Single Point of Failure and Recovery

**Description:** Failure of one device, cloud, database, or key destroys business continuity.

**Mitigations:**
- **Target:** public Single Point of Failure Registry with documented countermeasures
- **Research:** encrypted multi-device replication requirements; do not implement consensus without a validated need
- **Target:** event-sourced state with signed checkpoints
- **Target:** export and offline recovery without official servers
- **Current limitation:** ordinary JSON export is inspectable data, not a clean-machine backup and not trust continuity
- **Target:** dual-unlock Vault, padded canonical age recipient-mode backup, separately retained public trust-continuity material, and a clean-machine restore drill
- **Target:** recovery creates a new device identity/transport state, advances relevant epochs, and never revives unfinished authority or old session keys
- **Residual risk:** rotating recipients or keys does not make already copied old backups unreadable

### T6: Audit Log Tampering

**Description:** An attacker or compromised agent attempts to alter or delete operation history.

**Mitigations:**
- **Current:** device-signed event hash chain with internal-consistency checks;
  whole-file replacement, prefix truncation, and rollback require an external
  trusted head to detect
- **Current primitive / migration pending:** a role-separated Audit COSE signer exists; the ledger still uses its legacy device-signature encoding
- **Target:** periodic external or owner-device head anchoring when the deployment requires rollback completeness
- **Target:** an Auditor role that cannot execute external actions
- **Target:** tamper detection during recovery validation

### T7: Split-Brain in Distributed Mode

**Description:** Two nodes simultaneously issue conflicting authoritative writes (contracts, payments, permissions).

**Mitigations:**
- **Research:** leader lease with fencing tokens when a multi-writer requirement exists
- **Target:** authoritative vs. eventually-consistent data separation
- **Current foundation:** V2 idempotency and replay checks are process-local by default; the Workspace attaches persistent filesystem claims, tested through reopen and concurrent threads rather than separate OS processes
- **Target:** one transactional authorization bundle, durable revocation, version checks, fencing, and multi-node approval for high-value operations
- **Target:** device removal closes sessions and advances membership/authority state before future dispatch; if a removed device may have cached a DBK or derived data key, future data requires a new data-key epoch
- **Research:** Secure Mesh begins as synchronous one-to-one Noise; MLS is evaluated only for real group/asynchronous requirements

### T8: Supply Chain Attack (Plugins/Dependencies)

**Description:** Malicious or compromised plugin, dependency, or MCP server.

**Mitigations:**
- **Current foundation:** role-separated publisher manifest signature verification and exact artifact digest binding
- **Current:** dependency audit and dependency-review CI checks
- **Target:** SBOM and SLSA-aligned build provenance
- **Current foundation / Target:** adversarial plugin fixtures exist; the full completion gate remains incomplete
- **Current:** pure-compute plugins receive no network, filesystem, environment, WASI, or other host imports

### T9: Memory Poisoning

**Description:** Adversarial content corrupts long-term agent memory or enterprise state.

**Mitigations:**
- **Target:** memory writes validated by an independent checker
- **Target:** authoritative state changes only through signed events
- **Target:** source attribution on all business artifacts

### T10: Vault, Backup, Key Compromise, and Rollback

**Description:** An attacker copies an offline Vault or backup, exploits a weak recovery password or KDF resource setting, confuses key/algorithm/version context, reuses a nonce, restores a valid old snapshot, or retains an old decryptable backup after rotation.

**Mitigations:**

- **Current limitation:** the co-located raw Vault key makes whole-directory theft decryptable; current AEAD integrity does not provide key custody or rollback detection
- **Target:** RFC 0005 dual-unlock hierarchy: a closed Device Key Protector wraps a random 32-byte SQLCipher database key (DBK), while a bounded Argon2id-derived PWK wraps an independent Recovery KEK that separately wraps the DBK; neither unlock root directly encrypts business payloads
- **Target:** a pinned SQLCipher profile provides transactional page/journal encryption, locking, and integrity checks; closed XChaCha20-Poly1305 wrappers bind exact workspace/wrapper/key/KDF/algorithm context with fresh 24-byte nonces and never substitute for the database format
- **Target:** KDF parameters are bounded before allocation; authentication failure never tries another algorithm or legacy key
- **Target:** canonical padded snapshots use standard age v1 recipient mode; ordinary backups exclude device/privacy/audit signing keys, session/ratchet state, and unfinished effect authority
- **Boundary:** age authenticates ciphertext to its recipient but does not
  authenticate a sender. Restore authenticity comes from the recovery-wrapped
  snapshot DBK, authenticated internal manifest, and—when continuity is
  claimed—surviving authority/freshness evidence; otherwise the result is at
  most data rescue
- **Target:** backup eligibility comes only from a versioned core-owned object
  registry; unknown, caller-labelled, mismatched, and secret-bearing object
  kinds default to exclusion and fail snapshot construction
- **Target:** workspace-relative freshness detects a workspace or ledger prefix restored while protected device state survives
- **Research/deployment-dependent:** whole-device rollback requires an external monotonic anchor such as another owner device, hardware counter, or transparency service; an internally valid full snapshot cannot detect its own age
- **Residual risk:** deleting a wrapper or current key cannot erase historical keys and backups already copied by an attacker

## Automation Levels (Risk Control)

| Level | Capability | Examples |
| --- | --- | --- |
| L0 Suggest | Recommendations only | Strategy advice |
| L1 Draft | Generate but not execute | Email drafts, contract drafts |
| L2 Approve-then-execute | User confirms before action | Send email, deploy code |
| L3 Bounded automation | Auto within budget/scope limits | Value-free health and integrity checks |

Financial, legal, and irreversible operations: **maximum L2**.
RFC 0005 Program 1B backup creation is also L2: its independent recovery
wrapper requires an owner-present recovery-password ceremony. Unattended
scheduled backup would require a separately reviewed online backup authority
and key domain; it is not silently treated as L3.

## Out of Scope (Alpha)

- Invasive hardware extraction or side-channel attacks against correctly configured secure hardware
- A fully compromised, unlocked OS/root process or live memory capture. Offline
  database/workspace/backup copying remains in scope; whole-device at-rest
  protection additionally depends on a verified full-disk-encryption,
  encrypted swap/hibernation, and crash-dump policy. Without those deployment
  controls, swap, hibernation, and dump remnants are documented residual risk,
  not a Vault v2 guarantee.
- Full HSM and confidential computing deployments (Research)
- Global legal correctness guarantees

Custom cryptography and incorrect integration are not out of scope. The project prohibits custom primitives, ratchets, KEM hybrids, protocol patterns, cipher suites, nonce/key derivation, and authentication-failure downgrade. Locked libraries, official vectors, lifecycle tests, and integration review are release gates; a library audit is not a security proof for this product.

An unauthenticated local process is not treated as the owner merely because it
can reach a loopback port or run under the same account. Protecting against a
fully compromised OS/root process remains out of scope, but owner-sensitive
enrollment, migration, every business read/decrypt/list/preview/copy/backup/
export, approval, and effect transition require a separately admitted owner-
presence/session boundary before product enablement. Sensitive HTTP responses
use `Cache-Control: no-store`; export is a one-use authorized POST/download,
not a readable GET. Clipboard, download, autocomplete, preview, and browser
cache are plaintext effect surfaces with explicit UX/tests.

## Verification Requirements

Alpha release must pass:

- [x] Current deterministic-policy fixture rejects prompt attempts to self-authorize high-risk actions
- [x] Current import-free Wasm fixtures cannot access filesystem, network, environment, WASI, or undeclared host interfaces
- [x] Current policy fixture rejects Red data sent through a cloud-labelled tool
- [x] Capability V2 rejects same-process replay and idempotency conflicts
- [x] Attached Authority Store rejects covered duplicate claims after reopen and token races across threads
- [x] Attached Authority Store rejects approval reuse after token expiry/purge and store reopen until the signed approval's own expiry
- [ ] Approval reuse and the full reservation remain rejected across real subprocess validator races/restart, with durable revocation
- [ ] Transactional authorization-bundle revocation remains rejected across races and restart
- [ ] Full prompt-injection and data-disclosure paths pass the Alpha gauntlet
- [ ] Primary model failure does not block data access
- [x] Current audit-ledger fixture detects hash-chain/signature modification
- [ ] Recovery works without official cloud servers
- [ ] Vault v2 rejects missing/corrupt device protection without generating a replacement key or reading environment/CLI secrets
- [ ] The pinned SQLCipher build/profile, raw-key binding, memory-only temporary storage, transactional crash/reopen behavior, integrity checks, and wrong-key/corruption cases pass on every supported platform
- [ ] Typed XChaCha20-Poly1305 key-wrapper vectors, AAD/context substitution, nonce/tag/version corruption, parser limits, and no-panic cases pass
- [ ] Device and recovery routes independently unlock the DBK; recovery-record rotation requires an available DeviceKEK, an atomic recovery-commitment/device-wrapper update, and verification of both routes. Bounded Argon2id parameters pass official vectors and minimum-device benchmarks
- [ ] Standard age implementation interoperability, padding buckets, key exclusion, and clean-machine restore/new-identity drills pass
- [ ] Workspace/ledger old-prefix restore is rejected while protected device freshness survives; whole-device rollback remains explicitly unclaimed
- [ ] Future Noise two-node replay, sequence, revoke, re-handshake, metadata, and recovery drills pass before any E2EE product claim

Process-kill, concurrency, filesystem-fault, and recovery gates are tracked in
[ROADMAP.md](ROADMAP.md).

## Reporting

Security vulnerabilities: see [SECURITY.md](SECURITY.md) for responsible disclosure process.
