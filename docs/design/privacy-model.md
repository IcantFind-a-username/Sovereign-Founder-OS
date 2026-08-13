# Privacy Model

> **Status:** Target design, not a statement of current guarantees. Current
> behavior is documented in [ARCHITECTURE.md](../../ARCHITECTURE.md) and the
> relevant RFCs.

## Design Philosophy

We do not claim "absolute security." We pursue:

- **Privacy by design**
- **Local-first as the default product path**
- **Server-blind transport or storage only after verified E2EE exists**
- **Recovery without an official project server as a target**
- **Verifiable, scoped security claims**

Credibility comes from public threat models, code, tests, and audits — not adjectives.

## Maturity Boundary

| Maturity | Honest boundary |
| --- | --- |
| **Current** | The Vault prototype encrypts individual entries with AES-256-GCM, but stores its raw Base64 master key beside the ciphertext and leaves manifest names visible. The model gateway uses deterministic stand-ins. The plaintext JSON export supports inspection and limited verification; it is not backup or restore. There is no real local-model sandbox, multi-device sync, Secure Mesh, or E2EE transport. |
| **Target** | A pinned SQLCipher transactional business store whose random database key is independently wrapped by device and recovery unlock domains, a real capability-constrained local-model process, value-free disclosure evidence, and verified clean-machine recovery using separately handled encrypted business backup and public trust-continuity material. |
| **Research** | Owned Mesh, blind replicas, multi-device E2EE, and whole-device rollback anchors remain candidates gated by measured need, a separate accepted protocol, implementation, and review evidence. They have no current product preset or setup action. |

## Data Classification

The zones below are target policy language. A caller-selected color is never
proof that content is safe to disclose; authoritative provenance and the
boundary in [RFC 0004](../../rfcs/0004-data-sovereignty-boundaries.md) control
placement.

### `NonDisclosableSecret` — Handle Only

- Root and company master keys
- Device, privacy-authority, and audit-signing private keys
- Permanent credentials, recovery secrets, and session keys

`NonDisclosableSecret` is a separate non-model type, not a more sensitive
`Protected` value. These values never become model or owned-node payloads,
even for a verified local model. An authorized core operation receives only an
operation-scoped `OpaqueSecretHandle<Operation>` from the Vault, identity
service, or effects broker. The handle is not secret bytes, has no generic
serialization or display form, and cannot be accepted by a model request.
`Local Only` does not turn a secret or its handle into prompt content.

### Red Zone — This Device by Default

- Government ID documents
- Complete customer lists
- Full financial records and transaction history
- Full private email content
- Original contract documents
- Unpublished trade secrets

**Target rule:** Red business data begins with `ThisDevice` visibility and may
use only an admitted local runtime. Current code does not implement that trust
boundary: it only blocks caller-labelled Red from a provider that does not
self-report `Local`, so the label and provider trust remain bypassable. A
future exact subset may reach one
owner-approved company node only after Secure Mesh becomes implemented and
reviewed, and only under a purpose-, recipient-, operation-, expiry-, and
constraint-bound grant. That future owned-node endpoint can see the plaintext
it is authorized to compute over. Red business data is distinct from
`NonDisclosableSecret`, which no model or owned node receives.

### Amber Zone — Processed Before Cloud Use

- Purpose-minimized customer requirements from a registered transform
- Purpose-limited aggregate business metrics with a reviewed leakage contract
- Owner-reviewed email excerpts containing only exact approved values
- Document excerpts with minimum necessary fields

**Rule:** A registered local transform creates a purpose-bound approved
projection. The user sees the exact proposed values before transmission.
De-identification is not a universal permission and is never claimed without a
defined mechanism and evidence.

### Green Zone — Approved for Cloud Models

- Public market research
- Generic copywriting prompts
- Published product information
- Research questions without personal identifiers

## Data Disclosure Record (Target)

Every supported future cloud-model dispatch must generate a value-free record:

```text
provider          — closed approved provider ID
model             — closed approved model ID
fields_sent       — closed field dispositions and leakage-approved count bucket
classification    — closed policy class
purpose           — closed registered purpose ID
retention_policy  — closed provider-declared/requested retention ID
policy_approval   — opaque policy/authorization ID
time              — leakage-approved coarse bucket, never exact business time
```

