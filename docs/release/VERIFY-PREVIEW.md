# Verify a Developer Preview Release

Developer Preview **binaries** are a separate publication class from
provenance snapshots (`provenance-*`) and from SemVer product tags (`v*` /
`v0.1*`). Policy:
[preview-tag-policy.md](preview-tag-policy.md). Workflow:
[developer-preview.yml](../../.github/workflows/developer-preview.yml).

## Expected files

- `sovereign-cli-x86_64-unknown-linux-gnu.tar.gz` — Linux x86_64 CLI built with
  `cargo build -p sovereign-cli --release --locked`;
- `release-manifest.json` — tag object, peeled commit, tree, toolchain, `Cargo.lock`
  hash, workflow run URL, and listed payload assets (the manifest does not hash
  itself);
- `SHA256SUMS` — SHA-256 of every payload archive and `release-manifest.json`
  (never includes itself);
- `VERIFY.md` — this guide (copied from this file at release time).

A detached maintainer signature over `release-manifest.json` appears **only**
when [PROVENANCE.md](../../PROVENANCE.md) publishes a pinned trust-anchor
fingerprint. CI does not mint a stand-in maintainer key.

## Checksums and manifest

From a directory containing the release assets:

```bash
RELEASE_DIR="$(realpath PATH_TO_RELEASE_ASSETS)"
(
  cd "$RELEASE_DIR"
  sha256sum --check SHA256SUMS
)
python3 -m json.tool "$RELEASE_DIR/release-manifest.json"
```

## GitHub artifact attestations

When the release was built on GitHub-hosted runners, each payload file,
`release-manifest.json`, and `SHA256SUMS` should carry a GitHub artifact
attestation binding bytes to the Developer Preview workflow run recorded in the
manifest. Attestations are not a maintainer signature and do not convert this
preview into a `provenance-*` snapshot.

```bash
RELEASE_DIR="$(realpath PATH_TO_RELEASE_ASSETS)"
REPOSITORY=IcantFind-a-username/Sovereign-Founder-OS
SIGNER_WORKFLOW="$REPOSITORY/.github/workflows/developer-preview.yml"
EXPECTED_COMMIT="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["commit"])' "$RELEASE_DIR/release-manifest.json")"
TAG_REF="refs/tags/$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["tag"]["name"])' "$RELEASE_DIR/release-manifest.json")"

for artifact in \
  "$RELEASE_DIR/sovereign-cli-x86_64-unknown-linux-gnu.tar.gz" \
  "$RELEASE_DIR/release-manifest.json" \
  "$RELEASE_DIR/SHA256SUMS"
do
  gh attestation verify "$artifact" \
    --repo "$REPOSITORY" \
    --cert-identity "https://github.com/$SIGNER_WORKFLOW@$TAG_REF" \
    --source-ref "$TAG_REF" \
    --source-digest "$EXPECTED_COMMIT" \
    --signer-digest "$EXPECTED_COMMIT" \
    --cert-oidc-issuer https://token.actions.githubusercontent.com \
    --deny-self-hosted-runners \
    --predicate-type https://slsa.dev/provenance/v1
done
```

Inspect the workflow definition at `EXPECTED_COMMIT` before treating attestations
as sufficient for your threat model. They bind files to a GitHub workflow
execution; they do not prove production readiness or authorship outside GitHub.

## Meaning

Successful checksum and manifest inspection establishes consistency between the
downloaded files and the recorded tag/commit metadata. It does not establish
production readiness, legal compliance, or that the software is safe to run
without your own review.
