# S2 business experiment contract

**Revision 1 · Design candidate; implementation admission requires independent S2-00 acceptance.**
This is the sole S2 behavior/interface specification. Cards assign work and tests;
they do not duplicate these definitions. Stage status lives in [backlog](../../backlog.md).
The complete goal remains [models-and-goals §13.1](models-and-goals.md).

## 1. Surface, boundary and maturity

Add `sovereign business-demo --port 7789` (port 0 permitted), dispatched directly
to `business_demo::run(port)`. No root, import, provider, open-browser or persistence
argument. Startup prints `Playground: http://127.0.0.1:<actual-port>` and
`Business demo — synthetic exercise; memory only; no AI or external business actions.`.
The shared first-line protocol intentionally reuses the existing ChildServer
launcher; command, port and second line identify this separate S2 experiment. Ctrl-C
stops it. One process owns one shared exercise; tabs share that exercise.

Implementation lives in `apps/cli/src/business_demo.rs` and its explicitly
registered child modules; embedded assets live in `apps/cli/business-demo/`.
It is a separate command and HTTP server, not an additional legacy UI route.
`Commands::Playground` and `Commands::Ui` retain their existing arms. The CLI
must not call `data_dir`, `Store::open`, identity/Vault setup or legacy `ui::run`
on this command path. No S1 file, fixture, dependency or invariant changes.

```text
CLI business-demo -> business_demo::run -> tiny_http loopback server
                                     -> DemoState::apply -> case/evidence/
                                        proposal/review/delivery/billing/followup
                                     -> cloned DemoView -> embedded browser UI
business money parsing -------------> workspace::parse_amount_cents (pure only)
```

Approved runtime dependencies are already CLI normal dependencies: std,
serde/serde_json, tiny_http 0.12, rand and hex. Use existing locked selections and
features; no manifest/lockfile change is needed. No dependency on S1 internals.
The only allowed Workspace call is its public pure `parse_amount_cents`.
No calls to model, workflow execution, authority issuance, effects, vault,
filesystem, environment, process execution, arbitrary network client or clock
for business facts. OS entropy for instance identifiers and the listener,
HTTP request/response and startup/error IO are the declared runtime IO.

This is core-reviewed deterministic experiment code, not an OS sandbox or
authenticated owner surface. Editable text cannot be proven synthetic: seed
records are `seeded_synthetic`; all manual entries are `unverified_experiment_input`.
UI copy says to use invented data and never displays a protected/verified-data
badge. No persistence, browser storage, cookies, clipboard/export/download,
file import, remote assets or outbound requests. The design deliberately does
not bring legacy keys or real data into an unauthenticated command. Other local
processes can read/change this exercise; loopback/origin checks are not owner
authentication. Real data enablement remains subject to RFC 0004/0005 and S4.

tiny_http supplies framing/connection handling. S2 does not claim S1's exact
transport parser, source grammar, connection count or deadlines. Slow local
clients can impair this preview; this limitation must remain visible in the
handoff. It is not grounds for weakening any S1 test. Responses and inputs are
bounded at the application layer as below. No diagnostic logs include bodies.

Simulation decisions and history are process-local learning records, not
signed owner approvals, durable audit evidence, customer commitments or
financial/legal execution. Reset/restart is fixture recreation, not recovery.

## 2. Wire conventions and limits

JSON UTF-8 uses snake_case names and ordinary serde serialization. Every input
struct rejects unknown fields (`deny_unknown_fields`); no arbitrary JSON patch,
URL, filename, HTML or caller-supplied object ID creation. All JSON objects
listed below have exactly those fields. Optional values serialize as `null`,
not absent. Sets use duplicate-free arrays in insertion order. IDs are strings;
no UUIDs or business timestamps. Enums serialize to the literal names below.
Amounts/revisions/indices are integers within JavaScript's exact integer range.
Every enum except Currency uses serde rename_all="snake_case"; Currency's sole
output value has explicit serde rename="SGD". All input `currency` fields are
String, validated to SGD in domain code, so unsupported currency returns its
specified domain error rather than a JSON enum parse error. Type declarations
below omit String on human text and Id on identifiers; numeric/enum/container
types are stated explicitly. Proposal/review version references are u32.

Common aliases (wire types; corresponding Rust representations fixed here):

```text
Id = String                         // server-generated kinds below
Epoch = String                      // 32 lowercase hex chars, 16 random bytes
Revision = u32                      // 0..10_000
Day = u16                           // 0..365, relative to this exercise
Cents = u64                         // 0..100_000_000 per monetary object
Currency = enum { Sgd }              // wire "SGD" only
OriginKind = SeededSynthetic | UnverifiedExperimentInput
```

All human text is trimmed; required short fields are 1..120 UTF-8 bytes;
optional short fields 0..120. Long fields are 1..2,000 UTF-8 bytes, optionally
empty only when specified. Reject control characters except newline in long
fields; tabs/CR/NUL are rejected. No normalization beyond trimming. Do not
interpret HTML/Markdown, email addresses or embedded links. Contact has a
display name only, no real email/phone fields or sending destination.

