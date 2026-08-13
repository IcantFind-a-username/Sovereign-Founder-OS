# RFC 0004: Data Sovereignty Boundaries

**Status:** Draft; approved implementation target
**Stage:** 2
**Security impact:** Critical
**Implementation maturity:** Privacy boundary Target; deterministic providers are Current test stand-ins; Vault v2 Target in RFC 0005; Owned Mesh and Secure Mesh Research/Implementation None until a measured use case and separate accepted protocol RFC; MLS Research

## Summary

Sovereign Founder OS must protect business secrets, personal data,
credentials, private model context, and authoritative company state while
supporting both fully local AI and safe acceleration on low-power devices.

This RFC replaces caller-declared confidentiality routing with a typed,
provenance-bound boundary. Raw protected model requests are local-only.
User/company-owned nodes may receive protected work only after an
authenticated end-to-end encrypted Secure Mesh exists. A public model may
receive only an opaque, purpose-bound projection compiled locally from a
registered transform. Beginner presets and professional controls compile to
the same immutable policy contract.

The detailed rationale, UX, data flows, failure semantics, and attack matrix
are in
[`docs/superpowers/specs/2026-08-13-data-sovereignty-boundaries-v1-design.md`](../docs/superpowers/specs/2026-08-13-data-sovereignty-boundaries-v1-design.md).
This RFC is the normative target for the implementation plan when summaries
disagree. Current code remains the source of truth for protections that ship.

## Current gap

The current model gateway accepts a public `ModelRequest` whose prompt,
task, and `DataClass` are caller-controlled. A provider implementation also
self-reports whether it is local or cloud. The gateway explicitly permits an
Amber request to reach a cloud-labelled provider. These are useful Stage 2
stand-ins but they do not satisfy this RFC: a buggy caller can place protected
data in an Amber request and a provider can claim local trust.

Implementation MUST close that route before presets or UI claim this RFC's
protection.

The current Vault is also a cryptographic serialization prototype rather than
a production at-rest boundary: each entry uses AES-256-GCM, but the Base64 raw
master key is stored beside its ciphertext, manifest names are visible, and no
password KDF, platform key protector, backup, rotation, or rollback anchor is
implemented. RFC 0005 is a prerequisite for persistent privacy authority,
freshness secrets, or encrypted queue claims. The ordinary JSON export is not
a clean-machine backup.

The current deterministic model providers are routing stand-ins, not real AI.
A normal native process does not become trusted because it is local or listens
on localhost. A real model may receive protected input only after a core-owned
sandbox profile, authenticated IPC, artifact provenance, and resource limits
are verified.

## Normative vocabulary

The keywords MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are interpreted as
requirements for the supported Sovereign Runtime API boundary.

### Sensitivity

- `NonDisclosableSecret`: root/private key material, permanent credentials,
  recovery secrets, session keys, or equivalent values that are never model or
  owned-node inputs.
- `Protected`: raw or derived business secret, personal data, private content,
  or unknown dynamic value. This excludes `NonDisclosableSecret`.
- `RestrictedDerived`: transformed or externally generated content that may
  still reveal protected facts.
- `PublicContent`: fixed or authoritative published content with trusted
  provenance.

Unknown values MUST enter as `Protected`. Legacy caller-supplied Amber and
Green values also enter this boundary as `Protected`. `DataClass` MAY remain
as a compatibility/display field for existing capability and effect paths,
but it MUST NOT grant egress. It MAY only narrow a decision. The existing
policy engine's `cloud.*` denial remains defense in depth rather than the
public-inference authorization boundary.

Unknown values start with `ThisDevice` visibility and no transform or
declassification grant. `NonDisclosableSecret` MUST be represented outside
`TrustedValue<T>` and every model request type. Root/company/device private
keys, permanent credentials, recovery secrets, and equivalent non-exportable
secrets MUST never become model input or an owned-node payload. An authorized
core operation receives only a private, operation-scoped
`OpaqueSecretHandle<Operation>` from the vault, identity service, or effects
broker. The handle MUST NOT expose secret bytes, implement generic
serialization or value-bearing display/debug, or satisfy a model-input trait.

The compatibility Red display state compiles to `ThisDevice` by default. Only
after Secure Mesh becomes implemented and reviewed may a distinct signed grant
authorize an exact subset of Red business data for one owned-node identity,
purpose, operation, expiry, and constraint set. It never applies to
`NonDisclosableSecret`.

