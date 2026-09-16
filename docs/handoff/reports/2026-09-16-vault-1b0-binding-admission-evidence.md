# Vault Program 1B0 — SQLCipher binding-admission evidence

- **Outcome:** docs-only market survey. **1B0 remains blocked.**
- **Date:** 2026-09-16
- **Survey as of:** 2026-09-16 (crates.io API + GitHub; UTC)
- **Baseline (`main`):** `8a0e2964adcd3be73bbfc24b97a393c63006a933`
  (rebased after #165/#167; survey pins read from `d1ed46e` — those PRs
  did not change `crates/vault-v2-engine`)
- **Authoritative gate:** [RFC 0005](../../../rfcs/0005-dual-root-vault-and-recovery.md)
  (Accepted 2026-09-14, including Amendment 1 and Amendment 2) and
  [PR #148](https://github.com/IcantFind-a-username/Sovereign-Founder-OS/pull/148)
- **This file is not:** an RFC, a binding-admission amendment, a product
  Exact Effect, an implementation, or a protection claim.

Program 1B0 implementation is **not** licensed by this ledger. Accepting
RFC 0005 ≠ implementing 1B0 ≠ shipping product `ActiveV2`.

---

## 1. Gate restatement

RFC 0005 Status is **Accepted** (2026-09-14). Acceptance is design-only.
What it withholds for 1B0 is written in three places; quoted here.

**RFC 0005 — Design status (what `Accepted` does not license):**

> starting Program 1B0 implementation before a follow-up amendment admits
> a released Rust binding for SQLCipher exactly 4.17.0 (or a superseding
> amendment naming another exact release after applicability review of
> 4.19.0)

**RFC 0005 — Written Acceptance (what acceptance does not license),
item (1):**

> Starting Program 1B0 implementation before a follow-up amendment admits
> one released Rust binding that bundles the selected SQLCipher release
> under Amendment 1's four verification requirements — `rusqlite 0.40.2` /
> SQLCipher 4.14.0 remains the Program 1A-only profile; applicability
> review of upstream 4.19.0 must accompany any shipping pin.

**PR #148 (merged acceptance PR) — still blocked:**

> Starting Program 1B0 **implementation** before a follow-up amendment
> admits one released Rust binding that bundles SQLCipher exactly 4.17.0
> under Amendment 1's four verification requirements (or a superseding
> amendment naming another exact release after applicability review of
> 4.19.0). `rusqlite 0.40.2` / SQLCipher 4.14.0 remains the Program
> 1A-only profile.

**Amendment 1 (2026-08-26) — exact release, not a range:**

> The exact SQLCipher release this RFC admits for the upgraded profile is
> **`4.17.0`** (upstream release tag `v4.17.0`, 2026-07-07). No other
> version is selected: not a range, not "4.17.0 or later", and not a
> runtime "at least" check.

**Amendment 1 — four verification requirements** (all four must be
recorded in a follow-up amendment before any binding is admitted):

1. a released, tagged, registry-published Rust binding whose locked
   dependency resolution bundles SQLCipher exactly `4.17.0`;
2. an independent dependency diff and supply-chain review of the change
   from the admitted `rusqlite 0.40.2` / SQLCipher `4.14.0` profile, with
   reproducible hashes/builds, license evidence, and upstream source
   provenance;
3. no material unresolved security advisory against the binding or the
   bundled SQLCipher/OpenSSL sources — an advisory blocks admission
   rather than triggering improvised build plumbing; and
4. runtime and CI exact-match checks through the real engine: `PRAGMA
   cipher_version` returns exactly `4.17.0`, `PRAGMA cipher_provider` and
   the provider version match the reviewed profile, and the approved
   target-specific `PRAGMA compile_options` profile matches — any
   mismatch fails closed rather than continuing.

This ledger surveys requirement (1) in the public registry. It does
**not** perform (2), (3), or (4). Those remain future amendment work
after a released candidate exists.

Amendment 2 follow-up **5** (open, does not block design acceptance)
already pairs any future shipping pin with an applicability review of
upstream **4.19.0** (2026-09-08). That review is not done here.

---

## 2. Current Program 1A pin (from this tree)

Read from `crates/vault-v2-engine` on baseline `d1ed46e`, not from
memory.