Limits: 32 evidence records, 32 discovery assertions, 16 clarifications,
16 proposal versions, 32 review records, 16 engagements, 8 milestones and
16 tasks per engagement, 16 invoice drafts, 32 payments, 16 followups, and
10,000 successful mutations per reset. Reject at limit without changing state.
HTTP body <=32,768 actual bytes; serialized view <=1,048,576 bytes. All creation
and mutation paths check limits before commit. No silent truncation/eviction.

## 3. State and exact interfaces

All domain types, fields and functions are `pub(super)` inside the experiment
module tree, except the CLI entry `pub(crate) fn run(port: u16) ->
Result<(), Box<dyn std::error::Error>>`. None become public product contracts.
Owned structs derive `Clone`, `Debug`, `PartialEq`, serde `Serialize` where
returned, and `Deserialize` only for inputs. Avoid hand-written JSON encoders.
Do not change signed contract serialization or canonicalize signed bodies.

`DemoState` lives in `business_demo.rs`, with these fields accumulated only
when their owning implementation card lands:

```text
epoch: Epoch, revision: Revision, demo_day: Day,
case: Case,
evidence: Vec<Evidence>, clarifications: Vec<Clarification>,
proposals: Vec<ProposalVersion>, reviews: Vec<ReviewRecord>,
engagements: Vec<Engagement>, invoices: Vec<InvoiceDraft>,
payments: Vec<PaymentRecord>, followups: Vec<Followup>,
history: Vec<HistoryEntry>
```

`DemoView` is a detached clone with all the same fields plus:
`maturity: "synthetic_experiment"`, `roles: Vec<RoleAvailability>`,
`legal: LegalAvailability`, `summary: Summary` and
`available_commands: Vec<String>`. No reference into mutable state is returned.
Before a feature card lands its fields/commands are absent, rather than fake
success stubs; once introduced their fields obey this contract. `GET` never
mutates, fills in missing objects or changes IDs.

Exact root methods:

```rust
impl DemoState {
    pub(super) fn seeded(epoch: String) -> Self;
    pub(super) fn view(&self) -> DemoView;
    pub(super) fn apply(&mut self, request: CommandRequest)
        -> Result<DemoView, DemoError>;
}
pub(super) fn new_epoch() -> String;
pub(super) fn handle(state: &mut DemoState, request: &mut tiny_http::Request,
    bound_port: u16) -> tiny_http::Response<std::io::Cursor<Vec<u8>>>;
```

`new_epoch` uses `hex::encode(rand::random::<[u8; 16]>())`; generated only at
startup/reset. `seeded(epoch)` is pure and allows exact deterministic fixtures.
Epoch prevents cross-restart/reset stale actions; it is not an authorization
token. `revision` starts at 0 and increments once per successful mutation,
including an identical-value save. Reset returns a new epoch/revision 0 and
erases all prior exercise records. A new epoch must differ from the old one; reset may draw at most4 times,
then returns internal_error unchanged. No unbounded retry loop.

`CommandRequest { epoch: Epoch, expected_revision: Revision, command: Command }`.
`Command` uses `#[serde(tag="type", content="data", rename_all="snake_case")]`;
every variant's `data` is a named input struct, including `Reset {}`.
`apply` first checks epoch then revision; clone state, validate/apply command,
append history and enforce view-size/record limits, then replace original. Response-size overflow is limit_reached.
Any error leaves all state byte-for-byte unchanged. Reset validates the old
epoch/revision before generating a replacement. No automatic request replay.
History sequence is the new revision (old+1). action is the root command's
snake_case name. subject_id is the created/edited object's ID (company-1 for
save_company, case-1 for save_case, null for set_day); proposal_version is the
affected proposal's number for proposal/review/engagement/invoice commands,
otherwise null. Payment subject is its new payment ID and proposal_version null.
Reset produces no retained history record. No-op saves still consume a revision.

For features, child-module entry signatures are fixed:

```rust
pub(super) fn save_company(state: &mut DemoState, input: CompanyInput) -> Result<(), DemoError>;
pub(super) fn save_case(state: &mut DemoState, input: CaseInput) -> Result<(), DemoError>;
pub(super) fn apply_evidence(state: &mut DemoState, command: EvidenceCommand) -> Result<(), DemoError>;
pub(super) fn create_proposal(state: &mut DemoState, input: ProposalInput) -> Result<(), DemoError>;
pub(super) fn apply_review(state: &mut DemoState, command: ReviewCommand) -> Result<(), DemoError>;
pub(super) fn apply_delivery(state: &mut DemoState, command: DeliveryCommand) -> Result<(), DemoError>;
pub(super) fn apply_billing(state: &mut DemoState, command: BillingCommand) -> Result<(), DemoError>;
pub(super) fn apply_followup(state: &mut DemoState, command: FollowupCommand) -> Result<(), DemoError>;
```

Root command variants wrap the inputs below without a second nested command
tag. Root matches them into child enums with the same variant payloads. Child
commands mutate only the cloned candidate supplied by `apply`; no child
serializes responses, appends global history, allocates epochs or performs IO.