An approved transform MUST NOT relabel its source value as generally public.
It MAY produce an opaque public-compute job bound to one exact purpose,
recipient class, policy snapshot, projection, and expiry.

### Visibility

Visibility is a set of authorization tuples:

```text
(recipient, purpose, operation, expiry, constraints)
```

Recipient classes are `ThisDevice`, `OwnedMesh`, `NamedRecipient`, and
`PublicComputeProjection`. They are not a linear trust order. Grant B narrows
grant A when recipient, purpose, and operation match, B expires no later than
A, and B's constraints are at least as restrictive. Scope B narrows scope A
when every grant in B is subsumed by a grant in A. A transform creates a
derived value and a new scope; it does not widen the source value.

### Placement

Reserved placement values are `Local`, `OwnedNode`, and
`PublicProjection`. `Queued` is a `PlacementDecision`, not an executable
compute location. Policy authorization, observed runtime capability, and
executable placement MUST be distinct. Before Secure Mesh is implemented,
`OwnedNode` MUST NOT be executable or configurable. It returns only a
value-free “not available in this version” diagnostic or a local queue option.

## Required architecture

### Foundational privacy crate

A foundational `sovereign-privacy` crate MUST own:

- opaque trusted values and source provenance;
- preset and professional policy snapshots;
- purposes, visibility tuples, and transform registry;
- exposure previews and value-free manifests;
- local tasks and opaque pending public jobs;
- job expiry, binding, one-use attempt state, and route evidence;
- strict public-response validation and local rehydration.

The crate MUST NOT depend on the model gateway, policy engine, workflow, UI,
or provider networking. `sovereign-policy` and `sovereign-model` MAY depend on
it; privacy MUST NOT import them, and model MUST NOT import policy.

`sovereign-privacy` MUST NOT own, persist, or construct
`NonDisclosableSecret`. Secret bytes and operation handles belong to the Vault,
identity service, or effects broker that performs the authorized operation;
those handle types MUST expose no conversion into `TrustedValue<T>`,
`ModelRequest`, or a public-compute job.

### Local model boundary

Raw `ModelRequest` MUST be accepted only by `LocalModelProvider` instances.
Local provider identity and task identifiers MUST come from closed or
authoritative registries. A trait method or caller string MUST NOT establish
local trust.

The product MUST distinguish a core-reviewed built-in deterministic component,
a verified sandboxed local model, and an ordinary untrusted local process. Only
the first two may receive protected raw input. Provider flags, addresses, and
process location MUST NOT establish either trusted state. The Current
deterministic demo still runs through legacy caller labels and is not evidence
that this RFC boundary ships. Failure MUST return local alternatives and MUST
NOT change the confidentiality route.

### Public projection boundary

Only the privacy compiler may construct a pending public job. Its request
bytes MUST be canonical, size-bounded, and bound to:

- a job nonce and expiry;
- registered task, template, purpose, and transform versions;
- immutable policy snapshot and revocation epoch;
- exact outbound bytes and output topology;
- sealed/omitted/public dispositions and commitments;
- policy-eligible recipient/model classes and retention constraints; and
- the exact ordered closed provider identities approved for this job and
  Exact Effect authorization.

Classes establish eligibility only; they do not authorize a recipient. A
provider outside the approved ordered set requires a new local preview and
Program 1C0 approval. `Indeterminate` is never retried automatically, and a
retry policy cannot silently substitute another identity.

For real egress, a closed `ProviderTargetV1` identity tuple binds provider ID,
adapter artifact digest, endpoint origin/audience, account/tenant,
credential-handle ID, model ID, and immutable model/version descriptor. The
real-egress RFC MUST fix its canonical encoding and authenticated coordinator
storage before any
adapter is enabled. The protected coordinator assigns a pre-content random
`provider_target_id`, seals the immutable tuple under that ID in authenticated
state, and never reassigns it. Preview and live owner UI may resolve the tuple;
job, 1C0 approval, Capability V2, reservation, and dispatch bind the ordered
target IDs and coordinator state. Changing any tuple field creates a new ID,
recipient, preview, and approval.

Signed/value-free evidence does not serialize the protected tuple. It carries
only the random target ID, attempt outcome, and provider/model descriptors that
an authoritative registry independently marks approved-public. Account,
tenant, credential-handle, private endpoint/audience, and other sensitive or
linkable fields stay in protected coordinator state; neither their plaintext
nor a deterministic digest enters the ledger.

