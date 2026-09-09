# Founder MVP — Consultant Core v1

**Status:** Current (Experimental product slice), landed 2026-09-10 on `main`.
This document records what the MVP is, the boundaries it keeps, and what it
deliberately does not claim. For product work it takes precedence over the
2026-08-14 playground plans; it changes no RFC and relaxes no security gate.

## What the founder can do

One local web app (`sovereign ui`, loopback only, EN/中文) runs the
consultant loop the roadmap's v0.3 describes, on the existing kernel:

```text
lead → discovery notes → AI analysis → AI proposal draft → founder edits
     → send (signed approval, local .eml) → record acceptance
     → AI delivery plan → tasks → project done → AI invoice draft
     → issue (signed approval) → record payments → receivables
     → compliance check with cited rules, any time
```

Pages: Today (decisions inbox, team suggestions, guidance, evidence),
Company (profile and registration facts, backup/verify), Customers (stages,
discovery, projects, tasks, follow-ups, timeline), Documents (editable
drafts, approvals, receivables), Team (hire/run AI employees), Compliance
(rule pack checks and search), Security (unchanged).

## The kernel underneath, unchanged

- Every mutation validates, asks the deterministic policy engine, then
  commits audit-first to the signed hash chain; the vault holds the state.
- Sending still runs the RFC 0003 chain (owner-signed approval, Capability
  V2, verified sandbox step, audited local outbox write). Nothing leaves the
  device over the network.
- AI employees hold no keys and change nothing. A run produces a pending
  `Decision` carrying the exact `ProposedChange`; approval applies exactly
  that change in one commit with the decision. Model output must validate
  against the role's schema to replace the deterministic template, and the
  decision shows which one produced it and which provider answered.
- Model calls go through the gateway with disclosure records. Providers are
  the deterministic stand-ins plus an optional Ollama daemon over loopback
  configured in `model.json`; no cloud adapter exists.
- Compliance checks read founder-entered facts only, cite a source per
  rule, and never say "compliant"; the Singapore pack declares itself an
  unreviewed demo pack.

## Entity model (workspace version 2)

Existing: Venture, Customer, Document, Approval, ModelDisclosure. Added:
Venture registration facts (jurisdiction, currency, UEN, GST, incorporation
date, fiscal year end, revenue estimate); Customer stage, discovery notes,
jurisdiction, consent; Document revision, due date, project link,
acceptance; Project (+acceptance criteria), Task, FollowUp, Payment,
Employee, Decision (with ProposedChange), ComplianceReport. Version-1
vaults load through serde defaults and are stamped on the next commit.

## Honest limits

- Single device; the vault key sits beside the data (unchanged from v0.1).
- The loopback API has no owner authentication (the 1C0 gate remains).
- AI employees are bounded: six roles, deterministic templates without a
  model, validated JSON with one. No autonomy, no tools, no recursion.
- The compliance pack is a demo written from cited sources, not reviewed
  by a professional; coverage is Singapore only; other jurisdictions yield
  an explicit "not covered".
- The Ollama process is routed to, not confined; RFC 0004 still gates any
  real egress and any privacy claim.

## Where things live

`apps/cli/src/workspace/`: `erp_*` (business graph), `crew_*` (employees),
`compliance*` (rule pack, retrieval, checks), `model_config.rs`;
`apps/cli/src/ui_mvp.rs` (routes); `apps/cli/assets/` (`crm.js`,
`team.js`, `compliance.js`, `i18n-mvp.js`); `crates/model/src/ollama.rs`.