Users can review disclosure history in the Founder Command Center. Current
disclosure fields are unsigned demo telemetry and are not this target evidence.
“Value-free” forbids free-text labels and unkeyed digests of guessable business
values. V1 persistent correlation uses only opaque random IDs assigned before
content is known. Keyed/blinded commitments remain blocked Research until a
separate accepted RFC fixes their primitive/profile, encoding, blinding and
NDS key lifecycle.

## Encryption Architecture

### Dual-Unlock Vault (Target)

```text
Unlock domain A: Platform/OS protector ── Device Unlock KEK ───┐
                                                               ├─ XChaCha wrap/unwrap only
Unlock domain B: passphrase ── Argon2id ── PWK                  │
                                      └─unwrap─ Recovery KEK ───┘
                                                               │
                                            random SQLCipher Database Key
                                                               │
                                   pinned SQLCipher business DB + journal

Independent trust and freshness domain
device identity | privacy authority | audit signing | rollback/freshness state
```

Requirements:

- The two independent unlock domains wrap the database key; neither root directly
  encrypts business content.
- A recovery passphrase derives only a password-wrapping key. It wraps an
  independent random Recovery KEK and is not the database key, a signing key,
  or a session key.
- Device identity, privacy authority, audit signing, and freshness state are
  not derived from a Vault data root or copied into an ordinary business backup.
- A recovery ceremony creates new device identity and transport state while
  preserving verifiable business and audit history.
- Workspace-relative rollback detection is a Target. Detecting whole-device
  rollback needs state outside that snapshot and remains Research.
- Keys support versioned rotation and use reviewed libraries and profiles; the
  project does not invent cryptographic algorithms.

The current co-located-key Vault does not satisfy this architecture.

Delivery is deliberately segmented. **Program 1A** provides only the pinned
SQLCipher format, typed key wrappers, native-protector boundary, and a closed
side-by-side legacy importer; it exposes no product enrollment or workspace
migration. **Program 1C0** establishes the independently admitted owner session
and one-use approval issuer. **Program 1B0** proves backup/restore mechanics
over staging fixtures, while **Program 1C1** hands legacy role keys into
separate identity, approval, authority, and audit domains. **Program 1D** then
freezes the final legacy generation, rebuilds from that same snapshot, and
requires a `VerifiedMigration` equivalence proof before `PendingV2`; **Program
1B1** must clean-
restore that candidate and return its bound `RecoveryQualification` before 1D
may publish `ActiveV2`. In v0.1, legacy co-located-key workspaces remain an isolated,
explicitly labelled residual and receive no v2, backup, or recovery claim.

### Identity vs. Encryption Keys (Target)

- **Authentication:** WebAuthn / Passkey
- **Encryption:** Separate key hierarchy

Login compromise must not directly enable data decryption.

### Recovery Artifacts and Ceremony (Target)

Ordinary plaintext export is for supported inspection and portability. It does
not contain the keys, outbox files, authority records, journals, checkpoints,
artifacts, and trust state needed for clean-machine restore and must never be
presented as a backup.

Recovery onboarding creates and tests two separately handled artifacts:

1. an encrypted business backup unlocked through the independent recovery
   domain; and
2. public trust-continuity material sufficient to verify retained history and
   supported workspace-relative freshness without copying old private device,
   signing, authority, or transport-session keys.

Creating a Program 1B1 product snapshot is an explicit owner-present ceremony. The
online device route cannot unwrap the independent Recovery KEK, so the recovery
passphrase is required locally to wrap the new snapshot database key; the age
public recipient alone is insufficient. Unattended scheduled backup is not
part of this profile and would require a separate reviewed authority/key domain.

A restore is complete only after both inputs verify, migrations succeed, a new
device identity is enrolled, and a drill proves that covered work resumes
without duplicating an effect.

The backup uses two independently handled recovery inputs. The loss behavior
must be shown during onboarding and every recovery drill:

| Available inputs | Result for that backup |
| --- | --- |
| age identity + recovery passphrase + admitted trust-continuity material | Verified restore can be attempted; continuity is claimed only after every commitment, migration, and authority-transition check passes. |
| age identity + recovery passphrase, but no trust-continuity material | Business-data rescue under a new trust identity may be possible; verified history, freshness, and authority continuity are not claimed. |
| Recovery passphrase but no age identity | The encrypted backup transport cannot be opened. The passphrase alone cannot recover it. |
| age identity but no recovery passphrase | The outer backup can be opened, but the recovery-wrapped workspace key cannot be unlocked. The age identity alone cannot recover business data. |
| Trust-continuity material without both recovery inputs | Commitments may be inspected, but business data cannot be restored from that backup. |