| Pin | Source | Value |
| --- | --- | --- |
| Crate | `crates/vault-v2-engine/Cargo.toml` | `sovereign-vault-v2-engine` (`publish = false`) |
| Binding | same `Cargo.toml` | `rusqlite = { version = "=0.40.2", default-features = false, features = ["bundled-sqlcipher-vendored-openssl", "hooks", "limits"] }` |
| OpenSSL FFI crate | same `Cargo.toml` | `openssl-sys = { version = "=0.9.117", default-features = false }` |
| Lockfile binding | workspace `Cargo.lock` | `rusqlite` **0.40.2** (checksum `23f2a97d…`) |
| Lockfile SQLite/SQLCipher FFI | workspace `Cargo.lock` | `libsqlite3-sys` **0.38.2** (checksum `f1d20bef…`) |
| Lockfile OpenSSL | workspace `Cargo.lock` | `openssl-sys` **0.9.117**; `openssl-src` **300.6.1+3.6.3** |
| Declared SQLCipher | `crates/vault-v2-engine/src/lib.rs` `PINNED_SQLCIPHER_VERSION` | `"4.14.0"` |
| Runtime check | `crates/vault-v2-engine/tests/public.rs` `sqlcipher_runtime_is_exactly_4_14_0_for_released_profile` | `PRAGMA cipher_version` must match `4.14.0`; a `4.17*` runtime is a **failure**, not an upgrade |
| RFC body | RFC 0005 § Exact dependency and build | released lockfile resolution bundles SQLCipher exactly `4.14.0` through `libsqlite3-sys = 0.38.2`; MUST NOT be described as 4.17.0 or production-ready |

Program 1A may keep this 4.14.0 profile for the internal Experimental
engine only. It is not a 1B0, product-activation, or production pin.

---

## 3. Market survey (2026-09-16)

Sources: crates.io HTTP API (with a descriptive User-Agent), GitHub
releases / file contents / commits. Unknowns are marked **unknown**.

Amendment 1 requirement (1) is the only surveyable admission predicate
today: a **released, tagged, registry-published** crate whose **locked
resolution bundles SQLCipher exactly 4.17.0**. Host-linked SQLCipher
(the `sqlcipher` feature that searches the system) cannot pin an exact
bundled version. An unreleased git revision is not a registry binding;
RFC 0005 already forbids selecting the candidate beginning `62648175`
from that abbreviated identifier.

Requirements (2)–(4) are **not met** for any row: this ledger did not
run a dependency diff, a scoped advisory review, or engine PRAGMA
readback against a 4.17.0 candidate. They are recorded as **not
evaluated / blocked by (1)**.

### 3.1 Primary binding path (`rusqlite` / `libsqlite3-sys`)

