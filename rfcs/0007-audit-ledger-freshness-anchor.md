# RFC 0007: Audit-Ledger Freshness Anchor

**Status:** Draft; approved implementation target for the v0.1 anchor slice;
Amendment 1 Accepted 2026-09-16 (latest-head / generation protection vs
signing-key protection). RFC overall remains `Draft`; Amendment 1 is
Accepted Target design. Design Accept ≠ product acceptance: no RP1-06
product qualification, Runtime Phase 1 complete, whole-device rollback
detection, Wave D, 1C0, ActiveV2, or 1B0 claim.
**Stage:** v0.1 rollback-anchoring slice
**Security impact:** High
**Normative dependencies:** RFC 0003 (device/audit signing), THREAT_MODEL.md T6/T10

## Amendment 1 acceptance record

> **Written Acceptance — RFC 0007 Amendment 1 (2026-09-16)**  
> As sole maintainer I accept RFC 0007 Amendment 1 (latest-head / `freshness_generation` vs signing-key protection), proposed via merged PR #150 / `14b490e7`, as Target design. RFC 0007 overall remains `Draft`; this is design acceptance of the amendment only.  
>  
> **Decision.** On 2026-09-16 I confirmed `accept-150-with-waiver` (Grok Bot).  
>  
> **Discussion waiver.** This repository currently has a sole developer. CONTRIBUTING.md's 7-day community discussion window does not apply as a blocking gate: there is no community whose discussion could change the record. This is stronger than shortening seven days; the discussion step is waived because a sole-maintainer project has no independent discussants.  
>  
> **What acceptance licenses.** Implementation of enrolled `freshness_generation`, paired-restore rejection when enrolled generation survives outside the restored tree, limited-recovery failure modes, and RFC 0003 authority coupling (generation invalidation of consumed / revoke / dispatch state). The backlog item **Implement RFC 0007 Amendment 1** may start.  
>  
> **What acceptance does not license.** (1) An RP1-06 product checkmark or Runtime Phase 1 completion — those still require evidence after implementation. (2) Whole-device rollback detection (Research; T10). (3) Treating a co-located enrolled file as independently protected until RFC 0005 / 1C1 custody. (4) Wave D, 1C0 admission, product ActiveV2, or Program 1B0. (5) Using audit-chain verify alone as fresh authority after restore.  
>  
> **Explicit separation:** Accepting Amendment 1 ≠ RP1-06 product pass.

Recorded by the repository owner / sole maintainer (IcantFind-a-username /
Yiqun Xu) on 2026-09-16. No co-signers.

## Summary

The audit ledger today is a device-signed hash chain. `verify_chain`
(`crates/audit-ledger/src/lib.rs`) proves each event links to its predecessor
and was signed by the trusted device key — **internal consistency and device
binding only**. It proves nothing about *age* or *completeness*: an actor who
replaces `ledger.json` with an older, validly-signed **prefix** of the same
chain passes verification unchanged (THREAT_MODEL.md:36; T6 "rollback require an
external trusted head to detect"; the unchecked Verification Requirement at
THREAT_MODEL.md:258).

This RFC specifies a **freshness anchor**: a small, separately stored,
device-signed head commitment over the ledger's length and tip, plus an
open-time check that rejects a rewound ledger. It is the v0.1 mechanism for the
"workspace/ledger old-prefix restore is rejected while protected device
freshness survives" requirement, and it operationalizes T6's "owner-device head
anchoring" Target for the current ledger without waiting for the Vault v2
program.

It deliberately does **not** change the signed `AuditEventBody` wire shape.
Adding a sequence field to the event body would invalidate every existing
signed chain and audit token (the body is signed as `serde_json::to_vec` in
declaration order; field set and order are load-bearing). The anchor is a new
sidecar file, so existing chains stay valid.

## Non-negotiable invariants

1. The anchor is a separate file; the `AuditEventBody` signed shape is
   unchanged, and an existing ledger with no anchor is upgraded in place on the
   next append, never rejected for lacking one at first sight of a legacy root.
2. The anchor is device-signed over `{workspace binding, event_count,
   last_event_hash}` using the same device key and encoding as ledger events.
3. Open-time freshness **accepts** a ledger that is a forward extension of the
   anchored head and **rejects** a ledger whose length regressed below the
   anchor or whose event at the anchored index does not carry the anchored
   hash (a fork).
4. The protection is **conditional and stated honestly** (see Threat model): the
   anchor detects a rewind only when the anchor is protected independently of
   the ledger. It is never described as defeating an actor who can also forge
   the anchor.