`DemoError { code: ErrorCode, field: Option<String> }`; no submitted input or
provider/native error appears in its wire form. Error codes are
`invalid_input`, `unknown_reference`, `invalid_transition`, `stale_epoch`,
`stale_revision`, `stale_business_input`, `limit_reached`, `unsupported_currency`,
`internal_error` (500).
Field paths are fixed source-defined strings, never reflected user text.

## 4. Company, case, evidence and seed

There is exactly one editable company, one service, one organization/contact,
one opportunity and one discovery. This bounded S2 case is not multi-company
CRM. UI uses plural navigation labels only where they lead to the single case
list, with an explicit "one practice case" subtitle.

```text
Company { id, name, goal, origin }
Service { id, company_id, name, scope, currency, unit_price_cents, origin }
Relationship { id, organization, contact_name, stage: "lead"|"customer", origin }
Opportunity { id, relationship_id, service_id, title, next_step }
Discovery { id, opportunity_id, problem, budget_min_cents, budget_max_cents,
            procurement_constraint, assertions: Vec<Assertion> }
Case { id, input_revision: Revision, company: Company, service: Service,
       relationship: Relationship, opportunity: Opportunity,
       discovery: Discovery, jurisdictions: JurisdictionFacts }
CompanyInput { name, goal, service_name, service_scope, currency, unit_price: String }
CaseInput { organization, contact_name, relationship_stage, opportunity_title,
            next_step, problem, budget_min: String, budget_max: String,
            procurement_constraint, jurisdictions: JurisdictionFacts }
```

`goal`, `scope`, `next_step`, `problem`, `procurement_constraint` are long;
procurement_constraint may be empty. All other text is required short.
Money strings reuse `workspace::parse_amount_cents`; service price must be
positive; budget endpoints may be zero but minimum <= maximum. SGD is required.
Save is complete replacement of that input's fields, not a patch. Company/case
save sets their edited record origins to unverified and increments
`case.input_revision` once; it does not alter immutable proposal snapshots.
Lead→customer is a founder-entered practice classification only. It does not
accept a proposal, confirm demand, create an engagement or record income.

Server IDs at seed: `case-1`, `company-1`, `service-1`, `relationship-1`,
`opportunity-1`, `discovery-1`. New IDs are kind plus vector length+1, allocated
after checks. No deletes/reordering; reset starts again under a new epoch.

Seed: North Star Operations; goal "Make weekly reporting easier to run";
Reporting clarity sprint; scope "Assess weekly reporting and produce an improvement plan";
SGD 2,500.00; Acme / Alex Chen; lead; opportunity "Weekly reporting improvement";
next step "Arrange a 30 minute scoping call"; problem "Weekly reporting takes six hours";
budget SGD 3,000.00–5,000.00; constraint "Finance approval is still needed".
All text is explicitly seeded fiction. These are S2 values, not changes to S1.

S2-01A1 contains only company/service and its save/reset operations. S2-01A2
introduces remaining Case fields with this seed and the editable case page.

```text
Evidence { id, label, excerpt, origin }
Assertion { id, kind: "evidence_backed"|"assumption", text, evidence_ids: Vec<Id> }
Clarification { id, question, status: "open"|"answered_in_exercise",
                answer: Option<String>, evidence_id: Option<Id> }
EvidenceInput { label, excerpt }
AssertionInput { kind, text, evidence_ids }
ClarificationInput { question }
AnswerClarificationInput { clarification_id, answer, evidence_id }
```

`label` is short; excerpt/text/question/answer are long. Commands:
`add_evidence`, `add_assertion`, `add_clarification`, `answer_clarification`.
All evidence and assertions are append-only; answer allowed once only, with
an existing evidence ID. Evidence-backed assertions require 1..8 existing IDs;
assumptions require zero. These are support links, not verification that a
customer said or confirmed anything. Clarification answer is explicitly a
manual exercise answer. Evidence/assertion/clarification changes each advance
`case.input_revision`; proposals clone relevant evidence and clarifications.

S2-01B seed adds evidence-1 label "Fictional discovery note" with excerpt
"Weekly reporting takes six hours; budget is SGD 3,000–5,000; finance approval is needed."
One evidence-backed assertion "Reporting consumes six hours each week" references
evidence-1; one assumption "A reporting template may reduce repeat work" has no
references; clarification-1 "Who gives finance approval?" remains open.

## 5. Proposal versions and deterministic money

```text
ProposalLineInput { description, quantity: u32, unit_price: String }
ProposalLine { description, quantity: u32, unit_price_cents: Cents, total_cents: Cents }
ProposalInput { previous_version: Option<u32>, title, scope, assumptions,
                currency, lines: Vec<ProposalLineInput>, valid_through_day: Day }
ProposalSnapshot { case: Case, evidence: Vec<Evidence>, clarifications: Vec<Clarification> }
ProposalVersion { id, version: u32, previous_version: Option<u32>,
                  input_revision: Revision, snapshot: ProposalSnapshot,
                  title, scope, assumptions, currency, lines: Vec<ProposalLine>,
                  total_cents, valid_through_day }
```

