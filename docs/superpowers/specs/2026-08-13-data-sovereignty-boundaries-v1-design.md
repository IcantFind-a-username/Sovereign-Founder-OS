# Data Sovereignty Boundaries v1

**Status:** Approved product direction; the normative target design is mirrored in draft RFC 0004 and implementation is staged below.
**Date:** 2026-08-13
**Scope:** The first enforceable privacy and compute-placement boundary for Sovereign Founder OS.

## 1. Product promise

Sovereign Founder OS is designed to let people use useful AI without silently surrendering business secrets, personal data, credentials, or authoritative company state.

The default experience must be understandable to a non-specialist. A professional mode must expose the same underlying policy with more detail; it must not bypass enforcement. A powerful device must be able to run all inference and internal processing for a supported workflow locally; an explicitly approved delivery to a named business recipient remains a separate external effect. A low-power device may use user-controlled compute or a public cloud only through an explicit, enforceable data boundary.

This design does **not** claim perfect or absolute security. It defines a testable supported-API threat model, requires fail-closed behavior, and distinguishes current controls from future cryptographic and deployment controls.

Throughout this document, **Current** means merged code with passing repository tests, **Target** means an approved but unshipped release gate, and **Research** means a candidate whose requirements or implementation have not been accepted. Target and Research text is not a claim about the current product. The Trust Layer should appear in the default product only when it changes a founder's business outcome or data visibility; the main workflow remains task-first, with protocol detail in professional and security views.

### Current implementation gaps

| Area | Repository today | Required by this design |
| --- | --- | --- |
| Raw model route | A caller supplies `prompt`, `task`, and `DataClass`; Amber/Green can reach a provider that self-reports Cloud/Local trust. | Raw request accepted only by a closed local boundary; caller labels never grant egress. |
| Public compute | Deterministic cloud-labelled stand-ins accept the same raw request type. | Only a compiler-owned, purpose-bound projection reaches a closed public broker. |
| Disclosure record | Public fields, caller/provider strings, unsigned and forgeable; useful demo telemetry only. | Private broker-derived evidence, value-free, then signed through the audit ledger. |
| Presets/visibility | Target UX documentation only. | Deterministic immutable snapshot, simple/pro views, widening approval and revocation. |
| Owned-node E2EE | Not implemented. | Research concept and non-executable reserved type until measured need and a reviewed Secure Mesh protocol; no configuration CTA. |
| Queue | Workflow checkpointing exists but is not an encrypted private-compute queue. | Opaque encrypted-vault handles with expiry/cancel/revocation semantics. |
| Vault at rest | **Current prototype:** each item is AES-256-GCM encrypted, but the raw Base64 master key is stored beside the ciphertext and manifest names remain visible. | **Target:** pinned SQLCipher transactional business store with a random database key, closed device protector, independent recovery root, typed key wrappers, rotation, and tested migration. |
| Recovery/export | The JSON export is useful for inspection but is not a clean-machine backup or trust-continuity package. | **Target:** separately handled encrypted business backup and public trust-continuity material, followed by a verified restore ceremony and new device identity. |
| Local model isolation | Deterministic stand-ins exercise routing; they are not real AI or a sandboxed local model. | **Target:** a provenance-bound real model process with no ambient network/filesystem/environment access, authenticated IPC, model digest binding, and resource limits. |

The first privacy-boundary slice closes the legacy raw cloud authority; adding a parallel safe path while leaving the bypass public is not completion. The program-level first hardening slice is RFC 0005 Vault v2 because new long-term privacy secrets must not inherit the current co-located-key weakness.

## 2. Non-negotiable invariants

1. **Protected data is broader than PII.** Customer data, contracts, source code, strategy, financial records, unpublished work, internal prompts, model context, and derived business facts are protected unless an authoritative policy proves otherwise. Root/private keys, permanent credentials, recovery secrets, and session keys are stricter `NonDisclosableSecret` values outside the model-data taxonomy; they are never model or owned-node inputs.
2. **Unknown is protected.** A caller cannot make arbitrary content cloud-safe by labeling a field `public`, `green`, or with a trusted-looking name. The legacy caller-supplied `DataClass` is a conservative compatibility hint only; it can restrict a route but can never grant egress.
3. **Raw protected data never reaches a public model API.** Public cloud inference receives only a locally compiled projection whose complete outbound structure and metadata are independently allowed.
4. **Fully local remains a first-class path.** When local compute is available, the inference and processing steps of a supported workflow can prohibit all network model calls and keep task data on the device. A later named-recipient business effect discloses only the exact content the owner separately approves.
5. **Owned nodes are not public cloud recipients.** Raw protected data may reach an explicitly authorized user/company-owned node only through an authenticated end-to-end encrypted channel and an approved visibility scope.
6. **Target storage and transit boundary.** Protected authoritative data is encrypted at rest under user/company-controlled key domains only after the Vault v2 migration gate is met. Transit between authorized devices or owned nodes is E2EE only after a reviewed Secure Mesh implementation ships. The current co-located-key Vault and the absence of Secure Mesh do not satisfy this invariant. A public model never receives protected payload or decryption keys—only an approved projection.
7. **Insufficient compute never weakens policy.** V1 may select a smaller local
   model or a local queue. Owned-node compute remains Research and cannot be
   activated. Failure must not silently reclassify data or fall through to a
   public provider.