Public request bytes MUST become visible to an adapter only inside a closed,
recipient-bound dispatch operation. A dispatch record MUST exist first.
Callers MUST NOT be able to borrow the safe request and invoke multiple
adapters manually, manufacture attempt completion, or forge route evidence.

In the first implementation, the compiler, deterministic provider, broker,
attempt state machine, and evidence finalization all live inside
`sovereign-privacy`; `sovereign-model` may re-export only its high-level closed
gateway. No cross-crate public transition API exists. This stand-in has no
network capability and is process-local. A future real egress crate requires
a separate RFC and an unforgeable, authority-bound cross-crate broker design;
it MUST NOT reuse publicly callable manual attempt transitions.

### Response boundary

Provider output is untrusted and `RestrictedDerived` by default. A parser
MUST enforce UTF-8, total bytes, nesting depth, node count, recursive duplicate
keys, unknown fields, and exact registered topology before any rehydration.
Schema validity MUST NOT make content public. Combining any protected slot
with the response makes the result `Protected`.

The rehydrated visibility scope is the intersection of every sealed input
scope, the validated response scope, and the registered output-contract
scope. An empty intersection rejects. Rehydration cannot add authorization
tuples or trigger a widening transition. Provider bytes MUST NOT influence
recipient, purpose, placement, policy, capability, or authority.

### Evidence and preview

`ExposurePreview` is a local transient view that MAY show exact projection
values to the authorized user before dispatch. It MUST NOT implement generic
serialization or value-bearing `Debug`.

The beginner preview MUST answer, without protocol vocabulary: where the task
runs, who can see it, exactly what leaves this device, and what happens if the
route fails. A deterministic no-network stand-in MUST be labelled as local
deterministic processing of the task's customer data, not “cloud-assisted” or
real AI. The
professional view additionally shows purpose, transform/template, exact
recipient, exact approved projection values, sealed/omitted categories without
values, policy version/expiry, requested retention, fallback, output taint,
and evidence status. Preview failure MUST block dispatch.

Persisted `ExposureManifest`, disclosure evidence, logs, errors, metrics, and
receipts MUST be value-free. Their identifiers must be closed public values
or opaque local IDs. Evidence MUST be derived from job/broker state rather
than caller fields. Provider error or response excerpts MUST NOT be included.

## Presets

V1 exposes two selectable presets:

1. `AutoProtect` (default): prefer local, permit public compute only for a
   compiler-created projection, otherwise queue or return safe alternatives.
   It does not pre-authorize a future owned-node identity.
2. `LocalOnly`: no model or owned-node network dispatch for the task.

`OwnedMesh` is a reserved non-executable policy vocabulary and a Research
product concept, not a selectable v1 preset. Until Secure Mesh passes its RFC,
two-node, revocation, recovery, and independent-review gates, APIs MUST reject
activation and the UI MUST offer no fake configuration action. A future
activation binds exact member identities in a new signed policy transition;
it cannot inherit a grant from an older `AutoProtect` snapshot. Public compute
is unconditionally disabled while that future preset is selected; adding a
projection grant changes the effective mode and displayed name to `Custom`.

Professional controls refine the same policy. They MUST NOT expose a raw
cloud bypass. A refinement that changes a preset's security meaning MUST be
identified as `Custom`, not continue to display the stronger preset name.
Widening visibility or explicit declassification is a separate, previewed,
signed, purpose-bound transition.

Every policy decision binds an immutable snapshot. Ordinary edits affect new
decisions. Existing short-lived jobs retain the snapshot they were created
under unless a distinct revocation epoch is advanced.

The effective policy is the intersection of a non-overridable safety baseline,
the selected preset, and professional restrictions. Professional mode may
select or restrict pre-registered sources, transforms, purposes, providers,
and nodes, but MUST NOT register trust. Narrowing is freely expressible. A
widening requires a separate owner-reviewed diff and signed policy transition.

A snapshot binds its canonicalization version, workspace and issuer,
monotonic generation, registry digest, and revocation epoch and is stored under
authenticated protection. Security-narrowing transitions and emergency
revocation advance that epoch. Expiry, authority, revocation, registry
membership, and recipient capability are revalidated before every dispatch,
failover, and rehydration. Later widening MUST NOT retroactively authorize an
old job.

Named-recipient effects MUST upgrade and reuse the existing
prepared-invocation, Capability V2, durable authority, journal, and
effects-broker primitives. The current application-created signer is not
human-presence evidence. Owner approval MUST come from Program 1C0's
independently admitted, expiry-bound, one-use authority. This RFC binds
content, recipient, purpose, and policy commitments into that chain; it does
not create a parallel execution authority.

