# Roadmap

## Vision

Sovereign Founder OS is an AI operating system for building and running a
one-person business. It is not primarily a security framework.

The product evolves as two connected layers:

- **Founder OS** helps a founder find opportunities, win and serve customers,
  manage work and records, and make better decisions.
- **Sovereign Trust Layer / Runtime** constrains AI and tools so that work stays
  auditable, recoverable, portable, model-neutral, privacy-aware, and under the
  founder's control.

Every milestone must deliver a founder-visible outcome and the minimum trust
boundary required for that outcome. This roadmap defines evidence, not dates.
Protocol detail belongs in [ARCHITECTURE.md](ARCHITECTURE.md),
[THREAT_MODEL.md](THREAT_MODEL.md), and the [RFCs](rfcs/).

## Maturity legend

| Label | Meaning |
| --- | --- |
| **Current** | Implemented and verified here within the stated boundary. |
| **Experimental** | Runnable and tested, but narrow, simulated, or not hardened. |
| **Target** | Intended near-term outcome; not yet available. |
| **Research** | An option to validate before committing to it. |

“Current” never means universally secure or production-ready. “Complete” is
used only for a named exit criterion with evidence.

## Current verified state

The repository is a **Developer Preview / pre-release** with no tagged release.
The workspace currently passes 183 Rust tests plus formatting, lint,
file-size, locked dependency, TypeScript, and release-build gates.

### Founder product today

The loopback-only local web app has three real surfaces:

| Surface | Current capability | Important limit |
| --- | --- | --- |
| **Command Center** | Read-only business counts, pending decisions, deterministic next actions/risks, evidence rollups | No real company-stage, finance, project, or analytics model |
| **Workspace** | One company profile; append customers; fixed non-editable Offer/Invoice drafts; deterministic unsaved outreach suggestion; approve/reject; local `.eml`; revoke; unauthenticated manual-delivery marker; plaintext export | No real LLM, editing, CRM, projects, contracts, expenses, restore, or network send |
| **Security Center** | Identity/vault/audit/plugin/disclosure facts, state reconciliation, live attack gauntlet | Reconciliation checks selected event presence, not every workspace value |

Another machine can verify an export's format, device-key binding, and signed
audit chain. It cannot yet authenticate every workspace value or restore the
installation. Export excludes keys, artifacts, outbox files, authority records,
journals, and workflow checkpoints. The loopback server has no authenticated
owner session.

### Experimental Trust Layer slice

One built-in local effect assembles real primitives:

```text
deterministic policy
  → application-created owner-key signature
  → Capability V2
  → durable Authority Store claims
  → Execution Journal
  → admitted import-free Wasm computation
  → rooted local outbox write
  → audit-first workspace commit + device-signed hash chain
```

This is **Experimental**, not an exact external-effect boundary. The approval
binds document/resource preparation, not the final recipient and RFC 5322
bytes. The execution journal finishes before the trusted host writes the file.
An unauthenticated local API decision causes the backend to use the owner key.
It proves that authorization machinery precedes this local write; it does not
prove independently authenticated human intent or exact effect authorization.

### Foundation maturity

| Area | Maturity and honest boundary |
| --- | --- |
| **Identity, policy, capability** | Current cryptographic/deterministic primitives; owner ceremony, final effect binding, key lifecycle, and hardware/OS storage remain targets. |
| **Vault and audit** | Experimental AES-GCM entries and a replaceable-file, device-signed hash chain that proves internal consistency; the vault key sits beside the data, rollback anchoring, restore, and state-value commitments remain. |
| **Authority and execution** | Experimental filesystem claims plus crash journal. A known approval-expiry retention gap prevents a durable one-use approval claim; claims are not transactional, revocation is absent, and process-level coverage remains. |
| **Artifacts and Wasm** | Current verification/admission and tested import-free V2 pure computation; worker/cache are optional, cache bindings are partial, and Component/WIT/high-risk isolation remain. |
| **Model Gateway** | Experimental deterministic routing simulator. Caller classification and provider self-reported trust can authorize unsafe Amber/Green cloud routing; [RFC 0004](rfcs/0004-data-sovereignty-boundaries.md) must remove it before real egress. |
| **Workflow and releases** | Same-directory checkpoint resume and `v*` release automation exist; no lease, replication, real node failover, or tagged preview exists. |

## Milestone map and dependencies

Versions are integration labels, not a strict work sequence. Product discovery,
recovery design, domain-source work, and protocol research may proceed in
parallel; a release claim must still pass its own gates.

