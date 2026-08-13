# Data Sovereignty Boundaries v1

**Status:** Approved product direction; the normative target design is mirrored in draft RFC 0004 and implementation is staged below.
**Date:** 2026-08-13
**Scope:** The first enforceable privacy and compute-placement boundary for Sovereign Founder OS.

## 1. Product promise

Sovereign Founder OS is designed to let people use useful AI without silently surrendering business secrets, personal data, credentials, or authoritative company state.

The default experience must be understandable to a non-specialist. A professional mode must expose the same underlying policy with more detail; it must not bypass enforcement. A powerful device must be able to run all inference and internal processing for a supported workflow locally; an explicitly approved delivery to a named business recipient remains a separate external effect. A low-power device may use user-controlled compute or a public cloud only through an explicit, enforceable data boundary.

This design does **not** claim perfect or absolute security. It defines a testable supported-API threat model, requires fail-closed behavior, and distinguishes current controls from future cryptographic and deployment controls.

### Current implementation gaps

| Area | Repository today | Required by this design |
| --- | --- | --- |
| Raw model route | A caller supplies `prompt`, `task`, and `DataClass`; Amber/Green can reach a provider that self-reports Cloud/Local trust. | Raw request accepted only by a closed local boundary; caller labels never grant egress. |
| Public compute | Deterministic cloud-labelled stand-ins accept the same raw request type. | Only a compiler-owned, purpose-bound projection reaches a closed public broker. |
| Disclosure record | Public fields, caller/provider strings, unsigned and forgeable; useful demo telemetry only. | Private broker-derived evidence, value-free, then signed through the audit ledger. |
| Presets/visibility | Target UX documentation only. | Deterministic immutable snapshot, simple/pro views, widening approval and revocation. |
| Owned-node E2EE | Not implemented. | Non-executable configuration target until a reviewed Secure Mesh protocol ships. |
| Queue | Workflow checkpointing exists but is not an encrypted private-compute queue. | Opaque encrypted-vault handles with expiry/cancel/revocation semantics. |

The first implementation slice closes the legacy raw cloud authority; adding a parallel safe path while leaving the bypass public is not completion.

## 2. Non-negotiable invariants

1. **Protected data is broader than PII.** Customer data, contracts, source code, strategy, financial records, credentials, unpublished work, internal prompts, model context, and derived business facts are protected unless an authoritative policy proves otherwise.
2. **Unknown is protected.** A caller cannot make arbitrary content cloud-safe by labeling a field `public`, `green`, or with a trusted-looking name. The legacy caller-supplied `DataClass` is a conservative compatibility hint only; it can restrict a route but can never grant egress.
3. **Raw protected data never reaches a public model API.** Public cloud inference receives only a locally compiled projection whose complete outbound structure and metadata are independently allowed.
4. **Fully local remains a first-class path.** When local compute is available, the inference and processing steps of a supported workflow can prohibit all network model calls and keep task data on the device. A later named-recipient business effect discloses only the exact content the owner separately approves.
5. **Owned nodes are not public cloud recipients.** Raw protected data may reach an explicitly authorized user/company-owned node only through an authenticated end-to-end encrypted channel and an approved visibility scope.
6. **Protected storage is encrypted; protected transit is E2EE.** Protected authoritative data remains encrypted at rest under user/company-controlled keys. Transit between authorized devices or owned nodes is end-to-end encrypted; a relay or storage service receives ciphertext only. A public model receives no protected payload to decrypt—only an approved projection.
7. **Insufficient compute never weakens policy.** It may select a smaller local model, an owned node, or a queue. It must not silently reclassify data or fall through to a public provider.
8. **Every disclosure is inspectable.** Before dispatch, the system can explain which recipient class may see which projection, for what purpose, under which policy, and for how long. After dispatch, it records value-free evidence of what route was used.
9. **Outputs inherit risk.** Provider responses are untrusted and `RestrictedDerived` by default. They are parsed against a strict shape, classified, and combined with locally sealed values only after the external response is no longer able to influence placement. Rehydrated output is `Protected`.
10. **Authority is closed.** Model adapters, plugins, and ordinary callers cannot forge classification, bypass the egress broker, manufacture disclosure receipts, or self-authorize a wider scope through supported APIs.
11. **Cryptography is standard and reviewed.** The project will not invent encryption algorithms. Secure Mesh implementation requires a dedicated protocol RFC, domain separation, test vectors, external review, and audited libraries.

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