Every real public-provider or owned-node dispatch is likewise an external
effect authorized above the privacy crate. It binds job, request, recipient,
policy, transform, and expiry commitments into the existing Capability V2,
durable Authority Store, and execution journal. Privacy may own opaque
compiler state but MUST NOT become a second approval issuer or durable replay
authority. The first deterministic no-network stand-in remains explicitly
non-effectful and process-local.

Before any real network or named-recipient effect ships, a separate versioned
Exact Effect Protocol MUST bind canonical effect bytes, recipient/provider,
resource handle, policy, approval, capability, idempotency, runtime identity,
and broker operation into one durable state machine. It MUST reserve authority
atomically and distinguish `Succeeded`, `FailedBeforeDispatch`, and
`Indeterminate`. The signed audit ledger is an evidence projection, not the
transaction coordinator.

The protected coordinator assigns a pre-content random `effect_intent_id` and
stores the canonical bytes, recipient/provider, and other exact state
immutably under it. Approval, capability, reservation, and dispatch bind that
ID plus the protected state. Signed value-free evidence projects only the
random intent ID, closed outcome, and independently approved-public fields; it
does not contain recipient/content plaintext or a deterministic digest of
guessable values. Offline public proof of exact content requires the separate
commitment RFC and is not a v1 claim.

`FailedBeforeDispatch` is available only when broker-owned state proves that no
request byte became visible to an adapter. Only then may policy advance to the
next identity in the exact approved order. Once bytes are exposed—or their
non-exposure cannot be proved—timeout, provider error, panic, drop, crash, or
missing acknowledgement is terminal `Indeterminate`: no automatic retry and no
failover, even to another pre-approved provider.

## Compute failure

Insufficient compute MUST NOT weaken the policy. The v1 scheduler SHOULD try,
in authorized order, deterministic local computation, compatible small local
model, configured full local model, and an available public projection. If none
can run, it MUST return `Queued` or a closed `ComputeUnavailable` set such as
install/configure a local model, reduce the task, or wait. `OwnedNode` remains
an internal non-activatable Research type and is not a configure action. A
local-required task MUST NOT be convertible into a public job.

Queued work contains only an opaque handle to authenticated-encrypted vault
state, never a plaintext prompt or protected workflow JSON/log entry. Queue
records require expiry, cancel/delete, policy/revocation binding, and explicit
crash semantics. An initial process-local queue MUST be labelled non-durable.

Persistent queue encryption and freshness secrets MUST use the RFC 0005 Vault
v2 boundary. The current co-located-key Vault MUST NOT be used to promote a
queue, privacy authority, or runtime MAC key to a production protection claim.

## Recovery boundary

The ordinary workspace export, an encrypted business backup, and trust
continuity material are different artifacts. The current export is not a
restore mechanism. A Target clean-machine ceremony restores and verifies
business state, verifies retained public ledger/freshness commitments, creates
a new device identity and transport state, advances the relevant epochs, and
requires owner approval before external effects resume. It MUST NOT restore old
device/privacy/audit signing private keys, session/ratchet state, or unfinished
effect authority. Missing trust-continuity material may permit data rescue under
a new identity, but MUST NOT be presented as continuous verified history.

For one existing backup, the loss matrix is normative:

| Available inputs | Permitted claim |
| --- | --- |
| age identity + recovery passphrase + admitted trust-continuity material | Attempt verified restore; claim continuity only after all commitment, migration, and authority-transition checks pass. |
| age identity + recovery passphrase, without trust-continuity material | At most data rescue under a new trust identity; no verified history, freshness, or authority continuity. |
| Recovery passphrase without age identity | No restore: the backup transport cannot be decrypted. |
| age identity without recovery passphrase | No business-data restore: the recovery-wrapped workspace key cannot be unlocked. |
| Trust-continuity material without both recovery inputs | Commitments may be inspected; business data cannot be restored. |

A surviving enrolled device MAY create a replacement backup with new recovery
inputs. That does not recover an existing backup whose required input was lost.

## Local consumer isolation

Device locality is not trust. A raw-input consumer MUST be core-reviewed
deterministic code or execute behind a capability-constrained boundary with no
ambient network, filesystem, environment, or unapproved IPC. Arbitrary native
plugins or models do not receive protected input merely because they run on
the same device.

