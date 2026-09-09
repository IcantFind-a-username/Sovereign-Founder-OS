# Owner authenticator mechanism matrix

**Status:** Fixture characterisation. Nothing here is a product claim.
**Produced by:** `scripts/owner-auth-origin-preflight.sh`
**Frozen by:** [RFC 0006](../../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md)

This file records what a browser was *observed* to do with a `__Host-` cookie
and a WebAuthn credential on loopback. It exists because Program 1C0 wants to
bind an owner session to an origin, and a design must not assume behaviour it
has not measured.

Nothing in this document qualifies the mechanism for product use, and no row
here is owner admission. The honest boundary RFC 0006 states still holds: a
hostile same-account native process can win an empty-registry enrolment, and
that is not owner admission.

## Entry schema (frozen)

Every row carries exactly these fields.

| Field | Meaning |
| --- | --- |
| `qualification` | `protocol_fixture_only` or `mechanism_qualified_only` — never anything else |
| `os` / `arch` | The platform the observation was made on |
| `browser` | Exact browser build string, not a family name |
| `authenticator` | `virtual` with its protocol/transport, or the exact real authenticator |
| `observed` | The measured outcome, as booleans and outcome strings |
| `date` | When the observation was made |

Two qualifications, and no third:

- **`protocol_fixture_only`** — produced with a CDP virtual authenticator. It
  measures the *protocol*: what the browser does with origins, RP IDs and
  cookies. It says nothing about any real authenticator.
- **`mechanism_qualified_only`** — produced by an attended run against a real
  authenticator. It qualifies the *mechanism* on that exact platform and build,
  and nothing beyond it.

Rows are added only for runs that were actually observed to pass. A failed or
uncharacterised build is excluded rather than recorded with a caveat, and
excluding it never weakens `Secure` or any other attribute of the fixture.

**An empty real matrix is allowed, and is the current state. While it is empty
the mechanism is unqualified.** Nothing may cite this document as evidence that
owner authentication works on real hardware.

## What the preflight measures, and why these three things

| # | Question | Why it decides a design choice |
| --- | --- | --- |
| 1 | Can another port on the same host set and overwrite the session cookie? | If yes, the cookie alone cannot identify the owner's session |
| 2 | Can another port obtain an assertion over the same credential? | RP IDs are hosts, so a shared host may mean a shared credential |
| 3 | Is an IP address usable as an RP ID? | Decides the name the browser must use to reach the fixture |

## Virtual matrix (`protocol_fixture_only`)

| qualification | os / arch | browser | authenticator | observed | date |
| --- | --- | --- | --- | --- | --- |
| `protocol_fixture_only` | darwin 25.5.0 / arm64 | Chrome/152.0.7977.83 | virtual, ctap2, internal, resident, UV | cookie returned to its origin: **yes**; another port set the same `__Host-` cookie: **yes**; the legitimate value survived: **no**; hostile-origin assertion obtained: **yes** (user-verified); accepted at the legitimate origin: **no**; same-user-handle creation from the other origin: **succeeded**, leaving **1** credential on the authenticator | 2026-09-10 |

## Real matrix (`mechanism_qualified_only`)

*Empty.* No attended run against a real authenticator has been performed, so
the mechanism is unqualified. This is not an oversight to be papered over: an
attended run needs a person, a real authenticator, and explicit cleanup of the
platform credential manager afterwards, because WebAuthn gives a relying party
no API to delete a credential it created.

| qualification | os / arch | browser | authenticator | observed | date |
| --- | --- | --- | --- | --- | --- |
| — | — | — | — | — | — |

## What the observations mean for 1C0

Three conclusions, each following from a measured row above.

**A cookie cannot be the owner's session on loopback.** Cookies are keyed by
host, not by origin. The `__Host-` prefix constrains `Path`, `Domain` and
`Secure`; none of those mention the port. A second server on another port of
the same host set the same cookie name and the legitimate value did not
survive. Any design that treats possession of that cookie as proof of the
owner's session is defeated by any local process that can bind a port.

**The origin inside `clientDataJSON` is the load-bearing check.** A page on the
hostile port obtained a *user-verified* assertion over the same credential —
as expected, since the RP ID is the host. What separated the two was the
relying party comparing the origin the browser recorded. That comparison is
not an optimisation; remove it and the ports are indistinguishable.

**Same-user-handle creation replaces the credential.** A resident credential
created from the other origin with the same user handle left exactly one
credential on the authenticator: the original was replaced, not added
alongside. On real hardware this is a denial-of-service a local process can
inflict on the owner, and it is accepted as a residual risk for the fixture
rather than mitigated — the fixture has no owner to lock out.

One consequence for the fixture's shape: **an IP address cannot be a WebAuthn
RP ID.** The specification requires a registrable domain, and `create()` on an
IP origin answers `SecurityError`. The browser must therefore reach the fixture
as `http://localhost:7787`, while the socket still binds `127.0.0.1` — the name
in the URL changes, what the process listens on does not.

## Reproducing

```bash
./scripts/owner-auth-origin-preflight.sh --virtual --out preflight.json
```

Requires Node 22 or newer and a Chrome or Chromium build; set `CHROME_PATH` if
it is somewhere unusual. A platform with no browser exits 4 and records itself
as unqualified rather than failing. The virtual run deletes its authenticator,
its cookies and its browser profile; an attended `--real` run additionally
requires the operator to clear the platform credential manager by hand.
