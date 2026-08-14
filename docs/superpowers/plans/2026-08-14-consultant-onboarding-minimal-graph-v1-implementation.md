> **Status: Superseded — do not execute.**
>
> This historical plan would replace the current Experimental product UI and
> withdraw its export, disclosure, and integrity paths before an authenticated
> product router exists. Use the additive
> [Consultant Playground Standalone v2 plan](2026-08-14-consultant-playground-standalone-v2-implementation.md)
> instead; the body below is retained unchanged as design history.

# Independent Consultant Synthetic Playground Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give a first-time independent consultant a plain-language, fixed synthetic example of Company → Offer → Lead/Customer → Discovery, two safe practice actions, compiled search, and deterministic guidance without reading, accepting, serializing, exporting, or persisting the user's real business data.

**Architecture:** Add a physical leaf library crate, `sovereign-consultant-playground` (`publish = false`), that owns every Playground domain, HTTP, and embedded-asset production file. Its only direct dependencies are `serde`, `serde_json`, and `tiny_http`; it cannot depend on the CLI or any Sovereign/Workspace/Vault/dirs crate. The leaf contains a distinctly named `ConsultantPlaygroundGraph` with only compiled fixture values and process-local exercise state. One exact CLI-surface fixture pins the annotated `Cli` struct, complete annotated `Commands` enum, and entire `main`; `main` immediately matches `<Cli as clap::Parser>::parse().command`, and the `Ui` arm directly calls `sovereign_consultant_playground::run(port)`. Atomically withdraw all current real Workspace, Command Center, Security Center state, assistant, export, verify, and wildcard business browser routes. Keep Workspace v1 types, bytes, writers, reporting, export, verifier, and non-UI CLI behavior unchanged.

**Tech Stack:** Rust 1.97; a new `publish = false` leaf library whose direct dependencies are exactly existing `serde`, `serde_json`, and `tiny_http`; embedded semantic HTML/CSS; and zero-runtime-dependency JavaScript with JSDoc checked by TypeScript 5.5.4.

## Global Constraints

- Work only on `feature/consultant-onboarding` in `/workspace/scratch/c941427e8870/repo/.worktrees/consultant-onboarding`.
- Do not modify `apps/cli/src/workspace/**`, the persisted Workspace schema/version, repository/store behavior, reporting/export/verify semantics, or migration logic. Manifest/lock changes are limited to: registering `crates/consultant-playground` as a workspace member and workspace dependency, adding that one dependency to `sovereign-cli`, and recording that local package edge in `Cargo.lock`; no new third-party package is allowed.
- Preserve the current Workspace v1 CLI/domain walking skeleton byte- and behavior-compatibly: Venture, Customer, proposal/invoice documents, approval/evidence, local `.eml` outbox, disclosure, integrity, export, and verify tests continue unchanged.
- Withdraw unsafe unauthenticated browser registrations; preserving the walking skeleton does not mean preserving real-business HTTP access before owner sign-in and protected storage exist.
- `ConsultantPlaygroundGraph` is a fixed, ephemeral teaching value. It has no `Serialize`/`Deserialize` derive, no `save`, `load`, `export`, repository adapter, `From`/`Into` conversion to Workspace, file path, UUID generator, clock, or caller-provided business field.
- All Playground production code and assets live under `crates/consultant-playground`; no Playground domain, HTTP, asset, or helper module may remain under `apps/cli`. The leaf manifest has `publish = false`, no build script, no features, no dev dependency, and exactly three direct dependencies: `serde`, `serde_json`, and `tiny_http`.
- Every leaf production `.rs` file is recursively inventoried and scanned with one fail-closed source/manifest gate. It rejects filesystem, environment, path, process, outbound-network/client, Workspace/Store/Vault/dirs/Sovereign/audit/effect/export/model authority; `#[path]`, `include!`, `include_bytes!`, build-generated code, custom/unknown macros, and undeclared source files. The only code-generation exceptions are exact `include_str!` calls for the six named leaf-owned static assets plus a closed list of ordinary value macros actually used (`format!`, `println!`, `vec!`, `matches!`); adding any macro or asset path first requires changing the boundary fixture and attack review.
- After deleting the old CLI-local `ui.rs` and asset tree, `apps/cli/src/main.rs` is the only surviving CLI production source modified. An exact fixture covers its Clap import, annotated `Cli` struct, complete annotated `Commands` enum, and entire `fn main`. `main` is locked to `match <Cli as clap::Parser>::parse().command { ... }` followed by `Ok(())`; explicit trait qualification prevents an inherent `Cli::parse` helper from shadowing the derived parser. The `Ui` variant is exactly one built-in `u16 port` field with `#[arg(long, default_value_t = 7787)]`: no `value_parser`, `env`, `flatten`, custom default function, helper, or second field. Its match arm is exactly one direct `sovereign_consultant_playground::run(port)?` call. There is no statement, helper call, root derivation, environment read, or side effect between parse and dispatch. All non-UI arms are byte/behavior-compatible.
- The graph is exactly Company, Offer, Relationship (Lead/Customer), and Discovery. Do not add projects, tasks, contracts, invoices, accounting, pipelines, tags, custom fields, automations, chat, Crew, generic graph primitives, or model history.
- The fixed scenario is exactly North Star Operations / Reporting clarity sprint / Acme Ltd / Alex Chen at `alex.chen@example.test` / weekly reporting takes six hours / $3,000–$5,000 / finance must approve / 30-minute scoping call. Names, email, and numeric money remain literal; every other teaching fact, lifecycle label, guidance item, and search hit is represented by a closed semantic key and projected through complete compiled English and Simplified Chinese catalogs.
- Allowed actions are exactly `CorrectOfferPrice`, `PromoteAcmeToCustomer`, `ShowReportingSearch`, and `Reset`. They accept no strings, IDs, timestamps, prices, lifecycle values, query text, backend choice, or arbitrary JSON members.
- The only state changes are the compiled offer price $2,500→$3,500 and the compiled Acme status Lead→Customer. Search and guidance are pure reads. Reset reconstructs the exact fixture; restarting the CLI also resets it.
- The browser must not read, accept, process, verify, or export real business values. `/api/state`, `/api/command-center`, `/api/workspace`, `/api/export`, `/api/verify-export`, `/api/gauntlet`, `/api/workspace/assist`, every `/api/workspace/*` mutation, and every wildcard business route are unregistered.
- The browser Security Center becomes static, value-free advanced copy. It cannot receive a root path or call Store, Vault, identity initialization, ledger, integrity, disclosure, artifact, outbox, repository, or filesystem code. After cutover, browser inspection of real integrity/disclosure state, disclosure browsing, and creation of a fresh real-data export are unavailable. `sovereign integrity` retains its existing integrity summary, and `sovereign verify-export PATH` can only verify an export file the user already possesses; neither command browses disclosures or creates a new export.
- General real-data consultant forms and persistent graph work are a separate future plan. They remain blocked until the actual landed owner-authorization type and actual landed ActiveV2 storage selector are both construction requirements. Do not create stubs, booleans, environment flags, headers, path switches, acknowledgement gates, fake grants, or a legacy adapter here.
- Beginner-facing English and Simplified Chinese copy uses plain language: “Practice with this example only. You cannot enter or save your own business or customer data here. Real-data setup is unavailable in this preview.” Internal names such as `1C0`, `AuthorizedStore`, and `ActiveV2` appear only inside an optional advanced technical disclosure.
- Do not claim real AI, authenticated ownership, Vault v2, E2EE, secure cloud, encrypted backup/restore, complete field authentication, persistence, recovery, or production safety. The unchanged v1 export format remains plaintext portability with its existing proof limits, but this slice exposes no route or command that creates a fresh export; the Playground has no export action.
- Before Task 5 removes `/api/export`, product release ownership must decide whether users with existing v1 data must be able to create a fresh export in this release. If yes, the browser cutover is **Blocked**: do not ship either the old unauthenticated route or a replacement here; wait for actual owner authorization plus protected storage and expose one authenticated owner-protected export entry point in that later slice. If no, record explicit acceptance that fresh export creation and disclosure browsing are temporarily unavailable, while existing export files remain verifiable by CLI.
- Keep the zero-build, zero-runtime-dependency frontend. Do not add React, Tauri, npm runtime packages, a database, telemetry, analytics, remote fonts/assets, or network services.
- Accessibility floor: semantic landmarks/headings; skip link; visible button text or accessible name; `aria-describedby` for action explanations; typed errors with `role="alert"`; status updates with `aria-live="polite"`; keyboard-only operation; visible `:focus-visible`; no color-only state; 44×44 CSS-pixel targets; a 16px locale selector on small screens; reduced-motion support; and no page horizontal scroll at 375px.
- Tasks 1–5 introduce behavior and follow honest RED → focused GREEN → regression gate → review → commit. Tasks 6–9 are acceptance, attack, usability, and handoff gates expected to begin GREEN/Target; only a genuinely observed defect starts a new focused RED/fix cycle. Never fabricate a failing run.
- At push checkpoints, run the named focused and full gates first. Push only with explicit implementation-session authorization. This planning task itself leaves all files uncommitted and unpushed.

---

## Product Boundary and Authority Model

### Beginner-visible path

```text
Open “Consultant practice example”
  → review the four fixed Company / Offer / Lead / Discovery facts
  → correct the example price
  → promote the example lead to customer
  → use the provided “reporting” search example
  → read a factual next step without executing it
  → reset, or restart the app, to return to the identical fixture
```

There are no Company, Offer, Relationship, Discovery, search-query, import, paste, file-upload, export, or “I accept the risk” controls. The four-step business thread is a guided explanation, not a data-entry wizard. `Continue` changes view/focus only. Process-local action state is deliberately not a resume, backup, or recovery mechanism.

### Visual and interaction direction

Keep the current seal-and-ledger identity: warm paper surfaces, ink-navy actions, system Latin/CJK fonts, restrained borders, and the existing light/dark tokens. Present the example as a calm client brief with one numbered vertical business thread, not a dense operations dashboard. Use the existing navy accent for active steps/actions and neutral connectors for the thread; do not repurpose the bronze evidence color to imply verified business data. At desktop widths, pair the thread with a narrow “What needs you?” guidance rail; below 760px, place guidance after the current step in one column. Use no illustration, stock image, gradient, remote font, ornamental animation, or chart. Motion is limited to the existing 180ms focus/state transition and is removed under `prefers-reduced-motion`.

### Exact non-authority flow

```text
Compiled constants
  → physical leaf crate `sovereign-consultant-playground`
  → private ConsultantPlaygroundGraph (not serializable)
  → PlaygroundSession (process memory only)
  → one-way PlaygroundReadModel projection (serializable response DTO)
  → static HTML/JS renders with textContent

UI POST { action: closed wire enum }
  → PlaygroundHttpHandler<Mutex<PlaygroundSession>>
  → exact match to one closed domain action
  → no root/path/Store/Vault/audit/model/export/effect capability exists
```

`PlaygroundReadModel` may serialize because it is a one-way response projection of compiled values. It exposes no constructor accepting business strings and no conversion back to `ConsultantPlaygroundGraph` or persistent Workspace. The graph itself is non-serializable and non-persistable.

The dependency direction is one-way: `sovereign-cli → sovereign-consultant-playground`. The leaf cannot name or depend on `sovereign-cli`, any `sovereign-*` internal crate, `dirs`, or a persistence/network-client crate. `tiny_http` is used only for an inbound server bound to literal loopback `127.0.0.1`; the leaf has no outbound socket/client surface.

### Exact unauthenticated route manifest after cutover

| Method | Route | Purpose |
| --- | --- | --- |
| GET | `/` | Static synthetic Playground shell |
| GET | `/assets/styles.css` | Embedded CSS |
| GET | `/assets/i18n.js` | Embedded bilingual copy |
| GET | `/assets/app.js` | Shell/theme/locale wiring |
| GET | `/assets/consultant-ui.js` | Playground rendering/actions |
| GET | `/favicon.svg` | Existing embedded favicon |
| GET | `/api/playground/consultant` | Fixed synthetic read model |
| POST | `/api/playground/consultant/action` | Closed unit action only |

No wildcard route is allowed. Unknown routes return typed 404 JSON with `Cache-Control: no-store`. There is no browser export or verification endpoint in this slice.

### Future persistent product boundary

The real Company/Offer/Relationship/Discovery schema, legacy preflight/remediation, migration, recovery, export versioning, writer changes, and general forms belong to a future owner-authorized ActiveV2 implementation plan. That future plan must start from actual landed interfaces and prove that ActiveV2 enrollment precedes the first persistent business value. Nothing in this slice is a migration precursor or alternate storage authority.

### Persistence, version, migration, and export rules for this slice

- `ConsultantPlaygroundGraph` has no persisted format or schema version. Process restart is reconstruction from compiled constants, not decode, migration, restore, or recovery.
- No startup, GET, action, reset, or documentation step opens or rewrites a Workspace. There is no v1→v2 migration, preflight, enrollment flag, repository selection, dual read, dual write, or conversion in this plan.
- Workspace v1 remains the sole current persistent business format. Its Store, writers, ledger, reporting, plaintext export format, and compiled verifier keep their existing bytes and behavior; the slice changes only which unauthenticated browser routes are registered. Preserving `Store::export` internally does not imply a user-facing fresh-export command remains after `/api/export` is withdrawn.
- The Playground has no import, export, audit, backup, or verify format. Never place it inside the existing Workspace export or claim that its process-local changes are durable evidence.
- The only retained user-facing export operation is offline verification of an already-existing file through `sovereign verify-export PATH`. Real disclosure browsing and new export creation are unavailable after cutover unless the release is held for the later single owner-protected entry point described above.
- A future persistent Enterprise Graph plan begins only after actual owner sign-in/authorization and actual ActiveV2 protected storage have landed. It must design authorized legacy preflight/recovery and export evolution from those real interfaces; it must not serialize or promote this Playground type.

## Exact Playground Shape

```rust
struct ConsultantPlaygroundGraph {
    company: PlaygroundCompany,
    offer: PlaygroundOffer,
    relationship: PlaygroundRelationship,
    discovery: PlaygroundDiscovery,
}

struct PlaygroundCompany {
    name: &'static str,
    service_key: PlaygroundTextKey,
}

struct PlaygroundOffer {
    id: PlaygroundEntityId,
    title_key: PlaygroundTextKey,
    summary_key: PlaygroundTextKey,
    scope_key: PlaygroundTextKey,
    price_cents: u64,
    status: PlaygroundOfferStatus,
}

struct PlaygroundRelationship {
    id: PlaygroundEntityId,
    name: &'static str,
    contact_name: &'static str,
    email: &'static str,
    notes_key: PlaygroundTextKey,
    status: PlaygroundRelationshipStatus,
}

struct PlaygroundDiscovery {
    id: PlaygroundEntityId,
    relationship_id: PlaygroundEntityId,
    summary_key: PlaygroundTextKey,
    budget_min_cents: u64,
    budget_max_cents: u64,
    constraints_key: PlaygroundTextKey,
    next_step_key: PlaygroundTextKey,
}
```