Command `create_proposal`. First previous_version must be null; thereafter it
must equal the latest version. Versions are 1..16; IDs `proposal-1` etc.
Immutable once created; editing any title/scope/assumption/price/quantity/validity
creates a new version. Server clones the current case/evidence/clarifications;
clients cannot supply a snapshot, total, status or reviewed flag. `title` short;
scope long required; assumptions long may be empty; 1..8 lines, description
required short, quantity 1..100, positive unit price. Multiply checked integers,
then checked sum <=100,000,000 cents. No rounding is performed: reject decimal
precision greater than the existing parser permits. No discounts, percentages,
tax calculation or currency conversion; display "SGD, tax not calculated".
Validity day >= demo_day. UI defaults use current service fields; ordinary
deterministic prefill is labeled "From your exercise service", never AI output.

Create is allowed after return/reject/accept. It rejects while the latest review
state is submitted AND that proposal is current and unexpired. If inputs change
or validity expires during submission, allow a successor to supersede that stale
submission; its historical submitted record grants nothing. This prevents an
unreviewable submitted version from blocking all rework. Draft revision must match the latest predecessor. New
proposal's review state is always draft. Old records remain inspectable.
Currentness is `proposal.input_revision == case.input_revision` and the version
is latest. A change to inputs makes earlier currentness false even if the user
re-enters previous text; old approval never follows changed input/version.

## 6. Simulated review and rework

```text
ReviewRecord { id, proposal_version: u32,
               outcome: "submitted"|"returned"|"rejected"|"accepted",
               note, actor: "founder_simulation", input_revision: Revision }
SubmitReviewInput { proposal_version }
DecideReviewInput { proposal_version, outcome: "returned"|"rejected"|"accepted", note }
```

Commands `submit_review`, `decide_review`. Review state is derived from the
last record for that version, otherwise draft. Only the latest, current,
unexpired proposal can be submitted from draft, and decided from submitted.
Submit note is empty; decision note is long, required for returned/rejected,
optional for accepted. A decided version cannot be resubmitted: revise it to
create a new version. Reject stale version/input/day and duplicate decisions
at the backend; no UI-only safeguard. Accepted is "Accepted in exercise".
It does not use legacy Approval, TypedSigner, AuthorityStore or an owner key.

Review page shows exact version, previous-version field/line differences,
amount/currency, snapshot evidence and assumptions, currentness and status.
Return preloads the latest version into the editable quotation form. Save
creates its successor, which requires submit and a new decision. Canceling
an edit does not change state. No hidden auto-accept or auto-rework.

## 7. Delivery plan and task results

```text
MilestoneInput { title, due_day, acceptance_criteria }
TaskInput { title, milestone_index: u32 }
EngagementInput { proposal_version, milestones: Vec<MilestoneInput>, tasks: Vec<TaskInput> }
Milestone { id, title, due_day, acceptance_criteria,
            status: "planned"|"acceptance_draft", result_note: Option<String> }
DeliveryTask { id, milestone_id, title, status: "todo"|"done_in_exercise",
               result_note: Option<String> }
Engagement { id, proposal_version, milestones: Vec<Milestone>, tasks: Vec<DeliveryTask> }
CompleteTaskInput { engagement_id, task_id, result_note }
DraftAcceptanceInput { engagement_id, milestone_id, result_note }
```

Commands `create_engagement`, `complete_task`, `draft_acceptance`.
Create requires latest/current/unexpired accepted proposal and no engagement
already bound to it. One to eight milestones, one to sixteen tasks, each
milestone has at least one task. Task input index is zero-based into submitted
milestones; server creates `engagement-N-milestone-M` and `engagement-N-task-T`.
Dates >= demo_day; title short, criteria/result note long. Results require an
explicit manual note; "done in exercise" does not assert real work happened.
Draft acceptance requires all milestone tasks done and a result note; it never
claims client acceptance. Duplicate completion/draft rejects.

Only the latest engagement tied to the latest/current accepted proposal can
receive task/acceptance mutations. Historical engagements remain visible as
superseded. Editing scope means new proposal + fresh review + explicit new
engagement; never retarget old milestones. No auto-completion or task scheduler.

## 8. Invoice drafts, receivables and simulated receipts

```text
InvoiceInput { engagement_id, milestone_id, currency, amount: String, due_day }
InvoiceDraft { id, engagement_id, milestone_id, proposal_version, currency,
               amount_cents, due_day, status: "draft"|"issued_in_exercise",
               issued_day: Option<Day> }
IssueInvoiceInput { invoice_id }
PaymentInput { invoice_id, currency, amount: String, note }
PaymentRecord { id, invoice_id, currency, amount_cents, recorded_day: Day,
                note, origin: "simulated_payment" }
Receivable { invoice_id, currency, invoiced_cents, received_cents,
             remaining_cents, overdue: bool }
```

