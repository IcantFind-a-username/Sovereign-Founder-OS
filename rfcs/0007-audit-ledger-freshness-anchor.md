# RFC 0007: Audit-Ledger Freshness Anchor

**Status:** Draft; approved implementation target
**Stage:** v0.1 rollback-anchoring slice
**Security impact:** High
**Normative dependencies:** RFC 0003 (device/audit signing), THREAT_MODEL.md T6/T10

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