All graph types and `PlaygroundTextKey` are private to `consultant_playground`. They derive only traits needed for deterministic in-memory comparison (`Debug`, `Clone`, `Copy` where valid, `PartialEq`, `Eq`, `Ord` for key ordering), never serde traits. `PlaygroundTextKey::ALL` is a fixed array used by both locale projection and completeness tests. Its exact key/catalog contract is:

| Stable key | English | Simplified Chinese |
| --- | --- | --- |
| `company_service.operations_reporting` | Operations reporting consulting | 运营报表咨询 |
| `offer_title.reporting_clarity_sprint` | Reporting clarity sprint | 报表清晰度冲刺 |
| `offer_summary.clear_repeatable_reporting` | Make weekly reporting clear and repeatable | 让每周报表清晰且可重复执行 |
| `offer_scope.report_template_handover` | Review the current report, define a weekly template, and hand over a simple checklist | 审阅现有报表，制定每周模板，并交付一份简明清单 |
| `relationship_notes.reduce_reporting_work` | Interested in reducing weekly reporting work | 希望减少每周报表工作 |
| `discovery_problem.weekly_reporting_six_hours` | Weekly reporting takes six hours | 每周报表需要六小时 |
| `discovery_constraint.finance_approval` | Finance must approve | 需要财务批准 |
| `discovery_next_step.scoping_call_30_minutes` | 30-minute scoping call | 30 分钟范围确认通话 |
| `status.active` | Active | 进行中 |
| `status.lead` | Lead | 潜在客户 |
| `status.customer` | Customer | 客户 |
| `search_topic.reporting` | Reporting | 报表 |
| `entity.company` | Company | 公司 |
| `entity.offer` | Offer | 服务方案 |
| `entity.relationship` | Lead / customer | 潜在客户 / 客户 |
| `entity.discovery` | Discovery | 需求发现 |
| `guidance.correct_offer_price` | Correct the example price to `{price}` | 将示例价格改为 `{price}` |
| `guidance.promote_lead` | Promote `{relationship}` from lead to customer | 将 `{relationship}` 从潜在客户升级为客户 |
| `guidance.follow_next_step` | Follow the recorded next step: `{next_step}` | 按已记录的下一步执行：`{next_step}` |
| `guidance.complete` | All example exercises are complete | 所有示例练习均已完成 |

The enum variants, in `ALL` order, are exactly `CompanyServiceOperationsReporting`, `OfferTitleReportingClaritySprint`, `OfferSummaryClearRepeatableReporting`, `OfferScopeReportTemplateHandover`, `RelationshipNotesReduceReportingWork`, `DiscoveryProblemWeeklyReportingSixHours`, `DiscoveryConstraintFinanceApproval`, `DiscoveryNextStepScopingCall30Minutes`, `StatusActive`, `StatusLead`, `StatusCustomer`, `SearchTopicReporting`, `EntityCompany`, `EntityOffer`, `EntityRelationship`, `EntityDiscovery`, `GuidanceCorrectOfferPrice`, `GuidancePromoteLead`, `GuidanceFollowNextStep`, and `GuidanceComplete`. `as_str()` returns the stable-key column exactly.

Names (`North Star Operations`, `Acme Ltd`, `Alex Chen`), `alex.chen@example.test`, and money parameters remain identical in both locales. Every placeholder set must match across English and Chinese; missing keys, duplicates, blank translations, unexpected fallbacks, or raw English teaching strings in graph/query DTO fields fail compilation tests.

## File Map

| File | Responsibility |
| --- | --- |
| `scripts/run-exact-test-group.sh` | Lists a Rust test filter, requires named tests and a nonzero parsed manifest, then reruns every listed test individually with `--exact`. |
| `Cargo.toml` | Registers only the new leaf member/workspace dependency; no other dependency or profile change. |
| `Cargo.lock` | Records only the new local leaf package edge; no new third-party package. |
| `crates/consultant-playground/Cargo.toml` | `publish = false` leaf manifest; direct dependencies exactly `serde`, `serde_json`, and `tiny_http`; no build/features/dev dependencies. |
| `crates/consultant-playground/src/lib.rs` | Private module composition and the sole public entry point `run(port)`; no other public authority surface. |
| `crates/consultant-playground/src/graph.rs` | Non-serializable graph types, exact compiled fixture, and closed in-memory invariant check. |
| `crates/consultant-playground/src/keys.rs` | Closed semantic teaching-key enum and stable wire strings; no human-language teaching copy. |
| `crates/consultant-playground/src/actions.rs` | Closed actions and exact constant transformations; no generic setters. |
| `crates/consultant-playground/src/query.rs` | Pure fixed search example, business-thread projection, and read-only guidance. |
| `crates/consultant-playground/src/localize.rs` | Exhaustive compiled English/Simplified Chinese projection for every teaching key. |
| `crates/consultant-playground/src/read_model.rs` | One-way serializable response DTOs built from the private graph. |
| `crates/consultant-playground/src/contract.rs` | Deny-unknown-fields action wire enum and stable typed API error body. |
| `crates/consultant-playground/src/handler.rs` | Handler owning only `Mutex<PlaygroundSession>` and producing no-store responses. |
| `crates/consultant-playground/src/server.rs` | Exact static/Playground route manifest and literal loopback `tiny_http` dispatch. |
| `crates/consultant-playground/src/tests.rs` | Fixture, action, reset, search, guidance, HTTP, route, asset, and no-authority unit tests. |
| `crates/consultant-playground/tests/authority_boundary.rs` | Recursively inventories every leaf production source, pins the manifest/dependency surface, rejects every authority/macro/path/include escape, and carries injected-bypass regressions. |
| `crates/consultant-playground/assets/consultant-ui.js` | Renders fixed values, sends exact action-only JSON, and handles focus/status. |
| `crates/consultant-playground/assets/app.js` | Static shell, locale/theme, initial Playground load; no legacy API or business form binding. |
| `crates/consultant-playground/assets/index.html` | Semantic synthetic-only layout and static advanced security disclosure; no real-data controls. |
| `crates/consultant-playground/assets/styles.css` | Existing visual tokens plus business thread, responsive cards, focus/touch/reduced-motion rules. |
| `crates/consultant-playground/assets/favicon.svg` | Leaf-owned local-only SVG parsed into the closed asset-reference graph. |
| `crates/consultant-playground/assets/i18n.js` | Matching English/Chinese beginner and advanced copy maps. |
| `crates/consultant-playground/assets/tsconfig.json` | Includes both JavaScript files in the check-only type gate. |
| `apps/cli/Cargo.toml` | Adds exactly the leaf workspace dependency and removes direct `tiny_http` if no non-Playground CLI code still needs it. |
| `apps/cli/tests/consultant_playground_process.rs` | Spawns the real `sovereign ui` binary separately against two content-distinct OS-platform v1 canary roots; compares complete raw-loopback response transcripts/stdout/stderr and both recursive snapshots. |
| `apps/cli/src/main.rs` | Deletes the in-crate UI module, uses explicit `<Cli as clap::Parser>::parse()`, and makes the sole UI arm directly call leaf `run(port)`; other arms stay compatible. |
| `apps/cli/tests/fixtures/expected_consultant_playground_cli_surface.rs.txt` | Exact Clap import, annotated `Cli` struct, complete annotated `Commands` enum, and complete `main` fixture. |
| `apps/cli/tests/consultant_playground_cli_boundary.rs` | Compares all three CLI surfaces exactly; enforces the built-in-only UI port parser and absence of the legacy flag/browser launcher. |
| `apps/cli/src/ui.rs`, `apps/cli/assets/**` | Delete at atomic cutover; no duplicate browser authority or asset tree remains in the CLI. |
| `docs/product/consultant-playground-accessibility.md` | Manual acceptance checklist; all unrun rows remain Target. |
| `docs/product/consultant-playground-usability.md` | Five-consultant protocol and blank evidence table. |
| `docs/security/consultant-playground-attack-review.md` | Route/asset/canary/authority review and future prerequisite boundary. |

## Acceptance Trace

| Requirement | Decisive task/evidence |
| --- | --- |
| No persistent Workspace v2/migration/repository/export changes | Task 9 path-diff gate plus unchanged v1 test suite |
| Distinct non-serializable, non-persistable graph | Tasks 1–3 type/file boundary and Task 7 authority review |
| Fixture type exists before action tests | Task 1 produces graph/session; Task 2 consumes them |
| No `/api/state` or alternate real-data browser path | Task 5 exact route manifest and seeded canary/page-init test |
| Full shipped HTML/CSS/JS/SVG/handler graph is closed | Task 5 resource/event/network allowlist across every embedded asset and compiled manifest |
| Production code cannot read, derive, discard, or leak platform-root values | Task 5 physical leaf dependency boundary, complete leaf source/manifest inventory and uniform denylist, exact annotated CLI/Commands/main surface, plus two-root real-process equivalence |
| CLI parsing cannot invoke a custom data-root reader | Task 5 exact annotated `Cli`/complete `Commands`/`main` fixture, explicit trait-qualified parse, exact built-in `u16` UI port attribute, and forbidden parser-hook assertions |
| Real binary is independent of platform-root contents | Task 5 two content-distinct OS-specific data roots, identical full HTTP sequence, byte-identical complete response transcripts, canary-free stdout/stderr, and two unchanged recursive snapshots |
| RED does not launch an external browser | Task 5 probes old `ui --help`, appends legacy `--no-open` only to the old binary, then requires the final flag and every browser-launcher surface to be absent |
| No new client/network dependency | Task 5 exact leaf direct-dependency set, recursive production-source denylist, lockfile no-new-third-party check, literal loopback bind, and closed browser asset graph |
| No caller business values | Tasks 2 and 4 action-only domain/wire tests; Task 5 exact JS body test |
| Every teaching fact/search/guidance item is bilingual and keyed | Task 3 exhaustive `PlaygroundTextKey::ALL` catalogs, placeholder parity, used-key closure, and full English/Chinese projections |
| Search/guidance read-only | Task 3 byte-for-byte session comparison |
| Plain beginner copy; jargon advanced-only | Task 5 copy partition test and Task 8 comprehension protocol |
| Typed errors and no-store on every JSON path | Task 4 response matrix and Task 5 legacy/unknown route checks |
| Accessibility/i18n/mobile honesty | Task 5 automated contracts and Task 6 Target evidence checklist |
| Current v1 walking skeleton compatibility | Tasks 5, 7, and 9 run existing workspace/Stage 1/export/verify tests unchanged |
| Export/disclosure cutover is honest | Task 5 pre-cutover product release gate and exact advanced copy; Task 9 blocks if fresh export is required before owner protection |
| Real graph remains blocked on owner authorization + protected storage | Task 7 attack review and Task 9 current-state documentation check |
| GREEN commands cannot silently match zero tests | `run-exact-test-group.sh` parses `--list`, asserts expected exact names/nonzero, and reruns every listed test individually |

## Task 1: Add the non-serializable compiled Playground graph

**Files:**
- Create: `scripts/run-exact-test-group.sh`
- Modify: `Cargo.toml` (register the one new member/workspace dependency)
- Modify: `Cargo.lock` (new local package edge only)
- Create: `crates/consultant-playground/Cargo.toml`
- Create: `crates/consultant-playground/src/lib.rs`
- Create: `crates/consultant-playground/src/graph.rs`
- Create: `crates/consultant-playground/src/keys.rs`
- Create: `crates/consultant-playground/src/tests.rs`

**Interfaces:**
- Produces crate-private `ConsultantPlaygroundGraph::fixture() -> Result<Self, PlaygroundError>`.
- Produces crate-private `PlaygroundSession::new() -> Result<Self, PlaygroundError>` and test-only graph access inside the leaf.
- Does not yet produce actions, HTTP, serialization, routes, or UI.
- Establishes the physical dependency boundary before any feature code: `publish = false`; no build script/features/dev dependencies; direct dependencies exactly workspace `serde`, workspace `serde_json`, and `tiny_http = "0.12"`. The root workspace member/dependency and lock entry are the only allowed root-level changes.

- [ ] **Step 1: Write the fixture RED tests**

```rust
#[test]
fn consultant_fixture_is_exact_complete_and_deterministic() {
    let first = ConsultantPlaygroundGraph::fixture().unwrap();
    let second = ConsultantPlaygroundGraph::fixture().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.company.name, "North Star Operations");
    assert_eq!(first.company.service_key, PlaygroundTextKey::CompanyServiceOperationsReporting);
    assert_eq!(first.offer.title_key, PlaygroundTextKey::OfferTitleReportingClaritySprint);
    assert_eq!(first.offer.summary_key, PlaygroundTextKey::OfferSummaryClearRepeatableReporting);
    assert_eq!(first.offer.scope_key, PlaygroundTextKey::OfferScopeReportTemplateHandover);
    assert_eq!(first.offer.price_cents, 250_000);
    assert_eq!(first.relationship.name, "Acme Ltd");
    assert_eq!(first.relationship.contact_name, "Alex Chen");
    assert_eq!(first.relationship.email, "alex.chen@example.test");
    assert_eq!(first.relationship.notes_key, PlaygroundTextKey::RelationshipNotesReduceReportingWork);
    assert_eq!(first.relationship.status, PlaygroundRelationshipStatus::Lead);
    assert_eq!(first.discovery.relationship_id, first.relationship.id);
    assert_eq!(first.discovery.summary_key, PlaygroundTextKey::DiscoveryProblemWeeklyReportingSixHours);
    assert_eq!((first.discovery.budget_min_cents, first.discovery.budget_max_cents),
        (300_000, 500_000));
    assert_eq!(first.discovery.constraints_key, PlaygroundTextKey::DiscoveryConstraintFinanceApproval);
    assert_eq!(first.discovery.next_step_key, PlaygroundTextKey::DiscoveryNextStepScopingCall30Minutes);
}

#[test]
fn graph_module_declares_no_persistence_serialization_or_workspace_surface() {
    let source = include_str!("graph.rs");
    for forbidden in [
        "serde", "Serialize", "Deserialize", "Workspace", "Store", "Vault",
        "Path", "std::fs", "save", "load", "export", "Uuid", "chrono",
        "impl From", "impl Into",
    ] {
        assert!(!source.contains(forbidden), "forbidden graph surface: {forbidden}");
    }
}
```

The second test is a narrow source inventory, not transitive authority proof; Tasks 5 and 7 provide the decisive handler/canary evidence. Extend it to assert that `graph.rs` contains none of the English/Chinese catalog values above; it may contain only semantic enum variants and the literal name/email/money exceptions. Task 3 adds the same guard for `query.rs`.

- [ ] **Step 2: Run RED and capture the real failure**

```bash
cargo test -p sovereign-consultant-playground tests::consultant_fixture_is_exact_complete_and_deterministic --locked -- --exact --nocapture
```

Expected: compile failure because `consultant_playground` and its graph types do not exist. Do not add action tests yet.

- [ ] **Step 3: Implement only the graph and fixture**

