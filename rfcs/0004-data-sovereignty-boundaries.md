# RFC 0004: Data Sovereignty Boundaries

**Status:** Draft; approved implementation target
**Stage:** 2
**Security impact:** Critical

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

## Normative vocabulary

The keywords MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are interpreted as
requirements for the supported Sovereign Runtime API boundary.

### Sensitivity

- `Protected`: raw or derived business secret, personal data, credential,
  private content, or unknown dynamic value.
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
declassification grant. Root/company/device private keys, permanent
credentials, recovery secrets, and equivalent non-exportable secrets MUST
never become model input or an owned-node payload; components receive only
operation-scoped handles from the vault or effects broker.

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

Executable placement values are `Local`, `OwnedNode`, and
`PublicProjection`. `Queued` is a `PlacementDecision`, not an executable
compute location. Policy authorization, observed runtime capability, and
executable placement MUST be distinct. Before Secure Mesh is implemented,
`OwnedNode` MUST NOT be executable and MUST instead return a
configuration/queue option.

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

### Local model boundary

Raw `ModelRequest` MUST be accepted only by `LocalModelProvider` instances.
Local provider identity and task identifiers MUST come from closed or
authoritative registries. A trait method or caller string MUST NOT establish
local trust.

### Public projection boundary

Only the privacy compiler may construct a pending public job. Its request
bytes MUST be canonical, size-bounded, and bound to:

- a job nonce and expiry;
- registered task, template, purpose, and transform versions;
- immutable policy snapshot and revocation epoch;
- exact outbound bytes and output topology;
- sealed/omitted/public dispositions and commitments;
- allowed recipient/model classes and retention constraints.

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

Persisted `ExposureManifest`, disclosure evidence, logs, errors, metrics, and
receipts MUST be value-free. Their identifiers must be closed public values
or opaque local IDs. Evidence MUST be derived from job/broker state rather
than caller fields. Provider error or response excerpts MUST NOT be included.

## Presets

V1 defines three presets:

1. `AutoProtect` (default): prefer local, permit verified owned nodes when
   available, permit public compute only for a compiler-created projection,
   otherwise queue or return safe alternatives.
2. `LocalOnly`: no model or owned-node network dispatch for the task.
3. `OwnedMesh`: protected data may use this device and authenticated
   user/company-owned nodes; public compute is unconditionally disabled while
   this preset remains selected. Adding projection permission is a signed
   transition to `Custom` and changes the displayed preset name.

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

Named-recipient effects MUST reuse the existing prepared-invocation,
owner-signed approval, Capability V2, and effects-broker chain. This RFC binds
content, recipient, purpose, and policy commitments into that chain; it does
not create a parallel execution authority.

Every real public-provider or owned-node dispatch is likewise an external
effect authorized above the privacy crate. It binds job, request, recipient,
policy, transform, and expiry commitments into the existing Capability V2,
durable Authority Store, and execution journal. Privacy may own opaque
compiler state but MUST NOT become a second approval issuer or durable replay
authority. The first deterministic no-network stand-in remains explicitly
non-effectful and process-local.

## Compute failure

Insufficient compute MUST NOT weaken the policy. The scheduler SHOULD try,
in authorized order, deterministic local computation, compatible small local
model, configured full local model, verified owned node, and an available
public projection. If none can run, it MUST return `Queued` or a closed
`ComputeUnavailable` set such as install/configure a local model, configure an
owned node, reduce the task, or wait. A local-required task MUST NOT be
convertible into a public job.

Queued work contains only an opaque handle to authenticated-encrypted vault
state, never a plaintext prompt or protected workflow JSON/log entry. Queue
records require expiry, cancel/delete, policy/revocation binding, and explicit
crash semantics. An initial process-local queue MUST be labelled non-durable.

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

Protected authoritative storage remains encrypted under user/company keys.
Protected traffic between authorized devices/nodes MUST be end-to-end
encrypted so relays and storage services receive ciphertext only. Public
models receive no protected ciphertext or keys; they receive a safe
projection instead.

Secure Mesh is not implemented by accepting this RFC. A separate protocol RFC
must specify identity and membership, key verification and rotation, forward
secrecy, post-compromise recovery, replay and rollback handling, revocation,
job/request/response binding, metadata minimization, recovery, canonical test
vectors, and audited libraries. Signal-style designs are research input, not
a protocol to copy without adapting its messaging assumptions to job
execution.

An authorized owned-node endpoint sees the plaintext categories it computes
over. E2EE hides them from relays, transport, and blind storage; it does not
make the selected endpoint unable to read its job.

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
- local compute failure that cannot create or reach a public request.

Every implementation slice requires an independent adversarial review. A
release claim requires no known Critical or High finding inside the declared
supported-API threat model and all repository gates passing. This is not an
absolute-security claim.

## Rollout order

1. Close the legacy model egress authority and implement one fixed projection
   workflow with strict validation and value-free evidence.
2. Add presets, visibility contracts, previews, and professional policy
   refinement.
3. Add compute capability inventory, local proof, queue, and unavailable
   options; keep owned-node execution disabled.
4. Accept the Secure Mesh protocol RFC.
5. Implement and audit real owned-node E2EE and later real public egress.
6. Add purpose-specific transform/algorithm modules and the complete Trust
   Interface.

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
