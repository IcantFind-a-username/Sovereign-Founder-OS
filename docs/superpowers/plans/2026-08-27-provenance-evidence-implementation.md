# Provenance Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build and verify a deterministic, tamper-evident provenance evidence bundle in CI without publishing a release or pretending that an unsigned dry run is a final attestation.

**Architecture:** Small Python command-line tools build normalized Git artifacts, finalize an acyclic hash graph, and verify the result. A read-only GitHub Actions workflow supplies a Cargo.lock-scoped SBOM and uploads an unsigned dry-run artifact. Publishable mode remains locked behind a maintainer-controlled signed tag and detached manifest signature.

**Tech Stack:** Python 3 standard library, Git, GNU tar/gzip-compatible archives, Syft SPDX 2.3 JSON, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-08-27-public-provenance-release-design.md`

**Depends on:** `docs/superpowers/plans/2026-08-27-provenance-records-implementation.md`

## Global Constraints

- Dry-run output must say `publishable: false` and must never be presented as a signed release.
- Final mode must fail closed unless the input is an annotated, cryptographically valid `provenance-*` tag and a detached maintainer signature is supplied.
- Do not create or push tags, create GitHub releases, mint DOI records, or write external services.
- Hash dependencies must be acyclic: manifest hashes payloads; detached signature signs the manifest; `SHA256SUMS` hashes payloads, manifest, and optional signature; it excludes itself.
- The Git bundle must contain the exact requested ref, not all branches or unrelated refs.
- All external actions and downloaded workflow tools must be pinned by immutable digest.

---

## Task 1: Specify and Test the Evidence Layout

**Files:**

- Create: `docs/release/VERIFY.md`
- Create: `scripts/tests/test_provenance_evidence.py`

**Step 1: Document the layout**

Define required payloads:

- normalized source tarball;
- exact-ref Git bundle;
- Cargo.lock-scoped SBOM;
- verification instructions;
- `release-manifest.json`;
- optional detached maintainer signature;
- `SHA256SUMS`.

Document dry-run versus publishable requirements and exact verification commands.

**Step 2: Write failing integration tests**

Create a temporary Git repository fixture. Test:

- two builds from the same ref produce identical source archives;
- the bundle contains only the requested positive ref;
- manifest entries are sorted and hash only payloads;
- `SHA256SUMS` excludes itself and includes the manifest;
- a modified or missing payload fails verification;
- unsigned dry-run passes with `publishable: false`;
- publishable mode rejects a branch, unsigned tag, missing signature, or mismatched snapshot ID.

Run:

```bash
python3 -m unittest scripts.tests.test_provenance_evidence -v
```

Expected: FAIL because the commands do not exist.

---

## Task 2: Build Deterministic Git Payloads

**Files:**

- Create: `scripts/provenance_evidence.py`
- Create: `scripts/build-provenance-evidence.py`

**Step 1: Implement reusable primitives**

In `scripts/provenance_evidence.py`, implement:

- `run_git(root: Path, *args: str) -> str`
- `resolve_ref(root: Path, ref: str) -> tuple[str, str, str]`
- `normalized_source_archive(root: Path, ref: str, destination: Path, prefix: str) -> None`
- `exact_ref_bundle(root: Path, ref: str, destination: Path) -> None`
- `sha256_file(path: Path) -> str`
- `write_json(path: Path, value: object) -> None`

Use `git archive` and a gzip stream with no original filename and `mtime=0`. Set archive entry prefix from the validated snapshot/dry-run identifier.

**Step 2: Implement the builder CLI**

Accept:

```text
--repository PATH
--ref FULL_REF
--snapshot-id ID
--output-dir PATH
--mode dry-run|publishable
--repository-url URL
--workflow-ref REF
--workflow-run-url URL
```

Dry run accepts an explicit branch or annotated tag ref and records the ref
object separately from its peeled commit. Publishable mode remains locked and
fails closed until the maintainer trust anchor is configured.

The builder writes only the source archive, exact-ref bundle, and a machine-readable build metadata file. It must not synthesize an SBOM or signature.

**Step 3: Run focused tests**

```bash
python3 -m unittest scripts.tests.test_provenance_evidence.ProvenanceEvidenceTests -v
```

Expected: PASS.

---

## Task 3: Finalize and Verify the Hash Graph

**Files:**

- Create: `scripts/finalize-provenance-evidence.py`
- Create: `scripts/verify-provenance-evidence.py`

**Step 1: Implement finalization**

The finalizer accepts the evidence directory, an SBOM path already generated
from `Cargo.lock`, and the pinned generator/action identity. It:

1. validates filenames and required payloads;
2. copies the SBOM and `docs/release/VERIFY.md` into the evidence directory;
3. checks exact Cargo.lock package multiplicities, the Cargo.lock root hash, and
   the pinned Syft identity in the SBOM;
4. writes a sorted manifest containing repository, workflow, ref/tag object,
   peeled commit, tree, Cargo.lock/CFF hashes, compressor/tool versions, SBOM
   generator identity, and SHA-256 for payload assets only;
5. rejects unsafe paths, symlinks, unexpected inputs, and publishable mode;
6. writes sorted `SHA256SUMS` for payloads and manifest, excluding
   `SHA256SUMS` itself.

**Step 2: Implement independent verification**

The standalone verifier recomputes hashes, rejects extra/missing manifest
payloads, imports the bundle into an empty bare repository, runs full Git fsck,
checks the advertised ref/tag object, peeled commit, tree and material blobs,
rebuilds the source archive, and validates dry-run rules.

**Step 3: Run all evidence tests**

```bash
python3 -m unittest scripts.tests.test_provenance_evidence -v
```

Expected: PASS.

**Step 4: Commit the tooling**

```bash
git add docs/release/VERIFY.md scripts/provenance_evidence.py scripts/build-provenance-evidence.py scripts/finalize-provenance-evidence.py scripts/verify-provenance-evidence.py scripts/tests/test_provenance_evidence.py
git commit -m "feat(provenance): build verifiable evidence bundles"
```

---

## Task 4: Add a Read-Only Dry-Run Workflow

**Files:**

- Create: `.github/workflows/provenance-dry-run.yml`
- Create: `scripts/check-workflows.sh`

**Step 1: Pin workflow dependencies**

Resolve official current release commits for checkout, artifact upload, and
optional GitHub attestation. Record full commit SHAs in workflow `uses:`
fields. Pin both `actionlint` and Syft versions and verify their downloaded
archive SHA-256 values before execution.

**Step 2: Implement the workflow**

Trigger on `workflow_dispatch` and relevant pull-request paths. Use:

- `contents: read` by default;
- full Git history;
- the current event ref as an explicit full ref;
- Cargo.lock-scoped SBOM generation;
- build, finalize, and verify commands in dry-run mode;
- artifact upload with a clear `UNSIGNED-DRY-RUN` name and bounded retention.

Do not request `contents: write`, create a tag/release, or use `--clobber`.

If GitHub artifact attestation is enabled, isolate it behind a non-fork manual/default-branch condition and the minimum `id-token: write` plus `attestations: write` permissions. The evidence bundle itself remains `publishable: false`.

**Step 3: Lint and inspect permissions**

```bash
./scripts/check-workflows.sh
rg -n "contents: write|gh release|git push|--clobber" .github/workflows/provenance-dry-run.yml
```

Expected: actionlint passes and the forbidden publishing operations are absent.

**Step 4: Commit**

```bash
git add .github/workflows/provenance-dry-run.yml scripts/check-workflows.sh
git commit -m "ci(provenance): exercise evidence packaging"
```

---

## Task 5: Perform a Local End-to-End Dry Run

**Files:**

- Generate under an ignored temporary directory only.

**Step 1: Prepare a minimal local SBOM fixture**

Generate the real SBOM with the pinned tool if available. If the tool is unavailable locally, use a clearly labeled test fixture only for local plumbing verification and rely on CI for the real SBOM job. Never commit or publish the fixture as release evidence.

**Step 2: Build and verify**

```bash
tmp_dir="$(mktemp -d)"
python3 scripts/build-provenance-evidence.py --repository . --ref refs/heads/docs/provenance-release-design --snapshot-id local-dry-run --output-dir "$tmp_dir/evidence" --mode dry-run
python3 scripts/finalize-provenance-evidence.py --evidence-dir "$tmp_dir/evidence" --sbom "$tmp_dir/test-sbom.spdx.json" --sbom-generator syft --sbom-generator-version 1.51.0 --sbom-generator-source sha256:2a2e837a2c8d59ec9af5472ee22d3b04ee463c4e44476ecf993fd1e5ab6ebc7f
python3 scripts/verify-provenance-evidence.py --evidence-dir "$tmp_dir/evidence"
```

Expected: verification succeeds and the manifest says `publishable: false`.

**Step 3: Prove tamper detection**

Modify a copied payload in a second temporary directory and rerun verification.

Expected: non-zero exit with the altered filename identified.

---

## Task 6: Final Repository Verification

**Files:**

- Verify all files changed by both provenance plans.

**Step 1: Run Python and workflow checks**

```bash
python3 -m unittest discover -s scripts/tests -v
python3 scripts/validate-provenance.py
./scripts/check-workflows.sh
git diff --check
```

**Step 2: Run repository-required checks**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
./scripts/check-file-size.sh
```

Expected: all commands exit zero.

**Step 3: Confirm publication boundary**

```bash
git tag --list 'provenance-*'
rg -n "publishable" .github/workflows/provenance-dry-run.yml scripts docs/release
```

Expected: no new provenance tag exists; all automated output is explicitly dry-run/non-publishable.