Commands `create_invoice`, `issue_invoice`, `record_payment`. Invoice create
requires a current accepted engagement and milestone in acceptance_draft;
one invoice per milestone, due_day >= demo_day, SGD, positive amount. Sum of
all invoices for that engagement, including drafts, cannot exceed its proposal
total. No arbitrary amount inherited from projected income. Issue requires
draft status and still-current engagement; records current demo day and exposes
a simulated receivable. It sends/exports nothing. Already-issued history is
immutable; later scope revision does not erase debt or recorded receipts.

Payment requires an issued invoice, matching currency and positive amount
<= outstanding. Partial payments allowed; record repeated submitted requests
only once through epoch/revision concurrency, not implicit idempotent success.
Payment note required long. Old issued invoices can still receive simulated
payments after a newer engagement. Drafts cannot receive payments. Reject
overpayment/currency mismatch/missing reference without mutation.

Receivables are derived only for issued_in_exercise invoices; overdue iff
remaining>0 and demo_day>due_day. No separate mutable receivable database.
Dashboard totals each show SGD and precise labels: latest proposal projection,
latest accepted-order value, invoice drafts, simulated invoiced amount,
simulated received and simulated outstanding. accepted_order_cents uses the highest-version proposal with a last review
outcome accepted, retaining the historical order when a newer draft exists;
it does not imply that old approval authorizes newer work. projection_cents is
the latest proposal total or0. Invoice/payment totals sum their records exactly
once; receivables are joined by invoice ID. Do not sum all proposal versions
as revenue. No tax, bank reconciliation or accounting-compliance claim.

## 9. Follow-up, summary and role/legal extension points

```text
FollowupInput { title, due_day, reason, proposal_version: Option<u32> }
Followup { id, opportunity_id, title, due_day, reason,
           proposal_version: Option<u32>, status: "open"|"done_in_exercise",
           result_note: Option<String> }
CompleteFollowupInput { followup_id, result_note }
SetDayInput { demo_day }
HistoryEntry { sequence: Revision, action: String, subject_id: Option<Id>,
               proposal_version: Option<u32> }
RoleAvailability { role: "analyst"|"quotation"|"delivery"|"quality"|"legal",
                   status: "not_connected" }
LegalAvailability { status: "not_connected", coverage: Vec<Coverage> }
Coverage { region: "SG"|"EU"|"US", status: "not_reviewed" }
```

Commands `add_followup`, `complete_followup`, `set_day`. Followup references the
single opportunity and optionally an existing proposal; title short, reason
and result long; due_day >= demo_day. Completion once only, with note, no
customer-response claim. Demo day may advance monotonically to 365, never use
wall clock; reset returns day 0. Today's overview always says "Exercise day N".
History records successful commands with their allocated target, no raw text,
no hashes/signatures/timestamps, and is cleared by reset.

`Summary` fields: `currency`, `projection_cents`, `accepted_order_cents`,
`invoice_drafts_cents`, `simulated_invoiced_cents`, `simulated_received_cents`,
`simulated_outstanding_cents`, `open_clarifications`, `pending_reviews`,
`open_tasks`, `due_followups`, `next_step: String`. Count only current engagement
tasks; pending_reviews is1 iff the latest proposal is current, unexpired and
submitted, otherwise0; open_clarifications counts status open. Count due open followups with due_day<=demo_day. Next step deterministic
priority: stale latest proposal -> revise; submitted -> review; returned/rejected
-> revise; no proposal -> draft; accepted without current engagement -> plan;
current open tasks -> delivery; acceptance-draft milestone without invoice ->
draft invoice; draft invoice -> issue in exercise; outstanding -> review
receivables; due followup -> follow up; otherwise current opportunity.next_step.

`JurisdictionFacts { company_registration: Region, company_operation: Region,
customer_region: Region, data_regions: Vec<Region>, transaction: "B2B"|"B2C"|"unknown",
contract_choice: Region }`.
`Region` is a closed tagged object `{ region: "SG"|"EU"|"US"|"unknown",
subdivision: String|null }`. SG/unknown require null; EU allows null or exactly
two uppercase ASCII letters; US allows null or exactly two uppercase ASCII
letters. A supplied code is an unverified exercise fact, not evidence of a real
member-state/state or legal applicability. data_regions max 4 distinct values.
Seed registration/operation SG, customer unknown, data_regions [], transaction
B2B, contract choice unknown. UI always displays SG/EU/US coverage not reviewed;
missing EU member state/US state is visibly incomplete, never defaulted to SG.

S2 includes no task runner, RAG/source database, rule package or fake report.
Future S3-00/S3-L00 consume the following existing immutable anchors:
`epoch + case.id + input_revision + proposal.version`, proposal snapshot,
evidence IDs and jurisdiction facts. Later results must bind those anchors plus
role/skill version, model identity and source/rule versions, and be applied as
validated draft proposals through the OS boundary. Changing anchors makes a
result stale. S3 must freeze actual TaskSpec/TaskResult/ActionProposal schemas,
status/failure transitions and authenticated adapters; S2 does not pre-authorize
them. Legal rules must execute at the later backend transition boundary and
cannot be replaced with disabled buttons. Unknown/insufficient/conflicting/
expired/uncovered sources must not become acceptance. No real model route can
land before S3-M and no source fetch occurs under S2.