| Milestone | New founder outcome | Principal gate |
| --- | --- | --- |
| **v0.1** | Run the narrow local workflow honestly | Exact local-effect grant, owner authentication, fault tests, tagged preview |
| **v0.2** | Use a useful real local model without granting authority | RFC 0004 boundary, validated tasks, zero egress in `Local Only` |
| **v0.3** | Complete one consultant lead-to-invoice/follow-up loop | Minimal Enterprise Graph and resumable structured workflow |
| **v0.4** | Dispatch one reviewed email safely | Minimum restore first; at-most-once automatic dispatch; explicit uncertainty |
| **v0.5** | Use the consultant workflow repeatedly as an Alpha | Usability evidence, migrations, encrypted backup/restore |
| **v0.6** | Recover, rotate, and move sovereign state | Clean-machine restore, rollback rejection, device revocation |
| **v0.7** | Receive evidence-based priorities and bounded Crew help | Stable task/graph contracts, evaluations, no standing AI authority |
| **v0.8** | Install one constrained third-party extension | Stable Component/WIT boundary and adversarial conformance |
| **v0.9** | Use one reviewed Singapore domain pack | Versioned sources, uncertainty, escalation, professional review |
| **v1.0** | Reliably operate the supported consultant loop | All Community Edition gates below |

Hard dependencies:

- RFC 0004 precedes any real public-model claim.
- Persona/task validation begins in v0.1 and defines “useful” model work in
  v0.2.
- Exact durable effect authority and a minimum tested recovery package precede
  any irreversible network effect.
- Stable task/graph contracts precede reusable Crews or domain automation.
- Clean-machine restore precedes broader bounded autonomy.
- Stable extension and pack schemas precede an SDK/registry or `Verified` pack.
- Replication, leases, and fencing begin only for an accepted multi-node need.

## Version milestones

### v0.1 — Trustworthy local foundation

**Status:** Experimental baseline; hardening and first tagged preview remain.

**Founder outcome:** Keep a small local workspace, review a local action,
inspect evidence, export supported state, and verify the export format and
signed audit chain offline.

**Remaining work:** correct stale UI/docs claims; validate the consultant
persona and first private-AI tasks; mandate isolated compilation/cache on the
product path; authenticate owner presence; bind recipient/content/policy/expiry
into an opaque effect grant; make authorization claims transactional and
revocable; retain approval claims through approval expiry; upgrade audit/effect
ordering and rollback anchoring; protect local keys; add process-kill,
concurrency, and filesystem-fault tests; publish preview binaries.

**Exit criteria:**

1. A clean install completes the local founder flow without source knowledge.
2. No outbox effect exists without independently authenticated approval bound
   to the exact grant; replay, substitution, rejection, and interruption fail
   closed under concurrent process and fault-injection tests.
3. Export/integrity checks bind supported workspace values and document every
   excluded item.
4. CI builds one tagged, signed/checksummed Developer Preview with no
   production claim.

### v0.2 — Useful private AI

**Status:** Target.

**Founder outcome:** Use a real local model for validated summarization,
reasoning, and drafting without giving it authority or requiring a cloud
account.

**Work:** implement RFC 0004's local-only raw request and compiler-owned public
projection boundary; add `Auto Protect`, `Local Only`, and `My Devices &
Company Nodes`; expose value-free visibility records; add deterministic
placement/queue/`ComputeUnavailable`; integrate one replaceable local backend;
retain deterministic fallbacks; run privacy-canary and attack reviews. Owned
node execution remains disabled until a separate Secure Mesh protocol exists.

**Exit criteria:**

1. One real local backend completes three persona-validated tasks as
   suggestions only, with zero authoritative mutations.
2. `Local Only` produces zero public/owned-node broker observations; local
   compute failure queues or explains alternatives, never cloud-falls-back.
3. No dynamic caller value can reach a public adapter through the supported
   Rust API; the legacy Amber/Green route is gone.
4. Every model use has an understandable visibility record and no known
   Critical/High finding inside the declared API threat model.

### v0.3 — First real founder workflow

**Status:** Target; initial persona: independent consultant.

**Founder outcome:** Complete one coherent loop:

```text
lead → discovery context → proposal draft → founder review
     → delivery preparation → invoice draft → follow-up
```

**Work:** validate the workflow with founders; build only the required
`Founder`, `Company`, `Offer`, `Customer`, `Project`, `Task`, `Document`,
`Decision`, `Risk`, `InvoiceDraft`, and `Activity` state; add editing,
provenance, lifecycle, search, migration, local-AI drafts, and plain onboarding.