Transform definitions are signed, version-pinned registry objects bound to
code/schema digests, closed output metadata/topology, resource and I/O limits,
declared determinism, and security-review status. Registry mutation uses a
different authority from workflow execution and professional policy editing.

## Cryptography and Secure Mesh

**Current prototype limitation:** the co-located raw Vault key does not protect
against copying the Vault directory, and no Secure Mesh transport exists.
Neither “encrypted at rest” nor E2EE may be shown as a completed product state.

**Target Vault v2:** RFC 0005 pins a SQLCipher build and profile for the
transactional business database and gives it a random 32-byte database key
(DBK). A closed device-key protector wraps the DBK through the device domain.
A bounded Argon2id password-wrapping key wraps an independent random Recovery
KEK, which separately wraps the same DBK. Typed XChaCha20-Poly1305 wrappers
authenticate exact format, algorithm, workspace, wrapper, key-version, KDF,
and wrapped-key context; XChaCha is not used to invent a database format.
Device identity, privacy authority, audit signing, and freshness/rollback state
are separate trust domains. Removing a wrapper does not revoke a device that
cached the DBK; removal advances the relevant data-key epoch and rewrites data
under a new DBK before future access is claimed revoked.

**Target backup/recovery:** a transaction builds a new recovery SQLCipher
database from a closed core-owned backup registry; it is not a byte copy of the
live store. The canonical package is padded before standard age v1
recipient-mode encryption to a dedicated offline identity. Recovery decrypts
and verifies the package, unlocks the Recovery KEK through Argon2id, opens the
filtered database, creates a new device identity and transport state, rewraps
the DBK, and advances restore/authority/membership epochs. Ordinary backups
exclude old device/privacy/audit signing keys, session state, and unfinished
authority. Age is a backup format, not a live Vault or session protocol; no
custom age plugin is permitted.

Snapshot creation is owner-present in this profile. The device route cannot
derive or unwrap RecoveryKEK, so creating the new snapshot DBK wrapper consumes
the confirmed recovery password locally and zeroizes derived material. The age
public recipient alone cannot create a restorable snapshot. Unattended backup
needs a separate reviewed online backup authority/key design.

**Research Secure Mesh Phase 1 candidate:** only after a measured synchronous
owned-node requirement and a separate accepted RFC may this become Target. The
current candidate synchronous one-to-one profile, subject to that review, is
`Noise_XX_25519_ChaChaPoly_BLAKE2s`. Owner-verified static keys and the
workspace, profile/version, membership epoch, authority/revocation epoch,
policy digest, session, attempt, direction, and monotonic sequence are bound to
the prologue/transcript or authenticated envelope. A session is newly
handshaken or strictly bounded by jobs. Noise `Rekey()` has no fresh DH entropy
and MUST NOT be described as continuous post-compromise recovery; compromise
or revocation requires a fresh handshake or re-pairing. The protocol provides
transport confidentiality/authentication, not business exactly-once semantics.

**Research Phase 2:** RFC 9420 MLS/OpenMLS is evaluated only after a real need
for three-or-more members, formal Add/Remove, or asynchronous delivery. It is
not a transparent upgrade from Noise and MUST NOT be approximated with a
home-grown group layer. PQXDH/Double Ratchet, PQ MLS, and HPKE are Research
inputs; HPKE alone is not a replay, session, or post-compromise protocol.

An authorized owned-node endpoint sees the plaintext categories it computes
over. E2EE hides them from relays, transport, and blind storage; it does not
hide them from that endpoint or an unlocked compromised OS. Packet lengths,
timing, endpoints, and access patterns remain metadata unless a versioned
padding/batching policy explicitly covers them.

AEAD and signed chains cannot detect restoration of an entire device to an old
internally valid snapshot without an external surviving monotonic anchor.
Workspace-relative freshness is Target; whole-device rollback protection is
Research/deployment-dependent.

The project MUST NOT invent an AEAD/KDF, truncate tags, reuse a key across
algorithms, derive roots ad hoc, try another algorithm after authentication
failure, create a custom Noise pattern/ratchet/HPKE hybrid/MLS suite/PQ hybrid,
or use age as a mutable database or session protocol. Algorithm/profile changes
are explicit authenticated format migrations, never silent fallback.

## Required security tests

Implementation MUST include:

- arbitrary secret values in trusted-looking fields and every metadata string;
- transformation/formatting/hash/encoding/nesting taint propagation;
- public compile-fail proofs for trusted constructors, attempts, adapters, and
  evidence;
- cross-job and replay substitution of request, response, slot, policy, plan,
  and receipt;