Use the exact shape and semantic-key table above. Define private closed enums `PlaygroundEntityId::{Offer, Acme, Discovery}`, `PlaygroundOfferStatus::Active`, and `PlaygroundRelationshipStatus::{Lead, Customer}`. Only names/email use `&'static str`; all human-language teaching fields use `PlaygroundTextKey`, while money uses cents. `validate()` checks the Discovery points to Acme, the email ends in `.example.test`, the budget is exactly 300_000–500_000, the offer price is 250_000 or 350_000, and status is a closed enum value. It returns `PlaygroundError::InvalidFixture` without repair.

`PlaygroundSession` owns only a `ConsultantPlaygroundGraph`. Its constructor calls `fixture`; it exposes no graph setter and no public graph reference outside the module.

Create this exact reusable GREEN gate and mark it executable. It prints `cargo test -- --list`, parses only lines ending `: test`, fails on zero matches, requires every caller-supplied fully-qualified test name, and then reruns every listed name in its own `--exact` invocation:

```bash
#!/usr/bin/env bash
set -euo pipefail

if (( $# < 3 )); then
  echo "usage: $0 <package> <filter> <expected-test> [<expected-test> ...]" >&2
  exit 2
fi

package=$1
filter=$2
shift 2

listed=$(cargo test --locked -p "$package" "$filter" -- --list)
printf '%s\n' "$listed"
tests=$(printf '%s\n' "$listed" | sed -n 's/: test$//p')
if [[ -z "$tests" ]]; then
  echo "no tests matched filter: $package $filter" >&2
  exit 1
fi

for expected in "$@"; do
  if ! grep -Fqx -- "$expected" <<<"$tests"; then
    echo "missing expected test: $expected" >&2
    exit 1
  fi
done

while IFS= read -r test_name; do
  cargo test --locked -p "$package" "$test_name" -- --exact --nocapture
done <<<"$tests"
```

```bash
chmod +x scripts/run-exact-test-group.sh
```

- [ ] **Step 4: Run focused GREEN and unchanged Workspace regression**

```bash
./scripts/run-exact-test-group.sh sovereign-consultant-playground tests \
  tests::consultant_fixture_is_exact_complete_and_deterministic \
  tests::graph_module_declares_no_persistence_serialization_or_workspace_surface
./scripts/run-exact-test-group.sh sovereign-cli workspace::tests \
  workspace::tests::full_founder_flow_with_approval_and_evidence \
  workspace::tests::export_contains_state_and_verified_chain \
  workspace::tests::verify_export_accepts_genuine_bundle_and_rejects_tampering \
  workspace::tests::draft_assistant_records_a_persistent_data_disclosure
```

- [ ] **Step 5: Review and commit**

```bash
git diff --name-only | rg '^apps/cli/src/workspace/' && exit 1 || true
cargo metadata --locked --no-deps --format-version 1 > /tmp/consultant-playground-metadata.json
git diff -- Cargo.toml Cargo.lock crates/consultant-playground/Cargo.toml
git add scripts/run-exact-test-group.sh Cargo.toml Cargo.lock crates/consultant-playground
git commit -m "feat(playground): add fixed consultant teaching graph"
```

## Task 2: Add closed fixture actions after the graph exists

**Files:**
- Create: `crates/consultant-playground/src/actions.rs`
- Modify: `crates/consultant-playground/src/lib.rs`
- Modify: `crates/consultant-playground/src/tests.rs`

**Consumes:** `ConsultantPlaygroundGraph` and `PlaygroundSession` from Task 1.

**Produces:**
- `PlaygroundAction::{CorrectOfferPrice, PromoteAcmeToCustomer, ShowReportingSearch, Reset}`.
- `PlaygroundActionResult::{OfferPriceCorrected, RelationshipPromoted, ReportingSearchShown, Reset, AlreadyApplied}`.
- `PlaygroundSession::apply(&mut self, PlaygroundAction) -> Result<PlaygroundActionResult, PlaygroundError>`.

- [ ] **Step 1: Write action RED tests against the existing fixture type**

```rust
#[test]
fn fixed_actions_change_only_the_two_documented_business_fields() {
    let mut session = PlaygroundSession::new().unwrap();
    let initial = session.graph_for_test().clone();
    assert_eq!(session.apply(PlaygroundAction::CorrectOfferPrice).unwrap(),
        PlaygroundActionResult::OfferPriceCorrected);
    let corrected = session.graph_for_test().clone();
    assert_eq!(corrected.offer.price_cents, 350_000);
    assert!(equal_except_offer_price(&initial, &corrected));
    assert_eq!(session.apply(PlaygroundAction::PromoteAcmeToCustomer).unwrap(),
        PlaygroundActionResult::RelationshipPromoted);
    let promoted = session.graph_for_test().clone();
    assert_eq!(promoted.relationship.status, PlaygroundRelationshipStatus::Customer);
    assert!(equal_except_relationship_status(&corrected, &promoted));
}

#[test]
fn observational_and_reset_actions_are_deterministic() {
    let mut session = PlaygroundSession::new().unwrap();
    let before = session.graph_for_test().clone();
    assert_eq!(session.apply(PlaygroundAction::ShowReportingSearch).unwrap(),
        PlaygroundActionResult::ReportingSearchShown);
    assert_eq!(session.graph_for_test(), &before);
    session.apply(PlaygroundAction::CorrectOfferPrice).unwrap();
    session.apply(PlaygroundAction::Reset).unwrap();
    assert_eq!(session.graph_for_test(), &ConsultantPlaygroundGraph::fixture().unwrap());
}

#[test]
fn repeated_state_action_returns_already_applied_without_mutation() {
    let mut session = PlaygroundSession::new().unwrap();
    session.apply(PlaygroundAction::CorrectOfferPrice).unwrap();
    let before = session.graph_for_test().clone();
    assert_eq!(session.apply(PlaygroundAction::CorrectOfferPrice).unwrap(),
        PlaygroundActionResult::AlreadyApplied);
    assert_eq!(session.graph_for_test(), &before);
}
```

- [ ] **Step 2: Run RED**

```bash
cargo test -p sovereign-consultant-playground tests::fixed_actions_change_only_the_two_documented_business_fields --locked -- --exact --nocapture
```

Expected: compile failure for missing `actions` module and `apply`, while Task 1 fixture tests remain GREEN.

- [ ] **Step 3: Implement exact match arms, not generic setters**

Each match arm assigns only its compiled constant. `CorrectOfferPrice` directly updates the sole Offer. `PromoteAcmeToCustomer` directly updates the sole Relationship. `ShowReportingSearch` returns its code without mutation. `Reset` calls `ConsultantPlaygroundGraph::fixture`. Validate a cloned candidate before replacing session state. There is no string map, JSON value, generic patch, closure callback, or public mutable reference.

- [ ] **Step 4: Run GREEN and source inventory**

```bash
./scripts/run-exact-test-group.sh sovereign-consultant-playground tests \
  tests::consultant_fixture_is_exact_complete_and_deterministic \
  tests::graph_module_declares_no_persistence_serialization_or_workspace_surface \
  tests::fixed_actions_change_only_the_two_documented_business_fields \
  tests::observational_and_reset_actions_are_deterministic \
  tests::repeated_state_action_returns_already_applied_without_mutation
rg -n 'String|serde_json::Value|HashMap|Store|Workspace|Vault|std::fs|Path' crates/consultant-playground/src
```

The inventory may find `String` later only in Task 3 response DTOs, never in `graph.rs` or action inputs. Classify matches by file; do not call this scan authority proof.

- [ ] **Step 5: Commit**

```bash
git add crates/consultant-playground
git commit -m "feat(playground): add fixed consultant practice actions"
```

## Task 3: Add one-way read models, compiled search, and read-only guidance

**Files:**
- Create: `crates/consultant-playground/src/read_model.rs`
- Create: `crates/consultant-playground/src/query.rs`
- Create: `crates/consultant-playground/src/localize.rs`
- Modify: `crates/consultant-playground/src/lib.rs`
- Modify: `crates/consultant-playground/src/tests.rs`

**Consumes:** Task 2 `PlaygroundSession`, `PlaygroundAction`, and result codes.

**Produces:**
- Serializable `PlaygroundReadModel`, `PlaygroundSearchHit`, `PlaygroundGuidance`, and `PlaygroundResponse` DTOs.
- `PlaygroundTranslationCatalog { en, zh_cn }`, with one ordered `PlaygroundTranslation { key, text }` per `PlaygroundTextKey::ALL` entry in each locale.
- `PlaygroundSession::read(&self) -> PlaygroundResponse`.
- `PlaygroundSession::dispatch(&mut self, action: PlaygroundAction) -> Result<PlaygroundResponse, PlaygroundError>`.
- Stable targets `playground:offer`, `playground:relationship`, `playground:discovery`, and `playground:search`.

`PlaygroundResponse` has exactly `profile`, `real_data_enabled`, `persistence`, `graph`, `translations`, `search_results`, `guidance`, and `action_result`. `graph` is a `PlaygroundReadModel` containing literal names/email/money plus stable `*_key` fields, never localized fact text. Search hits contain only `entity_key`, `fact_key`, and optional literal name/money parameters. Guidance contains only `key`, stable `target`, and a closed typed parameter map whose values are either declared literal name/money values or another `PlaygroundTextKey`; for example, `{next_step}` is carried as `DiscoveryNextStepScopingCall30Minutes`, never as English text. `action_result` is `None` on `read()` and the closed result code on `dispatch()`. This single shape is used by tests, handler, and JavaScript.

- [ ] **Step 1: Write query/read-model RED tests**

```rust
#[test]
fn response_is_a_one_way_projection_marked_synthetic_and_nonpersistent() {
    let session = PlaygroundSession::new().unwrap();
    let response = session.read();
    assert_eq!(response.profile, "synthetic_playground");
    assert!(!response.real_data_enabled);
    assert_eq!(response.persistence, "none");
    assert_eq!(response.graph.company.name, "North Star Operations");
    assert_eq!(response.graph.company.service_key,
        "company_service.operations_reporting");
    assert_eq!(response.graph.relationship.email, "alex.chen@example.test");
    assert_eq!(response.graph.discovery.budget_min_cents, 300_000);
    assert_eq!(response.graph.discovery.budget_max_cents, 500_000);
    assert_eq!(response.action_result, None);
    assert!(response.search_results.is_empty());
    assert!(serde_json::to_value(&response).is_ok());
}

#[test]
fn compiled_reporting_search_and_guidance_are_read_only() {
    let mut session = PlaygroundSession::new().unwrap();
    let before = session.graph_for_test().clone();
    let searched = session.dispatch(PlaygroundAction::ShowReportingSearch).unwrap();
    assert_eq!(searched.action_result, Some(PlaygroundActionResult::ReportingSearchShown));
    assert!(searched.search_results.iter().any(|hit|
        hit.entity_key == "entity.discovery"
            && hit.fact_key == "discovery_problem.weekly_reporting_six_hours"));
    assert_eq!(session.graph_for_test(), &before);
    let first = session.read().guidance;
    let second = session.read().guidance;
    assert_eq!(first, second);
    assert_eq!(first[0].key, "guidance.correct_offer_price");
    assert_eq!(first[0].target, "playground:offer");
    assert_eq!(session.graph_for_test(), &before);
}

#[test]
fn no_read_dto_converts_back_to_graph_or_workspace() {
    let source = include_str!("read_model.rs");
    for forbidden in ["ConsultantPlaygroundGraph", "Workspace", "impl From", "impl Into"] {
        assert!(!source.contains(forbidden), "forbidden reverse projection: {forbidden}");
    }
}

#[test]
fn every_used_teaching_key_has_complete_exact_en_and_zh_projections() {
    let mut session = PlaygroundSession::new().unwrap();
    let initial = session.read();
    let corrected = session.dispatch(PlaygroundAction::CorrectOfferPrice).unwrap();
    let promoted = session.dispatch(PlaygroundAction::PromoteAcmeToCustomer).unwrap();
    let searched = session.dispatch(PlaygroundAction::ShowReportingSearch).unwrap();
    let reset = session.dispatch(PlaygroundAction::Reset).unwrap();
    let states = [&initial, &corrected, &promoted, &searched, &reset];
    let expected = expected_translation_fixture();
    for state in &states {
        assert_eq!(&state.translations, &expected);
    }
    assert_eq!(catalog_keys(&expected.en), PlaygroundTextKey::ALL.map(PlaygroundTextKey::as_str));
    assert_eq!(catalog_keys(&expected.zh_cn), PlaygroundTextKey::ALL.map(PlaygroundTextKey::as_str));
    assert_eq!(used_semantic_keys(&states),
        expected_used_semantic_keys_for_all_fixture_states());
    for key in PlaygroundTextKey::ALL {
        let en = expected.lookup(PlaygroundLocale::En, key).unwrap();
        let zh = expected.lookup(PlaygroundLocale::ZhCn, key).unwrap();
        assert!(!en.trim().is_empty() && !zh.trim().is_empty());
        assert_eq!(placeholder_names(en), placeholder_names(zh), "{key:?}");
        assert_ne!(en, zh, "unexpected locale fallback for {key:?}");
    }
}

#[test]
fn full_locale_projections_translate_facts_search_and_guidance_but_not_literals() {
    let mut session = PlaygroundSession::new().unwrap();
    let states = [
        ("initial", session.read()),
        ("price_corrected",
            session.dispatch(PlaygroundAction::CorrectOfferPrice).unwrap()),
        ("customer_promoted",
            session.dispatch(PlaygroundAction::PromoteAcmeToCustomer).unwrap()),
        ("search_shown",
            session.dispatch(PlaygroundAction::ShowReportingSearch).unwrap()),
        ("reset", session.dispatch(PlaygroundAction::Reset).unwrap()),
    ];
    for (state_name, response) in states {
        let en = project_for_test(&response, PlaygroundLocale::En).unwrap();
        let zh = project_for_test(&response, PlaygroundLocale::ZhCn).unwrap();
        assert_eq!(en, expected_fully_projected_fixture(PlaygroundLocale::En, state_name));
        assert_eq!(zh, expected_fully_projected_fixture(PlaygroundLocale::ZhCn, state_name));
        assert!(unresolved_semantic_keys(&en).is_empty());
        assert!(unresolved_semantic_keys(&zh).is_empty());
        assert_eq!(literal_identity_tuple(&en), literal_identity_tuple(&zh));
    }
}

#[test]
fn graph_and_query_hold_keys_not_human_language_catalog_values() {
    let sources = [include_str!("graph.rs"), include_str!("query.rs")].join("\n");
    for (_, en, zh) in expected_translation_rows() {
        assert!(!sources.contains(en), "raw English teaching text in domain/query: {en}");
        assert!(!sources.contains(zh), "raw Chinese teaching text in domain/query: {zh}");
    }
}
```