A still-enrolled device may create a new backup with new recovery inputs. That
is a new recovery artifact; it does not make a missing input for an existing
backup recoverable.

## Local-First Storage

- **Current:** authoritative workspace state is local; individual Vault entries
  are encrypted by the co-located-key prototype, and export is plaintext and
  non-restorable.
- **Target:** protected authoritative state uses the dual-unlock SQLCipher
  Vault, typed encrypted database metadata, versioned migration, and tested offline backup/restore without an
  official project server.
- **Research:** blind storage, multi-device sync, and Owned Mesh may be considered
  only after a measured need and reviewed E2EE protocol. E2EE would hide content
  from relays and blind storage, not from an authorized compute endpoint.

No UI may imply that Owned Mesh is configured or secure today. Until the
Research gates pass, there is no selectable Owned Mesh preset, node-setup CTA,
or simulated secure transport.

## Zero Trust Access

No agent, plugin, or tool is trusted by default — regardless of whether it is "official," "local," or "user-installed."

Every access is evaluated for:

```text
subject       — who is requesting
action        — what operation
resource      — what is being accessed
classification — data sensitivity level
device_trust  — is this device authorized
token_valid   — is the capability token current
risk_level    — computed risk score
approval      — is human approval required
```

## High-Risk Actions (Target: Never Silent)

These operations always require explicit human approval:

- Send email or direct messages
- Publish web or social media content
- Delete files
- Deploy code
- Modify production data
- Sign contracts
- Place orders or payments
- Create ad spend
- Make commitments to customers

The target system generates previews and diffs. The user signs the final
decision.

## Blockchain and Personal Data (Research / default reject)

**Personal data is never written to public blockchains.**

```text
Off-chain:  encrypted contracts, customer data, invoices
On-chain:   only an independently approved public/high-entropy commitment
```

The default architecture does not use a blockchain. A future multi-party case
must pass RFC review and may publish only already-public/high-entropy protocol
digests. Any keyed/blinded commitment needs its own accepted cryptographic RFC
and is not an available v1 construction. It must never publish a naked hash of a name, email, amount, title,
document, action/resource label, or other dictionary-testable value, nor an
exact personal/business event time. Signatures and state commitments are not
automatically public-safe merely because they contain no plaintext.

Deleting off-chain ciphertext and destroying **every** historical decrypting
key, wrapper, backup, and replica can make the protected payload unavailable;
public hashes, timestamps, signatures, and commitments remain permanently
visible and cannot be “deleted by key destruction.”

## Crypto Agility (Target / Research)

- **Target:** authenticated ciphertext headers record explicit format,
  algorithm, and key versions; migration is explicit, never “try another
  algorithm” after authentication failure.
- **Target:** keys and algorithms are separable and support versioned rotation
  and verified batch migration without cross-algorithm key reuse.
- **Research:** post-quantum transition waits for a standardized profile,
  maintained implementation, interoperability evidence, and review. ML-KEM is
  a KEM; ML-DSA and SLH-DSA are signatures—not interchangeable long-lived data
  encryption algorithms. The project will not invent a hybrid construction.

## User-Facing Privacy Indicator (Target)

Every AI task displays its privacy mode:

```text
Privacy: Local only
```

or

```text
Privacy: External projection proposed
The exact approved values listed below would be shared
```

Clicking reveals: exact recipient and approved projection values, purpose,
requested retention (not a provider guarantee), transform/model, failure
behavior, and a local-only alternative if available. The current deterministic
stand-in instead says “On-device fixed-template demonstration; no external
model contacted; not real AI.”

## What We Do Not Promise

- Never leaks
- Absolutely secure
- Unbreakable
- 100% legally correct in all jurisdictions

## Further Reading

- [THREAT_MODEL.md](../../THREAT_MODEL.md)
- [ARCHITECTURE.md](../../ARCHITECTURE.md)
- [RFC 0004: Data sovereignty boundaries](../../rfcs/0004-data-sovereignty-boundaries.md)
- [RFC 0005: Dual-root Vault and recovery](../../rfcs/0005-dual-root-vault-and-recovery.md)
- [Approved data-sovereignty design](../superpowers/specs/2026-08-13-data-sovereignty-boundaries-v1-design.md)
- [Historical sovereignty design](../archive/zh/02-Sovereign-Founder-OS-主权升级.md) (Chinese)