8. **Every disclosure is inspectable.** Before dispatch, the system can explain which recipient class may see which projection, for what purpose, under which policy, and for how long. After dispatch, it records value-free evidence of what route was used.
9. **Outputs inherit risk.** Provider responses are untrusted and `RestrictedDerived` by default. They are parsed against a strict shape, classified, and combined with locally sealed values only after the external response is no longer able to influence placement. Rehydrated output is `Protected`.
10. **Authority is closed.** Model adapters, plugins, and ordinary callers cannot forge classification, bypass the egress broker, manufacture disclosure receipts, or self-authorize a wider scope through supported APIs.
11. **Cryptography is standard, versioned, and reviewed.** The project will not invent primitives, ratchets, KEM hybrids, protocol patterns, cipher suites, nonce/key derivation, or authentication-failure fallbacks. Named profiles require domain separation, official vectors, locked implementations, integration review, and an explicit migration path. A library audit does not certify this project's protocol or key lifecycle.

### Cryptographic architecture and maturity

The approved **Target** storage hierarchy has two independent unlock domains. Neither unlock root directly encrypts business data, and signing or rollback state is never derived from a Vault data root.

```text
Platform/OS Device Key Protector ── Device Unlock KEK ──────────┐
Recovery passphrase ── Argon2id ── PWK ──wrap── Recovery KEK ──┤
                                                               └─XChaCha-wrap only── random SQLCipher DBK
                                                                                        │
                                                                      pinned encrypted DB + journal

Separate trust domains: device identity, privacy authority, audit signing,
                        and freshness/rollback state
Padded canonical snapshot ── age v1 recipient mode ── offline backup recipient
```

The passphrase derives only a password-wrapping key; it wraps an independent
random Recovery KEK and is not the database key, a signing key, or a session
key. SQLCipher supplies the reviewed page encryption, journal protection,
locking, and transaction machinery; XChaCha20-Poly1305 is used only for closed,
typed DBK/Recovery-KEK wrappers, not as a home-grown database format. A recovery
ceremony creates a new device identity and new transport state. Ordinary backups
exclude device, privacy-authority, audit-signing, Noise/MLS/ratchet private
state, and unfinished effect authority. RFC 0005 defines the Vault and recovery
target.

For an existing backup, both the age identity and recovery passphrase are
required to recover business data. Trust-continuity material is independently
required to claim verified history/freshness/authority continuity. With the
two recovery inputs but without trust-continuity material, the maximum claim is
data rescue under a new trust identity. Losing either the age identity or the
recovery passphrase makes that backup unable to restore business data; keeping
trust-continuity material alone does not change that. A surviving enrolled
device may create a replacement backup with new recovery inputs, but does not
repair a missing input for the old artifact.

Secure Mesh is separate from storage and remains **Research** until measured
synchronous owned-node demand justifies a separate accepted RFC. The current
Phase 1 candidate is an audited implementation of
`Noise_XX_25519_ChaChaPoly_BLAKE2s` for synchronous one-to-one jobs, with
owner-verified static keys and workspace, protocol profile, membership epoch,
authority epoch, policy digest, direction, attempt, and sequence bound to the
transcript/envelope. Noise `Rekey()` does not add new DH entropy and is not
continuous post-compromise recovery; compromise or revocation requires a new
handshake or re-pairing. **Research Phase 2** is RFC 9420 MLS/OpenMLS if real
three-or-more-member, formal Add/Remove, or asynchronous delivery requirements
justify its state and operational cost. PQXDH/Double Ratchet, PQ MLS, and HPKE
remain Research inputs; HPKE alone is not a session, replay, or recovery
protocol.

AEAD, signed chains, and freshness counters cannot detect restoration of an entire device to an older internally valid snapshot when no state survives outside that snapshot. Workspace-relative rollback detection is a Target; whole-device rollback detection requires a deployment-specific external monotonic anchor and remains Research.

## 3. Threat model for v1

### In scope

- A buggy or malicious workflow attempts to place protected text in a nominally public field.
- A compromised or prompt-injected model output attempts to change permissions, topology, or recipient.
- A provider adapter attempts to observe a request it was not selected to receive, repeat a disclosure, forge a response, or misattribute a receipt.
- A low-power device runs out of memory, lacks a compatible model, or loses connectivity.
- Logs, errors, debug output, metrics, receipts, lengths, identifiers, and serialized metadata accidentally reveal protected values.
- Replays, cross-job substitutions, stale jobs, malformed JSON, duplicate keys, oversized responses, topology changes, and provider failover.
- A non-specialist chooses a preset without understanding internal security terminology.

### Outside the v1 software boundary

