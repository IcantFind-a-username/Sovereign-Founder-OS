# Owner session 1C0 — the broker chain, the owner boundary, and the gates around them

- **Outcome:** sixteen pull requests, all merged to `main` (#59–#74)
- **Base:** `ddd386e` · **Head:** `dd31d72`
- **Plan:** `docs/superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md`
- **Contract:** [RFC 0006](../../../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md)

## The one thing to read first

**None of this is owner admission, and the work says so in its own types.**

RFC 0006 is blunt about it: a hostile process running under the same account
can win an empty-registry enrolment, and nothing in a fixture can tell that
process from the founder. So the ceremony's outcome is called
`FixtureBootstrap`, its getter is `enrolment_winner` rather than `owner`,
there is no `ProductOwnerAdmission` for it to become, and a test forbids four
names that would claim more than a fixture can.

The product's approve button is still reachable by any local process. The test
that says so — `an_unauthenticated_local_post_can_approve_today_1c0_pin`,
merged earlier in #53 — still passes, and it is meant to. When 1C0 lands for
real, that test fails, and its doc comment says to invert it rather than
delete it.

## What landed

**The broker chain** (#59–#66), one slice per step, each refusing to proceed
until the previous one held:

| Step | Property |
| --- | --- |
| bootstrap frame | one exact frame; trailing bytes refused, oversized refused without buffering |
| root classification | allow-list marker, no product state, not nested in any, symlinks refused before they are followed |
| loopback bind | one absolute monotonic deadline, captured before the address is published, shared by accept and every read |
| supervisor handshake | launch-key MAC via `verify_slice`, bound to the published nonce |
| process lock | a real advisory lock; "already held" and "could not tell" kept apart |
| redb store | reachable only with a `&HeldLock`, because there is no other constructor |
| connections | the broker mints keys, ids and expiries; callers choose none of them |

**The owner boundary** (#68–#70): a crate whose default build contains
nothing, a frozen configuration checked in its constructor, a session with two
expiries and an in-memory-only store, and a one-credential registry whose
ceremonies are one-use and expire at exactly 300 seconds.

**The gates** (#56, #57, #71, #73, #74), all in `test_changed.sh`'s always-on
block: the checked test-manifest runners, the origin preflight and its
mechanism matrix, the five feature-profile builds, the synthetic-corpus
scanner, and the derived-`Debug`-on-secrets lint.

**The kill matrix** (#72): the broker stopped at each step it claims to take,
asserting what it left behind. This is the only evidence that distinguishes
"the lock is taken after authentication" from "the lock is taken at some point
during a run that also authenticated".

Roughly a hundred tests, every one registered in `scripts/owner-effect-tests.tsv`.

## What is deliberately not here

**Task 5's WebAuthn adapter.** The ceremony's logic is complete and tested —
the registry takes `user_verified`, the credential id and the returned handle
as inputs, so no WebAuthn implementation is needed to prove any of its
properties. What is missing is the adapter that produces those inputs from a
browser response, pinned by the plan as `webauthn-rs = "=0.5.5"`.

Measured before adding it: **that pin resolves 116 packages.** They would be
optional and absent from a default build, and they would still enter the
audited lock file and become dependency-review surface. That is a judgement
about audit burden, not a coding step, so it is the owner's to make. The
backlog entry records the measurement and what to do once it is decided.

**Tasks 6–14 and 16** have not been started. Task 4 still owes
`platform_publish.rs` and the Linux-only migration gate; Task 15 owes the kill
script and its CI workflow, though the matrix itself is in.

## How the gates were verified

Every gate in this work was checked by breaking the thing it guards, because a
check that has never failed is indistinguishable from one that cannot:

- forwarding `fault-injection` from the CLI put the barrier string into the
  release binary, and the exclusion gate named all three symptoms;
- moving the process lock above the supervisor handshake made the kill matrix
  say "the lock was taken before any supervisor authenticated";
- measuring the session's absolute lifetime from last use, and checking CSRF
  before expiry, were each named exactly;
- a real address pasted into `crates/owner` was named with its file;
- `#[derive(Debug)]` on a secret-bearing struct was named with its field.

Two of those attempts did not apply on the first try, and the runs that
followed were green for the boring reason. The teeth are the second attempts.

## Corrections made to work that was already merged

- `log_out` wrote zeroes over an entry it was about to drop — something the
  compiler may elide, so it was a comforting no-op. It now removes the entry
  and says plainly that real scrubbing needs a type whose drop survives
  optimisation.
- The gate's clippy runs on default features, so **every fixture-gated module
  had never been linted at all**. Fixed, and the first run found the above.
- My own negative test data spelled out a live mailbox to prove live mailboxes
  are rejected. It now uses a reserved domain.
- One commit went onto `main` directly because I forgot to branch; it was
  moved to its branch and `main` reset before anything was pushed.

## Distance to v0.1

ROADMAP's v0.1 gate is owner authenticator/session 1C0, exact local-effect
grant, Vault 1A engine evidence, an explicit legacy warning, fault tests, and
a tagged preview. The fault tests landed earlier today; 1C0 is five of sixteen
plan tasks; there are no tags in the repository at all.
