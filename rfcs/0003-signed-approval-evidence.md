# RFC 0003: Signed Approval Evidence

**Status:** Draft; foundation implemented; Amendment 1 applied 2026-08-26
(transactional consumption bundle and durable revocation — see Amendments)
**Stage:** 1
**Security impact:** Critical

## Summary

Define approval-role-signed authorization evidence for one exact invocation
and how that evidence is bound into Capability V2. Before this
RFC, every approval-required request failed closed because no approval
protocol existed. After it, an approval is a COSE_Sign1 object signed under a
dedicated approval role, bound to the exact artifact, manifest, input,
resource, and policy-decision digests it approves, valid for a bounded
window. The target ceremony proves an independently authenticated human owner
reviewed the exact request. The current Workspace instead turns an
unauthenticated local API decision into a backend signature; it does not prove
human presence or review.

RFC 0002's invariants apply unchanged. This RFC adds the approval role and
object; it does not enable effectful execution, which still additionally
requires the durable Authority Store, crash-safe evidence, and reviewed host
interfaces.

## Non-Negotiable Invariants

1. An approval approves one exact prepared invocation under one exact policy
   decision — never a tool, a category, or a session.
2. Approval evidence is created only by a key trusted for the approval role.
   Keys trusted for publisher, authority, audit, admission, or cache roles
   must not verify as approvers, even with identical key bytes.
3. Missing, malformed, expired, mismatched, replayed, or unexpected approval
   evidence fails closed. Evidence supplied when policy does not require it
   is rejected, not ignored.
4. The capability token carries only the approval summary claim
   (`approval_id`, `approver_subject_id`, `approved_at_unix`); the full
   signed object is re-verified at consumption, so issuance and consumption
   observe the same evidence.

## Canonical Encoding and Role

The approval object follows RFC 0002's COSE profile exactly (JCS payload,
protected-header-only, deterministic CBOR, role-specific trust resolution):

```text
content-type   application/sovereign.approval+json;v=1
external AAD   sovereign:approval:v1
role name      approval
```

## Approval Claims (version 1)

```text
typ = sovereign.approval
version = 1
approval_id                    (UUID, unique per approval)
approver_issuer                (must equal the trusted record's issuer)
approver_key_id                (must equal the protected kid)
approver_subject_id            (human-readable owner identity)
audience, venture_id, subject_id, session_id
tool { tool_id, tool_version, operation }
component_digest, manifest_digest,
canonical_input_digest, resource_bindings_digest
primary_resource
policy_decision_id, policy_decision_digest
canonicalization_profile = rfc8785-jcs+sovereign-digest-v1
approved_at_unix, expires_at_unix
```

All fields are required; unknown fields and non-canonical payloads are
rejected.

## Temporal Rules

- `expires_at_unix - approved_at_unix` must be in `(0, 600]` seconds.
- Validation requires `approved_at_unix <= now < expires_at_unix`.
- The approval must postdate the policy decision it approves:
  `approved_at_unix >= evaluated_at_unix`.
- Capability V2's 30-second policy-freshness window exists to bound the gap
  between evaluation and issuance. A human cannot click in 30 seconds
  reliably, so when valid approval evidence is present the policy-age limit
  extends to the approval window (600 s). The signed object attests approval-
  role authorization; it is human-review evidence only when an independently
  authenticated preview/confirmation ceremony controls use of that key.

## Binding into Capability V2

- **Issuance:** when the policy decision requires approval, the issuer must
  be configured with an approval trust store and be given the signed
  approval object. It verifies the object against the request's exact
  prepared invocation and policy decision, then sets the token's
  `approval_evidence` summary claim. Plain issuance without evidence
  continues to fail closed. Evidence with a decision that does not require
  approval is rejected.
- **Consumption:** the validator requires the same signed approval object
  whenever the token carries an `approval_evidence` claim, re-verifies it
  against the presented invocation and decision, requires the summary claim
  to match the object exactly, and attempts to consume `approval_id` at most
  once within the supported replay backend.
  A token without evidence for an approval-required decision, or evidence
  where none is required, fails closed.

## Replay and Durability (honest labels)

Approval one-use accounting is process-local when no Authority Store is
attached. When the Workspace attaches the current filesystem Authority Store,
the durable approval claim now retains the verified signed approval's own
expiry rather than the shorter capability-token expiry. Tests expire and purge
the first token, reopen the store while the approval remains valid, reject a
second token's reuse, and purge the approval at its own expiry.