- A fully compromised operating system, kernel, hypervisor, or physical device.
- Traffic sent by arbitrary native code outside the supported broker API.
- A public provider truthfully deleting data or honoring retention promises without independent attestation.
- Production-grade authenticated recipient guarantees before Secure Mesh is implemented and audited.
- Statistical anonymity claims without a defined privacy budget and proof appropriate to the transform.

These exclusions must remain visible in documentation. They do not permit supported product APIs to overstate route identity or evidence.

## 4. Separate the three decisions

The current `Red / Amber / Green` value is too overloaded. V1 keeps compatible display labels while internally separating three dimensions.

### 4.1 Sensitivity

- `NonDisclosableSecret`: root/private key material, permanent credentials,
  recovery secrets, session keys, or equivalent non-model secrets.
- `Protected`: raw or derived business secret, personal data, private content,
  or unknown caller value. It excludes `NonDisclosableSecret`.
- `RestrictedDerived`: transformed content that still may identify, reconstruct, or reveal protected facts.
- `PublicContent`: fixed public content or authoritative published data whose trusted provenance is independently established.

`Unknown` enters the system as `Protected`. Legacy caller-supplied Amber and Green values also enter this boundary as `Protected`; their old labels remain available to capability/effect compatibility paths but do not establish provenance. Sensitivity comes from trusted source provenance, not the destination field selected by a caller. A transform does not relabel its input as reusable public content. It can produce only an opaque job bound to one purpose, recipient class, policy snapshot, expiry, and exact projection. The legacy policy engine's `cloud.*` rule remains defense in depth, not public-inference authorization.

An unknown value also starts with `ThisDevice` visibility and no transform/declassification grant. `NonDisclosableSecret` is represented outside `TrustedValue<T>` and every model request. Root keys, company/device private keys, permanent credentials, recovery secrets, and equivalent non-exportable secrets are never model inputs or owned-node payloads. Authorized core operations receive only a private, operation-scoped `OpaqueSecretHandle<Operation>` from the vault, identity service, or effect broker. The handle exposes no secret bytes, has no generic serialization or value-bearing display/debug form, and cannot satisfy a model-input trait.

The compatibility Red display state therefore compiles to `ThisDevice` by
default. A future exact subset of Red business data may reach one owned node
only after Secure Mesh is implemented and reviewed and a new signed grant binds
that node, purpose, operation, expiry, and constraints. No such grant can cover
`NonDisclosableSecret`.

### 4.2 Visibility scope

- `ThisDevice`: the Target default for protected business data; only the
  current device and its admitted local processes inside the trusted runtime
  boundary. Current code has no such admitted-runtime guarantee: it only blocks
  caller-labelled Red from providers that do not self-report `Local`.
- `OwnedMesh`: named user/company devices or nodes authorized by identity and policy.
- `NamedRecipients`: an explicit set of human or service recipients approved for a defined effect.
- `PublicComputeProjection`: a purpose-bound projection that a registered public inference recipient may process.

These labels are not a linear order: owned nodes, named recipients, and public compute are incomparable recipient classes. The enforceable scope is a set of authorization grants `(recipient, purpose, operation, expiry, constraints)`. Grant B narrows grant A when recipient, purpose, and operation match, B expires no later than A, and B's constraints are at least as restrictive. Scope B narrows scope A when every grant in B is subsumed by a grant in A. A transform creates a new derived value with its own scope; it never widens the source value. Adding a non-subsumed grant requires an authoritative policy transition and, where configured, human approval.

### 4.3 Compute placement

- `Local`: deterministic code, a small local model, or a full private model on this device.
- `OwnedNode`: authenticated E2EE dispatch to an authorized node.
- `PublicProjection`: compiler-mediated public inference over approved projection bytes only.

`Queued` is a placement decision used when no authorized compute is currently available; it is not an executable compute location.

Classification does not choose placement by itself. The policy compiler evaluates sensitivity, visibility scope, purpose, device capability, recipient registration, retention constraints, and output contract together.

## 5. User experience

### Reference founder scenario (Target)

The product acceptance path is one independent consultant handling a concrete
lead, not a generic chat demonstration:

1. the founder records Acme's discovery notes, budget range, named contacts,
   and delivery constraints as `Protected` business data on this device;
2. a deterministic component or admitted local model summarizes the notes and
   drafts a fixed-scope proposal; it receives no `NonDisclosableSecret` and
   gains no mutation or send authority;
3. if the founder proposes public inference, the preview shows the named
   provider and exact purpose-bound projection values before approval;
4. the founder edits and approves the final proposal for one named recipient
   through the separate exact-effect path;
5. delivery notes, invoice draft, and follow-up remain structured local
   business state, and an interruption resumes without duplicating a covered
   completed step.

This same slice must work with `Local Only` and zero public/owned-node broker
observations. Secure Mesh, automatic email delivery, backup/restore, and real
local AI are claimed only at their separate maturity gates.

### 5.1 Simple security presets

The default is **Auto Protect**.

