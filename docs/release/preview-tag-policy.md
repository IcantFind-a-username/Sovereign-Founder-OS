# Developer Preview tag and release policy

**Status:** Governance / planning (2026-09-13). This file is policy only.
**Does not:** enable `.github/workflows/release.yml`, cut any git tag, mint a
GitHub Release, or start Wave D / product 1C0 / `ActiveV2`.
**Implements later:** close-plan ticket **v01-33** (owner + CI, not unattended
Composer). Founder install copy is **v01-34**.
**Feeds:** honest-close gap review ([#141](https://github.com/IcantFind-a-username/Sovereign-Founder-OS/pull/141)).

Owner-approved honest close ([#121](https://github.com/IcantFind-a-username/Sovereign-Founder-OS/pull/121)):
**v0.1 = fixture/mechanism proof + honest product labels.** A Developer
Preview tag is not a product `v0.1` claim.

---

## 1. Tag families

Three publication classes. They are not interchangeable. Tag names are never
reused, moved, or force-pushed. Deletion is reserved for a genuine legal,
privacy, credential, or malware emergency.

| Family | Example | What it is | When it is allowed |
| --- | --- | --- | --- |
| **Developer Preview** | `developer-preview-2026.09.20.1` | Prerelease **binaries** plus checksums / attestations / notes. GitHub Release **must** be marked prerelease and must not be “latest”. | Owner authorizes a specific tag after §5. UTC date `D` in the name is the publication date. `N` starts at `1` and increases on the same UTC day. |
| **Provenance snapshot** | `provenance-2026.08.27.1` | Source/history evidence only. **No product binaries.** Separate class. | [PROVENANCE.md](../../PROVENANCE.md) and the public-provenance design. A dry-run bundle is not a snapshot. |
| **SemVer product** | `v0.2.0` | Supported product artifacts that claim a ROADMAP milestone. | **Owner-only**, and only after the owner records that every applicable milestone exit criterion is **Met**. Automatic `v*` publication stays disabled. |

### Naming rules for Developer Preview

```text
developer-preview-YYYY.MM.DD.N
```

- `YYYY.MM.DD` is the UTC calendar date of publication, zero-padded.
- `N` is a positive integer with no leading zeros (`1`, `2`, …).
- The annotated tagger date, GitHub Release date, and the date in the tag
  name must be the same UTC day. If publication slips, abandon that name and
  mint a newly dated tag. Do not retag.
- The tag must be annotated. Prefer a maintainer-signed tag once
  [PROVENANCE.md](../../PROVENANCE.md) publishes a pinned trust-anchor
  fingerprint; until then an unsigned annotated tag is allowed and the notes
  must say so (§3).
- The tag must resolve to a commit reachable from protected `main`.

CFF / citation version for a preview, if one is written at all, is
`YYYY.MM.DD-developer-preview.N`. Do not put `0.1.0` or `v0.1` in CFF for a
preview.

### Hard ban — product `v0.1`

**Never mint `v0.1`, `v0.1.0`, `v0.1.x`, or any other `v0.1*` ref** until the
owner states in a public commit or issue that honest-close exit criteria 1–5
are actually **Met** (not Partial, not “near-met”).

This ban is independent of whether a `developer-preview-*` tag already exists.
A preview does not become `v0.1` by renaming, retagging, or editing release
notes. Provenance snapshots already forbid a `v0.1` product tag for the same
reason.

Forbidden aliases for the same act: `v0.1-rc.1`, `v0.1-preview`,
`v0.1.0-developer-preview`, `release-0.1`.

---

## 2. Required artifacts

A Developer Preview GitHub Release is incomplete until every **required**
row is present. Optional rows may be omitted; if omitted, the notes must
name the omission. Partial publication (upload some platforms, then `--clobber`
the rest) is forbidden.

| Asset | Required? | Notes |
| --- | --- | --- |
| `sovereign` CLI, Linux x86_64, `cargo build -p sovereign-cli --release --locked` | **Required** | Same gate CI already runs. Archive name: `sovereign-cli-x86_64-unknown-linux-gnu.tar.gz`. |
| Same CLI for other triples the owner enabled (macOS arm64/x86_64, Windows) | Optional | Only if that runner actually built it. Do not attach an empty or cross-faked placeholder. |
| Desktop `.app` / `.dmg` (`./apps/desktop/build-bundle.sh`) | Optional | Ad-hoc signed only. Notes must say it is **not** Developer ID and a downloaded copy still needs an explicit Open / quarantine clear. |
| `SHA256SUMS` | **Required** | SHA-256 of every payload archive **and** `release-manifest.json`. Never hashes itself. |
| `release-manifest.json` | **Required** | Repository, tag name and object, peeled commit, tree, toolchain (`rustc 1.97.0` + host), `Cargo.lock` SHA-256, workflow run URL, asset names + SHA-256 + sizes. Acyclic: the manifest does not hash itself. |
| GitHub artifact attestations (SLSA provenance) | **Required when** the build runs on GitHub-hosted runners with OIDC | Bind each uploaded payload, the manifest, and `SHA256SUMS` to that workflow run. Attestations are not a maintainer signature and are not a provenance snapshot. |
| Detached maintainer signature over `release-manifest.json` | **Required when** a pinned trust anchor exists in [PROVENANCE.md](../../PROVENANCE.md); otherwise **forbidden to fake** | If no fingerprint is published, omit the file and say so in the notes. Do not attach a CI-generated key and call it the maintainer. |
| `VERIFY.md` (preview) | **Required** | Commands to check `SHA256SUMS`, print the manifest, and (when present) verify attestations. May live as `docs/release/VERIFY-PREVIEW.md` copied into the release. Do not reuse [VERIFY.md](VERIFY.md) as if this were a provenance bundle. |
| Provenance **notes** (in the Release body) | **Required** | Exact commit, tree, tag object, workflow run, what is signed vs only hashed, and the sentence that this is **not** a `provenance-*` snapshot. |
| Release notes with §3 non-claims | **Required** | GitHub Release body. Prerelease checkbox on. |

A preview **must not** attach a Git history bundle, source tarball labelled as
a provenance snapshot, SPDX SBOM claimed as a published snapshot, or a DOI /
SWHID. Those belong to `provenance-*`. A Cargo.lock-scoped SBOM may be
attached as optional extra evidence if generated by the same pinned Syft path
used in the dry-run workflow; it does not convert the preview into a
snapshot.

---

## 3. Required non-claims (release notes)

Every Developer Preview Release body must include the following block
verbatim, then a short “what this build actually is” paragraph that does not
contradict it.

```text
This is a Developer Preview — not a production release.

- Not production-ready. Not supported. Not a security certification.
- Not a SemVer product release. This tag is not v0.1, v0.1.0, or any
  ROADMAP milestone completion.
- Not product 1C0. The founder UI has no independently authenticated owner
  session. The Experimental local outbox is application-signed; an
  unauthenticated local API decision still uses the application-created
  owner key. Product 1C0 admission is a v0.2 gate (RFC 0006 G2).
- Experimental, and labelled that way: Consultant Core v1; the co-located
  vault key; the app-signed outbox; the model gateway (caller-declared
  class, self-reported provider trust). “Current” never means universally
  secure.
- Program 1A / vault-v2-engine, if mentioned, is a non-product engine
  (`publish = false`). No product Vault enrollment, backup, recovery,
  migration, PendingV2, or ActiveV2.
- Export verifies format, device-key binding, and the signed audit chain.
  It does not authenticate every workspace field. The archive is plaintext.
- No network email, no OAuth broker, no encrypted backup, no restore, no
  whole-device rollback detection, no Owned Mesh.
- A provenance snapshot is a different tag family (provenance-*). This
  preview does not satisfy or waive those gates.
```

Forbidden words and implications in the title, body, README blurb, or asset
names: “production-ready”, “production release”, “generally available”,
“stable”, “enterprise”, “certified”, “E2EE” / “encrypted at rest” for the
legacy workspace, “compliant” as a legal verdict, “product 1C0”, “owner
session on the founder UI”, “ActiveV2”, or any claim that RFC 0004 / `Local
Only` zero-egress has landed.

The GitHub Release title form is:

```text
Sovereign Founder OS — Developer Preview YYYY-MM-DD (N)
```

---

## 4. Later change plan for `release.yml` (do not implement here)

`.github/workflows/release.yml` is a **disabled SemVer product stub**:
`workflow_dispatch` only, `permissions: {}`, one job that prints that
automatic product publication is off. It must stay that way until a
**separate** product-release redesign meets [GOVERNANCE.md](../../GOVERNANCE.md)
and [SECURITY.md](../../SECURITY.md) (checksums, dependency-scoped SBOM,
signatures, build provenance, verification).

**v01-33 must not** turn this stub into a `v*` publisher, add `on.push.tags:
['v*']`, or restore the pre-disable matrix that let each platform job create
or replace a release.

Checklist for the later Composer / owner+CI ticket:

1. **Keep** `release.yml` as the disabled product gate. Add a file-header
   comment pointing at this policy. No tag trigger, no `contents: write`,
   no `softprops/action-gh-release` (or equivalent).
2. **Add** a new workflow, suggested path
   `.github/workflows/developer-preview.yml`, triggered only by annotated
   tags matching `developer-preview-[0-9]{4}\.[0-9]{2}\.[0-9]{2}\.[1-9][0-9]*`.
   Reject any ref that matches `v0.1`, `v0.1.*`, `v*`, or `provenance-*`.
3. Fail closed unless the tag peels to a commit on protected `main`, required
   CI on that commit is green, and the UTC same-day rule in §1 holds.
4. Rebuild from the tag (not from a floating `main` checkout):
   `cargo fmt`, clippy `-D warnings`, `cargo test --workspace --locked`,
   file-size, frontend `tsc`, dependency audit, secret scan, then
   `cargo build -p sovereign-cli --release --locked`.
5. Stage **all** assets into a **draft** Release from a single job. No
   `--clobber`, no concurrent `create release`, no error masking. A second
   job publishes only after the draft set is complete and hashes rematch.
6. Mark the published GitHub Release as **prerelease**. Never set it as
   latest. Enable immutable releases in repo settings before the first
   publish if the owner has not already.
7. Emit `SHA256SUMS`, `release-manifest.json`, preview `VERIFY.md`, and
   GitHub attestations. Attach a maintainer detached signature only when
   the pinned trust anchor exists and verifies.
8. Pin every Action to a full commit SHA. Job-level least privilege: build
   jobs `contents: read`; attestation job OIDC only; publish job contents
   write only after draft verification.
9. Tests: tag-regex accept/reject (including `v0.1` / `v0.1.0`),
   missing-asset fail, hash mismatch fail, double-publish fail, prerelease
   flag asserted.
10. Owner actions that automation must not impersonate: create the annotated
    tag, publish the trust anchor, approve a protected environment, press
    the final publish on the first preview.

Provenance dry-run (`.github/workflows/provenance-dry-run.yml`) stays a
source-evidence dry run. Do not merge the two workflows.

---

## 5. Preconditions before the first `developer-preview-*` tag

A labelled Developer Preview may be discussed after honesty lands. **Minting**
the first tag still needs the owner’s go-ahead and the v01-33 workflow. This
table uses the honest-close reading of exits 1–5
([gap review #141](https://github.com/IcantFind-a-username/Sovereign-Founder-OS/pull/141)).

| Gate | Honest-close meaning | Minting rule |
| --- | --- | --- |
| **#121** | ROADMAP exit 2 rewritten: fixture/mechanism + labels; product 1C0 → v0.2 | **Must merge** before any preview notes are written. |
| **Track A** (#118–#120, #122, #123, #136, #137) | Honesty pins (gateway, locality copy, T10 freeze, v1 blob pins, export boundary, leftover copy, stale admissions row) | **Must merge.** Exit 2 is **Met** only after #121 + Track A. |
| **Track B** (#124–#131) | Experimental-outbox integrity (journal recover, checkpoint pin, `ledger.head`, SIGKILL/two-process, adversarial, compile worker). **Not** 1C0. | **Must merge** before a preview that mentions fail-closed send. Unblock rustfmt (#124) and the #125 fixture flake first. |
| **Track C** (#132–#140) | Program 1A engine evidence, `publish = false`, no product enrollment | **Must merge** before notes mention 1A evidence. Exit 3 may remain **Partial / near-met**; residuals (fixture DB, getrandom matrix, importer failpoints) are optional hardening, not a Wave D substitute. |
| **Exit 1** | Clean install without source knowledge | May stay **Partial**. Notes must not claim a non-specialist install-from-preview path until **v01-34** (`--root` + founder copy). `git clone` + `cargo run` remains the documented path until then. |
| **Exit 2** | Honest Experimental outbox; no product-1C0 claim | **Must be Met** (#121 + Track A). Notes use the §3 block. Wave D is **not** required and is **forbidden** until v01-G2 is accepted. |
| **Exit 3** | 1A engine evidence, not product enrollment | Labelled **Partial / near-met** after Track C is acceptable for a preview. Do not claim product Vault v2 or `ActiveV2`. |
| **Exit 4** | Export binds values **and** documents exclusions | Stays **labelled Partial**. #123 **pins** the plaintext / unauthenticated workspace boundary. Do not “fix” binding to close v0.1. Notes must repeat that pin. |
| **Exit 5** | One tagged, signed/checksummed Developer Preview, no production claim | **This policy + v01-33**. Still **Missing** until the owner mints `developer-preview-*` with §2 artifacts. Meeting exit 5 does **not** mint `v0.1`. |
| **v01-G2** | RFC 0002 `low-risk-effectful` / `write_rfc5322` | Blocks **Wave D** only. Does **not** block a preview tag. |
| **Owner mint** | Explicit public approval of the exact tag name and UTC date | **Required.** Unattended Composer / workers do not create tags or GitHub Releases. |

### Still forbidden on the preview train

- Wave D fixture productization, v1 broker revival (`BrokerReady`, HMAC IPC,
  hidden child), WebAuthn on `/api/workspace/decide`, removal of
  `kernel_exec` `owner_approval_key`.
- Product 1C0 admission, Programs 1B0/1B1, 1C1, 1D `PendingV2` / `ActiveV2`.
- RFC 0004 product boundary, confined local-model worker, network email.
- A `v0.1` / `v0.1.0` tag.

---

## 6. Who may mint

| Actor | May draft this policy / v01-33 workflow PR | May create `developer-preview-*` | May create `v0.1` / `v0.1.0` |
| --- | --- | --- | --- |
| Composer / worker session | Yes (workflow later; not this PR) | **No** | **No** |
| CI on a tag the owner created | Build, checksum, attest, draft | Publish only after owner environment approval | **No** |
| Owner (Franz Xu) | Yes | Yes, after §5 | **Only** after recording that exits 1–5 are Met |

---

## References

- [GOVERNANCE.md](../../GOVERNANCE.md) — publication classes
- [SECURITY.md](../../SECURITY.md) — supported versions; SemVer gates
- [PROVENANCE.md](../../PROVENANCE.md) — snapshot class; no published snapshot yet
- [docs/release/VERIFY.md](VERIFY.md) — provenance dry-run verifier (not preview)
- [Public provenance design](../superpowers/specs/2026-08-27-public-provenance-release-design.md)
- Honest-close ROADMAP rewrite: PR #121
- Close plan (ticket map, includes v01-33): PR #117
- Gap review (current status, forbids product `v0.1`): PR #141
- `.github/workflows/release.yml` — disabled product stub (leave disabled)