## 10. HTTP routes and error behavior

Root uses current tiny_http, `Server::http(("127.0.0.1", port))`; resolve the
actual bound port (including port 0) from server_addr. Do not open a browser,
load files or inspect user state. One serialized state-mutating request loop.
Embed assets with include_str/include_bytes, never runtime path lookup.

Exactly these routes once their assets exist:

| Method | Raw target | Response |
| --- | --- | --- |
| GET | `/` | index.html |
| GET | `/app.js` | app.js |
| GET | `/api/demo` | `{ ok: true, view: DemoView }` |
| POST | `/api/demo/command` | CommandRequest -> `{ ok: true, view: DemoView }` |

No other routes, query strings, percent decoding, fragments, path normalization,
CORS, redirects, HEAD aliases, `/api/workspace`, exports or model endpoints.
Known route wrong method: 405 + Allow with its sole method. Unknown raw target:
404. Origin/Host validation precedes route lookup. tiny_http framing validation
precedes application handling; never claim preservation of bytes it normalizes.

Host must occur once and exactly equal `127.0.0.1:<bound_port>`; Origin, if
present, must occur once and equal `http://127.0.0.1:<bound_port>`. POST requires
that exact Origin and exactly one `Content-Type: application/json` (case-
insensitive type; optional `; charset=utf-8` only) plus exactly one
`X-Founder-Demo: 1`. These are browser-origin controls, not credentials.
Reject invalid Host/Origin with 403. No permissive localhost/unqualified Host.

POST requires `request.body_length()` Some(n), n<=32,768; unknown length ->
411, too large ->413. Read at most32,769 bytes and reject overflow; malformed
UTF-8/JSON, duplicate required JSON fields, missing/unknown fields or trailing
nonwhitespace ->400; unsupported content type ->415. Other application input
errors ->422; stale epoch/revision/input and invalid transition ->409;
unknown_reference ->422; limit_reached ->409. Missing custom header ->403.
GET with no Content-Length and no Transfer-Encoding is the normal empty-body
request and is accepted. GET Content-Length:0 is also accepted; any positive
length or Transfer-Encoding rejects400. Duplicate Content-Length, any
Transfer-Encoding, Expect or Connection:upgrade rejects400 before application
body reading; no claim is made about tiny_http pre-handler buffering. Failed requests never
change state. A reply lost after commit is indeterminate to the browser; reload
GET state before any next action. Never retry POST automatically.

All responses: `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`,
`Referrer-Policy: no-referrer`, `Content-Security-Policy: default-src 'none';
script-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'none';
base-uri 'none'; form-action 'none'; frame-ancestors 'none'`. Header value is one
line, no newline. HTML UTF-8, JS application/javascript UTF-8, JSON application/json.
tiny_http's Date/Server/Content-Length are library transport metadata, not
business clocks. No Set-Cookie/Access-Control-Allow-Origin. Error JSON exactly
`{ ok: false, error: { code: <fixed code>, field: <fixed path or null> } }`;
transport codes: `forbidden_origin`, `not_found`, `method_not_allowed`,
`length_required`, `payload_too_large`, `invalid_json`, `unsupported_media_type`,
`invalid_request`, `internal_error`. 500 has only internal_error and null field.

## 11. Browser contract

Zero dependencies; one module `app.js`, embedded HTML with inline static CSS.
One browser entry, fragment navigation `#today`, `#company`, `#opportunities`,
`#case`, `#proposal`, `#review`, `#delivery`, `#billing`. Unknown hash ->today.
Hash contains no business text/IDs. Company and case edit forms use server
state defaults and explicit Save buttons. No generic JSON editor. All user
values rendered by textContent or safe input.value; no innerHTML interpolation,
document.write, eval, markdown renderer, remote URL embedding or dynamic code.
Static element construction helpers are permitted, not a generic form framework.

`app.js` exports these testable pure helpers and UI entry points:

```javascript
export function requestFor(view, type, data) // CommandRequest plain object
export function proposalDiff(previous, current) // [{field, before, after}]
export function formatMoney(cents) // "SGD 2,500.00", no parsing/arithmetic totals
export function render(view, route) // void; app root is #app
export async function refresh() // Promise<void>
export async function dispatch(type, data) // Promise<void>
```

Bootstrap guarded by `typeof document !== 'undefined'` so Node imports run no
IO. Only refresh/dispatch fetch, literal same-origin URLs above, credentials
omit, cache no-store, redirect error, JSON content type, custom header for
POST. Browser supplies Origin; code must not attempt to forge it. One request
in flight; disable forms/navigation mutation buttons and mark aria-busy.
Success replaces the entire detached view and renders. HTTP409 refreshes state
and visibly explains stale edit; transport/JSON/500 failure hides authoritative
data/action controls behind a "Reload exercise state" button. Never display a
successful decision/payment while response status is unknown. Local unsaved
form values may survive a422 only in current DOM; render server error by field.
No timers, retries, background polling, storage, service worker or WebSocket.