| Available v1 preset | User-facing meaning | Enforced behavior |
| --- | --- | --- |
| **Auto Protect** | “Finish the task safely using my device first.” | Prefer local; allow public compute only for a compiler-validated projection; otherwise queue or explain the missing resource. It does not pre-authorize a future owned node. |
| **Local Only** | “Nothing for this task leaves this device.” | Disable owned-node and public-model dispatch. A missing model or capacity returns local alternatives and queue options. |

**My Devices & Company Nodes** is a Research product concept and a reserved non-executable policy vocabulary, not a selectable v1 preset. Until a Secure Mesh RFC, real two-node implementation, revocation/recovery drills, and independent review pass, the UI displays only “Research—this version will not connect another device” and offers no configuration action. Future activation creates a new signed policy transition bound to exact node identities; it cannot inherit authorization from an older `AutoProtect` snapshot.

Presets produce a versioned, immutable policy snapshot. Changing a preset affects new decisions and must not rewrite earlier evidence. An in-flight job stays bound to its original snapshot until its short expiry unless a separately versioned revocation epoch invalidates it; an ordinary policy edit is not an ambiguous implicit revocation.

Snapshots use a fixed schema version and canonical encoding. Timestamps, random identifiers, and map insertion order do not affect the deterministic policy digest. If a professional control changes a preset's security meaning, the UI and snapshot identify the result as `Custom`; it must not continue displaying a stronger preset name.

Every preset is bounded by a non-overridable safety baseline. Professional settings may freely narrow the effective policy by intersection. They may select or restrict pre-registered sources, transforms, purposes, providers, and nodes, but cannot register trust or self-label content. Any widening is a separate owner-reviewed diff and signed policy transition; it never occurs merely because a control changed.

Until Secure Mesh is implemented and verified, an owned-node preference is internal Research vocabulary only. It may produce a value-free “not available in this version” status for diagnostics, but it cannot produce an executable `OwnedNode` placement, simulate a secure transport, or expose a fake configuration action.

### 5.2 Professional mode

Professional mode edits the same policy model with controls for:

- data categories and authoritative sources;
- allowed device groups; node groups appear only after Owned Mesh is promoted
  from Research by a separate RFC;
- public projection templates and purposes;
- provider/model allowlists;
- retention and logging constraints;
- maximum projection size and field topology;
- required approval for scope widening;
- local model preference, resource ceilings, and queue deadline;
- output classification and export restrictions.

Professional mode cannot create a raw public-cloud escape hatch. Exceptional declassification must be a separately named, previewed, signed, purpose-bound action.

### 5.3 Visibility explanation

Every task answers four beginner questions: **where it runs**, **who can see it**, **exactly what leaves this device**, and **what happens if the chosen route fails**. Current states are worded distinctly:

> On-device processing. No task data leaves this device for this task.

> A local deterministic function processed the customer data shown for this task. No LLM or external service was contacted, and this is not real AI.

A future real external projection may instead say:

> External projection proposed. The named provider can see only the exact values listed below; protected customer and company context stays on this device. If validation or dispatch fails, the task stops or waits—it does not silently change route.

The compact copy does not claim that unrelated operating-system traffic is absent. The expanded pre-dispatch view uses a local, transient `ExposurePreview` to show purpose, transform/template, exact recipient identity, exact outbound field names and approved values, locally sealed/omitted categories without their values, policy version/expiry, requested provider retention (not an enforcement guarantee), fallback, output taint, and evidence status. `ExposurePreview` is not clonable, serializable, persistable, or printable through generic `Debug`. Persisted `ExposureManifest` and post-dispatch evidence are value-free: field paths/dispositions, approved counts, closed identifiers, and commitments only. The compact indicator renders only value-free manifest facts. If preview rendering fails, dispatch fails closed.

“Value-free” does not mean “safe to hash directly.” A digest or identifier
derived from a low-entropy business value remains dictionary-testable and is
protected metadata. V1 persistent correlation uses only a high-entropy opaque
ID assigned before content is known. A keyed/blinded commitment scheme is
`Research` and blocked until a separate accepted RFC fixes a maintained
primitive/profile, domain and encoding, blinding uniqueness/serialization,
audit-domain NDS key custody/rotation/recovery/export rules, vectors, and
dictionary/linkability tests. Exact timestamps and counts use
closed coarse buckets only where the leakage contract permits them. Public
cryptographic digests are reserved for already-public bytes or high-entropy
protocol objects; no unkeyed digest of a name, email, amount, title, action,
resource label, or other guessable value appears in a manifest, ledger, error,
path, or receipt.

## 6. Data-boundary algorithms

Different workflow steps use different algorithms. “Anonymize it” is not a universal permission.

### 6.1 Local raw processing

Protected input is supplied only to a local deterministic component or local model. The runtime blocks network-capable adapters from accepting the raw request type.

“Local” is placement, not trust. A raw-input consumer must be core-reviewed deterministic code or execute behind a capability-constrained boundary with no ambient network, filesystem, environment, or unapproved IPC access. Running arbitrary native model/plugin code on the same device does not authorize it to receive protected input.