`expected_translation_fixture()` and `expected_translation_rows()` are literal test fixtures containing all 20 rows from the exact key/catalog table above in that order; they do not call production localization code. `expected_used_semantic_keys_for_all_fixture_states()` is the exact set of every graph key, all four entity/status keys, `search_topic.reporting`, all reporting-search fact keys, and all four guidance keys reached across initial, corrected-price, promoted-customer, search-shown/completed, and reset states. `expected_fully_projected_fixture(locale, state_name)` is also independent literal test data: for each of those five states it contains every localized Company/Offer/Relationship/Discovery fact and status, every present search topic/entity/fact, every guidance sentence including the localized semantic `{next_step}` parameter, and the stable action-result code (whose visible label belongs to the separately parity-tested interface-copy map). Only the exact names/email/numeric-money tuple may be equal between its English and Chinese variants. Duplicate catalog keys fail ordered equality before lookup, and any stable-key-shaped token left in a rendered projection fails `unresolved_semantic_keys`.

- [ ] **Step 2: Run RED**

```bash
cargo test -p sovereign-consultant-playground tests::response_is_a_one_way_projection_marked_synthetic_and_nonpersistent --locked -- --exact --nocapture
cargo test -p sovereign-consultant-playground tests::compiled_reporting_search_and_guidance_are_read_only --locked -- --exact --nocapture
cargo test -p sovereign-consultant-playground tests::every_used_teaching_key_has_complete_exact_en_and_zh_projections --locked -- --exact --nocapture
cargo test -p sovereign-consultant-playground tests::full_locale_projections_translate_facts_search_and_guidance_but_not_literals --locked -- --exact --nocapture
```

Expected: compile failure for missing read/query modules; Tasks 1–2 remain GREEN.

- [ ] **Step 3: Implement pure projections**

Within the leaf, only `read_model.rs` imports serde until Task 4's `contract.rs` uses it for wire types. `query.rs` reads the private graph and emits semantic keys plus the declared literal name/money parameters. `localize.rs` contains two exhaustive wildcard-free matches over `PlaygroundTextKey`, one per locale, using the exact table above; it never falls back from Chinese to English. Projection resolves typed semantic parameters such as `{next_step}` before interpolation and rejects a missing key, wrong parameter kind, missing placeholder, or extra placeholder. The closed `Reporting` topic scans keyed Company→Offer→Relationship→Discovery facts and returns at most 10 keyed hits without accepting query text. Guidance order is keyed: correct price, promote lead, follow the recorded 30-minute scoping call, then complete. Guidance returns navigation only and cannot call `apply`.

- [ ] **Step 4: Run GREEN and checkpoint A review**

```bash
./scripts/run-exact-test-group.sh sovereign-consultant-playground tests \
  tests::consultant_fixture_is_exact_complete_and_deterministic \
  tests::fixed_actions_change_only_the_two_documented_business_fields \
  tests::response_is_a_one_way_projection_marked_synthetic_and_nonpersistent \
  tests::compiled_reporting_search_and_guidance_are_read_only \
  tests::every_used_teaching_key_has_complete_exact_en_and_zh_projections \
  tests::full_locale_projections_translate_facts_search_and_guidance_but_not_literals \
  tests::graph_and_query_hold_keys_not_human_language_catalog_values
./scripts/run-exact-test-group.sh sovereign-cli workspace::tests::full_founder_flow_with_approval_and_evidence \
  workspace::tests::full_founder_flow_with_approval_and_evidence
./scripts/check-file-size.sh
```

Checkpoint A asks whether the teaching graph is useful while remaining structurally unrelated to persistent Workspace. Reject the slice if any graph/action/query file imports serialization, persistence, root-path, audit, model, or effect authority.

- [ ] **Step 5: Commit; push only if separately authorized**

```bash
git add crates/consultant-playground
git commit -m "feat(playground): derive fixed search and guidance"
# Authorized implementation sessions may now run:
# git push -u origin feature/consultant-onboarding
```

## Task 4: Add typed synthetic HTTP contracts inside the leaf without cutting over the CLI UI

**Files:**
- Create: `crates/consultant-playground/src/contract.rs`
- Create: `crates/consultant-playground/src/handler.rs`
- Modify: `crates/consultant-playground/src/lib.rs`
- Modify: `crates/consultant-playground/src/tests.rs`

**Consumes:** Task 3 `PlaygroundSession::read/dispatch`.

**Produces:**
- `PlaygroundActionRequest { action: PlaygroundActionWire }` with `#[serde(deny_unknown_fields)]`.
- `ApiErrorBody { code: ApiErrorCode, field_errors: BTreeMap<FieldKey, FieldErrorCode> }`.
- `PlaygroundHttpHandler::new()`, `get()`, and `post(request)`; the handler owns only `Mutex<PlaygroundSession>`.
- Pure request dispatch for future routes `GET /api/playground/consultant` and `POST /api/playground/consultant/action` with no-store JSON; Task 5 creates the leaf server and performs the CLI cutover.
- `MAX_PLAYGROUND_ACTION_BODY_BYTES = 256`; body reading uses `take(257)` and rejects byte 257 regardless of `Content-Length`.

- [ ] **Step 1: Write contract/handler RED tests**

```rust
#[test]
fn playground_handler_constructs_without_root_store_or_backend() {
    let handler = PlaygroundHttpHandler::new().unwrap();
    let response = handler.get();
    assert_eq!(response.body["profile"], "synthetic_playground");
    assert_eq!(response.body["real_data_enabled"], false);
    assert_eq!(response.body["persistence"], "none");
    assert_eq!(response.body["graph"]["company"]["service_key"],
        "company_service.operations_reporting");
    assert_eq!(response.body["translations"]["en"].as_array().unwrap().len(),
        20);
    assert_eq!(response.body["translations"]["zh_cn"].as_array().unwrap().len(),
        20);
}

#[test]
fn action_request_accepts_only_one_closed_unit_value() {
    assert!(parse_action(json!({"action":"correct_offer_price"})).is_ok());
    for invalid in [
        json!({}),
        json!({"action":"correct_offer_price","price_cents":1}),
        json!({"action":"promote_acme_to_customer","name":"Real Client"}),
        json!({"action":"show_reporting_search","query":"Private"}),
        json!({"action":"reset","backend":"v2"}),
    ] {
        assert!(parse_action(invalid).is_err());
    }
}

#[test]
fn request_failures_have_stable_codes_independent_of_serde_prose() {
    for case in request_failure_cases() {
        let response = send_playground_request(case.request);
        assert_eq!(response.status, case.status, "{}", case.name);
        assert_eq!(response.json_body(), case.expected_body, "{}", case.name);
        assert_eq!(response.header("Cache-Control"), Some("no-store"));
    }
}

#[test]
fn every_playground_json_response_is_typed_and_no_store() {
    for response in playground_response_matrix() {
        assert_eq!(response.header("Cache-Control"), Some("no-store"));
        assert_eq!(response.header("Content-Type"), Some("application/json; charset=utf-8"));
        assert_eq!(response.header("Access-Control-Allow-Origin"), None);
    }
    assert_eq!(error_response(ApiErrorCode::InvalidRequest).status, 400);
    assert_eq!(error_response(ApiErrorCode::InvalidPlaygroundState).status, 422);
    assert_eq!(error_response(ApiErrorCode::NotFound).status, 404);
    assert_eq!(error_response(ApiErrorCode::Internal).status, 500);
}
```

`ApiErrorCode` is exactly `InvalidRequest`, `UnsupportedMediaType`, `PayloadTooLarge`, `MethodNotAllowed`, `ForbiddenHost`, `InvalidPlaygroundState`, `NotFound`, and `Internal`, mapped respectively to 400, 415, 413, 405, 403, 422, 404, and 500. `FieldKey` is exactly `Request` and `Action`; `FieldErrorCode` is exactly `Required`, `UnknownField`, and `UnknownValue`. Serialized enum values use snake_case. `request_failure_cases()` covers empty/malformed/non-object JSON, absent action, duplicate `action`, unknown member, unknown action, wrong content type, a 257-byte body, wrong method including `OPTIONS`, and forbidden Host. It asserts the complete JSON body—not a substring—and no body contains localized or serde-generated prose. No response sends a CORS allow-origin header.

- [ ] **Step 2: Run RED**

```bash
cargo test -p sovereign-consultant-playground tests::playground_handler_constructs_without_root_store_or_backend --locked -- --exact --nocapture
cargo test -p sovereign-consultant-playground tests::action_request_accepts_only_one_closed_unit_value --locked -- --exact --nocapture
cargo test -p sovereign-consultant-playground tests::request_failures_have_stable_codes_independent_of_serde_prose --locked -- --exact --nocapture
cargo test -p sovereign-consultant-playground tests::every_playground_json_response_is_typed_and_no_store --locked -- --exact --nocapture
```

Expected: compile failure for the missing leaf contract/handler. Do not touch the CLI UI module, routes, or assets in this task; Task 5 performs one atomic physical cutover, so no intermediate binary can expose two browser authorities.

- [ ] **Step 3: Implement the isolated handler and two additive routes**

The handler constructor accepts no arguments. Its struct has one field: `Mutex<PlaygroundSession>`. Contract parsing deserializes the original bounded raw bytes directly into the deny-unknown-fields struct so serde's duplicate-field rejection is not lost through `Value`. On failure, a separate bounded `serde_json::Value` parse may classify missing/unknown/value errors into the stable enum; it never authorizes acceptance, and an otherwise unclassifiable serde failure maps to `InvalidRequest` with an empty field map. Map errors by structural checks, never by inspecting serde prose. Exhaustively map the accepted wire enum to the domain enum. The leaf has no type or dependency through which the handler could accept a path, root, Store, Workspace, Vault, repository, audit, model, signer, cookie, or backend selector. Keep current CLI route behavior unchanged until Task 5.

- [ ] **Step 4: Run GREEN and old UI regressions**

```bash
./scripts/run-exact-test-group.sh sovereign-consultant-playground tests \
  tests::playground_handler_constructs_without_root_store_or_backend \
  tests::action_request_accepts_only_one_closed_unit_value \
  tests::request_failures_have_stable_codes_independent_of_serde_prose \
  tests::every_playground_json_response_is_typed_and_no_store \
  tests::every_used_teaching_key_has_complete_exact_en_and_zh_projections \
  tests::full_locale_projections_translate_facts_search_and_guidance_but_not_literals
./scripts/run-exact-test-group.sh sovereign-cli workspace::tests::full_founder_flow_with_approval_and_evidence \
  workspace::tests::full_founder_flow_with_approval_and_evidence
```

- [ ] **Step 5: Commit**

```bash
git add crates/consultant-playground
git commit -m "feat(ui-api): add isolated consultant playground routes"
```

## Task 5: Atomically cut the CLI UI over to the physical synthetic-only leaf

**Files:**
- Create: `crates/consultant-playground/src/server.rs`
- Modify: `crates/consultant-playground/src/lib.rs`
- Modify: `crates/consultant-playground/src/tests.rs`
- Create: `crates/consultant-playground/tests/authority_boundary.rs`
- Create: `crates/consultant-playground/assets/index.html`
- Create: `crates/consultant-playground/assets/styles.css`
- Create: `crates/consultant-playground/assets/i18n.js`
- Create: `crates/consultant-playground/assets/app.js`
- Create: `crates/consultant-playground/assets/consultant-ui.js`
- Create: `crates/consultant-playground/assets/favicon.svg`
- Create: `crates/consultant-playground/assets/tsconfig.json`
- Modify: `Cargo.toml`
- Modify: `apps/cli/Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `apps/cli/src/main.rs`
- Delete: `apps/cli/src/ui.rs`
- Delete: `apps/cli/assets/index.html`
- Delete: `apps/cli/assets/styles.css`
- Delete: `apps/cli/assets/i18n.js`
- Delete: `apps/cli/assets/app.js`
- Delete: `apps/cli/assets/favicon.svg`
- Delete: `apps/cli/assets/tsconfig.json`
- Create: `apps/cli/tests/fixtures/expected_consultant_playground_cli_surface.rs.txt`
- Create: `apps/cli/tests/consultant_playground_cli_boundary.rs`
- Create: `apps/cli/tests/consultant_playground_process.rs`
- Create: `docs/security/consultant-playground-attack-review.md` (release prerequisite decision plus initial boundary evidence)

**Consumes:** Task 4's isolated leaf handler and typed contracts.

**Produces:** One physical Playground authority leaf; the exact route and asset manifest; a complete recursive leaf production-source and direct-dependency boundary; an exact annotated CLI/Commands/main surface that permits no custom UI parse hook and whose UI arm can only call leaf `run(port)`; static value-free advanced security details; and both in-process and two-root real-binary canary isolation evidence. It produces no CLI-local Playground module, module-graph parser, storage adapter, or persistent format.

- [ ] **Release prerequisite: decide fresh-export availability before writing the cutover RED**

Record one dated product decision in `docs/security/consultant-playground-attack-review.md`: `fresh v1 export required for this release: yes/no`, decision owner, and rationale. `yes` makes Task 5 **Blocked** until actual owner authorization plus protected storage provide one authenticated export-creation entry point; stop without removing any route. `no` authorizes the synthetic cutover only with release/help copy stating that new real-data export creation and disclosure browsing become temporarily unavailable and that the CLI only verifies an export file the user already has. Silence or an ambiguous answer is `yes`/Blocked; a warning acknowledgement or hidden legacy route is never an alternative.

- [ ] **Step 1: Write the physical-boundary and immediate-dispatch RED tests first**

In `crates/consultant-playground/tests/authority_boundary.rs`, recursively inventory every file below `crates/consultant-playground/src`. The exact production manifest is:

```rust
const EXPECTED_LEAF_PRODUCTION_RUST: &[&str] = &[
    "src/actions.rs",
    "src/contract.rs",
    "src/graph.rs",
    "src/handler.rs",
    "src/keys.rs",
    "src/lib.rs",
    "src/localize.rs",
    "src/query.rs",
    "src/read_model.rs",
    "src/server.rs",
];

const EXPECTED_LEAF_TEST_ONLY_RUST: &[&str] = &["src/tests.rs"];
const EXPECTED_LEAF_ASSETS: &[&str] = &[
    "assets/app.js",
    "assets/consultant-ui.js",
    "assets/favicon.svg",
    "assets/i18n.js",
    "assets/index.html",
    "assets/styles.css",
    "assets/tsconfig.json",
];

const FORBIDDEN_LEAF_AUTHORITY_TOKENS: &[&str] = &[
    "data_dir", "workspace", "Workspace", "Store", "Vault", "dirs",
    "fs", "env", "path", "Path", "PathBuf", "File", "OpenOptions",
    "process", "Command", "Stdio", "Child", "current_exe",
    "net", "network", "TcpStream", "UdpSocket", "ToSocketAddrs", "SocketAddr",
    "stdin", "from_listener", "listener", "SslConfig", "Server::https",
    "reqwest", "ureq", "hyper", "curl", "surf", "awc", "isahc", "tokio",
    "tungstenite", "quinn", "tonic", "lettre",
    "sovereign_", "sovereign-", "audit", "ledger", "effect", "outbox",
    "export", "disclosure", "repository", "model", "unsafe", "extern crate",
    "chrono", "uuid", "rand", "SystemTime", "Instant", "thread",
];