proposalDiff compares title/scope/assumptions/currency/valid_through_day and each
line index's description/quantity/unit_price_cents/total_cents, then total_cents;
missing previous/new values display `—`. It also reports input_revision change.
No textual diff library. Form defaults are generated from current service/case
or latest version, always labeled manual deterministic prefill.

UI has persistent synthetic/memory-only banner and five roles "Not connected";
legal coverage "Not reviewed" with jurisdiction facts, no green compliance
badge. UI sections appear only when listed in available_commands; unfinished
features have a plain "Later stage" explanation, no active fake buttons.
Every control has a label, error/status live region, visible keyboard focus,
no forced animation; layouts fit375x812 and1280x900 without horizontal scrolling.
Browser locale is English for S2; S1's English/Chinese support is untouched.
Reset asks via an inline two-button confirmation before submitting reset;
reload does not reset; restart/reset clears forms/history and creates new epoch.

## 12. Test ownership and commands

Tests are introduced in the card that implements behavior, before behavior.
Inline Rust child-module tests use shared `DemoState::seeded("00".repeat(16))`
fixtures; do not duplicate amount parsers, state builders or per-field JSON
serializers. Table-driven rejection tests must check unchanged full view as
well as error. Every command is exercised by at least one success and one
rejection test; reference/card tables below provide required names.

First card owns shared `apps/cli/tests/business_demo_http.rs` HTTP tests. Reuse
`#[path = "../../../crates/consultant-playground/tests/support/transport.rs"]`
for raw_request/request/Response and the shared RustLexer by equivalent path
when needed. Do not copy the parser. Its `ChildServer::start_command` is reused unchanged with
`Command::new(env!("CARGO_BIN_EXE_sovereign"))` and args
`["business-demo", "--port", "0"]`. The first-line startup prefix deliberately
matches its existing parser; assert the second line identifies Business demo.
Use `stop`, `captured_output` and existing RAII cleanup, never a new child
launcher, pipe capture, startup parser or HTTP parser.
Use existing shared capture helpers where their interfaces allow. Imports must
exercise included helpers or narrowly annotate genuinely unused imported test
support, never suppress production warnings. No helper changes in S1 files.

Required per implementation candidate:

```sh
cargo test -p sovereign-cli --locked business_demo -- --list
cargo test -p sovereign-cli --locked business_demo
cargo test -p sovereign-cli --locked --test business_demo_http
node --experimental-default-type=module --test --test-reporter=tap apps/cli/business-demo/app.test.mjs
npx -y -p typescript@5.5.4 tsc -p apps/cli/business-demo/tsconfig.json
cargo fmt --all --check
cargo clippy -p sovereign-cli --all-targets --locked -- -D warnings
git diff --check
./scripts/test_changed.sh
```

List output must contain every owning card's exact names before acceptance;
no filtered zero-test pass. Nodes tests zero skipped/failed and required names
present. The extra tsconfig must run explicitly: existing scoped gate does not
discover apps/cli/business-demo automatically. New files count toward1200 Rust/
800 frontend ceilings even before tracked; reviewer checks their wc counts.