This is still a partial durability boundary. Token, idempotency, and approval
claims are ordered filesystem operations, not one recoverable transaction;
partial failure burns earlier claims and fails closed. Durable revocation, a
full real-subprocess validator race, and an independently admitted owner
ceremony remain Targets. Amendment 1 (below) pins the exact transaction and
revocation protocol the first two Targets must implement; this paragraph
remains the Current description until the queued implementation entries land. RFC 0003 is therefore not a complete human-approval
or product-authority claim. Approval evidence is also necessary but not
sufficient for an external effect: independent owner presence, exact effect
preview/payload binding, durable intent/result ordering, and a reviewed host
coordinator remain required.

## Threat Cases (tested)

- Approval signed by an untrusted, revoked, or cross-role key.
- Approval bound to different invocation digests or a different policy
  decision than presented.
- Expired approval, future-dated approval, approval predating its policy
  decision, and out-of-range lifetime.
- Approval reuse across two tokens before persistent-record purge (second
  consumption denied), and after the first token's expiry/purge plus Authority
  Store reopen while the signed approval remains valid.
- Evidence supplied when policy does not require approval.
- Token claiming approval evidence that does not match the presented object,
  or presented without any object.
- Legacy behavior preserved: issuance without evidence fails closed for
  approval-required decisions.

Target regression coverage still adds a true subprocess validator race and
durable revocation; current reopen coverage does not establish either one.

## Amendments

### Amendment 1 (2026-08-26): transactional consumption bundle and durable revocation

**Scope and status.** This amendment is protocol design, accepted as the
implementation target for the queued `crates/authority`, `crates/capability`,
`apps/cli/src/workspace/`, and `tests/adversarial/` backlog entries. Nothing
below is implemented at amendment time; the honest-labels paragraphs above
stay the Current description until those entries land. It covers the
"one-transaction reservation" and "revocation" parts of RFC 0002's remaining
authorization work (Authorization and Replay; Phase C). Exact effect binding
and owner-authority integration (1C0) are separate v0.1 work and are not
changed here. Single-device scope: one filesystem, one trusted runtime clock;
no network coordination is designed or implied.

#### a. On-disk bundle transaction

The store gains one directory, `bundles/`, beside `tokens/`, `approvals/`,
and `idempotency/`. Every file uses the store's existing publication
primitive unchanged: exclusive `create_new` temporary (mode 0600 on Unix),
`sync_all`, `hard_link` to the final name (link-to-existing fails on every
supported platform, so exactly one racer publishes), directory fsync on Unix,
4 KiB record cap.

The bundle identity is deterministic from what it consumes:
`bundle_hex = SHA-256("sovereign:authority-bundle:v1" || token_id bytes ||
approval_id bytes || idempotency_key bytes || invocation_fingerprint)`,
lower-hex. Determinism is load-bearing: a crashed consumer that retries with
the same token, approval, idempotency key, and invocation reconstructs the
same bundle and resumes it instead of colliding with it.

`AuthorityRecord` gains one optional field, `bundle_hex: Option<String>`
(absent in every existing record; old records keep parsing, and old code
ignores the new field). `consume_bundle` runs these steps in order:

1. **Intent.** Publish `bundles/<bundle_hex>` recording the three ids, the
   fingerprint, `created_at_unix`, and `expires_at_unix = ` the latest of the
   three part expiries. If the name exists, parse and require field equality
   with this request (mismatch is `CorruptRecord`); equality means this is a
   retry — continue.
2. **Revocation pre-check.** If `revoked-tokens/<token_id>` or
   `revoked-approvals/<approval_id>` exists, fail closed with `Revoked`.
3. **Token claim.** Publish `tokens/<token_id>` with this `bundle_hex`. If it
   exists: same `bundle_hex` → retry, continue; any other value or absent →
   `AlreadyConsumed`.
4. **Idempotency bind.** As today, plus `bundle_hex`: existing record with
   same fingerprint and same bundle → continue; same fingerprint, foreign or
   absent bundle → `IdempotencyReplay`; different fingerprint →
   `IdempotencyConflict`.
5. **Approval claim.** As step 3 for `approvals/<approval_id>`.
6. **Commit.** Re-check revocation as in step 2, then publish
   `bundles/<bundle_hex>.committed` (a copy of the intent record plus
   `consumed_at_unix`). Created → the one `Authorized` outcome for this
   bundle. Already exists → `AlreadyConsumed`: some racer of this same bundle
   committed first; whether the effect itself already ran is the execution
   journal's question, never this store's.

Only a durable `.committed` marker authorizes proceeding to the effect. A
caller that has not observed its own commit succeed must treat the bundle as
unauthorized.

