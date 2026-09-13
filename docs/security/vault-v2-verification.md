# Vault v2 engine (Program 1A) — evidence and honest readiness ledger

**Status:** Internal engine qualification only. Nothing here enrolls a product
workspace, turns on Vault v2 for founders, or substitutes for the conjunctive
activation gates in RFC 0005.

**Contract:** [RFC 0005](../../rfcs/0005-dual-root-vault-and-recovery.md) Program 1A  
**Gate:** `scripts/qualify-vault-v2.sh`  
**Plan:** `docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`

## What this document is for

To record what the **experimental engine process** demonstrates today, what is
measured rather than assumed, and what remains **Target** — so neither security
marketing nor a green CI job gets mistaken for founder-data protection.

There is no product “ready” flag here and no path to flip one on by editing
this file.

## The honest boundary

**Linking SQLCipher inside a dedicated engine binary is not workspace
confidentiality.** Program 1A deliberately does not move business payloads,
ledger bodies, workflows, authority stores, or legacy role keys into the v2
store. Until the persistence ledger in Task 6 and Programs 1B–1D complete,
founder data on disk stays where it already was; the engine crate is
`publish = false` and is not linked from `sovereign-cli`.

**This is not recovery.** Read-only recovery UI, dual-root wrappers, and
backup/restore qualification are later tasks. No recovery password surface
exists in Program 1A Task 1 evidence.

**This is not hardware-backed custody.** Native key-store jobs (Task 5) may
prove a software integration path only; they are not TPM or Secure Enclave
assurance.

**Bare `cargo` is not evidence** for this crate: ambient variables can reshape
the vendored SQLCipher/OpenSSL build (`crates/vault-v2-engine/build_gate.rs`).
The checked-in wrapper constructs a positive-allowlist child environment, pins
the admitted host triple, records tool digests, and runs `--frozen --offline`
after lock acquisition.

**Platform jobs prove `OsProtected` software integration only** (Task 5). Passing
`.github/workflows/vault-platform.yml` does not enroll a workspace, write
`vault.format`, or link the engine into `sovereign-cli`.

## Maturity label (current slice)

| Area | Label | Notes |
| --- | --- | --- |
| Engine crate skeleton + dependency pins | **Experimental** | `sovereign-vault-v2-engine` in workspace; not in product graph |
| SQLCipher 4.14.0 runtime pin | **Experimental** | `tests/public.rs` readback; not a connection factory yet |
| Source-closure / AST gate (partial) | **Experimental** | `tests/ast_gate.rs`; FFI exactness still queued |
| Build-time ambient override gate | **Experimental** | `build.rs` + `tests/build_gate.rs`; wrapper is the real gate |
| Native key-store roundtrip (isolated CI namespace) | **Experimental** | `vault-platform.yml` + `platform-qualifier` feature; not product enrollment |
| Linux staging durability (storage tests) | **Experimental** | `engine::storage::tests` via platform Linux job |
| Non-activation guards (`publish = false`, no CLI edge) | **Experimental** | `tests/non_activation.rs`, `check-vault-v2-non-activation.sh` |
| Product Vault v2 / `ActiveV2` | **Target** | RFC 0005 |
| Whole-workspace persistence ledger | **Target** | Task 6 (partial row coverage only) |
| Filtered backup / 1B1 restore qualification | **Target** | Separate program |
| Identity / role-key handoff (1C) | **Target** | RFC 0005 conjunctive gates |

## What Task 1 evidence covers today

Each row is backed by tests run through `scripts/qualify-vault-v2.sh full` (or
the focused `cargo` passthrough commands named in the Program 1A plan).

| Property | Where |
| --- | --- |
| Locked graph resolves bundled SQLCipher 4.14.0, not host SQLite | `tests/public.rs` |
| Vendored OpenSSL answers only after keying | `tests/public.rs` |
| Ambient dependency-shaping variables are rejected at build time | `tests/build_gate.rs`, `build.rs` |
| Recursive source closure is complete; unsafety outside declared boundary fails | `tests/ast_gate.rs` |
| Profile / leakage / hostile OpenSSL tests (named in plan) | `engine::sqlcipher::tests` in binary target |

## What is explicitly not claimed

- End-to-end encryption of founder workspaces or backups.
- Confidentiality of data in transit (no network layer in this crate).
- Recovery completeness or dual-root activation.
- Adversarial cross-crate invariants beyond what is already listed in
  `tests/adversarial` (vault-specific adversarial rows are later tasks).
- musl, wasm, embedded, or cross-compiled engine builds (only the three Task 5
  native triples are admitted: `x86_64-unknown-linux-gnu`,
  `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`).
- Hardware-backed key custody (TPM / Secure Enclave / measured boot).

## Operator notes

1. Acquire dependencies once per clean environment: `cargo fetch --locked` at the
   repository root (not inside the wrapper — the one-time unlocked transition is
   out of band per RFC 0005).
2. Run qualification: `./scripts/qualify-vault-v2.sh full`
3. The wrapper deletes its private `CARGO_TARGET_DIR` on exit; do not point it
   at a shared cache and treat the result as reproducible evidence.
4. Ordinary workspace CI may exclude this package; engine evidence must come
   from the wrapper job, not a bare `cargo test --workspace`.
5. Platform evidence: set `SFO_VAULT_PLATFORM_NAMESPACE=sfo-ci:<run>:<attempt>`
   and run `./scripts/run-vault-platform-qualifier.sh <host-triple>` on a native
   runner matching that triple.

## Merge / stack notes (v01-22)

- **PR #139** (`cursor/closed-kinds-legacy-importer-v01-21-5071`): importer and
  closed schema this ticket builds on.
- **PR #133** (`cursor/qualify-vault-v2-a4ac`): `qualify-vault-v2.sh` and Task 1
  evidence ledger — merged into this branch when absent on the base.

## Evidence manifest

`qualify-vault-v2.sh full` prints a manifest (tool paths, versions, SHA-256,
lock digest, steps run). Archive that output next to the CI job log when
recording qualification — it is not checked into git automatically.