- one attempt per unique provider identity across repeated gateway calls;
- expiry before dispatch, during health, during response, and before failover;
- recursive duplicate keys and exact byte/depth/node/topology boundaries;
- identical fixed-slot projection payloads for different protected values and
  lengths; transport envelopes may differ only in declared value-independent
  nonce/time fields, and evidence comparison uses a defined normalized
  value-independent view;
- canary scans over Debug, Display, errors, logs, CLI output, serialization,
  metrics, and decoded JSON;
- a fully local workflow with zero public/owned-node observations;
- local compute failure that cannot create or reach a public request;
- the deterministic stand-in labelled as on-device demonstration rather than
  real AI or cloud assistance;
- all persistent privacy/queue secrets blocked until their applicable RFC 0005
  custody, recovery, owner-authority, and product-activation gates pass, with
  no co-located-key or environment-variable fallback;
- explicit migration/version parsing with no authentication-failure downgrade;
- future Secure Mesh replay, revoke, re-handshake, sequence, metadata-padding,
  and endpoint-visibility tests before any E2EE claim.

Every implementation slice requires an independent adversarial review. A
release claim requires no known Critical or High finding inside the declared
supported-API threat model and all repository gates passing. This is not an
absolute-security claim.

## Rollout order

1. Implement RFC 0005 Program **1A**: pin the SQLCipher format, typed DBK
   wrappers, native-protector boundary, and closed side-by-side legacy importer
   without exposing product enrollment.
2. Implement Program **1C0**: one independently admitted, expiry-bound owner
   session and one-use approval issuer. Then implement Exact Effect for the
   local outbox using that same authority. The application-created signer does
   not satisfy this gate.
3. Implement Program **1B0** filtered-backup/restore mechanics over staging
   fixtures and Program **1C1** role-key custody and handoff. They may proceed
   in parallel with step 2; 1B0 cannot qualify a product workspace.
4. Implement Program **1D** owner-authorized activation: exclusively freeze the
   final legacy generation, rebuild the candidate from that same read snapshot,
   and require an unforgeable `VerifiedMigration` binding frozen source to
   candidate content before creating `PendingV2`. Execute Program **1B1**
   through 1C0 to clean-
   restore that candidate, and accept only its bound `RecoveryQualification`
   before publishing `ActiveV2` and closing legacy writers. Until ActiveV2,
   legacy workspaces remain an isolated residual and MUST NOT inherit Vault v2,
   backup, or recovery claims. Activated mode disables outbox unless the exact
   local-effect gate from step 2 passed. Persistent privacy authority,
   freshness, and encrypted queued work wait for the applicable custody,
   recovery, authority, and activation gates.
5. Close legacy model egress, then add the pure privacy compiler, one fixed
   no-network projection, strict validation, and value-free evidence.
6. Add one real local model behind a verified sandbox; failure stays local.
7. Add `AutoProtect`/`LocalOnly`, visibility previews, capability inventory,
   queue, and unavailable options. Keep Owned Mesh non-executable Research.
8. Validate whether founders need synchronous owned-node compute. Only then
   promote a separately reviewed Noise Phase 1 RFC from Research to Target.
7. Evaluate MLS separately if multi-member/asynchronous requirements emerge.
8. Add purpose-specific transform modules and the complete Trust Interface.

## Rejected alternatives

- Trust caller-selected `Red / Amber / Green`: the caller can mislabel data.
- Trust a provider's `local` flag: an adapter can self-authorize raw access.
- Encrypt raw data to a public model: the model cannot infer without seeing
  plaintext, so this does not solve the boundary.
- Use one universal anonymizer: safety is purpose-, dataset-, transform-, and
  recipient-specific.
- Fall back to public cloud when local compute fails: availability must not
  silently widen confidentiality.
- Copy a messaging E2EE protocol unchanged: model-job dispatch has different
  replay, response, scheduling, and evidence requirements.
- Store a raw root key beside ciphertext, use a password directly as the DBK,
  or silently regenerate a missing key: these turn loss/corruption into
  undetectable data loss or false protection.
- Reuse AES and XChaCha keys or try legacy decryption after v2 authentication
  fails: authenticated version selection must prevent downgrade and confusion.
- Use age as a live Vault/session, HPKE as a complete session protocol, or a
  custom Noise group layer: each omits required state and lifecycle semantics.
- Back up old device/session/signing private keys by default: recovery creates
  a new approved endpoint instead of reviving a revoked one.