The product distinguishes three states: a core-reviewed built-in deterministic
component, a **Target** verified sandboxed local model, and an untrusted
ordinary local/localhost process. Only the first two may receive protected raw
input. The **Current** deterministic demo still relies on legacy caller labels,
so its existence does not prove this boundary ships. Trusted component status
comes from a closed core registry; sandboxed-model status additionally binds a
core-owned sandbox profile, authenticated IPC, model artifact
digest/provenance, and resource limits. An adapter, address, or provider flag
cannot self-report either state. Local-model failure returns
wait/install/reduce-task options and never changes the confidentiality route.

### 6.2 Fixed-slot sealing and local rehydration

For templates such as a public drafting skeleton, protected values are replaced locally with typed opaque slots. The public model may fill only a registered response topology. Strict validation completes before local slot rehydration. A model cannot add slots, choose recipients, or alter protected values. Schema validity does not make an output public: a validated response remains `RestrictedDerived`, and rehydrating any protected slot makes the joined result `Protected`. The rehydrated scope is the intersection of every sealed input scope, the validated response scope, and the registered output-contract scope; an empty intersection rejects. Rehydration cannot add grants or trigger a widening transition, and provider bytes never influence recipient, purpose, placement, policy, capability, or authority.

### 6.3 Purpose-bound minimization

An authoritative transform selects the minimum approved fields for one registered purpose. Every output leaf has trusted provenance and a disposition: public copy, sealed slot, omitted, locally aggregated, or rejected. Arbitrary literals and dynamic caller text remain protected.

Transform definitions are registry-owned, signed, version-pinned objects bound to code/schema digests, closed output metadata and topology, declared determinism, resource/I/O restrictions, and security-review status. Registry mutation uses authority distinct from ordinary workflow execution and professional policy editing. Transforms run without ambient I/O.

### 6.4 Aggregation and privacy mechanisms

Deterministic aggregation may be approved when it has a documented reconstruction analysis. Differential privacy, private set intersection, secure aggregation, and similar mechanisms are separate algorithm modules; none may claim safety until its threat model, parameters, privacy budget, and tests are accepted.

### 6.5 E2EE owned-node execution

Raw protected work can move from a low-power device to an authorized owned node only through Secure Mesh. The protocol must bind sender device, recipient node, company/workspace, job nonce, policy digest, expiration, request commitment, and response commitment. Intermediaries see ciphertext and routing-minimum metadata only.

The authenticated endpoint necessarily sees the plaintext categories it is authorized to compute over. E2EE hides them from relays, transport, and blind storage; it does not make the receiving node unable to read its job.

Secure Mesh is not shipped. Its first protocol RFC must specify the fixed Noise Phase 1 profile above, authenticated owner-approved membership, replay/sequence handling, expiry, bounded session/job lifetime, metadata/padding budget, device removal, re-handshake, and recovery. Exactly-once business execution remains an application-layer broker/journal property, not a Noise guarantee. The project must not evolve its one-to-one membership into a home-grown group protocol; the MLS Research gate is evaluated separately when real product requirements justify it.

### 6.6 Explicit declassification

Some complete business workflows eventually require sending protected data to a customer, lawyer, accountant, payment processor, or other named recipient. That is an effect, not model inference. The system presents the exact content and recipient, applies policy and human approval, issues a one-use capability, performs the effect through a recipient-bound broker, and records signed evidence.

## 7. Architecture and dependency boundaries

```text
Authoritative data sources
        │
        ▼
Sovereignty Core
  ├─ provenance-bound classification
  ├─ preset / professional policy snapshot
  ├─ visibility and purpose compiler
  └─ exposure manifest + placement plan
        │
        ├──────── Local executor / local model
        │
        ├──────── [Research] Secure Mesh broker ──E2EE──► owned node
        │
        ├──────── Privacy compiler ──────────► public projection broker
        │                                      │
        │                                      ▼
        │                              strict response validator
        │                                      │
        └──────────────── local rehydration ◄──┘
                                               │
                                               ▼
                                  value-free signed evidence
```

### 7.1 Sovereignty Core

A new foundational `sovereign-privacy` crate owns trusted data labels, visibility scopes, policy snapshots, transform registry, placement decisions, exposure previews/manifests, opaque compiler products, and their state transitions. It depends only on foundational serialization/cryptographic libraries and compatibility contracts; it has no provider networking, UI, workflow, or policy-engine dependency.

Dependency direction is one-way:

```text
privacy ──depends on──► contracts + foundational libraries
policy  ──depends on──► privacy
model   ──depends on──► privacy
CLI / workspace ─────► policy + model + privacy
```

`sovereign-policy` and `sovereign-model` may depend on privacy. Privacy never imports them, model never imports policy, and the generic workflow crate remains independent. Closed provider/model/task identifiers needed for route evidence are defined at the privacy boundary rather than accepted as arbitrary adapter strings.

### 7.2 Privacy compiler

The compiler consumes provenance-bound structured inputs and a registered transform. It returns either:

- a local/owned-node task containing protected context; or
- an opaque pending public-compute job containing only compiler-owned safe request bytes, sealed local context, expiry, policy/plan digests, and one-use routing state.