Each wave's integration uses exact S1-11 gates (not a relaxed substitute):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build -p sovereign-cli --release --locked
cargo test -p sovereign-cli --release --locked --test playground_isolation -- --nocapture
node --experimental-default-type=module --test --test-reporter=tap crates/consultant-playground/tests/ui.test.mjs
npx -y -p typescript@5.5.4 tsc -p crates/consultant-playground/tsconfig.json
npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/tsconfig.json
cargo tree -p sovereign-consultant-playground --locked
./scripts/check-file-size.sh
git diff --check
./scripts/test_changed.sh
```

Reuse successful unchanged-source exact-candidate evidence; no repeated
mandatory planning/revalidation-only cards. S1-11 live legacy transcript
comparison and S1 physical/source/manifest checks remain required at S2-07.
New S2 process test uses FakeRoot from `apps/cli/tests/support/isolation.rs`
to verify no canary response and unchanged roots. Runtime nonmutation is
complementary to reviewed dependency/call paths, not proof of no reads.
S2 does not require output byte identity across randomly generated epochs.

## 13. Founder checklist and S2 acceptance

S2-07 reviewer executes this in a real browser at both target viewport sizes,
against a release binary bound to candidate commit; records screenshots,
console/network results and actual observations. API/Node tests are additional
evidence, not browser substitutes. Open a retained local preview when handing
off; give live PID/URL only after checking it is still reachable.

1. Run `cargo run -p sovereign-cli -- business-demo --port 7789` or release
   `./target/release/sovereign business-demo --port 7789`; open printed URL.
   Today identifies the fiction, memory lifetime, day0 and unfinished AI roles.
2. Company: change service price2500→3500 and scope text; Save. Case: edit the
   problem and contact, budget3000–5000 and unknown jurisdiction facts; Save.
   Navigate today/case; values remain linked, no manual copy/JSON.
3. Add manual fictional evidence, an assumption and a clarification. Answer
   using the new evidence. Check unverified labels and evidence links; none
   say customer-confirmed. Invalid missing evidence must show rejection.
4. Create proposal v1 from service defaults, quantity1 at3500.00, valid day30.
   It displays SGD3500.00 and tax not calculated. Submit; return with note
   "Please reduce the workshop scope". Edit to3200.00 and narrowed scope;
   Save v2. Inspect version differences; v1 decision did not approve v2.
5. Submit v2 and accept in exercise. Open second tab before a mutation; submit
   its stale form after first tab saves and confirm409/reload, no extra record.
6. Create engagement from v2: milestone "Reporting review", due day14, criterion
   "A sample report and review notes are attached to the exercise result";
   one task "Prepare reporting review". Mark done with an explicit fictional
   result note, then draft acceptance. No client acceptance/real completion claim.
7. Draft invoice3200.00 due day20 for that milestone; draft receivable remains
   absent. Issue in exercise; outstanding3200.00. Record simulated1000.00;
   outstanding2200.00. Try3000.00 overpayment; reject unchanged. Compare all
   dashboard labels; no projection/order/draft/receipt conflation.
8. Add followup due day21, reason "Discuss the next review"; advance day21;
   Today shows due followup and overdue outstanding balance. Complete with
   fictional outcome. Review history/version/reference links.
9. Stop server, click refresh: show failure and disable mutation controls.
   Restart/reload: new epoch and fresh seed, no recovered-history claim.
   Reset after making an edit: seed restored, old tab stale, can replay steps.
10. Try SG/EU/US facts with missing subdivision; all legal coverage remains
   Not reviewed and five roles Not connected. No legal/model success inferred.

Also verify keyboard navigation/focus, form labels, escaping `<img src=x onerror=...>`
as literal text, same-origin-only network, stale inputs, unsupported currency,
negative/overflow amounts, exact-version and duplicate transitions. Do not run
real user recruitment, provider calls, external messages or finances.

Ten-minute completion and five-consultant usability remain unmeasured targets.
Report no timings/percentages until their measurement script is committed.
S2-07 records accepted/changes_requested/blocked, exact source/binary/artifact
hashes, tests actually executed, browser screenshots and missing observations.
No user-usability, security guarantee or complete-MVP assertion. Nominate S3-00
with S3-M and S3-L dependencies; final MVP-00/01/02 integration still required.

## 14. Requirement map, reuse and unresolved boundaries

| Requirement | Contract / owning cards | Evidence |
| --- | --- | --- |
| Goal1–2; S2-01 | §1–4,11 / S2-01A1,A2,01B | live editable case; HTTP/state tests |
| Goal3 | §9 / S2-01A2,06 | disconnected role display; actual roles deferred S3 |
| Goal4 | §5–6,11 / S2-02,03 | version/rework/transport failure browser checks |
| Goal5 | §5–8 / S2-02–05 | deterministic totals, stale review, referenced delivery/billing |
| Goal6 | §1–3,10 / S2-01A1,A2 | reset/epoch, boundary/canary, unchanged S1 |
| Goal7 | §12 / every card,S2-07 | executed unit/HTTP/Node/browser/full/legacy gates |
| Goal8 | §13 / S2-06,07 | release launch, founder checklist, screenshots/live preview |
| Goal9 | §9 / S2-01A2,03 | unknown facts/disconnected coverage; S3-L required |
| blueprint6.2–6.3; S2-04–06 | §7–9,13 / S2-04–06 | one continuous delivery→billing→followup case |

Reuse: workspace::parse_amount_cents (pure decimal validation), existing serde
derives, tiny_http and locked rand/hex, shared raw HTTP Response/parser, FakeRoot,
RustLexer for token-level allowed import/call checks, existing S1 gates and
test_changed. New domain validators/state transitions, detached projections,
plain DOM form helpers are declared S2-specific work. The command launcher,
startup parser, RAII cleanup and complete pipe capture are reused unchanged. No new canonical writer, digest, scanner/parser or general workflow engine.

Not reused operationally: Store commit/append_events and reporting/export;
ops create/request_send/decide/confirm_delivery; render_document UUID/time;
WorkflowRunner persistence and kernel/authority/model/vault operations. They
carry persistent/authority effects or lack proposal-version semantics. Their
future integration belongs to reviewed product/S3-M contracts, not simulation.

No S2 behavior remains delegated to a worker decision. Open dependencies are
outside S2: actual owner-presence/one-use authorization and protected coordination;
RFC0005 acceptance/activation and storage inventory; S3-M credential/egress;
S3 role execution; S3-L retrieval/legal rules/professional-review separation;
MVP shared UI/model integration. A hypothetical need to relax S1, add persistent
data, true approval or egress requires a new reviewed contract/RFC as applicable,
not reuse of an exercise epoch or simulation decision. Existing full-goal owner
authorization does not supply those missing product mechanisms.

> 复用以上，禁止重新实现同类工具；需要新工具先在简报回复中申报。
