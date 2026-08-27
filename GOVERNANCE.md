# Governance

## Project Structure

Sovereign Founder OS is an open source project governed by its maintainers and community, with transparency as a core value.

## Maintainers

Initial maintainer: project founder (see GitHub repository owner).

As the project grows, additional maintainers will be added based on sustained, high-quality contributions — especially in security-critical areas.

## Decision Making

### Routine Changes

- Bug fixes, documentation, tests: maintainer review and merge
- Non-controversial features within current roadmap stage: maintainer discretion

### Significant Changes

Require RFC process (see [CONTRIBUTING.md](CONTRIBUTING.md)):

- Architecture changes affecting trust boundaries
- Cryptographic design changes
- Policy engine rule format changes
- New official Jurisdiction Packs marked "Verified"

### Security Changes

- Dual review required once team size permits
- Security advisories coordinated through [SECURITY.md](SECURITY.md) process
- No security fixes behind paywalls

## Publication Classes

### SemVer product releases

Product releases use semantic versions and ship supported product artifacts.
Their release gate remains unmet until an implementation enforces all tests and
security scans, changelog review, checksums, dependency-scoped SBOMs, signed
artifacts, build provenance, and verification instructions. Pre-Alpha product
versions will use `0.x.y`; a workspace package version is not by itself a
published release.

Automatic `v*` publication is disabled until that separate product-release
workflow exists and the applicable [roadmap](ROADMAP.md) gates pass.

### Provenance snapshots

Provenance snapshots use `provenance-YYYY.MM.DD.N`. They preserve one exact
source tree, selected history, metadata, and verification assets as evidence of
repository chronology. They contain no product binaries, do not satisfy or
waive product-release gates, and do not imply production readiness or security
certification.

The publication design and current status are recorded in
[PROVENANCE.md](PROVENANCE.md). An unsigned CI dry run is not a published
snapshot.

## Verified Jurisdiction Packs

A Jurisdiction Pack may be marked **Verified** only when:

- Rules traceable to legal/tax sources with effective dates
- Test cases cover common scenarios
- At least one qualified local professional has reviewed the pack
- Uncertainty and escalation paths are documented

Unverified packs are labeled **Draft** and must not be presented as legal or tax advice.

## Trademark

Use of project trademarks is governed by [TRADEMARK.md](TRADEMARK.md). Governance decisions do not grant trademark rights.

## Commercial Services

Commercial offerings (encrypted sync, managed recovery, verified packs, enterprise support) must:

- Not degrade the free core's security or functionality
- Not require uploading plaintext business data to official servers
- Not be the only recovery path

## Future: Foundation or SIG

When external contributors hold real governance authority (Stage 7 exit criteria), the project may transition to a formal governance structure (e.g., technical steering committee or foundation). This will be proposed via RFC with community input.

## License

Project code and documentation: [Apache 2.0](LICENSE).
