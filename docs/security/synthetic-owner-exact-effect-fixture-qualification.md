# Synthetic owner / exact-effect fixture — evidence and limitations

**Status:** Fixture qualification. Nothing here enables a product workspace.
**Contract:** [RFC 0006](../../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md)
**Gate:** `scripts/check-owner-effect-documentation.sh`

## What this document is for

To record what the fixture demonstrates and, more carefully, what it does not
— in one place, so that neither has to be reconstructed later from commit
messages by someone deciding whether to rely on it.

There is no status field here, and no transition. A document that could be
edited to say "qualified" would eventually be edited to say it.

## The honest boundary

**This fixture is not owner admission.** A hostile process running under the
same account as the founder can win the empty-registry enrolment, and nothing
in the fixture can tell that process from the founder. Every mechanism below
holds *given* an established credential; none of them establishes who
established it.

Three specific things that might look like they close that gap and do not:

- **The hidden subcommand is not a barrier.** `__owner-effect-broker` is
  absent from a default build, which keeps it out of the product. It is
  present in a fixture build, where any local process can invoke it. Obscurity
  is not admission.
- **The bootstrap key is connection authentication, not provenance.** It
  proves the process on the other end of the pipe holds the key the parent
  generated. A same-account caller can supply its own syntactically valid
  frame and become the parent, which is deliberately indistinguishable and is
  tested as unqualified fixture control.
- **Same-user-handle credential replacement is a denial of service that is
  accepted, not mitigated.** The preflight observed a second port replacing a
  resident credential rather than adding alongside it. On real hardware that
  is something a local process can inflict on the owner. The fixture has no
  owner to lock out, so it is recorded as a residual.

## What is demonstrated

Each row is a property with tests behind it, all registered in
`scripts/owner-effect-tests.tsv`.

| Property | Where |
| --- | --- |
| Nothing is bound, locked or opened before a supervisor authenticates | `broker_kill_matrix` — the process is killed between each pair of steps |
| One absolute monotonic deadline bounds the whole unowned window | `broker_listener` |
| The supervisor is authenticated by MAC over the published nonce, verified in constant time | `broker_supervisor` — all 256 single-bit tag changes |
| Sole writable ownership is a real lock the kernel releases on death, not a claim in a file | `broker_process_lock` |
| The store is unreachable without the lock, by construction | `broker_store` |
| Credentials are minted by the broker; callers choose no key, id or expiry | `broker_connections` |
| A root is classified by the broker itself and cannot be aimed at product state | `fixture_root_lifecycle`, `broker_bootstrap_frame` |
| A session is not an approval; approval binds five fields and dies with a restart | `exact_approval` |
| Evidence records that an effect happened, never what it was | `effect_v1` — including a dictionary-oracle test |
| Everything an effect needs is reserved in one transaction or not at all | `reservation_atomicity` |
| The HTTP surface accepts exactly one origin and a closed route list | `fixture_loopback` |
| The composed message is deterministic and unreadable from outside the crate | `exact_fixture_prepare` |

## What is measured rather than assumed

The origin preflight
([matrix](owner-auth-mechanism-matrix.md)) established three facts with a real
browser rather than from the specification:

- cookies are not isolated by port — a second port set and overwrote the
  `__Host-` session cookie;
- a page on another port obtained a **user-verified** assertion over the same
  credential, because an RP ID is a host;
- an IP address cannot be a WebAuthn RP ID at all, which is why the fixture is
  reached as `http://localhost:7787` while the socket binds loopback.

The real-authenticator matrix is **empty**. No attended run against real
hardware has been performed, so the mechanism is unqualified on every
platform.

## What is not demonstrated

- **Owner admission**, as above.
- **The WebAuthn adapter.** The ceremony's logic is implemented and tested —
  the registry takes user-verification, the credential id and the returned
  handle as inputs — but the adapter that produces those from a browser
  response is not written. It requires `webauthn-rs`, whose pin resolves 116
  packages including a binding to the system OpenSSL, and that is an audit
  decision rather than a coding step.
- **Product effects.** No product route, command, or workspace is reachable
  from any of this, and the corpus is a compile-time constant.
- **Encryption of the fixture store.** Redb is ACID and crash-safe. It is not
  encrypted and not authenticated, so even synthetic plaintext persistence
  generalises to nothing about founder data.

## The conjunctive gates that remain

RFC 0006 lists these as gates that must **all** be complete before product
use, not alternatives: Program 1B1 clean-machine recovery qualification,
Program 1C1 identity and role-key custody, Program 1D `ActiveV2`, and a
protected-payload review. None is addressed here.