- `Protected`: raw or derived business secret, personal data, credential, private content, or unknown caller value.
- `RestrictedDerived`: transformed content that still may identify, reconstruct, or reveal protected facts.
- `PublicContent`: fixed public content or authoritative published data whose trusted provenance is independently established.

`Unknown` enters the system as `Protected`. Legacy caller-supplied Amber and Green values also enter this boundary as `Protected`; their old labels remain available to capability/effect compatibility paths but do not establish provenance. Sensitivity comes from trusted source provenance, not the destination field selected by a caller. A transform does not relabel its input as reusable public content. It can produce only an opaque job bound to one purpose, recipient class, policy snapshot, expiry, and exact projection. The legacy policy engine's `cloud.*` rule remains defense in depth, not public-inference authorization.

An unknown value also starts with `ThisDevice` visibility and no transform/declassification grant. Root keys, company/device private keys, permanent credentials, recovery secrets, and equivalent non-exportable secrets are never model inputs or owned-node payloads. Components receive only operation-scoped handles from the vault or effect broker.

### 4.2 Visibility scope

- `ThisDevice`: only the current device and its local processes inside the trusted runtime boundary.
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

### 5.1 Simple security presets

The default is **Auto Protect**.

| Preset | User-facing meaning | Enforced behavior |
| --- | --- | --- |
| **Auto Protect** | “Finish the task safely using my device first.” | Prefer local; allow authorized owned nodes; allow public compute only for a compiler-validated projection; otherwise queue or explain the missing resource. |
| **Local Only** | “Nothing for this task leaves this device.” | Disable owned-node and public-model dispatch. A missing model or capacity returns local alternatives and queue options. |
| **My Devices & Company Nodes** | “Protected data may use only devices I control.” | Prefer local, then authorized E2EE owned nodes; public compute is disabled. |

Presets produce a versioned, immutable policy snapshot. Changing a preset affects new decisions and must not rewrite earlier evidence. An in-flight job stays bound to its original snapshot until its short expiry unless a separately versioned revocation epoch invalidates it; an ordinary policy edit is not an ambiguous implicit revocation.

Snapshots use a fixed schema version and canonical encoding. Timestamps, random identifiers, and map insertion order do not affect the deterministic policy digest. If a professional control changes a preset's security meaning, the UI and snapshot identify the result as `Custom`; it must not continue displaying a stronger preset name.

Every preset is bounded by a non-overridable safety baseline. Professional settings may freely narrow the effective policy by intersection. They may select or restrict pre-registered sources, transforms, purposes, providers, and nodes, but cannot register trust or self-label content. Any widening is a separate owner-reviewed diff and signed policy transition; it never occurs merely because a control changed.

Until Secure Mesh is implemented and verified, an owned-node preference is configuration intent only. It may produce `ComputeUnavailable::ConfigureOwnedNode`, but it cannot produce an executable `OwnedNode` placement or simulate a secure transport.

### 5.2 Professional mode

Professional mode edits the same policy model with controls for:

- data categories and authoritative sources;
- allowed device and node groups;
- public projection templates and purposes;
- provider/model allowlists;
- retention and logging constraints;
- maximum projection size and field topology;
- required approval for scope widening;
- local model preference, resource ceilings, queue deadline, and owned-node fallback;
- output classification and export restrictions.

Professional mode cannot create a raw public-cloud escape hatch. Exceptional declassification must be a separately named, previewed, signed, purpose-bound action.

### 5.3 Visibility explanation

Every task exposes a compact statement such as:

> Local processing. No task data leaves this device.

or:

> Cloud-assisted. The provider can see a generic task instruction and two approved public fields. Customer name, contract text, and company context stay on this device.

The compact copy is precisely “No task data leaves this device”; it does not claim that unrelated operating-system traffic is absent. The expanded pre-dispatch view uses a local, transient `ExposurePreview` to show recipient class, exact outbound field names and public values, locally sealed/omitted categories without their values, purpose, provider-declared/requested retention rule, policy version, and fallback behavior. `ExposurePreview` is not clonable, serializable, persistable, or printable through generic `Debug`. Persisted `ExposureManifest` and post-dispatch evidence are value-free: field paths/dispositions, approved counts, closed identifiers, and commitments only. The compact indicator renders only value-free manifest facts.

## 6. Data-boundary algorithms

Different workflow steps use different algorithms. “Anonymize it” is not a universal permission.

### 6.1 Local raw processing

Protected input is supplied only to a local deterministic component or local model. The runtime blocks network-capable adapters from accepting the raw request type.

“Local” is placement, not trust. A raw-input consumer must be core-reviewed deterministic code or execute behind a capability-constrained boundary with no ambient network, filesystem, environment, or unapproved IPC access. Running arbitrary native model/plugin code on the same device does not authorize it to receive protected input.

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

Secure Mesh is a target in this design, not a current implementation claim. Its RFC must evaluate Signal-style lessons—authenticated identity keys, forward secrecy, post-compromise recovery, replay handling, device changes, key verification, and safe multi-device membership—against job execution rather than copying a messaging protocol blindly.

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
        ├──────── Secure Mesh broker ──E2EE──► owned node
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

Named-recipient effects reuse the existing exact invocation, owner-signed approval, Capability V2, and effects-broker chain. The privacy boundary binds content, recipient, purpose, and policy commitments into that chain; it does not create a second execution-authority system.

Every real public-provider or owned-node dispatch is likewise an external effect authorized above privacy. It binds the job, request, recipient, policy, transform, and expiry commitments into the existing Capability V2, durable Authority Store, and execution journal. Privacy cannot become a second approval issuer or durable replay authority. The deterministic no-network stand-in is explicitly process-local and non-effectful.

### 7.5 Trust Interface

The CLI/UI renders closed, value-free explanations and evidence. It never derives visible sizes or messages from rehydrated protected output, and error/debug surfaces use fixed redacted variants.

## 8. Core contracts

The detailed implementation plan may adjust names to fit existing crates, but it must preserve these semantic types:

```text
TrustedValue<T>          provenance + sensitivity + allowed scope
PolicySnapshot           preset/pro rules + version + digest
TaskIntent               registered purpose + requested output contract
PlacementPlan            ordered authorized compute placements
ExposureManifest         value-free explanation of visibility
ExposurePreview          transient local-only exact pre-dispatch view
LocalTask                protected raw context, not public-dispatchable
PendingPublicJob         opaque projection + sealed local context + one-use state
OwnedNodeJob             target type; unconstructible until verified Secure Mesh exists
DisclosureEvidence       broker-derived route/outcome commitments
ComputeUnavailable       queue/configure-node/install-model/reduce-task options
```

All constructors that establish trust are private or registry-owned. Public structs containing evidence or authority have private fields and read-only accessors. Serialization is allowed only for explicitly reviewed wire/evidence formats. The legacy public-field `DisclosureRecord` is not security evidence and is replaced or sealed during the first slice; signed ledger persistence consumes only broker-derived evidence.

`PolicySnapshot` additionally binds canonicalization version, workspace/issuer, monotonic generation, transform/provider/node registry digest, and revocation epoch, and is authenticated in protected storage. The runtime revalidates expiry, revocation, policy authority, registry membership, and recipient capability immediately before every dispatch/failover and before rehydration. A security-narrowing transition or emergency revocation advances the revocation epoch and invalidates pending/queued work; a non-security metadata edit need not. A later widening never retroactively authorizes an old job.

## 9. Failure semantics