**Exit criteria:**

1. A non-technical test founder completes the loop without Trust Layer jargon.
2. Structured exportable business state—not chat history—is authoritative.
3. AI changes remain drafts until founder or deterministic authority commits
   them.
4. Process interruption resumes without repeating a completed covered step.

### v0.4 — First controlled network effect

**Status:** Target; RFC-first and recovery-gated.

**Founder outcome:** Review exact email content and recipient, then authorize
at most one automatic dispatch while understanding provider uncertainty.

**Work:** accept an email-effect RFC; bind recipient, headers, body,
attachments, account, approval, policy, and idempotency; broker OAuth as a
handle; persist intent before dispatch; defend against substitution, injection,
exfiltration, crashes, replay, and timeout-after-success; retain local `.eml`.

**Exit criteria:**

1. A model cannot send, choose the final recipient, or expand attachments.
2. One approval causes at most one automatic provider dispatch. An ambiguous
   result requires reconciliation or new approval, never blind retry.
3. Outcomes are `AcceptedByProvider`, `FailedBeforeDispatch`, or
   `Indeterminate`; recipient delivery is never inferred from acceptance.
4. A minimum encrypted restore drill passes before credentials/network are
   enabled, and the full attack matrix passes against a test account.

### v0.5 — Founder Operations Alpha

**Status:** Target.

**Founder outcome:** Use the consultant workflow repeatedly to manage
priorities, customers, projects, documents, decisions, follow-ups, and basic
money visibility.

**Work:** product-first Command Center; customer/project/delivery views; basic
revenue, expense, receivable, and cash views; search/history; onboarding,
accessibility, stable migrations, and advanced Trust views on demand.

**Exit criteria:**

1. Pilot founders use the supported loop over multiple weeks without terminal
   or kernel knowledge.
2. State remains exportable and covered by documented integrity/migration
   checks.
3. A clean install restores the encrypted Alpha package before pilots entrust
   multi-week records to it.
4. Telemetry, if added, is opt-in and cannot collect protected content by
   default.

### v0.6 — Recovery and sovereign data

**Status:** Target.

**Founder outcome:** Lose or replace an installation and restore verified
business state without an official server.

**Work:** encrypted backup destinations/packages, recovery codes, key rotation,
device revocation, versioned migrations/rollback, recovery UX, and restore
drills. Multi-node work waits for a measured need.

**Exit criteria:**

1. A fresh machine restores identity/history and resumes without duplicate
   effects.
2. Revoked devices cannot decrypt newly rotated state.
3. Corrupt, stale, rolled-back, or incomplete backups fail visibly.
4. Recovery succeeds without official infrastructure.

### v0.7 — Founder intelligence

**Status:** Target.

**Founder outcome:** Receive evidence-based priorities and use a temporary,
bounded AI Crew for one business goal.

**Work:** goals, assumptions, experiments, feedback, metrics, decision history,
stage-aware prioritization, independent critique, and explanation. Crews
propose/evaluate; they never gain standing authority.

**Exit criteria:**

1. Recommendations cite facts, assumptions, uncertainty, and remain distinct
   from founder decisions.
2. Removing/replacing models cannot corrupt state or evidence.
3. Evaluations demonstrate useful prioritization plus abstention when evidence
   is insufficient.
4. One Crew improves a defined workflow result without expanding authority;
   only reviewed results/evidence persist.

### v0.8 — Secure extension ecosystem

**Status:** Target.

**Founder outcome:** Install one third-party extension and understand its exact
data/effect access.

**Work:** stable manifest/SDK, Component/WIT interface, reviewed host brokers,
publisher/provenance UX, budgets, revocation/update policy, conformance kit,
and narrowly constrained MCP interoperability.

**Exit criteria:**

1. A third party ships one useful extension without changing core code.
2. Undeclared network, filesystem, environment, credential, and host access is
   mechanically denied.
3. Installation explains effective permissions/visibility; revocation stops
   future access.
4. Conformance and malicious-fixture suites pass.

### v0.9 — Reviewed domain packs

**Status:** Target; first candidate: Singapore.

**Founder outcome:** Receive source-backed reminders/templates and
uncertainty-aware administrative/legal/tax assistance with professional
escalation.

Every rule binds jurisdiction, source, effective date, review date,
assumptions, uncertainty, and escalation.

**Exit criteria:**

