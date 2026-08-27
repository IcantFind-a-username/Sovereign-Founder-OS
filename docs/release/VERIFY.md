# Verify a Provenance Evidence Bundle

The current automated bundle is an **unsigned dry run**. It tests deterministic
source packaging and tamper detection; it is not an immutable release, a
maintainer signature, an independent publication timestamp, or a legal
originality finding.

## Expected Files

- `SNAPSHOT-source.tar.gz`: normalized source for one exact ref;
- `SNAPSHOT.bundle`: a Git bundle containing that exact positive ref;
- `SNAPSHOT.sbom.spdx.json`: Syft SPDX 2.3 dependency evidence generated from
  and checked exactly against the recorded `Cargo.lock` blob;
- `build-metadata.json`: resolved ref, commit, tree, and build tool versions;
- `release-manifest.json`: SHA-256 and size for payload assets only;
- `SHA256SUMS`: hashes payloads and the manifest, but never itself;
- `VERIFY.md`: this guide.

A future publishable bundle must additionally contain a detached maintainer
signature over `release-manifest.json` and independently verifiable trust-anchor
instructions. No such trust anchor is configured today.

When a maintainer manually starts the dry-run workflow, GitHub also records
artifact attestations for each evidence file. Those attestations bind files to
that workflow execution; they do not replace the missing maintainer signature
or convert the dry run into a published snapshot.

## Automated Verification

The bundle is self-contained. From any directory containing the evidence:

```bash
python3 scripts/verify-provenance-evidence.py \
  --evidence-dir PATH_TO_EVIDENCE
```

The verifier imports the exact bundle ref into a new bare repository, runs full
Git object checking, and checks every checksum, manifest entry, ref/tag object,
peeled commit, tree, material hash, Cargo.lock SBOM package identity, Syft
generator version, and byte-for-byte rebuilt source archive. It rejects missing,
altered, extra, symlinked, or wrongly named payloads and any bundle marked as
publishable.

## GitHub Artifact Attestations

GitHub attestations exist only for a manually dispatched run whose source ref
was the repository default branch. For this repository, the expected source ref
is `refs/heads/main`. First run the self-contained verifier above; then bind
GitHub verification to the exact commit recorded by that verified bundle:

```bash
EVIDENCE_DIR=PATH_TO_EVIDENCE
REPOSITORY=IcantFind-a-username/Sovereign-Founder-OS
SIGNER_WORKFLOW="$REPOSITORY/.github/workflows/provenance-dry-run.yml"
CERT_IDENTITY="https://github.com/$SIGNER_WORKFLOW@refs/heads/main"
EXPECTED_COMMIT="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["commit"])' "$EVIDENCE_DIR/release-manifest.json")"

for artifact in "$EVIDENCE_DIR"/*; do
  [ -f "$artifact" ] || continue
  gh attestation verify "$artifact" \
    --repo "$REPOSITORY" \
    --cert-identity "$CERT_IDENTITY" \
    --source-ref refs/heads/main \
    --source-digest "$EXPECTED_COMMIT" \
    --signer-digest "$EXPECTED_COMMIT" \
    --cert-oidc-issuer https://token.actions.githubusercontent.com \
    --deny-self-hosted-runners \
    --predicate-type https://slsa.dev/provenance/v1
done
```

Use `--repo`, not the broader `--owner`, and retain the complete certificate
identity shown above. `--cert-identity` compares the certificate SAN exactly;
do not replace it with a partial or regular-expression workflow match. In this
direct, non-reusable workflow, the source and signer workflow revisions are the
same event commit; pinning both digests prevents a later revision of the same
workflow path from satisfying this check.

`--source-ref refs/heads/main` verifies the signed source-ref claim, but the CLI
does not independently prove that `main` is or was the repository's default or
protected branch. It also has no direct filter for the `workflow_dispatch` event
or one exact workflow run ID. Inspect the workflow at `EXPECTED_COMMIT` and, when
those additional checks matter, evaluate the verified JSON certificate/run
metadata separately. Do not treat workflow-controlled predicate fields as an
independent identity source. GitHub attestations bind bytes to a GitHub-hosted
workflow execution; they do not supply the missing maintainer signature or prove
authorship, originality, safety, or publication time outside GitHub.

## Manual Inspection

Inspect the manifest and bundle without checking out their contents:

```bash
EVIDENCE_DIR="$(realpath PATH_TO_EVIDENCE)"
BUNDLE_REPOSITORY="$(mktemp -d)"
python3 -m json.tool "$EVIDENCE_DIR/release-manifest.json"
git -C "$BUNDLE_REPOSITORY" init --bare
git -C "$BUNDLE_REPOSITORY" bundle list-heads "$EVIDENCE_DIR/SNAPSHOT.bundle"
git -C "$BUNDLE_REPOSITORY" bundle verify "$EVIDENCE_DIR/SNAPSHOT.bundle"
(
  cd "$EVIDENCE_DIR"
  sha256sum --check SHA256SUMS
)
```

Extract the archive only into a new empty directory after checking its entry
names. Do not run code from an evidence bundle merely to verify its hashes.

## Evidence Meaning

Successful verification establishes internal consistency between the supplied
files and the exact ref imported from the supplied Git bundle. It does not
establish who authored the material, when a third party first observed it,
whether similar ideas existed elsewhere, or whether the software is safe for
production.