5. A missing anchor over a **non-empty** ledger that has already been anchored
   once fails closed; a first-run empty root and a never-yet-anchored legacy
   ledger initialize normally (invariant 1).

## Anchor format

A sidecar file `ledger.head` beside `ledger.json`, written with the same
crash-safe temp+fsync+rename+dir-fsync primitive and, on Unix, owner-only
(0600) mode. Its signed body is:

```text
version              (u16, = 1)
workspace_binding    (the trusted device public key b64 the ledger is bound to)
event_count          (u64: number of events in the anchored ledger)
last_event_hash      (the anchored tip; GENESIS_HASH when event_count == 0)
```

The body is hashed and the hash signed by the device key exactly as ledger
events are (`sign_legacy_v1` over the hash), so anchor verification reuses the
existing device-signature path and trust anchor. The anchor carries no business
values — length and tip hash only.

## Save ordering and crash semantics

Append is unchanged. On persist, `ledger.json` is written **first**, then
`ledger.head`. The two writes are each individually atomic but not one
transaction, which is safe by construction:

- **Crash after the ledger, before the anchor:** the ledger is one event ahead
  of the anchor, same prefix. Open sees a *forward extension* of the anchored
  head → accepted (invariant 3). The next append re-anchors. This is the
  fail-forward direction: a durably recorded event is never rejected because
  the anchor lagged.
- **Crash after both:** consistent; open accepts.
- There is no crash window that turns a genuine ledger into a rejected one; the
  only rejections are a length regression or a fork, neither of which a crash
  between two monotonic writes can produce.

## Open-time freshness check

On workspace open (and in `integrity_check`), after `verify_chain` succeeds and
the anchor is present:

1. Verify the anchor's device signature and that its `workspace_binding` equals
   the ledger's trusted device key; a mismatch fails closed.
2. Reject if `ledger.len() < anchor.event_count` (**rewind**).
3. Reject if the event at index `anchor.event_count - 1` does not have
   `event_hash == anchor.last_event_hash` (**fork/substitution**); for
   `event_count == 0` require the ledger's genesis position to be consistent.
4. Otherwise accept: the ledger is the anchored head or a forward extension of
   it.

A rejection is surfaced as a distinct, clearly worded error (a rewound or
forked ledger), never a silent pass and never conflated with the internal
`ChainBroken`.

## Threat model and honest boundary

**What it detects.** A ledger prefix-rewind or fork, **when the freshness
anchor is protected independently of the ledger** — for example a deployment
that exposes or syncs `ledger.json` while keeping `ledger.head` and the device
key on protected local storage, or (later) when the device key lives in the
Program 1C1 / RFC 0005 device protector rather than beside the data.

**What it does not.** In the v0.1 co-located-key layout, `device.json` sits in
the workspace root beside `ledger.json` and `ledger.head`. An actor who can
write the whole directory can also read the device key and re-sign a fresh
anchor for any prefix, defeating detection. That is **whole-device rollback**,
which THREAT_MODEL.md:195 (T10) keeps Research/deployment-dependent — it needs
an external monotonic anchor (another owner device, hardware counter, or
transparency service) — and is explicitly out of scope here. This RFC makes no
whole-device-rollback claim.

**Why it is still worth landing in v0.1.** The mechanism (a) detects accidental
rollback and partial restores, (b) defeats any actor with ledger-write but not
device-key-read access, and (c) is a no-op change to the signed event shape
that **gains teeth automatically** the moment key custody improves under RFC
0005 / Program 1C1 — the anchor is then signed by a key the ledger-writer cannot
reach. Landing the format and the check now means the freshness guarantee turns
on with key custody rather than requiring a second migration of the ledger.

## Relationship to other RFCs and the threat model

- **RFC 0005** names "workspace-relative freshness detects a workspace or ledger
  prefix restored while protected device state survives" as a Target Vault
  mitigation. This RFC is the v0.1, ledger-scoped realization of that idea using
  the current device-signed ledger; RFC 0005's Vault v2 freshness can supersede
  or subsume it once the device protector exists.
- **THREAT_MODEL.md** T6/T10 and the Verification Requirement at :258 are the
  requirements this RFC satisfies; its delta records this mechanism as Current
  (conditional) rather than Target.
- The ledger's legacy device-signature encoding is unchanged; the separate
  migration to a role-separated Audit COSE signer (T6 "migration pending") is
  orthogonal and not addressed here.

## Conformance tests

The queued implementation entries are done when these named tests pass:

- `crates/audit-ledger`:
  `an_old_prefix_ledger_is_rejected_against_a_current_anchor`,
  `a_forked_chain_at_the_anchored_index_is_rejected`,
  `a_forward_extension_of_the_anchored_head_is_accepted`,
  `a_missing_anchor_over_a_previously_anchored_ledger_fails_closed`,
  `a_first_run_and_a_never_anchored_legacy_ledger_initialize`,
  `the_signed_AuditEventBody_shape_is_unchanged` (the `crates/contracts`
  golden-shape tests still pass).
- `apps/cli` workspace:
  `a_reverted_ledger_is_refused_at_open`,
  `a_whole_directory_rollback_is_not_detected_documented_boundary`.

## Change control

The anchor format, the accept/reject rules, and the honest boundary are frozen.
Weakening the rewind/fork rejection, or restating the boundary as a
whole-device-rollback defense, is a security-critical amendment to this RFC.

## Amendments

### Amendment 1 (Accepted 2026-09-16): latest-head and generation protection vs signing-key protection

**Scope and status.** This amendment is protocol and threat-model design for
Runtime Phase 1 finding F09 and acceptance row RP1-06 (see
`docs/security/runtime-phase-1-development-guide.zh-CN.md`). It closes a gap in the
v0.1 anchor slice without retracting what already landed: a device-signed
`ledger.head` still rejects a rewound or forked ledger **when the anchor on
disk is strictly newer than the ledger**. It does **not** reject the paired
restore of an **old** ledger **and** an **old** anchor that were captured
together — both remain internally consistent and validly signed. Protecting
the signing key from a ledger-only writer therefore does **not** by itself
protect **latest** head or generation state. Written Acceptance is recorded
in [Amendment 1 acceptance record](#amendment-1-acceptance-record).
Implementation of enrolled generation, paired-restore rejection, and
authority coupling may start. Nothing below is a claim that RP1-06 has
passed as a product checkmark.

#### a. Two protection domains (normative separation)

| Domain | What it protects | What it does not prove |
| --- | --- | --- |
| **Signing-key protection** | That `ledger.head` (and ledger events) were signed by the trusted device key and match `{binding, event_count, last_event_hash}` | That the opened workspace is the **latest** state the owner enrolled, not an older snapshot |
| **Latest-head / generation protection** | That open uses a head **at or above** the highest **enrolled** generation for this workspace binding — no silent downgrade | Whole-device rollback when every secret and enrolled record restores from the same snapshot (Research; unchanged) |

Implementations MUST NOT describe key custody alone (for example moving the
device key into an RFC 0005 protector) as satisfying RP1-06 or T10
workspace-relative freshness **until** latest-head / generation rules in this
amendment are implemented and evidenced.

#### b. Enrolled freshness generation (Target)

The v0.1 anchor body (§ Anchor format) remains unchanged for backward
compatibility. Amendment 1 adds a **separate enrolled record** (name and
storage profile are implementation choices; it MUST NOT be implied by
`ledger.json` or `ledger.head` alone):

```text
freshness_generation   (u64, strictly monotonic for this workspace_binding while enrolled)
enrolled_at_unix       (u64, informational)
workspace_binding      (same trusted device public key b64 as the ledger)
```

**Enrollment.** The first time a deployment claims workspace-relative
freshness beyond the v0.1 slice, it MUST persist an enrolled generation (≥ 1)
in storage that is **not** restored as a unit with an arbitrary old workspace
tree unless that restore is explicitly classified as limited recovery (§ e).
Each time the ledger head anchor advances to a new tip that the product treats
as authoritative, the implementation MUST bump `freshness_generation` and
persist the enrolled record **before** or **atomically with** treating open as
successful — never accept a lower generation after a higher one was enrolled.

**Open-time rules (in addition to § Open-time freshness check).** After
`verify_chain` and anchor verification succeed:

1. If an enrolled record exists for this binding, reject if
   `anchor_generation < enrolled.freshness_generation` (**generation
   downgrade**) even when the anchor signature and ledger prefix match.
2. If an enrolled record exists, reject if the presented anchor does not
   carry the generation expected for this head (once generation is wired into
   the anchor sidecar or an adjacent signed field — see § c).
3. If enrollment was required by product policy and the enrolled record is
   **missing**, fail closed into **limited recovery** (§ e); do not silently
   accept the on-disk anchor as “current.”

**Non-downgradeable semantics.** Restoring `ledger.json` and `ledger.head`
from the same backup without the **current** enrolled generation MUST NOT
produce a normal open. The honest v0.1 co-located layout may still allow a
**full-directory** writer to rewrite enrolled state together with the ledger;
that remains whole-device rollback (Research). The Target is enrolled state
that survives a workspace-tree-only restore (RFC 0005 device protector, OS
secure storage, owner-held trust-continuity material, or another profile named
in the implementation plan).

#### c. Anchor generation binding (Target wire extension)

When generation protection is implemented, the signed anchor commitment MUST
include `freshness_generation` (u64) in the signed body (sidecar version bump,
not a change to `AuditEventBody`). The enrolled record and the anchor MUST
agree on generation for the same `{workspace_binding, event_count,
last_event_hash}`. A forward extension of the ledger without a matching
generation bump fails closed.

Until that wire extension ships, implementations MUST document the residual:
paired old ledger + old anchor acceptance when nothing outside the restored
tree remembers a higher generation.

#### d. Authority and revocation state (cross-RFC 0003)

Audit-chain verification and anchor freshness MUST NOT be treated as proof
that **authority consumption, reservations, or revocations** are still valid
for new effects.

- **Generation invalidation.** When `freshness_generation` advances through
  recovery, re-enrollment, or any path that replaces enrolled state, all
  prior consumed-bundle markers, dispatch handles, session epochs, and
  effect-intent authority bound to the superseded generation MUST be treated
  as **invalid for new execute paths** without a new owner ceremony — even if
  `verify_chain` over an old ledger prefix would still pass in isolation.
- **Recovery isolation.** Recovery protocols MUST align with RFC 0003
  Amendment 1 roll-forward semantics and Program 2 / RFC 0006 recovery
  isolation: restoring business data does not automatically restore execution
  authority; stale grants MUST NOT revive because the audit log looks
  internally consistent.
- **Revocation durability.** Durable revocations remain authoritative across
  crash; they do not override generation downgrade detection — a rewound
  authority store plus an old anchor is still a security failure, not a
  silent return to “revoked still revoked, therefore safe to dispatch old
  grants.”

#### e. Missing anchor and missing enrolled state (fail closed)

| Condition | Required behavior |
| --- | --- |
| Non-empty ledger, anchor previously written, `ledger.head` missing | Unchanged: fail closed (§ Non-negotiable invariants). |
| Enrollment claimed, enrolled generation missing | Fail closed into **limited recovery** — explicit operator/owner mode with narrowed capabilities, diagnostic surfacing, and no default full workspace open. |
| Enrolled generation present, ledger/anchor pair inconsistent with it | Fail closed; do not pick the “best effort” older head. |
| Owner completes limited recovery | MUST bump to a **new** generation strictly above any value found in restored artifacts; MUST NOT silently re-use pre-restore grants. |

Limited recovery is not a silent security downgrade: UI and APIs MUST distinguish
it from normal open (exact strings are product choices; the distinction is
normative).

#### f. Whole-device rollback (unchanged honesty)

This amendment does **not** expand scope to defeat an actor who restores an
internally valid full-device snapshot including device key, enrolled
generation, ledger, anchor, and authority store. THREAT_MODEL.md T10
Research/deployment-dependent language remains the ceiling. Workspace-relative
freshness means **protected latest state survives a rollback of the
replaceable workspace tree**, not that the device proves its own age from
itself alone.

#### g. Conformance tests (implementation target; names illustrative)

Accepted Target; implementation may start. Expected additions beyond §
Conformance tests:

- `paired_old_ledger_and_old_anchor_rejected_when_enrolled_generation_survives`
- `a_generation_downgrade_is_rejected_even_with_valid_signatures`
- `missing_enrolled_generation_after_enrollment_enters_limited_recovery`
- `authority_consume_state_not_assumed_fresh_from_audit_chain_alone`
- `apps/cli`: `a_paired_workspace_tree_restore_is_refused_when_generation_survives_outside_tree`

Existing v0.1 tests remain required; they document the conditional slice, not
RP1-06 completion.

#### h. What stays blocked

- Flipping RFC 0007 overall from `Draft` to `Accepted` on an implementation
  branch (this record accepts **Amendment 1** only).
- Claiming RP1-06 product qualification or Runtime Phase 1 completion.
- Using audit freshness alone to authorize effects after restore.
- Whole-device rollback detection without an external monotonic anchor
  (Research).
- Treating a co-located enrolled file as independently protected until
  RFC 0005 / 1C1 custody.
- Wave D, 1C0 admission, product ActiveV2, or Program 1B0.