1. One narrow pack passes appropriate professional review.
2. Every conclusion traces to versioned sources and assumptions.
3. Missing/stale facts cause uncertainty or escalation, not invented certainty.
4. The product never presents a pack as autonomous professional advice.

### v1.0 — Community Edition: Consultant Core

**Status:** Target; defined by evidence, not a date.

The first Community Edition is deliberately scoped to the validated consultant
loop. Broader Founder OS modules remain directions, not hidden v1 gates.

A non-technical founder can:

1. install, upgrade, and roll back the supported app;
2. repeatedly complete the supported lead-to-invoice/follow-up loop;
3. use a fully local AI path and switch between two independently implemented
   model backends without corrupting state;
4. approve exact external actions and understand data visibility and evidence;
5. export, clean-machine restore, and offline-verify all state needed to resume
   without an official service;
6. recover from covered process/device failure without silent duplicates or
   hidden `Indeterminate` outcomes;
7. pass automated, adversarial, migration, recovery, and appropriate fuzz/
   property suites with no known Critical/High finding in the declared API
   threat model;
8. install reproducible signed/checksummed releases with SBOM, provenance,
   supported-platform matrix, and honest limitations;
9. rely on independent external review of protocols used for public egress,
   owned-node compute, or irreversible effects, with material findings resolved
   or release-blocking.

v1.0 does not mean perfect security, every business type, or every jurisdiction.

## Parallel development tracks

| Track | Near-term | Later |
| --- | --- | --- |
| **Product** | Consultant loop, onboarding, customer/document lifecycle | Daily operations, validated new personas |
| **Intelligence** | Safe local model, useful drafts/summaries | Crews, recommendations, multimodal work |
| **Trust & Security** | RFC 0004 and exact local-effect boundary | Controlled network effects, external review |
| **Data & Recovery** | Minimal graph, migrations, state integrity | Restore, rotation, justified multi-device recovery |
| **Ecosystem** | Component/WIT contracts | SDK, registry, constrained interoperability |
| **Domain Knowledge** | Pack schema/source research | Reviewed jurisdictions and domains |

Each release cycle advances a Product outcome and its required Trust/Data
gates. Infrastructure-only releases require an explicit dependency reason.

## Explicit non-goals

Early versions will not replace licensed professionals, rebuild a full ERP,
support every business type, create an autonomous CEO, give LLMs final
high-impact authority, implement custom cryptography/blockchain, put private
data on public chains, depend on one provider/server, offer unrestricted
email/payments/contracts/browser/native plugins, or claim absolute security or
production/enterprise assurance without scoped evidence.

## Post-v1.0 research and expansion

Not committed milestones:

- authenticated E2EE Secure/Resilient Mesh, blind replicas, fencing, and
  multi-device trust—only for a measured availability/low-power requirement;
- cryptographic agility and standards-based post-quantum migration;
- audit anchoring/distributed trust only for a real multi-party problem;
- HSMs, enterprise identity, multiple approvers, managed sync, HA, and SIEM
  without weakening the Community baseline;
- broader Founder OS modules: Venture Studio; product/software/content
  builders; deeper growth/feedback; contracts/obligations; richer finance;
- additional jurisdictions/professions with domain experts.

## Contribution opportunities

Start with [CONTRIBUTING.md](CONTRIBUTING.md) and an issue with acceptance
criteria. Founders and UX contributors can validate workflows/language;
frontend contributors can improve onboarding/accessibility; AI/ML contributors
can build adapters/evaluations; Rust engineers can work on state/brokers/
recovery; security researchers can add attacks/fuzzing; domain professionals
can review packs; localization/documentation contributors can improve access.
Subsystem contracts should keep most contributions out of the security kernel.

## Roadmap governance

Claims are checked against code, tests, and releases. Roadmap inclusion does
not accept an RFC; RFC acceptance does not claim implementation.

An RFC is required for trust-boundary, authority, persistent-state,
external-effect, major protocol/dependency, plugin-isolation, distributed,
blockchain, or high-impact legal/tax changes. Small UI/docs/isolated fixes do
not require one.

RFC design status is `Draft`, `Accepted`, `Rejected`, or `Superseded`, separate
from implementation status. [CONTRIBUTING.md](CONTRIBUTING.md) requires a public
draft, at least seven days of discussion for substantial changes, and
maintainer rationale. Security-sensitive RFCs also require a threat-model
delta, adversarial test plan, migration/rollback analysis, and independent
review when a release gate calls for it.

Roadmap changes state the founder outcome, evidence, risks, migration impact,
and affected tracks. Historical plans remain context, not commitments.
