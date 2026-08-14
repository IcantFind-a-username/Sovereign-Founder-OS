# RFC 0003: Signed Approval Evidence

**Status:** Draft; foundation implemented
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
ceremony remain Targets. RFC 0003 is therefore not a complete human-approval
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