#[test]
fn leaf_manifest_dependencies_sources_and_authority_surface_are_exact() {
    assert_eq!(recursive_rust_inventory("src"), expected_all_leaf_rust());
    assert_eq!(recursive_asset_inventory("assets"), EXPECTED_LEAF_ASSETS);

    let manifest = parse_manifest(include_str!("../Cargo.toml"));
    assert_eq!(manifest.package_name(), "sovereign-consultant-playground");
    assert_eq!(manifest.publish(), Some(false));
    assert_eq!(manifest.direct_dependency_names(), set!["serde", "serde_json", "tiny_http"]);
    assert_eq!(manifest.normalized_dependency_declarations(), vec![
        Dependency::workspace("serde"),
        Dependency::workspace("serde_json"),
        Dependency::version("tiny_http", "0.12"),
    ]);
    assert!(manifest.target_specific_dependencies().is_empty());
    assert!(manifest.build_dependencies().is_empty());
    assert!(manifest.dev_dependencies().is_empty());
    assert!(manifest.features().is_empty());
    assert!(manifest.build_script().is_none());

    for source in load_exact_production_sources(EXPECTED_LEAF_PRODUCTION_RUST) {
        let code = strip_comments_and_literal_contents(&source.text);
        for token in FORBIDDEN_LEAF_AUTHORITY_TOKENS {
            assert!(!contains_rust_token_sequence(&code, token),
                "forbidden authority `{token}` in {}", source.path);
        }
        for scheme in ["http://", "https://", "file://", "ftp://"] {
            assert!(!source.text.contains(scheme),
                "forbidden production URL `{scheme}` in {}", source.path);
        }
        assert!(!code.contains("#[path"));
        assert!(!code.contains("include!("));
        assert!(!code.contains("include_bytes!("));
        assert!(!code.contains("macro_rules!("));
        assert_eq!(extract_macro_invocations(&code),
            expected_safe_macro_invocations_for(source.path));
        assert_eq!(extract_attributes(&code),
            expected_safe_attributes_for(source.path));
    }

    assert_eq!(exact_include_str_asset_map(), BTreeMap::from([
        ("../assets/index.html", "/"),
        ("../assets/styles.css", "/assets/styles.css"),
        ("../assets/i18n.js", "/assets/i18n.js"),
        ("../assets/app.js", "/assets/app.js"),
        ("../assets/consultant-ui.js", "/assets/consultant-ui.js"),
        ("../assets/favicon.svg", "/favicon.svg"),
    ]));
    assert_eq!(module_declaration_inventory(all_production_sources()), vec![
        PrivateModule::new("src/lib.rs", "actions"),
        PrivateModule::new("src/lib.rs", "contract"),
        PrivateModule::new("src/lib.rs", "graph"),
        PrivateModule::new("src/lib.rs", "handler"),
        PrivateModule::new("src/lib.rs", "keys"),
        PrivateModule::new("src/lib.rs", "localize"),
        PrivateModule::new("src/lib.rs", "query"),
        PrivateModule::new("src/lib.rs", "read_model"),
        PrivateModule::new("src/lib.rs", "server"),
        PrivateModule::cfg_test("src/lib.rs", "tests"),
    ]);
    assert_eq!(extract_server_bind_expression(source("src/server.rs")),
        r#"Server::http(("127.0.0.1", port))"#);
    assert_eq!(server_constructor_inventory(all_production_sources()), [
        r#"Server::http(("127.0.0.1", port))"#,
    ]);
    assert_eq!(public_item_inventory(), ["pub fn run"]);
}

#[test]
fn obvious_indirect_authority_and_code_inclusion_bypasses_are_rejected() {
    for injected in [
        r#"fn p() -> bool { super::data_dir().join("vault").exists() }"#,
        r#"fn p() { let _ = workspace(); }"#,
        r#"fn p() { let _ = std::env::var("HOME"); }"#,
        r#"fn p() { let _ = std::fs::read("ignored"); }"#,
        r#"fn p() { use std as x; let _ = x::fs::read("ignored"); }"#,
        r#"fn p() { std::process::Command::new("ignored"); }"#,
        r#"include!("generated.rs");"#,
        r#"#[path = "../escape.rs"] mod escape;"#,
        r#"unknown_macro!(hidden_authority);"#,
    ] {
        assert!(validate_leaf_source("src/server.rs", injected).is_err(),
            "boundary accepted injected source: {injected}");
    }
}
```

This is deliberately a physical crate/source check, not a home-grown Rust module reachability parser. Recursive inventory fails for every new `.rs` file until it is classified in the exact production/test-only manifest; reject symlinks and require every canonical inventory path to stay under the leaf root. The exact root-module inventory makes all nine feature modules private and permits `src/tests.rs` only through one `#[cfg(test)] mod tests`; no other file may declare a module, so test-only authority cannot be compiled into production. Every production file receives the same token-aware denylist. Authority terminal identifiers such as bare `fs`, `env`, `path`, `process`, and `net` are denied regardless of a `std`, alias, `self`, or `super` prefix, so `use std as x; x::fs` cannot evade it. The scanner removes comments and string contents before identifier checks so copy cannot create exceptions, then separately rejects remote/file URL schemes in raw production source and pins every `include_str!` literal to one of the six leaf-owned assets. `extract_macro_invocations` is a simple lexical `identifier!` inventory: allowed invocations are exact, per-file fixtures limited to the six pinned `include_str!` calls and ordinary `format!`, `println!`, `vec!`, or `matches!` calls actually present; every qualified, custom, generated, or newly added macro fails. Attributes are likewise exact per file, limited to required `derive`, `serde`, and the single `cfg(test)` test-module declaration; `#[path]` always fails. No build script or generated source exists, and `run` is the sole public item.

The normalized dependency-declaration assertion is decisive: aliases, renamed packages, target-specific sections, and hidden extra dependency tables fail even if their visible key resembles an allowed crate. The leaf cannot compile a call to CLI `data_dir`, Workspace, Vault, `dirs`, filesystem/environment/path/process APIs, stdin, clock/random/thread sources, or an outbound client through a dependency. `tiny_http` is the sole networking dependency; the all-source constructor inventory contains exactly one literal inbound `127.0.0.1` `Server::http` call, while HTTPS/from-listener/raw socket/client APIs and remote URL literals fail the uniform source gate. Test-only boundary/process code may use filesystem, environment, process, Vault, and `TcpStream` only to prove isolation and is never included in `EXPECTED_LEAF_PRODUCTION_RUST`.

In `apps/cli/tests/fixtures/expected_consultant_playground_cli_surface.rs.txt`, store the exact Clap import followed by these three complete intended items in order. This is the literal fixture content, not pseudocode:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sovereign", about = "Sovereign Runtime CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize local device identity, vault, and ledger
    Init,
    /// Run the story-driven secure kernel demo (real signatures, real denials)
    Demo {
        /// Run straight through without pausing between acts
        #[arg(long)]
        fast: bool,
    },
    /// Run a mechanical check of the import-free Phase A Wasmtime path
    SandboxCheck,
    /// Show vault entry names
    Status,
    /// Run the synthetic consultant practice example on 127.0.0.1
    Ui {
        /// Port to bind on loopback
        #[arg(long, default_value_t = 7787)]
        port: u16,
    },
    /// Demonstrate model-gateway health-aware failover and the Red-data guard
    ModelCheck,
    /// Demonstrate durable workflow checkpoints resuming across a crash
    WorkflowDemo,
    /// Verify an existing exported bundle offline
    VerifyExport {
        /// Path to an export file you already have; this command does not create one.
        path: PathBuf,
    },
    /// Self-audit: reconcile local state against the signed audit chain
    Integrity,
    /// Internal: compile one artifact from stdin in a killable worker process.
    /// Not for direct use — spawned by the runtime to isolate untrusted
    /// Wasmtime compilation. Reads digest(32)||module bytes, writes the
    /// serialized module to stdout.
    #[command(name = "__compile-worker", hide = true)]
    CompileWorker,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match <Cli as clap::Parser>::parse().command {
        Commands::Init => cmd_init()?,
        Commands::Demo { fast } => demo::run(fast, data_dir())?,
        Commands::SandboxCheck => cmd_sandbox_check()?,
        Commands::Status => cmd_status()?,
        Commands::Ui { port } => sovereign_consultant_playground::run(port)?,
        Commands::ModelCheck => cmd_model_check(),
        Commands::WorkflowDemo => cmd_workflow_demo()?,
        Commands::VerifyExport { path } => cmd_verify_export(&path)?,
        Commands::Integrity => cmd_integrity()?,
        Commands::CompileWorker => {
            let code =
                sovereign_sandbox::run_compile_worker(std::io::stdin().lock(), std::io::stdout());
            std::process::exit(i32::from(code));
        }
    }
    Ok(())
}
```

Then add:

```rust
#[test]
fn consultant_playground_cli_surface_is_exact_and_main_dispatches_directly() {
    let source = include_str!("../src/main.rs");
    let actual = extract_exact_cli_surface(source).unwrap();
    let expected = include_str!(
        "fixtures/expected_consultant_playground_cli_surface.rs.txt"
    );
    assert_eq!(normalize_line_endings(&actual), normalize_line_endings(expected));

    let main = extract_brace_balanced_item(source, "fn main()").unwrap();
    assert!(main.starts_with(concat!(
        "fn main() -> Result<(), Box<dyn std::error::Error>> {\n",
        "    match <Cli as clap::Parser>::parse().command {",
    )));
    assert!(!main.contains("Cli::parse"));
    assert_eq!(count_occurrences(main, "Commands::Ui"), 1);
    assert_eq!(count_occurrences(main,
        "Commands::Ui { port } => sovereign_consultant_playground::run(port)?"), 1);
    for forbidden in ["let cli", "data_dir()", "std::env", "std::fs", "std::process::Command"] {
        assert!(!text_before_ui_arm(main).contains(forbidden),
            "shared pre-dispatch authority: {forbidden}");
    }
}