- Unknown source, transform, purpose, provider, field, or metadata: local-required or reject.
- Missing local compute under `Local Only`: queue or `ComputeUnavailable`; never public fallback.
- Missing owned-node keys, revoked node, identity mismatch, stale membership, or failed authentication: no plaintext dispatch; queue or reject.
- Projection compiler mismatch, expiry, digest mismatch, topology mismatch, revocation epoch, registry membership, or authenticated snapshot mismatch: terminally reject the job.
- Provider failure: reject duplicate registered identities, bind each adapter instance to its closed identity, snapshot health once, and attempt each eligible identity at most once under an explicit retry policy; preserve complete attempt history across repeated gateway calls.
- Expiry: check immediately before dispatch and immediately after response. A response arriving after expiry remains recorded as an attempted disclosure, but it cannot be rehydrated or trigger another provider disclosure.
- Malformed or oversized response: terminal invalid-response evidence; never rehydrate.
- Process crash after public dispatch: record `Indeterminate` on recoverable durable state once durability is implemented. The first in-memory version must state its crash limitation honestly.
- UI explanation failure: fixed redacted error; no fallback stringification of protected inputs.

## 10. Evidence and observability

Evidence records may contain only approved public identifiers or opaque local identifiers, policy/plan/request/response digests, timestamps, recipient class, closed provider/model identifiers, transform version, field dispositions/counts, attempt outcomes, and raw public-response byte counts. In-memory broker evidence becomes signed durable evidence only when appended through the existing signed audit ledger; the design does not call an unsigned `DisclosureRecord` non-repudiable.

They must not contain raw prompts, protected values, sealed values, rehydrated output, arbitrary caller labels, provider error text, response excerpts, or lengths derived from private content. Evidence is proof of software state transitions inside the supported boundary; it is not, by itself, proof of a provider's real-world retention behavior.

## 11. Verification strategy

Every implementation slice follows test-first development and an independent attack review.

### Required positive tests

- All three presets compile to deterministic policy snapshots.
- A fully local workflow completes with zero public or owned-node broker calls.
- Before Secure Mesh, a low-power device reports an authorized owned-node target as unavailable/configurable and does not dispatch. Real owned-node selection is tested only with the later E2EE implementation.
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

1. **Privacy boundary recovery:** provenance-bound classification, local-only raw model requests, compiler-owned public projections, strict response validation, closed route evidence, and adversarial corpus.
2. **Policy and visibility v1:** `Auto Protect`, `Local Only`, `Owned Mesh`, professional overrides, exposure manifests, and simple/pro explanations.
3. **Compute placement v1:** capability inventory, deterministic placement order, fully local proof, owned-node target contract, queue, and `ComputeUnavailable` options. No fake network node.
4. **Secure Mesh RFC:** identity, membership, key lifecycle, envelope/session design, replay/expiry, metadata minimization, recovery, test vectors, and audit requirements. This stage does not claim E2EE is shipped.
5. **Secure Mesh implementation:** only after RFC review; audited primitives and libraries, real two-node integration tests, revocation/replay/rotation/compromise drills, and external audit preparation.
6. **Workflow algorithm registry:** purpose-specific sealing, minimization, aggregation, and explicit declassification modules so complete business chains can cross different boundaries without a universal anonymization shortcut.
7. **Trust Interface:** beginner presets, professional policy editor, pre-dispatch visibility preview, evidence history, and safe alternatives when compute is unavailable.

## 13. Acceptance boundary for this development cycle

The current cycle may claim completion only for stages actually merged and tested. At minimum it will:

- replace the unsafe raw Amber-to-cloud model route with a compiler-owned public projection boundary;
- keep and prove a fully local path;
- add deterministic preset and visibility contracts;
- add compute-unavailable/queue/owned-node target placement without pretending E2EE transport exists;
- publish a concrete Secure Mesh RFC and threat-test plan;
- add an independent adversarial suite and close all discovered Critical/High findings before handoff.

Production E2EE, real model providers, durable multi-node evidence, and third-party audit remain clearly marked targets until separately implemented and verified.