The legacy generic gateway is removed as an egress authority. Raw `ModelRequest` is accepted only by a `LocalModelProvider` collection; provider self-reporting cannot turn it into a cloud route. `DeterministicProvider::cloud` cannot accept raw requests. Public adapters can receive only the opaque compiler product through a closed broker operation. In the first slice, that broker is a deterministic no-network stand-in with fixed registered identities; real egress remains a separate reviewed stage.

### 7.3 Compute scheduler

The scheduler uses a policy-approved ordered strategy:

1. deterministic local algorithm;
2. compatible small local model;
3. configured full local model;
4. authorized E2EE owned node, only after Secure Mesh exists and verifies the target capability;
5. compiler-mediated public projection, if the task has one;
6. local queue or `ComputeUnavailable` with safe options.

The scheduler cannot convert a local-required task into a public job.

An authorized target, an observed runtime capability, and an executable placement are separate concepts. Policy may authorize a future owned node, but placement cannot select it until authenticated Secure Mesh capability is present.

Queued work stores only an opaque handle to authenticated-encrypted vault state, never a plaintext prompt or protected workflow JSON/log entry. Queue records have an expiry, cancellation/deletion state, policy/revocation binding, and crash semantics. Until a durable encrypted queue is implemented, the v1 queue is process-local and documented as non-durable.

### 7.4 Egress and effect brokers

Public inference, owned-node execution, and named-recipient effects are distinct brokers with distinct request types. Route evidence is created by the broker and job state machine, not caller-supplied strings. A dispatch record exists before request bytes become visible to an adapter. In the first slice, the deterministic no-network provider, broker, attempt state machine, and evidence finalization all live inside `sovereign-privacy`; `sovereign-model` only re-exports the high-level closed gateway. No public cross-crate manual attempt transition exists. A future real egress crate requires its own RFC and unforgeable authority-bound broker design.

Named-recipient effects upgrade and reuse the existing prepared-invocation,
Capability V2, durable authority, journal, and effects-broker primitives. The
current application-created signer is not accepted as human-presence evidence:
the owner approval must come from Program 1C0's independently admitted,
expiry-bound, one-use authority. The privacy boundary binds content, recipient,
purpose, and policy commitments into that chain; it does not create a second
execution-authority system.

Every real public-provider or owned-node dispatch is likewise an external effect authorized above privacy. It binds the job, request, recipient, policy, transform, and expiry commitments into the existing Capability V2, durable Authority Store, and execution journal. Privacy cannot become a second approval issuer or durable replay authority. The deterministic no-network stand-in is explicitly process-local and non-effectful.

### 7.5 Trust Interface

The CLI/UI renders closed, value-free explanations and evidence. It never derives visible sizes or messages from rehydrated protected output, and error/debug surfaces use fixed redacted variants.

## 8. Core contracts

The detailed implementation plan may adjust names to fit existing crates, but it must preserve these semantic types:

```text
TrustedValue<T>          provenance + sensitivity + allowed scope
NonDisclosableSecret     non-model secret bytes; private to a trusted secret owner
OpaqueSecretHandle<Op>   operation-scoped reference; never a model input
PolicySnapshot           preset/pro rules + version + digest
TaskIntent               registered purpose + requested output contract
PlacementPlan            ordered authorized compute placements
ExposureManifest         value-free explanation of visibility
ExposurePreview          transient local-only exact pre-dispatch view
LocalTask                protected raw context, not public-dispatchable
PendingPublicJob         opaque projection + sealed local context + one-use state
OwnedNodeJob             reserved Research type; unconstructible in v1
DisclosureEvidence       broker-derived route/outcome commitments
ComputeUnavailable       queue/install-model/reduce-task/wait options
```

All constructors that establish trust are private or registry-owned. Public structs containing evidence or authority have private fields and read-only accessors. Serialization is allowed only for explicitly reviewed wire/evidence formats. The legacy public-field `DisclosureRecord` is not security evidence and is replaced or sealed during the first slice; signed ledger persistence consumes only broker-derived evidence.

`sovereign-privacy` does not own or persist `NonDisclosableSecret`. The Vault,
identity service, or effects broker owns the secret and its operation-specific
handle; no conversion exists from that handle into `TrustedValue<T>`,
`ModelRequest`, `LocalTask`, or `PendingPublicJob`.

`PolicySnapshot` additionally binds canonicalization version, workspace/issuer, monotonic generation, transform/provider/node registry digest, and revocation epoch, and is authenticated in protected storage. The runtime revalidates expiry, revocation, policy authority, registry membership, and recipient capability immediately before every dispatch/failover and before rehydration. A security-narrowing transition or emergency revocation advances the revocation epoch and invalidates pending/queued work; a non-security metadata edit need not. A later widening never retroactively authorizes an old job.

