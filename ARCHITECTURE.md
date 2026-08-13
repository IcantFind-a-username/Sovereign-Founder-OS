# Architecture

## Overview

Sovereign Founder OS is the complete product. In the target architecture, its business modules will share an authoritative Sovereign Enterprise Graph, use the Crew Orchestrator to coordinate AI work, and rely on the Sovereign Trust Layer for controlled execution and continuity.

Target product hierarchy:

```text
Sovereign Founder OS
├── Venture Studio
├── AI Crew
│   └── Crew Orchestrator
├── Product & Delivery
├── Customers & Growth
├── Finance / Legal / Tax
├── Founder Command Center
└── Sovereign Trust Layer
    └── Sovereign Runtime
        ├── Model Mesh
        ├── Policy Engine
        ├── Secure Vault
        ├── Audit Ledger
        ├── Tool Sandbox
        └── Recovery Mesh
```

The current implementation focuses on the Sovereign Runtime secure kernel. That is an implementation sequence, not a separate product identity: every runtime capability exists to support real Founder OS workflows.

## Current Developer-Preview Architecture

The executable workspace currently consists of the Rust CLI, thirteen Runtime
crates, and a cross-crate adversarial test package:

```text
sovereign-cli
├── contracts
├── identity
├── artifact
│   ├── publisher COSE manifest → VerifiedArtifact → PreparedInvocation
│   └── content-addressed store + signed admission record → AdmittedArtifact
├── policy
├── capability
│   ├── Capability V1 (legacy Phase A compatibility)
│   └── exact invocation-bound Capability V2
├── authority
│   └── experimental persistent token/approval/idempotency claims
├── execution
│   └── crash journal; a no-terminal Started marker requires reconciliation
├── effects
│   └── rooted local outbox write/revoke broker
├── vault
├── audit-ledger
├── sandbox
│   ├── import-free Wasmtime path (Phase A)
│   ├── verified admitted-artifact V2 pure-compute path
│   ├── authenticated Core Wasm v2 input ABI
│   └── optional killable compiler worker + signed cache
├── model
│   └── deterministic routing/failover simulator
└── workflow
    └── same-directory durable checkpoints and resume

sovereign-adversarial-tests
```

The local app already exposes a minimal Workspace, read-only Founder Command
Center, and Security Center. It stores a company profile, customers, fixed
Offer/Invoice drafts, approvals, local outbox lifecycle, disclosures, and audit
history. This is not yet the target Sovereign Enterprise Graph.

The current primitives include role-separated signatures and trust stores,
deterministic policy decisions, Capability V2, an encrypted-entry vault, a
    device-signed hash chain behind an append-only API, local artifact admission,
    persistent individual authority claims, crash-journaled pure computation, authenticated canonical
guest input, and resource-constrained import-free Wasmtime execution. The
verified V2 executor requires an `AdmittedArtifact`; loading re-derives digests
from stored bytes and fails closed. A killable compilation worker and signed
compiled cache exist, but are optional and are not attached to every product
path.

The assembled local-outbox path is **Experimental**, not a general effect or
plugin boundary. The application currently creates owner approval evidence
after an unauthenticated loopback/API decision. Its capability binds document
and resource preparation, not the final recipient or exact RFC 5322 bytes, and
the execution journal completes before the trusted host writes the outbox
file. The Authority Store persists individual filesystem claims, but current
tests do not prove a true multi-process boundary and approval records may be
purged before the approval itself expires. The complete token/approval/
idempotency reservation is not one transaction and has no revocation API. Core
Wasm guests cannot invoke host effects.

The Model Gateway contains deterministic stand-ins and an unsafe legacy
classification/trust API; it is not a real Model Mesh. Workflow checkpoints
support another runner over the same directory, not replicated node failover.
The broader Enterprise Graph, Crew Orchestrator, Domain Packs, Recovery Mesh,
production Component/WIT host interfaces, real model adapters, and network
effects are targets. See [ROADMAP.md](ROADMAP.md),
[RFC 0002](rfcs/0002-wasm-sandbox-and-plugin-capabilities.md), and
[RFC 0004](rfcs/0004-data-sovereignty-boundaries.md).