#[test]
fn consultant_playground_cli_ui_parser_is_builtin_u16_only() {
    let source = include_str!("../src/main.rs");
    let cli = extract_annotated_item(source, "struct Cli").unwrap();
    let commands = extract_annotated_item(source, "enum Commands").unwrap();
    let ui = extract_enum_variant(commands, "Ui").unwrap();
    assert_eq!(normalize_rust_tokens(ui), normalize_rust_tokens(r#"
        /// Run the synthetic consultant practice example on 127.0.0.1
        Ui {
            /// Port to bind on loopback
            #[arg(long, default_value_t = 7787)]
            port: u16,
        }
    "#));
    assert_eq!(variant_fields(ui), [("port", "u16")]);
    assert_eq!(all_clap_attribute_inventory(cli, commands), [
        "Cli:derive(Parser)",
        "Cli:command(name=sovereign,about=Sovereign Runtime CLI,version)",
        "Cli.command:command(subcommand)",
        "Commands:derive(Subcommand)",
        "Commands::Demo.fast:arg(long)",
        "Commands::Ui.port:arg(long,default_value_t=7787)",
        "Commands::CompileWorker:command(name=__compile-worker,hide=true)",
    ]);
    assert_eq!(extract_exact_use(source, "clap"),
        "use clap::{Parser, Subcommand};");
    assert!(module_or_import_aliases_named(source, "clap").is_empty());
    assert!(!rust_token_sequences(source).contains_any(&[
        "mod clap", "extern crate clap", "use crate as clap", "use self as clap",
    ]));
    for forbidden in [
        "value_parser", "env", "flatten", "default_value",
        "default_value_os_t", "default_values_t", "default_missing_value",
        "default_value_if", "default_value_ifs", "no_open",
    ] {
        assert!(!clap_attribute_key_inventory(cli, commands, "Ui")
            .contains(forbidden), "forbidden UI parser hook: {forbidden}");
    }
}

#[test]
fn consultant_playground_cli_legacy_browser_surfaces_are_removed() {
    let cli_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(!cli_root.join("src/ui.rs").exists());
    let source = include_str!("../src/main.rs");
    assert!(!source.contains("no_open"));
    assert!(!source.contains("--no-open"));
    for production in load_all_cli_production_rust_recursively(cli_root.join("src")) {
        for forbidden in [
            "launch_browser", "\"xdg-open\"", "Command::new(\"open\")",
            "Command::new(\"cmd\").args([\"/C\", \"start\"",
        ] {
            assert!(!normalized_source(&production).contains(forbidden),
                "legacy browser surface `{forbidden}` in {}", production.path);
        }
    }
}
```

`extract_exact_cli_surface` concatenates the exact Clap import, complete annotated `Cli` item, complete annotated `Commands` item, and complete `main` item with the same separators as the fixture. Its brace-balanced scanner strips comments/string contents only while finding boundaries; comparison uses original bytes, so every Clap attribute, field, variant, doc line, command arm, and whitespace-bearing source line is pinned. `extract_annotated_item` includes all contiguous outer attributes. The separate semantic assertions make the security reason legible: the import is exactly from the external `clap` crate with no local-module/alias shadow; explicit `<Cli as clap::Parser>::parse()` bypasses any inherent shadow helper; `Ui` has only the built-in `u16` parser and fixed numeric `default_value_t`; no `value_parser`, environment parser, flattening, custom default, helper field, legacy `no_open`, or browser launcher can survive. Review the fixture diff against pre-cutover `main.rs` and require that only the documented UI/verify help, removal of `no_open`, explicit parse expression, and direct leaf arm differ; all non-UI variants, attributes, fields, and arms remain exact.

- [ ] **Step 2: Write the route, complete-asset, and real-binary RED tests**

```rust
#[test]
fn unauthenticated_route_manifest_is_exactly_static_plus_playground() {
    assert_eq!(UNAUTHENTICATED_ROUTE_MANIFEST, &[
        RouteSpec::get("/"),
        RouteSpec::get("/assets/styles.css"),
        RouteSpec::get("/assets/i18n.js"),
        RouteSpec::get("/assets/app.js"),
        RouteSpec::get("/assets/consultant-ui.js"),
        RouteSpec::get("/favicon.svg"),
        RouteSpec::get("/api/playground/consultant"),
        RouteSpec::post("/api/playground/consultant/action"),
    ]);
}

#[test]
fn every_legacy_real_or_security_state_route_is_unregistered() {
    for (method, route) in legacy_route_matrix() {
        let response = test_route(method, route, br#"{"canary":"REAL_BROWSER_CANARY"}"#);
        assert_eq!(response.status, 404, "{method:?} {route}");
        assert_eq!(response.header("Cache-Control"), Some("no-store"));
    }
}
```

`legacy_route_matrix()` includes GET `/api/state`, `/api/command-center`, `/api/workspace`, `/api/export`; POST `/api/verify-export`, `/api/gauntlet`, `/api/workspace/assist`, every current `/api/workspace/*` mutation; plausible future Company/Relationship/Discovery/Search paths; both methods for wildcard prefixes; and one arbitrary unknown route. There is no fallback dispatcher.

`complete_shipped_asset_graph_uses_only_playground_endpoints_and_existing_elements` loads all six embedded assets and asserts:

- HTML fetch-bearing references equal the five static routes; `href`, `action`, `formaction`, `ping`, `src`, `srcset`, `poster`, `data`, `manifest`, `<base>`, and meta refresh contain no remote, protocol-relative, `data:`, `blob:`, `javascript:`, or unclassified URL.
- CSS has no `@import` or `url(...)`; favicon SVG has no script, `foreignObject`, event attribute, external `href`/`xlink:href`, animation reference, or CSS URL.
- JavaScript's only network calls are the two literal same-origin `fetch` endpoints. Reject `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon`, `WebTransport`, `RTCPeerConnection`, dynamic import, workers, navigation writes, URL-bearing element/property creation, synthetic form submission, and unknown network APIs.
- Only locale and theme keys use `localStorage`; there is no cookie, session storage, IndexedDB, Cache API, service worker, exercise persistence, form/input/textarea/file/paste/contenteditable, `innerHTML`, dynamic script/link/image, or download surface.
- Every event, DOM id, fragment, guidance target, action literal, and POST body is exact and resolvable. The only POST expression is `JSON.stringify({ action })`.
- The full English/Chinese interface copy map and the server's 20 semantic teaching keys have exact parity, placeholder parity, no fallback, and complete projections for initial/corrected/promoted/search/reset states. Names/email/money are the only untranslated values.
- Semantic landmarks, heading order, skip link, names/descriptions, alerts/live regions, 44px targets, visible focus, 760px single-column breakpoint, wrapping, reduced motion, and zoom-safe viewport rules are present.
- Beginner copy contains none of `1C0`, `ActiveV2`, `AuthorizedStore`, `Vault v2`, repository, migration, or E2EE; those terms may appear only in the optional advanced object/details. Safety/AI/persistence claims remain denied.

Required beginner notices remain exactly the English and Simplified Chinese sentences in Global Constraints. The advanced disclosure is exactly: “Real workspace export creation and disclosure browsing are unavailable in this preview. The CLI can verify an export file you already have, but it cannot create a new export. The integrity command shows a summary only.” and “此预览版暂不支持创建真实工作区导出或浏览披露记录。CLI 只能验证你已有的导出文件，不能创建新导出。完整性命令只显示摘要。”

In `apps/cli/tests/consultant_playground_process.rs`, write the genuine black-box RED. It must execute the Cargo-built `sovereign` binary, never call leaf functions directly:

```rust
#[test]
fn real_ui_process_is_independent_of_two_distinct_platform_canary_roots() {
    let alpha = PlatformDataFixture::new("alpha");
    let beta = PlatformDataFixture::new("beta");
    alpha.seed_valid_v1_vault(distinct_canary_workspace(CanarySet::Alpha));
    beta.seed_valid_v1_vault(distinct_canary_workspace(CanarySet::Beta));
    assert_ne!(alpha.seeded_plaintext_sha256(), beta.seeded_plaintext_sha256());
    let alpha_before = recursive_snapshot(alpha.platform_home());
    let beta_before = recursive_snapshot(beta.platform_home());

    let alpha_run = run_real_sovereign_ui_and_capture(&alpha, exact_http_sequence());
    let beta_run = run_real_sovereign_ui_and_capture(&beta, exact_http_sequence());

    assert_eq!(alpha_run.ui_help_stdout, beta_run.ui_help_stdout);
    assert_eq!(alpha_run.ui_help_stderr, beta_run.ui_help_stderr);
    assert!(!alpha_run.legacy_no_open_used,
        "final CLI must remove the legacy --no-open flag");
    assert!(!beta_run.legacy_no_open_used,
        "final CLI must remove the legacy --no-open flag");
    assert!(!help_has_exact_long_flag(&alpha_run.ui_help_stdout, "--no-open"));
    assert_eq!(alpha_run.complete_response_transcript,
        beta_run.complete_response_transcript);
    for canary in ALL_ALPHA_AND_BETA_CANARIES {
        assert!(!contains_bytes(&alpha_run.complete_response_transcript, canary));
        assert!(!contains_bytes(&beta_run.complete_response_transcript, canary));
        assert!(!contains_bytes(&alpha_run.ui_help_stdout, canary));
        assert!(!contains_bytes(&alpha_run.ui_help_stderr, canary));
        assert!(!contains_bytes(&beta_run.ui_help_stdout, canary));
        assert!(!contains_bytes(&beta_run.ui_help_stderr, canary));
        assert!(!alpha_run.stdout.contains(canary));
        assert!(!alpha_run.stderr.contains(canary));
        assert!(!beta_run.stdout.contains(canary));
        assert!(!beta_run.stderr.contains(canary));
    }
    assert_eq!(recursive_snapshot(alpha.platform_home()), alpha_before);
    assert_eq!(recursive_snapshot(beta.platform_home()), beta_before);
}

fn safe_ui_args_for_red_or_green(
    binary: &Path,
    port: u16,
    isolated_environment: &[(OsString, OsString)],
) -> (Vec<OsString>, bool, CapturedOutput) {
    let help = run_captured(binary, ["ui", "--help"], isolated_environment);
    assert!(help.status.success());
    let legacy_no_open = help_has_exact_long_flag(&help.stdout, "--no-open");
    let mut args = vec![OsString::from("ui"), OsString::from("--port"),
        OsString::from(port.to_string())];
    if legacy_no_open {
        // Baseline RED safety only: suppress current ui.rs browser subprocesses.
        args.push(OsString::from("--no-open"));
    }
    (args, legacy_no_open, help)
}
```

Each `PlatformDataFixture` redirects every applicable OS platform-data/home variable into its own `TempDir`, probes the resolved native data directory in a test-only child, proves the path is beneath that temp root, and then seeds a valid current v1 Vault at the exact location the unchanged CLI would use. Alpha/Beta contain the same shape but different venture, customer, document, disclosure, email, timestamp, and UUID canaries; their plaintext hashes must differ. Fail safely rather than touch a developer directory if native redirection cannot be proven. Snapshot the entire redirected platform home only after seeding.

`exact_http_sequence()` is one immutable ordered raw HTTP/1.1 byte vector used for both real processes. It GETs `/`, all five assets, and the Playground read route; POSTs all four actions; covers malformed/duplicate/extra/unknown/oversize/wrong-content-type/wrong-method/`OPTIONS`/forbidden-Host cases; probes every legacy route and an arbitrary unknown route. The transcript includes each exact status line, every header in deterministic order, declared body length, and every body byte.

Before either server spawn, `safe_ui_args_for_red_or_green` runs the same binary's `ui --help` inside that fixture's child-only environment. If and only if the exact legacy `--no-open` long flag is present, it appends that flag to the old binary's RED invocation so current `ui.rs` cannot run `open`, `cmd /C start`, or `xdg-open`. It records the branch and full help bytes rather than hiding it. The intended post-cutover GREEN requires both recorded booleans false, exact help without `--no-open`, deletion of `ui.rs`, and the source-level launcher-denial test above. Thus the pre-cutover RED still exercises the old HTTP server safely and fails for the intended route/authority/CLI-surface reasons; it never creates an external browser side effect.

Capture complete server stdout/stderr; terminate/wait in `Drop`; and require byte-identical help/transcripts, canary-free streams, and unchanged recursive snapshots. The help probe is included between the before/after snapshots and its stdout/stderr is also canary-scanned. This catches any root read, derived/discarded read that changes output, leak, or write at the real binary boundary; the physical leaf plus exact annotated CLI/Commands/main fixture prevents silent parser or pre-dispatch reads even when their value is discarded.

- [ ] **Step 3: Run the genuine RED batch**

```bash
cargo test -p sovereign-consultant-playground --test authority_boundary leaf_manifest_dependencies_sources_and_authority_surface_are_exact --locked -- --exact --nocapture
cargo test -p sovereign-consultant-playground --test authority_boundary obvious_indirect_authority_and_code_inclusion_bypasses_are_rejected --locked -- --exact --nocapture
cargo test -p sovereign-cli --test consultant_playground_cli_boundary consultant_playground_cli_surface_is_exact_and_main_dispatches_directly --locked -- --exact --nocapture
cargo test -p sovereign-cli --test consultant_playground_cli_boundary consultant_playground_cli_ui_parser_is_builtin_u16_only --locked -- --exact --nocapture
cargo test -p sovereign-cli --test consultant_playground_cli_boundary consultant_playground_cli_legacy_browser_surfaces_are_removed --locked -- --exact --nocapture
cargo test -p sovereign-consultant-playground tests::unauthenticated_route_manifest_is_exactly_static_plus_playground --locked -- --exact --nocapture
cargo test -p sovereign-consultant-playground tests::complete_shipped_asset_graph_uses_only_playground_endpoints_and_existing_elements --locked -- --exact --nocapture
cargo test -p sovereign-cli --test consultant_playground_process real_ui_process_is_independent_of_two_distinct_platform_canary_roots --locked -- --exact --nocapture
```

Expected genuine failures: `server.rs`, the six leaf assets, exact source/asset manifest tests, CLI leaf dependency/direct arm, annotated CLI/Commands/main fixture, and process test do not exist; current `Cli` uses the unqualified inherent-call form, `Ui` still has `no_open`, and current `ui` calls CLI-local `ui::run(port, data_dir(), ...)`, opens/reads the current Workspace, registers real routes, and serves old assets. The real-binary RED detects and supplies old `--no-open`, so it suppresses the existing browser launcher before exercising HTTP; it then fails on the intended boundary/route assertions and the required final absence of the flag. Record these failures before implementation. Do not manufacture a later acceptance RED.

- [ ] **Step 4: Implement the atomic physical cutover**

Implement `pub fn run(port: u16) -> Result<(), Box<dyn std::error::Error>>` as the leaf's only public item. It constructs `PlaygroundHttpHandler::new()` with no argument, binds `tiny_http` only to `("127.0.0.1", port)`, prints exactly “loopback practice example · synthetic data only · Ctrl-C to stop”, and dispatches only the exact compiled manifest. Body reading uses `take(257)` and rejects byte 257. Host is a literal loopback/localhost allowlist checked before dispatch. Every JSON success/error/404 uses `Cache-Control: no-store`, exact JSON content type, no CORS allow-origin, and stable typed error codes; static assets use the stated safe cache policy.

Add `sovereign-consultant-playground = { workspace = true }` to `apps/cli/Cargo.toml`. Remove its direct `tiny_http` dependency if no other CLI production file uses it. Modify the root workspace and `Cargo.lock` only for this local leaf package/dependency edge; `cargo tree -p sovereign-consultant-playground --edges normal --depth 1` must show exactly the package plus `serde`, `serde_json`, and `tiny_http`, and `cargo update` must not float unrelated packages.

Delete `mod ui`, `apps/cli/src/ui.rs`, and the complete old `apps/cli/assets` tree. Remove `no_open` and every browser-launch behavior. Keep `Ui` to the single built-in `port: u16` field with literal `#[arg(long, default_value_t = 7787)]`; add no `value_parser`, `env`, `flatten`, helper/default function, or other field. Change `main` to immediate `match <Cli as clap::Parser>::parse().command`, not `Cli::parse()`, a wrapper, or a stored `cli` value. Change only the documented UI/verify help, removal of the old flag, explicit parse expression, and direct UI arm in `main.rs`; all other CLI attributes, variants, fields, and command arms remain exact in the checked-in surface fixture. The `Ui` arm directly calls `sovereign_consultant_playground::run(port)?` and has no block/helper/root argument.

Create the calm four-step fixed business thread described above, using an always-visible synthetic-only notice; read-only Company/Offer/Lead-or-Customer/Discovery cards; fixed price, promotion, reporting-search, and reset buttons; deterministic “What needs you?”, “What changed in the example?”, and “What is already recorded?” guidance; and the exact advanced export/disclosure copy. Add no business form or arbitrary input. `consultant-ui.js` uses `textContent`, resolves only the stable semantic-key catalogs and typed placeholders, maps guidance targets through one literal map, and sends only `JSON.stringify({ action })`. `app.js` owns theme/locale/startup, issues only the initial GET, and rerenders cached synthetic data on locale changes.

Render loading, retryable load failure, action-busy, `already_applied`, pre-search empty, defensive post-search no-result, reset confirmation/status, and restart-reset explanation. Map stable API error/field codes through i18n; do not parse backend prose or render persistence/conflict/revision/resume/backup/recovery claims.

- [ ] **Step 5: Run complete focused GREEN and compatibility gates**

```bash
./scripts/run-exact-test-group.sh sovereign-consultant-playground authority_boundary \
  leaf_manifest_dependencies_sources_and_authority_surface_are_exact \
  obvious_indirect_authority_and_code_inclusion_bypasses_are_rejected
./scripts/run-exact-test-group.sh sovereign-consultant-playground tests \
  tests::playground_handler_constructs_without_root_store_or_backend \
  tests::action_request_accepts_only_one_closed_unit_value \
  tests::request_failures_have_stable_codes_independent_of_serde_prose \
  tests::every_playground_json_response_is_typed_and_no_store \
  tests::unauthenticated_route_manifest_is_exactly_static_plus_playground \
  tests::every_legacy_real_or_security_state_route_is_unregistered \
  tests::complete_shipped_asset_graph_uses_only_playground_endpoints_and_existing_elements \
  tests::accessibility_i18n_and_beginner_copy_contract \
  tests::every_used_teaching_key_has_complete_exact_en_and_zh_projections \
  tests::full_locale_projections_translate_facts_search_and_guidance_but_not_literals
./scripts/run-exact-test-group.sh sovereign-cli consultant_playground_cli_ \
  consultant_playground_cli_surface_is_exact_and_main_dispatches_directly \
  consultant_playground_cli_ui_parser_is_builtin_u16_only \
  consultant_playground_cli_legacy_browser_surfaces_are_removed
./scripts/run-exact-test-group.sh sovereign-cli real_ui_process \
  real_ui_process_platform_data_dir_probe_child \
  real_ui_process_is_independent_of_two_distinct_platform_canary_roots
npx -y -p typescript@5.5.4 tsc -p crates/consultant-playground/assets/tsconfig.json
cargo tree -p sovereign-consultant-playground --edges normal --depth 1
./scripts/run-exact-test-group.sh sovereign-cli workspace::tests \
  workspace::tests::full_founder_flow_with_approval_and_evidence \
  workspace::tests::export_contains_state_and_verified_chain \
  workspace::tests::verify_export_accepts_genuine_bundle_and_rejects_tampering \
  workspace::tests::draft_assistant_records_a_persistent_data_disclosure
./scripts/run-exact-test-group.sh sovereign-cli workspace::stage1_suite \
  workspace::stage1_suite::no_host_effect_exists_until_the_owner_approves \
  workspace::stage1_suite::the_authorized_effect_ran_only_on_the_verified_v2_path
cargo test --workspace --all-targets --locked
```

The exact-runner first parses `cargo test -- --list`, asserts the caller's named tests and a nonzero result, and then reruns every parsed test individually with `--exact`; none of these GREEN commands may silently pass on zero tests.

- [ ] **Step 6: Manual smoke and checkpoint B**

```bash
cargo run -p sovereign-cli -- ui --port 7787
```

Repeat the automated process sequence manually against two newly seeded, content-distinct temporary platform-data directories; inspect HTML/CSS/favicon/JS/API network traffic, switch English/Chinese, reset, and restart. Record only observed evidence: complete transcript equality, exactly two API routes, no other network API/resource, no real values in response/stdout/stderr, and no directory byte change. Do not enter real data. This supplements, never replaces, the real-binary test.

- [ ] **Step 7: Commit; push only if explicitly authorized**

```bash
git add Cargo.toml Cargo.lock apps/cli/Cargo.toml apps/cli/src/main.rs \
  apps/cli/tests/consultant_playground_cli_boundary.rs \
  apps/cli/tests/fixtures/expected_consultant_playground_cli_surface.rs.txt \
  apps/cli/tests/consultant_playground_process.rs \
  crates/consultant-playground docs/security/consultant-playground-attack-review.md
git add -u apps/cli/src/ui.rs apps/cli/assets
git commit -m "feat(ui): isolate the synthetic consultant playground leaf"
./scripts/check-file-size.sh
# Authorized implementation sessions may now run:
# git push
```

## Task 6: Run accessibility, bilingual, mobile, and visual acceptance

**Files:**
- Create: `docs/product/consultant-playground-accessibility.md`
- Modify only for genuine defects: `crates/consultant-playground/assets/**`
- Modify only for genuine defects: `crates/consultant-playground/src/tests.rs`
- Modify only for genuine defects: `crates/consultant-playground/tests/authority_boundary.rs`

**Interfaces:** No planned new behavior. This is an expected-GREEN acceptance/defect-discovery gate because Task 5 drove the automated contracts before implementation.

- [ ] **Step 1: Run the already-driven automated acceptance tests**

```bash
./scripts/run-exact-test-group.sh sovereign-consultant-playground tests \
  tests::accessibility_i18n_and_beginner_copy_contract \
  tests::complete_shipped_asset_graph_uses_only_playground_endpoints_and_existing_elements
./scripts/run-exact-test-group.sh sovereign-consultant-playground authority_boundary \
  leaf_manifest_dependencies_sources_and_authority_surface_are_exact \
  obvious_indirect_authority_and_code_inclusion_bypasses_are_rejected
./scripts/run-exact-test-group.sh sovereign-cli consultant_playground_cli_ \
  consultant_playground_cli_surface_is_exact_and_main_dispatches_directly \
  consultant_playground_cli_ui_parser_is_builtin_u16_only \
  consultant_playground_cli_legacy_browser_surfaces_are_removed
./scripts/run-exact-test-group.sh sovereign-cli real_ui_process \
  real_ui_process_platform_data_dir_probe_child \
  real_ui_process_is_independent_of_two_distinct_platform_canary_roots
./scripts/run-exact-test-group.sh sovereign-consultant-playground tests \
  tests::every_used_teaching_key_has_complete_exact_en_and_zh_projections \
  tests::full_locale_projections_translate_facts_search_and_guidance_but_not_literals
npx -y -p typescript@5.5.4 tsc -p crates/consultant-playground/assets/tsconfig.json
```

Expected: GREEN. The helper prints each `--list` result, requires the named tests and a nonzero parsed list, then reruns every listed test with `--exact`; any missing/zero test fails the gate. Do not label this run RED.

- [ ] **Step 2: Create the manual checklist with every observation Target**

Rows start `Target — not run`. Required viewports are 375×667, 667×375, 768×1024, 1024×768, and 1440×900. At each record browser/OS/date/evidence path for English/Chinese, light/dark, keyboard-only, 200% browser zoom, text-only/largest practical text, reduced motion, no horizontal page scroll, no sticky overlap, 44px targets, focus after Continue/action/error/reset/details close, and no affordance that appears to accept/save/export real data. Record a named contrast tool and measured ratios for primary/secondary text, notices, buttons, links, focus rings, and status colors in both themes; targets are 4.5:1 for normal text and 3:1 for large text and non-text focus/control boundaries.

Run one screen-reader pass with VoiceOver, NVDA, or Orca and record the actual tool/version. No conformance claim exists before evidence.

- [ ] **Step 3: Handle only genuine defects with a new focused RED cycle**

For each observed failure, add one test to the owning Task 5 contract when automatable, capture that actual failure, apply the smallest asset fix, rerun focused plus all UI tests, and record Pass/Fail honestly. Do not deliberately leave Task 5 incomplete to manufacture this cycle.

- [ ] **Step 4: Commit evidence/checklist separately**

```bash
git add docs/product/consultant-playground-accessibility.md crates/consultant-playground
git commit -m "test(ui): record consultant playground accessibility gate"
```

If no production defect was found, the commit contains the Target checklist only; it does not claim a pass.

## Task 7: Run the independent attack and future-prerequisite review

**Files:**
- Modify: `docs/security/consultant-playground-attack-review.md`
- Modify only for newly discovered defects: `crates/consultant-playground/src/tests.rs`
- Modify only for newly discovered defects: `crates/consultant-playground/tests/authority_boundary.rs`
- Modify only for newly discovered defects: `apps/cli/tests/consultant_playground_process.rs`
- Modify only for newly discovered defects: `apps/cli/tests/consultant_playground_cli_boundary.rs`

**Interfaces:** Expected-GREEN regression gate. No new product capability, migration, persistence, or fabricated RED.

- [ ] **Step 1: Run the decisive regression matrix**

```bash
./scripts/run-exact-test-group.sh sovereign-consultant-playground tests \
  tests::every_used_teaching_key_has_complete_exact_en_and_zh_projections \
  tests::full_locale_projections_translate_facts_search_and_guidance_but_not_literals \
  tests::graph_and_query_hold_keys_not_human_language_catalog_values \
  tests::unauthenticated_route_manifest_is_exactly_static_plus_playground \
  tests::complete_shipped_asset_graph_uses_only_playground_endpoints_and_existing_elements
./scripts/run-exact-test-group.sh sovereign-consultant-playground authority_boundary \
  leaf_manifest_dependencies_sources_and_authority_surface_are_exact \
  obvious_indirect_authority_and_code_inclusion_bypasses_are_rejected
./scripts/run-exact-test-group.sh sovereign-cli consultant_playground_cli_ \
  consultant_playground_cli_surface_is_exact_and_main_dispatches_directly \
  consultant_playground_cli_ui_parser_is_builtin_u16_only \
  consultant_playground_cli_legacy_browser_surfaces_are_removed
./scripts/run-exact-test-group.sh sovereign-cli real_ui_process \
  real_ui_process_platform_data_dir_probe_child \
  real_ui_process_is_independent_of_two_distinct_platform_canary_roots
./scripts/run-exact-test-group.sh sovereign-cli workspace::tests \
  workspace::tests::full_founder_flow_with_approval_and_evidence \
  workspace::tests::export_contains_state_and_verified_chain \
  workspace::tests::verify_export_accepts_genuine_bundle_and_rejects_tampering \
  workspace::tests::draft_assistant_records_a_persistent_data_disclosure
./scripts/run-exact-test-group.sh sovereign-cli workspace::stage1_suite \
  workspace::stage1_suite::no_host_effect_exists_until_the_owner_approves \
  workspace::stage1_suite::the_authorized_effect_ran_only_on_the_verified_v2_path
```

Expected: GREEN. Each helper call proves nonzero `--list` output, asserts the expected fully-qualified names, and reruns every listed test separately with `--exact`. Do not label this run RED.

- [ ] **Step 2: Review the complete attack surface and record evidence**

Assign this review to someone who did not author Tasks 1–5. For each threat record entry point, denial, exact test, observed result, remaining limitation, and maturity (`Current test evidence`, `Target`, or `Research`):

- every unauthenticated route, including `/api/state`, is in the exact allowlist/denial matrix;
- initial page load plus HTML/CSS/JS/favicon contain only the exact local resource graph and two Playground API literals; CSS imports/URLs, SVG active/external content, unlisted browser network APIs, and external resources are absent;
- every bound DOM ID exists and no stale listener aborts startup;
- action body is exactly one wire enum and extra values fail before domain dispatch;
- the handler has no root/path/Store/Vault/repository parameter;
- the in-process router and two actual `sovereign ui` children pointed at content-distinct OS-specific canary roots keep both trees byte-identical, produce byte-identical complete response transcripts for the same HTTP sequence, and emit no canary in any response/stdout/stderr byte;
- the physical leaf recursively inventories every production/test-only source and asset; every production file passes the same filesystem/environment/path/process/outbound-network/Workspace/Store/Vault/Sovereign/audit/effect/export/model denylist; path overrides, code inclusion, generated code, and unknown macros/attributes fail; the exact injected `super::data_dir().join("vault").exists()` source fails;
- the leaf manifest is `publish = false`, has no build/features/dev surface, and has exactly `serde`, `serde_json`, and `tiny_http` as direct dependencies; the lockfile adds no third-party package;
- the complete checked-in annotated `Cli`/`Commands`/`main` fixture pins every Clap attribute and field; `Ui` is one built-in `u16` with fixed numeric default and no parser/env/flatten/helper hook; `main` begins with immediate `match <Cli as clap::Parser>::parse().command`, has no shared pre-dispatch statement, and its sole UI arm is the direct leaf `run(port)` call;
- the process RED supplies legacy `--no-open` only when old `ui --help` advertises it, preventing external browser launch; final help, CLI source, and deleted `ui.rs` prove both the flag and launcher are gone;
- Graph has no serialization/persistence/export/conversion surface and read DTO is one-way;
- every fact/search/guidance value is a stable semantic key with complete exact English/Chinese compiled projections, matching placeholders, no fallback, and literal-only name/email/money exceptions;
- guidance/search/focus cannot invoke an action or effect;
- hostile test-only read-model strings render through `textContent` and cannot create markup/routes;
- no model, UI, fixture action, or static Security details receives mutation/effect authority;
- current Workspace v1 source and behavior remain untouched;
- release evidence records whether fresh export is required; if it is, cutover is Blocked pending one actual owner-protected export entry point; if it is not, copy says fresh export/disclosure browsing are unavailable and CLI verification accepts only an existing file.

Ad hoc grep is supplementary. The physical leaf dependency boundary, complete recursive production-source/asset manifest, exact annotated CLI/Commands/main fixture, route manifest, uniform authority denial, backend rejection tests, and two-root real-process equivalence are decisive.

- [ ] **Step 3: Verify future integration by dependency, not guessed code**

Inspect actual landed owner-sign-in and protected-storage branches only at implementation time. Record exact conflicts in `main.rs`, the leaf invocation, response headers, route construction, and Security Center ownership. The future real graph plan must require both landed primitives together before any real value is accepted:

```text
authenticated owner authorization
AND selected ActiveV2 protected storage
  → new persistent graph schema/migration/preflight plan
  → authenticated business routes and general forms
```

Do not merge/cherry-pick them here, predeclare their concrete Rust generic shape, add a parallel session/backend, or convert the Playground graph into the persistent graph.

- [ ] **Step 4: Open a focused RED only for a genuinely discovered defect**

If review finds a new in-scope defect, add one behavior test at its owning boundary, capture the failure, apply the minimum fix, and rerun Step 1. If a fix needs owner authentication, protected storage, migration, backup/recovery, a real model, or a new proof primitive, keep the affected capability absent and mark the dependency Blocked.

- [ ] **Step 5: Commit and hold checkpoint C**

```bash
git add docs/security/consultant-playground-attack-review.md \
  crates/consultant-playground apps/cli/tests/consultant_playground_process.rs \
  apps/cli/tests/consultant_playground_cli_boundary.rs
git commit -m "test(security): review consultant playground isolation"
# Push only with explicit authorization after Task 9's full gate.
```

No known Critical/High may be waived. Any failure of seeded isolation, route closure, or root-free construction blocks the Playground.

## Task 8: Define and run the five-consultant usability protocol

**Files:**
- Create: `docs/product/consultant-playground-usability.md`
- Create only after sessions run: `docs/product/evidence/consultant-playground-2026-08/README.md`

**Interfaces:** Documentation/evidence only. No telemetry or hidden recording. This task has no RED/GREEN fiction; all unrun results remain Target.

- [ ] **Step 1: Write the protocol with plain facilitator copy**

Recruitment target: five independent consultants who manage their own leads/client work, including at least two non-developers and at least one primarily Simplified-Chinese participant. Do not use project contributors for the primary five. Obtain consent before notes/recording and collect no real customer/business values.

Use the participant's preferred-language prompt verbatim:

```text
This is a practice example. Do not enter or substitute any real business or
customer information. North Star Operations offers a “Reporting clarity
sprint.” Acme Ltd is the example lead. Review the four facts, correct the
example price, promote the example lead to customer, use the provided reporting
search, and explain the next step shown by the home guidance. Then reset the
example and explain what this preview can and cannot do with your own data.
```

```text
这是一个练习示例。请不要输入或替换任何真实的业务或客户信息。
North Star Operations 提供“报表清晰度冲刺”服务。Acme Ltd 是示例潜在客户。
请查看四项信息，修改示例价格，将示例潜在客户升级为客户，使用预设的“报表”
搜索，并说明首页指引显示的下一步。然后重置示例，并说明这个预览版可以和
不可以对你的数据做什么。
```

- [ ] **Step 2: Observe the exact tasks without coaching**

1. Identify Company, Offer, Lead, and Discovery in the business thread.
2. Distinguish the reusable Offer record from a proposal document described in help copy.
3. Correct the fixed example price.
4. Promote the fixed example Lead to Customer.
5. Find the Discovery with the provided reporting search.
6. Switch English↔Simplified Chinese after search and after one state change; confirm facts, status, search hits, and guidance all change language while names, email, and money remain the same.
7. Follow a guidance link and recognize that it only navigates.
8. Reset, restart, and recognize that progress is not saved.
9. Explain that this preview has no control for their own customer data and cannot be activated by accepting a warning.
10. Find optional advanced details only if asked where technical limits live; naming internal programs is not a success requirement.

Record completion, time, wrong turns, assistance count, consented quote, device/accessibility setup, attempted real-data entry, and security-boundary explanation. If a participant begins to disclose real data, stop them and record only that the guardrail failed—not the value.

- [ ] **Step 3: Declare thresholds before sessions**

All remain Targets until observed:

- at least 4/5 complete tasks 1–8 without Security Center or engineering terminology;
- median unassisted walkthrough time ≤8 minutes;
- all 5 understand guidance is navigation, not automatic work;
- all 5 observe complete locale switching for facts/search/guidance, with zero unexplained English fallback other than the declared name/email/money literals;
- at least 4/5 distinguish Offer from proposal document;
- all 5 understand the example resets and does not save their business;
- all 5 understand real-data controls are absent and warning acceptance cannot enable them;
- zero participant enters real customer/business data;
- zero observed keyboard trap, clipped primary action, unexplained state, or horizontal page scroll.

If fewer than five sessions run, status is `Target — incomplete sample`. Never average missing values or count coached completion as unassisted.

- [ ] **Step 4: Preserve evidence honestly**

The initial table is:

| Evidence | Status | Result |
| --- | --- | --- |
| Participants recruited | Target | Not run |
| Four-fact comprehension | Target | Not run |
| Fixed actions/search/reset | Target | Not run |
| Complete fact/search/guidance locale projection | Target | Not run |
| Guidance non-authority | Target | Not run |
| Real-data guardrail comprehension | Target | Not run |
| Accessibility observations | Target | Not run |

Store only redacted notes, build commit, environment, and consent/retention statement. Do not store participant contacts in the repository.

- [ ] **Step 5: Commit the protocol separately**

```bash
git add docs/product/consultant-playground-usability.md
git commit -m "docs(product): define consultant playground usability evidence"
```

Do not create or commit an evidence directory until sessions produce real redacted evidence.

## Task 9: Final compatibility, product-balance, and handoff gate

**Files:**
- Modify only if observed facts changed: `README.md`
- Modify only if observed facts changed: `ROADMAP.md`
- Modify only if observed facts changed: `ARCHITECTURE.md`
- Modify: `docs/product/consultant-playground-accessibility.md`
- Modify: `docs/product/consultant-playground-usability.md`
- Modify: `docs/security/consultant-playground-attack-review.md`

- [ ] **Step 1: Run focused behavior gates and reject zero tests**

```bash
./scripts/run-exact-test-group.sh sovereign-consultant-playground tests \
  tests::consultant_fixture_is_exact_complete_and_deterministic \
  tests::fixed_actions_change_only_the_two_documented_business_fields \
  tests::every_used_teaching_key_has_complete_exact_en_and_zh_projections \
  tests::full_locale_projections_translate_facts_search_and_guidance_but_not_literals \
  tests::graph_and_query_hold_keys_not_human_language_catalog_values \
  tests::playground_handler_constructs_without_root_store_or_backend \
  tests::request_failures_have_stable_codes_independent_of_serde_prose \
  tests::unauthenticated_route_manifest_is_exactly_static_plus_playground \
  tests::complete_shipped_asset_graph_uses_only_playground_endpoints_and_existing_elements \
  tests::accessibility_i18n_and_beginner_copy_contract
./scripts/run-exact-test-group.sh sovereign-consultant-playground authority_boundary \
  leaf_manifest_dependencies_sources_and_authority_surface_are_exact \
  obvious_indirect_authority_and_code_inclusion_bypasses_are_rejected
./scripts/run-exact-test-group.sh sovereign-cli consultant_playground_cli_ \
  consultant_playground_cli_surface_is_exact_and_main_dispatches_directly \
  consultant_playground_cli_ui_parser_is_builtin_u16_only \
  consultant_playground_cli_legacy_browser_surfaces_are_removed
./scripts/run-exact-test-group.sh sovereign-cli real_ui_process \
  real_ui_process_platform_data_dir_probe_child \
  real_ui_process_is_independent_of_two_distinct_platform_canary_roots
npx -y -p typescript@5.5.4 tsc -p crates/consultant-playground/assets/tsconfig.json
```

Every helper invocation must print its `--list` output, parse at least one `: test` line, assert every expected fully-qualified name exactly, and rerun every parsed name with `--exact`; a cargo exit code of zero with zero parsed tests fails this step.

- [ ] **Step 2: Prove persistent v1 is untouched and dependency changes equal the one leaf edge**

```bash
test -z "$(git diff --name-only origin/main...HEAD -- apps/cli/src/workspace)"
test -z "$(git diff --name-only origin/main...HEAD -- crates | rg -v '^crates/consultant-playground/')"
test "$(git diff --name-only origin/main...HEAD -- ':(glob)**/Cargo.toml' Cargo.lock | sort)" = \
  $'Cargo.lock\nCargo.toml\napps/cli/Cargo.toml\ncrates/consultant-playground/Cargo.toml'
git diff --word-diff=plain origin/main...HEAD -- Cargo.toml apps/cli/Cargo.toml Cargo.lock \
  crates/consultant-playground/Cargo.toml
cargo tree -p sovereign-consultant-playground --edges normal --depth 1
./scripts/run-exact-test-group.sh sovereign-cli workspace::tests \
  workspace::tests::full_founder_flow_with_approval_and_evidence \
  workspace::tests::export_contains_state_and_verified_chain \
  workspace::tests::verify_export_accepts_genuine_bundle_and_rejects_tampering \
  workspace::tests::draft_assistant_records_a_persistent_data_disclosure
./scripts/run-exact-test-group.sh sovereign-cli workspace::stage1_suite \
  workspace::stage1_suite::no_host_effect_exists_until_the_owner_approves \
  workspace::stage1_suite::the_authorized_effect_ran_only_on_the_verified_v2_path
```

If the branch started from a base other than `origin/main`, record the starting SHA before implementation and substitute that exact SHA in all diff commands. Review the manifest/lock word diff: the root adds only the member/workspace dependency, the CLI adds only the leaf dependency and removes now-unused direct `tiny_http`, the leaf has exactly its three permitted dependencies, and the lock adds only the local package edge with no new registry package. Do not allowlist any other Workspace/schema/dependency change into this slice.

- [ ] **Step 3: Run the full repository gate**

```bash
./scripts/check-file-size.sh
bash -n scripts/run-exact-test-group.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build -p sovereign-cli --release --locked
npx -y -p typescript@5.5.4 tsc -p crates/consultant-playground/assets/tsconfig.json
git diff --check
git status --short
```

Record observed counts/output and commit SHA. Do not reuse a historical test count.

- [ ] **Step 4: Run the final self-review with file/test evidence**

- **User value:** Can a beginner understand four fixed facts, complete two fixed changes, use compiled search, and interpret guidance without infrastructure terminology?
- **Synthetic boundary:** Is `ConsultantPlaygroundGraph` non-serializable/non-persistable and unrelated to Workspace, with only a one-way read DTO?
- **Persistent compatibility:** Are Workspace v1 source, bytes, writers, reporting, export, verifier, and tests untouched, with manifest/lock drift limited to the one physical leaf edge and its three already-present libraries?
- **Browser isolation:** Is the compiled route manifest exact, `/api/state` absent, handler root-free, full shipped asset graph closed, and seeded real data/files unchanged?
- **Production authority closure:** Is all Playground production code physically inside the `publish = false` leaf; does recursive inventory classify every leaf source/asset; does every production source pass the same fs/env/path/process/outbound-network/Workspace/Store/Vault/Sovereign authority denylist; do path/code inclusion and unknown macros/attributes fail; and does the injected `super::data_dir().join("vault").exists()` source fail?
- **CLI parse/dispatch closure:** Does the exact annotated `Cli`/complete `Commands`/`main` fixture pin every Clap attribute; restrict `Ui` to built-in `u16` plus fixed numeric default with no value parser/env/flatten/helper; require immediate `<Cli as clap::Parser>::parse()`; forbid shared pre-dispatch work; and retain one direct leaf UI arm?
- **RED side-effect safety:** Did the baseline process probe append old `--no-open` before starting the legacy server, and do final help/source/path tests prove both that flag and every browser launcher are removed?
- **Real-process isolation:** Did two content-distinct OS-specific canary roots remain byte-identical while actual `sovereign ui` children returned byte-identical complete response transcripts to the same full HTTP sequence and emitted no canary on stdout/stderr?
- **Network boundary:** Are HTML/CSS/JS/SVG references closed, browser network APIs limited to two same-origin fetches, browser launch absent, the leaf's only server bind literal loopback, client/source dependencies denied, and manifest/lock changes exactly the reviewed leaf edge?
- **No direct authority:** Can UI/model/search/guidance do no more than request the four compiled actions; are effect/audit/export/persistence capabilities absent?
- **Semantic localization:** Does every graph fact, status, search hit, and guidance item use a stable key with exact complete English/Chinese catalogs and placeholder parity, while only names/email/money remain literal?
- **Claims/copy:** Does primary bilingual copy say example-only/no own-data save in plain language, with internal program names advanced-only and no safety overclaim?
- **Errors/cache:** Are action errors stable typed codes and all JSON/404 paths no-store?
- **Accessibility/i18n/mobile:** Are automated tests green and manual rows either evidenced or Target?
- **Usability:** Are all five-person results Target until actually run, and do thresholds measure understanding rather than jargon recall?
- **Future gate:** Does every real-data form/schema/migration/export-version claim remain Blocked on actual owner authorization plus ActiveV2 protected storage?
- **Export cutover:** Is the dated release decision present? If fresh export is required, is Task 5 still Blocked? Otherwise, do all docs/copy say disclosure browsing and fresh export creation are unavailable and CLI only verifies an existing export file?

- [ ] **Step 5: Update product docs only to observed synthetic maturity**

The strongest allowed Current line is: “Synthetic consultant Playground: one fixed in-memory Company/Offer/Lead/Discovery example, two fixed actions, compiled bilingual search/facts, and read-only guidance. It cannot accept, save, inspect, verify, or export your business data.” Add: “After browser cutover, creating a fresh real-data export and browsing disclosures are unavailable. The CLI can verify an export file you already have; it cannot create one.” Do not claim Current Community structured state or v0.3 completion. If fresh export is a release condition, make no Current update because cutover is Blocked pending the single owner-protected entry point. Keep owner sign-in, protected persistent graph, migration, real-data forms, proposal workflow expansion, real model, and recovery at their actual Target/Blocked maturity.

- [ ] **Step 6: Run the final consistency scan**

```bash
rg -n 'WorkspaceV2|Workspace v2|migrate_v1|workspace.migrate|ValidatedWorkspace|SelectedBusinessRepository|syn =|workspace_writer_boundary|ExportCoverage::current_v2' \
  docs/superpowers/plans/2026-08-14-consultant-onboarding-minimal-graph-v1-implementation.md \
  .superpowers/consultant-plan-report.md
rg -n '1C0|ActiveV2|AuthorizedStore|Vault v2|repository|migration' crates/consultant-playground/assets
rg -n '/api/(state|command-center|workspace|export|verify-export|gauntlet)' \
  crates/consultant-playground/src crates/consultant-playground/assets
rg -n 'Store|Workspace|Vault|Path|std::fs|serde|Serialize|Deserialize|export|save|load' \
  crates/consultant-playground/src/graph.rs crates/consultant-playground/src/actions.rs
rg -n '#\[path|include!\(|include_bytes!\(|macro_rules!|std::fs|std::env|std::path|std::process|std::net|data_dir|workspace' \
  crates/consultant-playground/src -g '!tests.rs'
rg -n 'XMLHttpRequest|WebSocket|EventSource|sendBeacon|WebTransport|new Worker|SharedWorker|@import|url\(' \
  crates/consultant-playground/assets
rg -n 'Operations reporting consulting|Reporting clarity sprint|Weekly reporting takes six hours|Finance must approve|30-minute scoping call|运营报表咨询|每周报表需要六小时' \
  crates/consultant-playground/src/graph.rs crates/consultant-playground/src/query.rs
rg -n 'CLI (can|still|continues to) (create|browse)|CLI (可以|仍可)(创建|浏览)' \
  README.md ROADMAP.md ARCHITECTURE.md crates/consultant-playground/assets docs/product docs/security
test -z "$(find apps/cli/src apps/cli/assets -type f 2>/dev/null | rg 'consultant_playground|/http/|/ui\.rs$|apps/cli/assets')"
```

The first scan should find only this explicit forbidden-term command or future-boundary discussion; classify and remove any current-slice persistent task. Internal program names may appear in assets only inside advanced details. Route, leaf-authority, code-inclusion/macro, forbidden network-API, and raw-domain-language scans must return no production match. The last scan must find no positive claim that CLI creates exports or browses disclosures; required negative copy is reviewed separately. Task 3's catalogs and Task 5's physical leaf manifest/dependency gate, exact annotated CLI/Commands/main fixture, all-asset graph, two-root process test, and exact-test runner are decisive; grep is supplementary.

- [ ] **Step 7: Final commit/reviews and conditional push checkpoint D**

```bash
git add README.md ROADMAP.md ARCHITECTURE.md \
  docs/product/consultant-playground-accessibility.md \
  docs/product/consultant-playground-usability.md \
  docs/security/consultant-playground-attack-review.md
git commit -m "docs(product): report consultant playground boundary and evidence"
git log --oneline origin/main..HEAD
git diff --stat origin/main...HEAD
# Push only with explicit implementation-session authorization:
# git push
```

Request one product/spec review and one security/code-quality review. Fix every in-scope Critical/High, rerun the full gate, and hand off: synthetic user outcome; exact browser isolation evidence; unchanged v1 compatibility; usability/accessibility maturity; commit/branch; limitations; and future owner-sign-in/protected-storage dependency.

## Plan Self-Review

- **User value:** One concrete practice scenario teaches the minimum independent-consultant vocabulary and decisions without asking for real data or security expertise.
- **Schema/scope:** The only new graph is a private four-record teaching value. It adds no persistent schema, migration, repository, generic graph, ERP module, or frontend stack.
- **Persistent compatibility:** Workspace v1 source/bytes/behavior, internal export generation, and the CLI existing-file verifier remain untouched; final path/dependency diffs make that a release gate. This is not a claim that the CLI can create a fresh export.
- **Authority:** The graph is non-serializable/non-persistable; the HTTP handler owns only a process-local session; the UI sends only a unit action; search/guidance/focus are pure.
- **Browser security:** `/api/state` and every real/state/assistant/export/verify/wildcard route are absent. The physical leaf, complete recursive leaf source/asset manifest, exact annotated CLI/Commands/main parser/dispatcher, uniform authority denylist, complete HTML/CSS/JS/SVG graph, root-free constructor, and two-root actual-binary transcript/stdout/stderr/directory equivalence test are decisive.
- **TDD honesty:** Fixture type lands before action tests. New behavior Tasks 1–5 have genuine REDs. Every GREEN group parses `--list`, requires expected names/nonzero, and reruns every listed test with `--exact`; acceptance, attack, usability, and final tasks open RED only for observed defects.
- **Claims/copy:** Primary bilingual copy says example-only/no own-data entry or save. All teaching facts/search/guidance are stable-keyed with complete compiled English/Chinese projections. Internal release names are optional advanced details, not participant success criteria.
- **Export/audit honesty:** Playground has no export/audit. The internal v1 plaintext export format and verifier remain unchanged, but after cutover no user-facing operation creates a fresh export and disclosures cannot be browsed. CLI verifies only an already-existing export. A release that requires fresh export blocks cutover until one owner-protected entry point exists.
- **Accessibility/i18n/mobile:** Automated contracts are driven before UI implementation; manual results remain Target until recorded across five viewports and a named screen reader.
- **Usability evidence:** Target is five independent consultants; success measures understanding and safe behavior, never memorization of program/type names. No result is invented.
- **Future compatibility:** Persistent graph/migration/forms are a separate plan based on actual landed owner authorization and ActiveV2 protected storage, never a conversion of this teaching graph.