Recipient/model classes determine policy eligibility only. Each
`PendingPublicJob`, placement plan, preview, and Program 1C0/Exact Effect
authorization binds one exact ordered set of pre-content random
`provider_target_id` values. Each resolves in protected coordinator state to
one immutable tuple of provider identity, adapter artifact digest, endpoint
origin/audience, account/tenant, credential-handle ID, model ID, and immutable
model/version descriptor. Only the live owner preview may transiently resolve
the full tuple. A changed tuple receives a new ID, and an ID outside the set
requires a new preview and approval; a retry policy cannot substitute it
silently, and `Indeterminate` never auto-retries. Value-free evidence follows
the projection rule below.

## 9. Failure semantics

- Unknown source, transform, purpose, provider, field, or metadata: local-required or reject.
- Missing local compute under `Local Only`: queue or `ComputeUnavailable`; never public fallback.
- Missing owned-node keys, revoked node, identity mismatch, stale membership, or failed authentication: no plaintext dispatch; queue or reject.
- Projection compiler mismatch, expiry, digest mismatch, topology mismatch, revocation epoch, registry membership, or authenticated snapshot mismatch: terminally reject the job.
- Provider failure: reject duplicate registered identities and bind each adapter
  instance to its closed identity. The broker may move to the next exact
  pre-approved identity only when its own state proves `FailedBeforeDispatch`
  and no request byte was exposed—for example, a pre-dispatch health failure.
  Once bytes become visible to an adapter, or non-visibility cannot be proven,
  timeout, error, panic, drop, crash, or missing acknowledgement terminates as
  `Indeterminate`; it never retries or fails over. Preserve the complete
  attempt history across repeated gateway calls.
- Expiry: check immediately before dispatch and immediately after response. A response arriving after expiry remains recorded as an attempted disclosure, but it cannot be rehydrated or trigger another provider disclosure.
- Malformed or oversized response: terminal invalid-response evidence; never rehydrate.
- Process crash after public dispatch: record `Indeterminate` on recoverable durable state once durability is implemented. The first in-memory version must state its crash limitation honestly; absence of durable evidence never authorizes retry.
- UI explanation failure: fixed redacted error; no fallback stringification of protected inputs.

## 10. Evidence and observability

Evidence records may contain only approved public identifiers, pre-content
opaque random identifiers, public-byte/high-entropy protocol digests, coarse
time/count buckets allowed by the leakage contract, recipient class, closed
provider/model identifiers, transform version, field dispositions, and attempt
outcomes. The blocked keyed/blinded construction above cannot be used by v1.
Exact provider/account/tenant/endpoint/credential tuples and exact effect
recipient/content remain only in protected coordinator state under
pre-content random `provider_target_id`/`effect_intent_id` values. Durable
value-free evidence projects those random IDs, closed outcomes, and only fields
an authoritative registry independently classifies approved-public; it never
serializes the protected tuple/content or a deterministic digest of it.
In-memory broker evidence becomes signed durable evidence only when appended
through the existing signed audit ledger; the design does not call an unsigned
`DisclosureRecord` non-repudiable.

They must not contain raw prompts, protected values, sealed values, rehydrated output, arbitrary caller labels, provider error text, response excerpts, or lengths derived from private content. Evidence is proof of software state transitions inside the supported boundary; it is not, by itself, proof of a provider's real-world retention behavior.
They also must not contain unkeyed/deterministic digests of low-entropy business
values or exact timestamps/counts whose leakage was not separately approved.
Adversarial tests enumerate plausible names, emails, prices, actions, titles,
and resource labels and prove that offline dictionary matching cannot recover
them from persisted evidence.

## 11. Verification strategy

Every implementation slice follows test-first development and an independent attack review.

### Required positive tests

- `AutoProtect` and `LocalOnly` compile to deterministic policy snapshots; public activation of the reserved `OwnedMesh` vocabulary is rejected until Secure Mesh ships.
- A fully local workflow completes with zero public or owned-node broker calls.
- Before Secure Mesh, a low-power device may report only that owned-node
  compute is unavailable in this version; it cannot configure or dispatch it.
  Real owned-node tests begin only after a future RFC is promoted to Target.
- A registered fixed-slot workflow uses byte-identical public projection across different protected values and rehydrates locally. Minimization/aggregation transforms instead declare and test their specific leakage contract.
- Exposure explanations are stable and understandable in simple and professional views.

### Required adversarial tests

- Caller puts a high-entropy secret into a field named `topic`, `public`, provider ID, model ID, purpose, task ID, retention policy, and error text.
- Derived/formatted/hashed/encoded/nested values cannot become public without an approved transform.
- Cross-job request, response, slot, plan, policy, and receipt substitution.
- Duplicate provider identifiers, repeated gateway calls, failover after failure, expiry during health/dispatch/response, panic/drop/forget, and process-recovery limitations.
- Duplicate JSON keys at every depth, unknown fields, invalid UTF-8, trailing data, depth/node/byte boundaries, numbers, huge strings, and control/confusable Unicode.
- Fixed-slot projection payloads are identical for materially different protected values and lengths. Transport envelopes may differ only in declared value-independent nonce/time fields, and evidence is compared through a defined normalized value-independent view.
- `Local Only` and local compute failure produce no public request type and no broker observation.
- Public APIs cannot construct trusted values, disclosure evidence, attempts, or adapters that bypass recipient binding.
- Debug, Display, error, log, CLI output, serialized plan/evidence, and metrics contain no canary or decoded canary.