| Field | Evidence |
| --- | --- |
| Crate | `rusqlite` |
| Latest crates.io release | **0.40.2** (published 2026-08-08) |
| Companion FFI crate | `libsqlite3-sys` **0.38.2** (same day) |
| License (crate) | MIT |
| Bundled-SQLCipher features | `bundled-sqlcipher`, `bundled-sqlcipher-vendored-openssl` (and host `sqlcipher`) |
| Claimed bundled SQLCipher on **released** tag `v0.40.2` | **4.14.0** — `libsqlite3-sys/upgrade_sqlcipher.sh` on `refs/tags/v0.40.2` sets `SQLCIPHER_VERSION="4.14.0"` |
| Claimed bundled SQLCipher on **unreleased** default branch | **4.17.0** — same script on `master` / commit [`62648175`](https://github.com/rusqlite/rusqlite/commit/62648175c23f84b45238f4a1fbb0133b75ce68f1) (2026-07-14, “Bump bundled SQLCipher to version 4.17.0”) |
| Can pin **exactly 4.17.0** from a crates.io release? | **No.** Newest registry release is 0.40.2; its 2026-08-08 notes are “Lower MSRV to 1.88.0” only — no SQLCipher bump. 0.40.2 was cut after `62648175` landed on `master` and still vendors 4.14.0. |
| Amendment 1 req. (1) | **Fail** |
| Req. (2)(3)(4) | Not evaluated (blocked by (1)) |
| Last release | 2026-08-08 |
| 4.18.0 / 4.19.0 in rusqlite tree | GitHub code search on 2026-09-16 found **no** `4.18.0` / `4.19.0` hits in `rusqlite/rusqlite` |

RFC 0005 already names this unreleased revision: “The reviewed candidate
revision beginning `62648175` carries 4.17.0 but is unreleased and
unsigned in this dependency path; it MUST NOT be selected silently.”
That statement is still true on 2026-09-16.

### 3.2 Other registry candidates

| Crate | Latest release | Last release | Claimed SQLCipher | Pin exactly 4.17.0? | License | Amendment 1 |
| --- | --- | --- | --- | --- | --- | --- |
| `libsqlite3-sys` | 0.38.2 (2026-08-08) | 2026-08-08 | Bundled amalgamation is the rusqlite 0.40.2 path → **4.14.0** on the released tag | **No** | MIT | (1) fail; (2)(3)(4) not evaluated |
| `sqlx` / `sqlx-sqlite` | 0.9.0 (2026-05-21) | 2026-05-21 | **No SQLCipher feature.** `sqlite` / `sqlite-bundled` / `bundled` only. `sqlx-sqlite` depends on `libsqlite3-sys >=0.30.1, <0.38.0` (does not even take 0.38.2) | **No** — does not bundle SQLCipher | MIT OR Apache-2.0 | (1) fail |
| `diesel` | 2.3.13 (2026-09-04) | 2026-09-04 | `sqlite` feature optional-depends on `libsqlite3-sys >=0.17.2, <0.39.0`. **No SQLCipher feature** in the 2.3.13 feature list | **No** — host/bundled SQLite, not a SQLCipher 4.17.0 bundle | MIT OR Apache-2.0 | (1) fail |
| `tokio-rusqlite` | 0.8.0 (2026-09-06) | 2026-09-06 | Wrapper. Depends on `rusqlite ^0.40.1`. Forwards `bundled-sqlcipher*` features; locked SQLCipher follows rusqlite 0.40.x → **4.14.0** | **No** | MIT | (1) fail |
| `r2d2_sqlite` | 0.35.0 (2026-07-06) | 2026-07-06 | Pool wrapper. Depends on `rusqlite ^0.40`. Forwards `bundled-sqlcipher*` | **No** | MIT | (1) fail |
| `rusqlcipher` | 0.14.9 (2018-05-02) | 2018-05-02 | Historical SQLCipher wrapper; last crates.io release **2018**. Bundled SQLCipher version **unknown** (not 4.17.0; 4.17.0 did not exist) | **No** | MIT | (1) fail |
| `libsqlcipher-sys` | 0.9.0 (2018-05-02) | 2018-05-02 | Native bindings to **system** `libsqlcipher` (`sqlcipher` feature). Does not vendor a pin | **No** — cannot lock exactly 4.17.0 | MIT | (1) fail |
| `sqlcipher-src` | 0.1.1 (2017-02-11) | 2017-02-11 | “The package provides SQLCipher.” Last release **2017**. Exact bundled version **unknown**; cannot be 4.17.0 | **No** | Apache-2.0/MIT (crates.io version license field) | (1) fail |
| `sqlcipher-provider` | 0.1.0 (2016-02-29) | 2016-02-29 | Provider crate, 2016. Exact SQLCipher **unknown** | **No** | Apache-2.0/MIT | (1) fail |
| `libsqlite3-sys-ic` / `rusqlite-ic` | 0.25.0 / 0.28.1 (2023-01-17) | 2023-01-17 | Frozen rusqlite forks. SQLCipher **unknown**; not a 2026 4.17.0 bundle | **No** | MIT | (1) fail |
| `libsql-rusqlite` | crate `newest_version` reports `0.10.0-pre.4` (updated 2026-06-02); version list also shows `0.33.0` (2024-09-20) | 2026-06-02 (pre) / 2024-09-20 (0.33.0) | libSQL fork of rusqlite, **not** a SQLCipher 4.17.0 vendor path. Exact SQLCipher **n/a** / **unknown** | **No** | MIT (0.33.0) | (1) fail |
| `evault-store-sqlcipher` | 0.1.0 (2026-05-14) | 2026-05-14 | Application store. Depends on `rusqlite ^0.39` + feature `sqlcipher`. Not a vendor of 4.17.0 | **No** | MIT | (1) fail |

crates.io search `q=sqlcipher` (first page, 2026-09-16) also returned
application crates (`anamn-core`, `aegis-vault-pqc`, `walletkit-db`,
`signal-desktop-core`, …). Those consume a binding; they are not
admissible SQLCipher vendor crates. There is **no** crates.io crate
named `sqlcipher`.

GitHub `SQLCIPHER_VERSION="4.17.0"` in a **released rusqlite tag** was
not found. Application repositories that *expect* a 4.17.0 runtime
(e.g. a local `EXPECTED_SQLCIPHER_VERSION`) are not registry bindings
and were not treated as candidates.

### 3.3 Upstream SQLCipher (context, not a Rust binding)

| Tag | Published | Notes |
| --- | --- | --- |
| `v4.14.0` | 2026-03-17 | Program 1A admitted bundle |
| `v4.17.0` | 2026-07-08 | Amendment 1 selected exact release |
| `v4.18.0` | 2026-08-18 | Considered in Amendment 1; **not** selected |
| `v4.19.0` | 2026-09-08 | RFC follow-up 5. Release notes: two **low-risk** core issues related to `sqlcipher_export` and `hexkey` URI; plus export-alias escaping and other maintenance. This project **bans** `sqlcipher_export` / hexkey URI entry points, so 4.19.0 is **not automatically exploitable in this profile** — and is also **not automatically admitted**. |
| `v5.0.0-beta` | 2026-09-15 | Prerelease. **Out of scope.** Not a 1B0 pin. |

No released Rust binding surveyed above bundles 4.19.0 either.

---

## 4. Gap

**Does any released Rust binding today admit SQLCipher exactly 4.17.0
under Amendment 1 requirement (1)?**

**No.**

Therefore Amendment 1 requirements (2)–(4) cannot start, and **Program
1B0 implementation remains blocked.** This document does not change
that.

The closest object is still the **unreleased** rusqlite commit
`62648175` (SQLCipher 4.17.0 on `master`). RFC 0005 already refuses to
select it from that abbreviated identifier. rusqlite 0.40.2 (2026-08-08)
did not ship it.

---

## 5. Recommended next governance step

This ledger **does not choose product enablement**, does not flip RFC
status, and does not start 1B0, FFI, or Wave D.

The three governance paths named in the task remain open and are
**not** product decisions:

1. **(a) Wait for a binding release.** Still required for Amendment 1
   requirement (1). Watch crates.io `rusqlite` / `libsqlite3-sys` for a
   tagged registry release whose locked `bundled-sqlcipher*` resolution
   is exactly `4.17.0` (or whatever exact release a later amendment
   names). Do not git-pin `62648175`.
2. **(b) Draft a binding-admission amendment once a candidate exists.**
   The follow-up amendment is the only vehicle that can name one exact
   crate version and record all four verification requirements. **Do
   not draft it until (a) produces a released candidate** (or the owner
   separately accepts an exact-source plan after review — also an RFC
   amendment, not a silent Cargo change).
3. **(c) Applicability review path for 4.19.0.** Already listed as RFC
   0005 Amendment 2 follow-up 5. Upstream 4.19.0 exists (2026-09-08)
   with low-risk `sqlcipher_export` / `hexkey` URI fixes. Merge that
   review with the future binding-admission amendment. **No silent
   bump** from 4.17.0 to 4.19.0 (or to 5.x). Selecting 4.19.0 would
   require a **superseding amendment**, not this evidence file.

Until (a) lands a registry crate, 1B0 stays blocked even if (c) is
later written. Until (b) records the four checks, a published crate
still does not admit the profile.

Program 1A may continue on the existing `rusqlite =0.40.2` / SQLCipher
**4.14.0** Experimental engine with no product path.

---

## 6. Non-claims

- Accepting RFC 0005 ≠ implementing Program 1B0 ≠ shipping product
  `ActiveV2`.
- This evidence document ≠ product Exact Effect, ≠ filtered backup, ≠
  enrollment, ≠ workspace migration, ≠ format selection, ≠ a product
  dependency edge to `sovereign-vault-v2-engine`.
- Surveying crates.io ≠ Amendment 1 verification, ≠ supply-chain review,
  ≠ advisory clearance, ≠ `PRAGMA cipher_version` evidence.
- Unreleased rusqlite `62648175` ≠ an admitted binding.
- SQLCipher 4.19.0 existing upstream ≠ an admitted shipping pin.
- `PINNED_SQLCIPHER_VERSION = "4.14.0"` and a green Program 1A engine
  test ≠ founder-data confidentiality.

**1B0 remains blocked.**