The remaining sections describe the target architecture unless they
explicitly state a current capability.

## Target Runtime and Trust Flow (Planned)

```text
Founder Command Center + Product Modules
              │
              ▼
       Mission Compiler
  (natural language → structured enterprise tasks)
              │
              ▼
   Sovereign Enterprise Graph
  (company, customers, contracts, assets, tax, security)
              │
              ▼
       Crew Orchestrator
 (assembles roles; plans never execute)
              │
              ▼
┌──────────────────────────────────┐
│      Sovereign Trust Layer       │
│  classification / permissions /  │
│  risk / approval / jurisdiction  │
└──────────────────────────────────┘
              │
              ▼
     Capability Token Issuer
  (short-lived, scoped, revocable tokens)
              │
              ▼
┌─────────────┬─────────────┬─────────────┐
│ Model Mesh  │ Tool Sandbox│ Domain Packs│
│ multi-vendor│ isolated    │ legal, tax, │
│ routing     │ execution   │ business    │
└─────────────┴─────────────┴─────────────┘
              │
              ▼
       Verification Layer
  (rules, second-model review, schema validation)
              │
              ▼
      Signed Event Ledger
              │
              ▼
 Encrypted Replication & Recovery Mesh
```

## Target Six-Plane Architecture (Planned)

| Plane | Responsibility |
| --- | --- |
| **Intelligence** | Models, agents, planning, reasoning |
| **Policy** | Deterministic permissions, risk, approval |
| **Execution** | Tools, browser, files, code execution |
| **Data** | Encrypted enterprise state, local storage |
| **Trust** | Identity, keys, signatures, audit, software provenance |
| **Recovery** | Replication, checkpoints, failover, disaster recovery |

## Sovereign Enterprise Graph (Planned)

The authoritative state of a company. Key entities:

```text
Founder, Legal Entity, Jurisdiction, Customer, Supplier,
Product, Service, Contract, Invoice, Payment, Asset,
Intellectual Property, Tax Obligation, Compliance Obligation,
Security Asset, Credential, Incident, Business Assumption,
Experiment, Metric, Decision, Approval, Artifact
```

Every agent operation must:

1. Read authorized enterprise state
2. Produce a structured plan
3. Request execution permissions
4. Produce verifiable deliverables
5. Update enterprise state
6. Leave a non-repudiable operation record

## Mutually Constrained Autonomy

| Role | Can Do | Cannot Do |
| --- | --- | --- |
| **Planner** | Create plans | Hold real tool credentials |
| **Policy Guard** | Allow/deny actions | Generate business goals |
| **Executor** | Execute approved actions | Expand its own permissions |
| **Auditor** | Verify and record | Execute external actions |
| **Recovery Controller** | Restore system | Modify normal business records |
| **Human Owner** | Final approval | Be bypassed for high-risk ops |

## Target Agent Execution Flow (Planned)

```text
Untrusted external content
        │
        ▼
Untrusted Content Zone
  (data only, never system instructions)
        │
        ▼
AI Planner / Analyst
  (proposes plan and actions)
        │
        ▼
Deterministic Policy Engine
  (validates permissions, scope, risk, approval)
        │
        ▼
Capability Token Issuer
  (short-lived, resource-bound token)
        │
        ▼
Sandboxed Executor
  (minimum privilege, temporary credentials)
        │
        ▼
Auditor + Signed Event Ledger
```

**Critical invariant:** "What the model suggests" and "What the system allows" are always separated.

## Crew Orchestrator (Planned)

The Crew Orchestrator turns a business goal into a temporary, constrained AI team. Agents are not permanently assigned roles. Crews are assembled per task based on:

- Current venture stage
- Task type
- Required tools
- Data sensitivity
- Cost budget
- Error risk
- Human approval requirements

Typical ephemeral roles: Researcher, Strategist, Builder, Critic, Operator, Evaluator.

When the task completes, the crew dissolves. Only results, evidence, and decision records persist.

## Model Mesh (Planned)

A unified Model Gateway routes requests to:

| Model Type | Use Case |
| --- | --- |
| Local or authorized owned-node model | Protected-data classification, extraction, summarization, and reasoning |
| Public cloud model | Compiler-created, purpose-bound public projections only |
| Strong reasoning model | Strategy and research within the effective data boundary |
| Multimodal model | Web, images, documents, video |
| Coding model | Websites and prototypes |

Every supported dispatch records value-free visibility and route evidence.
Provider-declared cost, latency, quality, and retention are metadata—not proof
that a provider enforced a promise.

Failover may move only among recipients already authorized by the same immutable
policy snapshot and compiled request. Provider self-description and caller
labels never grant data access. Raw protected values remain local or on an
explicitly authorized E2EE owned compute endpoint; public providers receive
only compiler-created projections. See [RFC 0004](rfcs/0004-data-sovereignty-boundaries.md).

## Plugin Architecture (Experimental foundation)

The target architecture treats plugins as **untrusted by default**. The current
repository implements import-free Wasmtime paths plus a pure-compute foundation
for publisher verification, local admission, exact invocation binding, durable
authority when attached, and authenticated Core Wasm v2 input. It is not a
general Component/WIT extension boundary.

- Signed manifest declaring exact permissions
- Low-risk plugins: WASM/WASI sandbox
- High-risk tools: ephemeral container or micro-VM
- No shared memory with core process
- No permanent API keys
- No arbitrary network access

## Event Sourcing (Partially Implemented)

The current audit-ledger crate exposes an append-only API over a replaceable
local JSON file whose entries form a device-signed hash chain. Verification
proves internal chain consistency; without an independently trusted device key
and anchored head it cannot detect whole-bundle replacement, valid-prefix
truncation, or rollback. The target architecture builds authoritative
enterprise state from richer events such as:

```text
event_id, venture_id, actor_id, action, resource,
capability_id, timestamp, payload_hash, previous_event_hash,
device_signature, policy_decision_hash
```

Internal tamper detection exists in the current ledger prototype. Authenticated
head checkpoints, rollback detection, derived snapshots, and recovery replay
are planned.

## Technology Stack (Planned)

| Layer | Technology | Scope |
| --- | --- | --- |
| Sovereign Runtime | **Rust** | Vault, crypto, policy, capability tokens, audit ledger, sandbox, mesh |
| UI & SDK | **TypeScript + React + Tauri** | Desktop app, Founder Command Center, approval UI |
| Agent Workers | **Python** (isolated, untrusted) | Workflows, RAG, domain packs, evals |
| Protocols | JSON Schema, Protobuf/gRPC, WIT/WASI, MCP, A2A | Contracts, IPC, plugins, tools |

Python workers must never hold root keys or permanent permissions.

## Repository Layout (Planned)

```text
sovereign/
├── apps/          desktop, cli, demo-founder
├── crates/        kernel, vault, policy, sandbox, model-router, ...
├── packages/      sdk, contracts, plugin-sdk, ui
├── workers/       agents, evals, domain-runtime
├── packs/         founder, security, jurisdictions
├── tests/         adversarial, chaos, recovery, conformance
├── security/      threat-model, attack-trees, disclosures
├── rfcs/
└── docs/
```

## Target Comparison with Personal AI Assistants

| Personal AI Assistant | Sovereign Founder OS |
| --- | --- |
| Capability-first | Business-operation first, backed by enforceable trust boundaries |
| Chat and channel driven | Enterprise state and workflow driven |
| Model fallback | Model, node, key, data, and policy multi-layer fallback |
| Plugins may run in-process | Plugins isolated by default |
| Single gateway | Multi-node state and recovery mesh |

See [Why Not Another Agent?](docs/positioning/why-not-another-agent.md) for the full positioning.

## Further Reading

- [Distributed systems](docs/design/distributed-systems.md) — target replication, failover, and split-brain prevention design
- [THREAT_MODEL.md](THREAT_MODEL.md) — adversary model and mitigations
- [Privacy model](docs/design/privacy-model.md) — target Red/Amber/Green data zones
- [Historical Chinese project plan](docs/archive/zh/03-开源项目企划书-v0.1.md) — early design context, not a current specification