### Release gate

- Formatting, linting with warnings denied, full workspace tests, file-size limits, locked release build, frontend type checks, dependency/security scans, and a combined stdout/stderr canary scan.
- No known Critical or High finding inside the declared supported-API threat model.
- Security-sensitive protocol or cryptographic changes receive independent specialist review before a production claim.

## 12. Staged implementation

This design is intentionally decomposed. Each stage gets its own plan, tests, review, commit, and remote push.

1. **Vault Program 1A — format, custody, and importer engine:** pin SQLCipher,
   implement typed dual-domain key wrappers and fail-closed native-protector
   behavior, and prove a side-by-side closed legacy importer. It exposes no
   product enrollment or real-workspace migration. Known business entries may
   import in fixtures; legacy approval/admission/authority keys are blocked
   pending role-specific handoff, and unknown kinds fail closed.
2. **Vault Program 1C0 — owner authenticator and one-use authority:** admit one
   expiry-bound, CSRF-resistant owner session and approval issuer. Exact Effect
   and every later Vault ceremony consume this authority; neither may create an
   application-local owner signer.
3. **Authority + Exact Effect Protocol v1 (local outbox first):** bind exact
   effect bytes/recipient/provider/resource to the 1C0 approval, Capability V2,
   durable authority reservation, execution journal, effect outcome, and signed
   evidence. Fix approval retention and define `Indeterminate`; no real network
   effect bypasses this state machine.
4. **Vault Program 1B0 — backup/restore mechanics:** add the separately
   encrypted business backup, trust-continuity artifact, loss-matrix tests,
   rotation, and clean-machine harness from a closed backup registry. Snapshot
   creation
   is an owner-present recovery-password ceremony because the device route
   cannot unwrap RecoveryKEK; unattended backup requires a separate reviewed
   online backup authority/key design. 1B0 consumes only internal fixtures and
   cannot qualify a product workspace.
5. **Vault Program 1C1 — role-key custody and handoff:** protect and rotate
   legacy identity, approval, admission, runtime-authority, and audit keys in
   their separate trust domains. These keys never become business DB rows.
6. **Vault Program 1D + 1B1 — product activation and real recovery
   qualification:** inventory every persisted product value, move business data
   and sensitive display evidence
   into v2, prevent v1/v2 split-brain writers, and activate only through one-use
   owner authorization after 1A, 1B0, 1C0, and 1C1. Exclusively freeze the
   final legacy generation, rebuild the candidate from that same retained read
   snapshot, and require an unforgeable `VerifiedMigration` binding frozen
   source and candidate content before creating `PendingV2`. Then require 1B1
   to restore it cleanly and return a bound
   `RecoveryQualification` before publishing `ActiveV2`. Outbox is disabled
   unless the exact-bound local-effect slice above has already passed.
7. **Privacy boundary and compiler:** provenance-bound classification, local-only raw model requests, compiler-owned fixed projections, strict response validation, closed route evidence, immutable policies, and adversarial corpus. The public provider remains a deterministic no-network stand-in.
8. **Verified local AI:** add one real model behind a no-network/no-ambient-filesystem sandbox with authenticated IPC, artifact provenance, resource ceilings, untrusted output handling, and an end-to-end founder drafting workflow.
9. **Compute placement and Trust Interface:** capability inventory, `AutoProtect`/`LocalOnly`, simple/pro visibility preview, encrypted-handle queue, fully local proof, and safe `ComputeUnavailable` options. Owned Mesh remains a non-executable Research state with no configuration CTA.
10. **Secure Mesh requirements validation:** only after a validated synchronous owned-node use case may a separate Noise Phase 1 RFC be promoted from Research to Target; implementation then requires real two-node revocation/replay/re-pair/recovery drills and external assessment before any E2EE claim.
11. **MLS requirements study:** move from Research only if multi-member or asynchronous group requirements make pairwise Noise insufficient. This is not an automatic upgrade.
12. **Workflow algorithm registry:** purpose-specific sealing, minimization, aggregation, and explicit declassification modules so complete business chains can cross different boundaries without a universal anonymization shortcut.

Every stage retains a runnable founder walking skeleton. A trust-layer stage is not accepted if the existing local consultant/customer drafting workflow regresses or the default product makes security configuration the primary task.

## 13. Acceptance boundary for this development cycle

The current program claims only independently merged and tested stages. Documentation acceptance requires a consistent Current/Target/Research vocabulary and reviewed RFCs/plans. Code acceptance for each stage requires its own red/green evidence, adversarial review, fresh repository gates, commit, and remote push. A failed or unavailable gate is reported, never converted into a passing claim.

This cycle does not by itself deliver dual-root Vault protection, Argon2id recovery, age restore, real cloud egress, Secure Mesh, MLS, production E2EE, whole-device rollback protection, or a third-party audit. Each becomes Current only after its own implementation and verification gate. The ordinary JSON export remains non-restorable until the recovery stage proves otherwise.