#### b. Crash recovery

Recovery is roll-forward only; nothing is ever deleted to recover, so there is
no release/resume race to referee:

- A crash before step 6 leaves a partial bundle. It authorized nothing (no
  committed marker), it denies every foreign consumer (claims exist), and the
  same consumer completes it by retrying with the same inputs (every step is
  idempotent for the owning bundle). This removes the burned-claims asymmetry
  the honest-labels section admits: a same-input retry now completes instead
  of dying on its own earlier claims.
- A crash after step 6 is the store's existing fail-closed direction,
  unchanged: authority consumed, effect not run, re-issuance (a fresh
  approval ceremony) is the recovery path.
- Stale partial claims self-expire with their subjects: `purge_expired`
  additionally removes `bundles/` entries (intent and marker) once their
  `expires_at_unix` passes, exactly the existing purge rule — safe because an
  expired token or approval is independently rejected by the validator's
  temporal checks. Fresh authority never collides with stale claims because
  fresh ids produce fresh names.
- Accepted cost, stated plainly: if the token expires after a partial bundle
  crash while the approval stays valid, the approval remains denied (claimed
  by the uncommitted bundle) until its own expiry. That is the fail-closed
  direction and matches today's burned-approval behavior; the fix is a fresh
  approval, never claim takeover, which would reintroduce the release race
  this design eliminates.

#### c. Durable revocation

Two directories, `revoked-tokens/` and `revoked-approvals/`, keyed by subject
id, using the same publication primitive and the same record shape
(`kind = "revoked-token" | "revoked-approval"`, `consumed_at_unix` carrying
the revocation time, `expires_at_unix` = the subject's expiry, supplied by
the caller). `revoke_token` / `revoke_approval` return a three-way outcome:
`Revoked` (record published, subject unconsumed), `AlreadyRevoked` (record
already present), or `RevokedAfterConsumption` (subject's claim already
existed under a committed bundle or a legacy record — the record is still
published so the late revocation is durable and auditable, and the caller can
tell the owner the effect had already been authorized). Consumption checks
revocation at steps 2 and 6; the legacy single-claim methods gain the step-2
pre-check as defense in depth. A revocation record that fails to parse is
`CorruptRecord` and fails consumption closed — an unreadable revocation must
deny, never allow. Revocation records purge on the same expired-subject rule
as everything else.

#### d. Concurrency contract

- Per name (any directory): the hard-link publish admits exactly one writer;
  every loser observes the winner's complete record.
- Per bundle: exactly one `Authorized` outcome ever (the `.committed`
  create); racing retries of the same bundle get `AlreadyConsumed`.
- Across bundles contending for a shared part: exactly one bundle holds the
  claim; all others get `AlreadyConsumed`, whether the holder is committed or
  still partial.
- Revoke vs. consume: the commit-time re-check (step 6) is the serialization
  point. A revocation observed there aborts with `Revoked`. A revocation
  published after that observation but before the marker lands leaves both
  records present, which reads unambiguously as `RevokedAfterConsumption` —
  one durable outcome either way, and both orders fail closed for the loser.
- Clock: `now_unix` comes from the trusted runtime clock (RFC 0002); the
  store never trusts a caller-supplied validation timestamp beyond it.

#### e. Conformance tests

The queued implementation entries are done when exactly these named tests
pass, plus the entries' own criteria:

- `crates/authority` transaction entry:
  `a_retried_bundle_after_any_interruption_completes_without_burning_claims`,
  `racing_bundles_over_the_same_token_have_exactly_one_winner`,
  `racing_retries_of_the_same_bundle_authorize_exactly_once`,
  `a_reopened_store_answers_a_partial_bundle_identically`,
  `a_foreign_uncommitted_bundle_denies_other_consumers`,
  `purge_removes_expired_bundles_and_their_claims`.
- `crates/authority` revocation entry:
  `a_revoked_token_fails_closed_across_reopen`,
  `revoking_a_consumed_claim_reports_the_distinct_outcome`,
  `a_revoke_vs_consume_race_ends_in_one_durable_outcome`,
  `a_corrupt_revocation_record_fails_closed`.
- `crates/capability` entry:
  `an_interrupted_bundle_no_longer_burns_earlier_claims`,
  `a_revoked_approval_is_rejected_with_the_typed_error`.
- `apps/cli` workspace entry: `a_revoked_delivery_cannot_dispatch`,
  `expired_authority_records_are_purged_on_open`.

Interruption points are exercised by driving the protocol's step functions
directly (the implementation must expose them to tests); a mocked filesystem
is not required and not wanted.
