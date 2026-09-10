# MVP walkthrough without a model — every route, every page, two defects fixed

- **Outcome:** the founder MVP driven end to end by script and by browser on a fresh workspace and on the demo workspace; two defects fixed in `#91`; four product findings queued; the live-model half of the backlog item still blocked
- **Base:** `02508dd` (main at the start of the session)
- **Backlog item:** "Live verification with a real Ollama model and the five-consultant usability protocol" — the half of it that needs no model
- **Machine:** the founder's Mac, macOS 26.5 arm64, `target/debug/sovereign ui` on ports 7791 (demo data) and 7793 (fresh `HOME`)

## The one thing to read first

**Ollama is not installed on this machine**, so the part of the backlog item
that names a real model — which prompts validated per role, which fell back to
the template, and why — did not run and is not reported here. Installing
software while the founder was asleep was not mine to do. Every proposal in
this walkthrough was produced by the built-in template drafter, and the app
says so on every one of them (`模板 · 产出方: local-drafter`, `model_backed:
false`). Nothing below is evidence about model quality.

What did run is everything else: 60 API steps through every workspace route
on a fresh installation, a DOM-level survey of all eight pages, full-page
screenshots of each, and the business flow from an empty company to a paid
invoice with a compliance check on the way.

## What was found

Two defects that a reviewer would have hit within minutes, both fixed in
`#91` with tests that were shown to fail against the previous code:

**Backup verification refused every real backup.** The Company page's
"verify a backup file" posts the exported bundle to `/api/verify-export`,
which read it under the general 64 KiB request cap. A workspace with one
customer, three documents and ninety audit events exports at 118 KB. So the
route that exists to check a backup refused any backup worth checking. It now
reads up to 32 MiB — bounded, for the same reason the general cap exists — and
`an_export_past_the_general_cap_still_verifies` proves the round trip at a
size above 64 KiB.

**The Security page marked the product's own admission records as failing
verification.** The send path admits the built-in delivery tool under the
owner's admission key (`founder-device.workspace`, minted into the vault).
The page verified every record against the *demo command's* key
(`founder-device.local`). So on a fresh install, after the first send, the one
admitted tool showed `admission record failed verification against the demo
trust anchor` — with nothing wrong. The page now verifies against the keys
that actually sign, owner first and demo second, from one function
(`workspace::admission_trust`) that the send path's key derivation shares.
`a_record_the_send_path_wrote_verifies_on_the_security_page` pins it.

Four product findings, queued in `docs/backlog.md` rather than fixed, because
each is a rule for the founder to set:

1. **A rejected or revoked document is a dead end.** Reject a send request and
   the document becomes `rejected`; revoke an approved-but-undelivered one and
   it becomes `revoked`. Neither can be edited (`update_document` requires a
   draft) or resubmitted (`request_send` requires a draft). The only way on is
   a new document, which loses the revision thread. P2.
2. **Role cards are half-translated.** Each card's title and description have
   `en`/`zh` variants; its *reads / delivers / cannot* lines exist only in
   English, so the Chinese Team page shows three English lines per card. P3.
3. **Venture-level model disclosures show `(unknown)` as the customer**, and
   their audit resource is `customer:00000000-…`. A company-level compliance
   check has no customer; it should say so. P3.
4. **Two copy/layout nits**: the compliance page's history box says "no checks
   yet" when exactly one report exists (the list deliberately excludes the
   report shown above it); on the Documents page a long title pushes the
   second action button onto its own line. P3.

Two things that looked like findings and were not: English proposal titles on
the demo workspace came from my own earlier API scripts, which passed no
`lang` — the frontend's `api()` helper injects it on every call; and
"proposal_writer" as an employee name on the Customers page was the name I gave
at hire time, rendered as `title · name`.

## What held

Everything else in the walk behaved, and behaved precisely. The steps that
"failed" beyond the defects above were all the API refusing something it
should refuse, with a message that said exactly why:

- `only an approved, undelivered document can be confirmed delivered` — on a
  second confirm.
- `customer_id is required` — on a timeline without one.
- `unknown purpose draft_offer` — the purposes are `draft_discovery_summary`,
  `draft_proposal`, `review_draft`.
- `unknown variant Lead, expected lead or customer`.

The security gauntlet passed 11/11 on the fresh workspace. The audit chain
verified with 90 events at the end of the walk and 0 integrity findings.
Receivables went from `500000 open` to `0 paid` on one recorded payment. The
company compliance check produced 10 findings and the invoice check 3, each
carrying its source, its "who to ask", and the facts it read. Privacy presets
switched both ways and the projection preview answered. Model status reported
`local-drafter · healthy · real_model: false`, which is the truth.

Visually, all eight pages rendered without overflow, `undefined`, `NaN` or
missing strings, in light and dark, in Chinese; screenshots were sent to the
founder.

## What still needs a person

- **Install Ollama, pull a model, enable `model.json`**, then re-run the role
  runs from the walk with `lang` set both ways and record pass/fall-back per
  role. That is the other half of the backlog item, and it is the founder's
  machine.
- **Decide the document rule** in finding 1: "editing a rejected or revoked
  document produces a new draft revision" is the obvious candidate; the
  rejection stays in the audit chain either way.
- **The five-consultant usability protocol** wants five people, not one
  script.

## The walk, for whoever runs it next

Sixty steps against `http://127.0.0.1:7793` with `Host: 127.0.0.1:7793` and
`Content-Type: application/json`, in this order — each `POST` unless marked:

`workspace/venture` → `workspace/profile` (SG, SGD, UEN, FYE 12) →
`workspace/customer` → `customer/update` (discovery notes, stage `lead`) →
`workspace/roles` → `employee/hire` × 6 → `employee/status` paused then hired
→ `employee/run` analyst → `decision` approve → `employee/run`
proposal_writer → `decision` approve → `document/update` (amount 5000) →
`request-send` → `decide` reject → `request-send` again (**stuck**) → new
`workspace/offer` → `request-send` → `decide` approve → `revoke` →
`request-send` again (**stuck**) → new offer → `request-send` → `decide`
approve → `confirm-delivery` → `offer/accepted` → `employee/run`
delivery_planner → `decision` approve (1 project, 6 tasks) → `task` add →
`task/done` × 7 → `project/status` done → `employee/run` invoice_clerk →
`decision` approve → `request-send` invoice → `decide` approve →
`employee/run` quality_checker → `decision` approve → `receivables` →
`payment` → `receivables` → `follow-up` → `follow-up/done` →
`compliance/check` company (zh) → `compliance/check` invoice → `compliance/rules`
("GST") → `employee/run` compliance_checker → `decision` approve →
`work-suggestions` → `timeline` → `GET command-center` → `assist` →
`privacy/state` → `privacy/preset` local_only → `privacy/preview`
draft_proposal → `privacy/preset` auto_protect → `GET model/status` →
`GET export` → `verify-export` (**was refused**) → `gauntlet` → `GET state`.

The P2 backlog item "HTTP-layer tests for the MVP routes" is the place this
sequence belongs, as Rust tests over the existing `tests/support/ui_server.rs`
harness; two of its steps are already there as of `#91`.
