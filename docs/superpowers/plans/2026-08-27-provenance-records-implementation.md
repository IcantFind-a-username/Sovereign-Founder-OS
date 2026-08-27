# Provenance Records Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add precise, machine-checked records that identify the project's documented design concepts, their repository evidence, and the maintainer's authorship representation without changing Apache-2.0 code rights or overstating legal conclusions.

**Architecture:** A root provenance record is the human-readable source of truth. Existing policy files link to it and distinguish product releases from provenance snapshots. A Python standard-library validator checks stable cross-file invariants and candidate-only CFF fields. CI runs both unit tests and the official CFF schema validator.

**Tech Stack:** Markdown, CFF 1.2.0 YAML, Python 3 standard library, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-08-27-public-provenance-release-design.md`

## Global Constraints

- Keep source code under Apache-2.0; do not claim that copyright protects abstract ideas.
- Describe provenance as the maintainer's signed representation, not an independent adjudication.
- Use exact commit, tree, and path evidence; distinguish recorded Git timestamps from independently observed publication dates.
- Do not add snapshot `version` or `date-released` to `CITATION.cff` until a dated candidate is created.
- Do not create a tag, GitHub release, DOI, signing key, or publication claim in this plan.
- Preserve current maturity labels and state explicitly that no immutable provenance snapshot exists yet.

---

## Task 1: Add a Test-Driven Metadata Validator

**Files:**

- Create: `scripts/validate-provenance.py`
- Create: `scripts/tests/test_validate_provenance.py`

**Step 1: Write failing tests**

Use `unittest` and temporary fixture directories. Cover:

- the repository's expected title, maintainer identity, repository URL, and Apache-2.0 license;
- `version` and `date-released` both absent for an undated candidate;
- rejection when only one candidate field is present;
- acceptance only when `version` matches `YYYY.MM.DD-provenance.N`, `date-released` matches `YYYY-MM-DD`, and both encode the same date;
- required provenance markers and links in the policy documents;
- rejection of absolute ownership, independent-verification, and idea-copyright claims.

Run:

```bash
python3 -m unittest scripts.tests.test_validate_provenance -v
```

Expected: FAIL because the validator does not exist.

**Step 2: Implement the smallest validator**

Implement named functions:

- `parse_top_level_cff(text: str) -> dict[str, str]`
- `validate_cff(root: Path) -> list[str]`
- `validate_document_markers(root: Path) -> list[str]`
- `validate_repository(root: Path) -> list[str]`
- `main() -> int`

Use no third-party YAML dependency. Parse only top-level scalar CFF keys needed by the invariant checks; leave full CFF schema validation to the official action.

**Step 3: Run the focused tests**

```bash
python3 -m unittest scripts.tests.test_validate_provenance -v
```

Expected: PASS.

**Step 4: Commit**

```bash
git add scripts/validate-provenance.py scripts/tests/test_validate_provenance.py
git commit -m "test(provenance): validate evidence metadata"
```

---

## Task 2: Create the Human-Readable Provenance Record

**Files:**

- Create: `PROVENANCE.md`
- Modify: `NOTICE`
- Modify: `CITATION.cff`
- Modify: `README.md`

**Step 1: Identify exact historical evidence**

For each core concept named in the design specification, locate the earliest repository commit that contains the relevant text or implementation and record:

```bash
git log --reverse --format='%H %T %aI %cI' -- PATH
git show COMMIT_SHA:PATH
```

Do not infer an independent publication date from these timestamps.

**Step 2: Add `PROVENANCE.md`**

Include:

- scope and explicit Apache-2.0 boundary;
- Franz Xu's authorship and chronology representation;
- exact commit SHA, tree SHA, path, and concept status for each core concept;
- the timestamp-evidence limitation;
- current status: repository history exists, but no immutable snapshot/DOI has yet been published;
- future snapshot identifier and verification model;
- a correction/contact process.

**Step 3: Align adjacent records**

- Make `NOTICE` identify Franz Xu and point to `PROVENANCE.md` without claiming ownership of abstract ideas.
- Keep `CITATION.cff` undated but add the provenance record to the abstract or keywords if schema-valid.
- Add a concise README link explaining what the record proves and what it does not prove.

**Step 4: Run the validator**

```bash
python3 scripts/validate-provenance.py
```

Expected: PASS.

**Step 5: Commit**

```bash
git add PROVENANCE.md NOTICE CITATION.cff README.md
git commit -m "docs(provenance): record design chronology"
```

---

## Task 3: Tighten Contribution, Trademark, and Release Governance

**Files:**

- Create: `DCO`
- Modify: `CONTRIBUTING.md`
- Modify: `TRADEMARK.md`
- Modify: `GOVERNANCE.md`
- Modify: `SECURITY.md`
- Modify: `.github/workflows/release.yml`

**Step 1: Add prospective contribution attestation**

Add Developer Certificate of Origin 1.1 and require `Signed-off-by` for future contributions. Explain that sign-off concerns contribution authority and provenance, not assignment of copyright or ownership of ideas.

**Step 2: Correct trademark wording**

Use a factual source-identification policy. Do not imply registration or guaranteed protection. Preserve nominative references and clear fork naming guidance.

**Step 3: Separate release classes**

- Define SemVer product releases separately from `provenance-YYYY.MM.DD.N` snapshots.
- State that provenance snapshots are evidence publications, not maturity claims.
- Make release/security promises conditional on the implemented workflow and exact asset set.

**Step 4: Disable unsafe automatic product publishing**

Replace the `v*` publishing trigger with a manual, non-publishing informational workflow. It must not create releases, upload assets, or request write permissions.

**Step 5: Validate and commit**

```bash
python3 scripts/validate-provenance.py
git add DCO CONTRIBUTING.md TRADEMARK.md GOVERNANCE.md SECURITY.md .github/workflows/release.yml
git commit -m "docs(governance): separate provenance publication"
```

---

## Task 4: Enforce Records in CI

**Files:**

- Modify: `.github/workflows/ci.yml`

**Step 1: Add deterministic checks**

Add steps that run:

```bash
python3 -m unittest scripts.tests.test_validate_provenance -v
python3 scripts/validate-provenance.py
```

Add the official CFF conversion/validation action pinned to an immutable commit SHA. Grant no additional permissions.

**Step 2: Inspect the workflow diff**

```bash
git diff --check
git diff -- .github/workflows/ci.yml
```

Expected: the checks run on the existing CI triggers and no publishing permission is introduced.

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci(provenance): enforce attribution records"
```

---

## Task 5: Verify the Records Subsystem

**Files:**

- Verify all files changed above.

**Step 1: Run focused verification**

```bash
python3 -m unittest scripts.tests.test_validate_provenance -v
python3 scripts/validate-provenance.py
git diff --check
```

**Step 2: Run repository-required verification**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
./scripts/check-file-size.sh
```

Expected: all commands exit zero.

**Step 3: Review claims manually**

Search for forbidden overclaims and inspect every match:

```bash
rg -n "protect(ed|s)? (the )?(idea|concept)|independently verified|immutable snapshot exists|registered trademark|original and proprietary" README.md PROVENANCE.md NOTICE CITATION.cff CONTRIBUTING.md TRADEMARK.md GOVERNANCE.md SECURITY.md
```

Expected: no misleading claim remains.
