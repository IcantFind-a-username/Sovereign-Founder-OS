# Open-Source Security Cross-Validation

**Status:** Research note
**Reviewed:** 2026-08-13
**Scope:** Architecture mechanisms and trust boundaries, using primary project
specifications, formal papers, RFCs, and official documentation only

## What this document does not prove

This review is not a security certification, formal verification of
Sovereign Founder OS, penetration test, third-party audit, or compliance
assessment. An upstream proof, audit, security model, or deployment result
applies only to the upstream version, configuration, assumptions, and system
boundary that it covers. None transfers automatically to this repository, its
dependencies, host integration, deployment, or operations.

The decisions below mean:

- **Adopt:** use the referenced standard or implementation in the same narrow
  role, subject to dependency review, tests, and release gates.
- **Adapt:** copy a design discipline or boundary, while treating the
  Sovereign implementation as new and independently reviewable.
- **Defer:** keep as `Research` until a demonstrated requirement and explicit
  gate justify the complexity.
- **Reject:** do not use the mechanism for the stated role or security claim.

`Current` means repository code and tests presently provide the named narrow
foundation. `Target` is an accepted but unshipped contract. `Research` is not
an implementation commitment. “Adopt” never changes `Target` to `Current` by
itself.

## Decision matrix

| System and primary source | Transferable mechanism | Decision and program state | Sovereign use | Boundary that does not transfer |
| --- | --- | --- | --- | --- |
| [seL4 proof assumptions](https://sel4.systems/Verification/assumptions.html) and [proof overview](https://sel4.systems/Verification/proofs.html) | State the exact trusted computing base, theorem, configuration, and residual assumptions; treat hardware, boot, DMA, and timing channels explicitly. | **Adapt — Target discipline. Defer — Research substrate.** | Maintain an assurance-case register for every security claim and its runtime assumptions. Revisit seL4 only if a high-assurance owned-node profile has requirements that justify a separate OS/application architecture. | seL4 verification covers specified kernel configurations and assumptions, not Sovereign code, ordinary Linux hosts, device firmware, integrations, or timing-channel freedom. Running an application “on seL4” is not whole-system verification. |
| [Capsicum capability model paper](https://www.usenix.org/legacy/events/sec10/tech/full_papers/Watson.pdf) | Enter a mode without ambient global namespaces; pass explicit object capabilities; attenuate rights monotonically. | **Adapt — Target.** Direct FreeBSD dependency is **Defer — Research**. | Keep brokers, opaque grants, pre-opened resources, and per-operation narrowing at the host boundary. Consider Capsicum as defense in depth for a future FreeBSD deployment. | File descriptors and capability mode do not define business authorization, approval, provenance, revocation, audit durability, or cross-platform isolation. |
| [Wasmtime security model](https://docs.wasmtime.dev/security.html) and [security-bug boundary](https://docs.wasmtime.dev/security-what-is-considered-a-security-vulnerability.html) | WebAssembly has no ambient system calls; host imports define authority. WASI filesystem access is capability-oriented. Engine limits and host-output handling remain embedder duties. | **Adopt — Current pure-compute foundation; Target hardening for effectful plugins.** | Continue the exact-version, import-free Core Wasm path for pure computation. Add only reviewed WIT worlds, per-call broker checks, memory/fuel/epoch/host-call/output ceilings, and inert presentation of guest output. | Wasmtime cannot validate business semantics, prevent an overpowered host import, confer authority, guarantee exactly-once effects, or make arbitrary native tools safe. Sandbox vulnerabilities and host bugs remain possible. |
| [SQLCipher security design](https://www.zetetic.net/sqlcipher/design/), [SQLCipher API](https://www.zetetic.net/sqlcipher/sqlcipher-api/), and [SQLite atomic commit](https://sqlite.org/atomiccommit.html) | Mature encrypted page/journal handling, per-page HMAC, transactional locking and crash recovery, raw random database keys, integrity checks, rekey, and backup building blocks. | **Adopt — Vault v2 Target. Reject — a new custom encrypted object store.** The released 4.14-backed Rust binding is limited to a non-product engine; backup and activation require the separately reviewed version gate below. | Pin SQLCipher and its crypto provider/profile; pass a random DBK through a small audited raw-key binding; keep temporary storage in memory; use full transactions and integrity/reopen tests. Device and Argon2id recovery domains wrap only the DBK with typed XChaCha envelopes. | SQLCipher does not protect a DBK stored beside the database, authenticate the owner, classify rows, filter backups, prevent valid-old rollback, encrypt non-database logs/exports/outbox files, or erase old copies. Its AES-CBC plus HMAC page construction is an upstream profile, not permission to invent a similar construction. SQLite durability still depends on documented VFS/filesystem assumptions. |
| [`keyring` 4.1.5 documentation](https://docs.rs/keyring/4.1.5/keyring/) and [repository](https://github.com/hwchen/keyring-rs) | Its v1 API selects macOS Keychain, Windows Credential Manager, or a Unix Secret Service provider by target/feature configuration. | **Adapt — Vault device-custody Target.** | Pin features, inspect the locked dependency tree on each target, and require isolated real-provider set/get/delete jobs. Label a passing generic OS store only `OsProtected`; a separate backend-specific review is required for `HardwareBacked`. | Automatic provider selection and a successful roundtrip do not prove hardware backing, non-exportability, application-specific ACL strength, user presence, or correct behavior when the service is locked/unavailable. Never fall back to a sample/file/env provider. |
| [TUF specification](https://theupdateframework.github.io/specification/latest/) | Separated root/targets/snapshot/timestamp roles, threshold trust, versioned metadata, consistent snapshots, expiry, and compromise recovery for software updates. | **Adopt — Target for releases, plugins, and update metadata.** | Use a conforming TUF implementation and repository profile for distributing signed binaries, plugin artifacts, policy packs, and trust-root rotations. | TUF metadata is not a runtime capability, user approval, code-safety proof, live Vault rollback anchor, or substitute for protecting an endpoint after installation. |
| [in-toto getting started](https://in-toto.io/docs/getting-started/) and [in-toto Attestation Framework](https://github.com/in-toto/attestation) | Signed layouts/link metadata and typed attestations describe who performed supply-chain steps and what materials/products resulted. | **Adapt — Target.** | Emit and verify build provenance, test, SBOM, and release attestations; bind accepted subjects to immutable artifact digests and the TUF target. | Provenance says how an artifact was produced. It does not make the artifact safe, authorize execution, approve data access, or prove a runtime fact merely because an attestation field says so. |
| [PySyft official documentation](https://docs.openmined.org/) | Govern remote computation over data held by a datasite, with review/approval of submitted code and returned results. | **Defer — Research.** | Study only if governed remote analytics over independently administered datasets becomes a validated product requirement. | PySyft's Python datasite and data-science topology is not the local Vault, the narrow owned-node job protocol, E2EE membership, or Sovereign's signed effect-authority model. Adopting it would add a large separate trusted surface. |
| [MCP specification](https://modelcontextprotocol.io/specification/2025-11-25), [authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization), and [security guidance](https://modelcontextprotocol.io/docs/tutorials/security/security_best_practices) | Interoperable tool/resource transport; remote authorization profiles require correct resource/audience binding and reject token passthrough; tool descriptions and annotations remain input from a server. | **Adapt — Target compatibility. Reject — authority source.** | Terminate MCP at a closed core broker. Convert a strictly validated request into Sovereign resource grants, policy checks, approval, one-use consumption, effect execution, and signed audit evidence. Treat all server content and tool output as untrusted/tainted. | An MCP connection, OAuth token, server identity, tool annotation, schema, consent UI, or model selection does not grant Sovereign authority or make returned content safe. Raw protected data is denied by default. |
| [Noise Protocol Framework](https://noiseprotocol.org/noise.html) | Composable authenticated handshakes and transport cipher states with explicit patterns, prologue, DH, cipher, and hash choices. | **Adapt — Research candidate; Implementation None.** | If a validated synchronous one-to-one owned-node use case emerges, specify and independently review the candidate `Noise_XX_25519_ChaChaPoly_BLAKE2s` profile, owner-verified static keys, transcript bindings, replay/sequence rules, padding, expiry, and re-pairing before promoting it to Target. | Noise does not supply membership governance, application authorization, exactly-once effects, queue durability, traffic-analysis resistance, or continuous post-compromise recovery merely by calling `Rekey()`. No E2EE claim exists before real two-node tests and review. |
| [RFC 9420 MLS](https://www.rfc-editor.org/rfc/rfc9420.html) and [OpenMLS](https://github.com/openmls/openmls) | Standardized asynchronous group key agreement, epochs, authenticated membership changes, forward secrecy, and post-compromise security goals. | **Defer — Phase 2 Research.** | Evaluate RFC 9420 and a pinned, reviewed OpenMLS integration only when real three-or-more-member, formal Add/Remove, or asynchronous-delivery requirements make pairwise Noise insufficient. | MLS is not a transparent upgrade from Noise. Delivery/authentication services, credential validation, persistent state, welcome/join handling, rollback, and application authority still require design and operations. Using OpenMLS does not audit that integration. |
| [age v1 specification](https://age-encryption.org/v1) | A small interoperable file-encryption format with X25519 recipient stanzas and a separate standard passphrase-recipient construction. | **Adopt — Vault backup Target. Reject — live Vault/session protocol and custom recipient.** | Encrypt one canonical padded immutable Vault snapshot to a dedicated offline X25519 recipient; verify interoperability with upstream age. Keep RFC 0005's Argon2id recovery chain inside the snapshot. | age recipient mode provides recipient confidentiality, not creator/sender provenance. It also does not hide recipient stanza type/count, total padded length, file timing, or endpoint access. Standard age passphrase mode uses scrypt; replacing it with a custom Argon2 stanza/plugin would leave the standard profile and is forbidden. |
| [RFC 9106 Argon2](https://www.rfc-editor.org/rfc/rfc9106.html) | Argon2id memory-hard password derivation with encoded version, salt, memory, time, and parallelism parameters. | **Adopt — Vault recovery Target.** | Derive an ephemeral 32-byte PWK that wraps an independent random Recovery KEK. Benchmark a versioned floor on the weakest supported device and enforce ceilings before allocation. | Argon2id does not turn a weak password into a random key, prevent offline guessing, provide device binding, encrypt Vault data, or serve as an authentication/authorization protocol. It is not the DBK. |
| [Apple Private Cloud Compute](https://security.apple.com/documentation/private-cloud-compute/), [core requirements](https://security.apple.com/documentation/private-cloud-compute/corerequirements), [no privileged runtime access](https://security.apple.com/documentation/private-cloud-compute/noprivilegedaccess), and [verifiable transparency](https://security.apple.com/documentation/private-cloud-compute/verifiabletransparency) | Minimize retained state and privileged access; make software identity attestable and externally inspectable; prevent operator targeting; design privacy-preserving observability. | **Adapt — future owned-node Target principles. Defer — Research system.** | For an owned-node profile, prefer allowlisted measured releases, no general production shell/debug path, ephemeral request keys/state, value-free telemetry, and independently inspectable release evidence. | PCC relies on Apple's custom hardware, fleet controls, attestation, transparency service, operational separation, and published threat boundary. Ordinary cloud VMs or user-owned nodes are not PCC-equivalent, and Sovereign MUST NOT imply they inherit its guarantees. |

### SQLCipher binding version gate

At this review date, released
[`rusqlite 0.40.2`](https://github.com/rusqlite/rusqlite/releases/tag/v0.40.2) /
`libsqlite3-sys 0.38.2` bundles SQLCipher 4.14.0, while
[SQLCipher 4.17.0](https://github.com/sqlcipher/sqlcipher/releases/tag/v4.17.0)
is the current upstream release and appears only in an unreleased rusqlite
revision. [SQLCipher 4.15.0](https://github.com/sqlcipher/sqlcipher/releases/tag/v4.15.0)
also fixed a defensive-mode bypass in `sqlcipher_export`. Program 1A therefore
must not silently equate “latest Rust binding” with “latest SQLCipher,” must not
use `sqlcipher_export`, dynamic `ATTACH`, or extension loading in the supported
path, and must record the exact runtime `cipher_version` and compile options.
A released 4.14.0-backed binding may be used only for the non-product engine
slice after a scoped advisory review; backup or product activation is blocked
until an accepted dependency source carries the reviewed upstream version. An
unreleased git revision is a separate supply-chain decision, not an automatic
upgrade.

The locked bundled build compiles SQLite's extension-loading machinery, and
the upstream build scripts accept environment variables that can change SQLite
limits/compile flags or defeat vendored OpenSSL selection. Sovereign therefore
does not claim those symbols are absent: the Vault build rejects dependency-
shaping ambient overrides, records provider/version/compile options, omits the
Rust loading feature, explicitly disables the C and SQL loading routes on every
connection, and exposes no raw connection or loading API. A runtime authorizer
is defense in depth, not a substitute for the build/profile gate.

SQLCipher documents that `sqlite3_key` applies the same input parsing rules as
`PRAGMA key`. Passing 32 arbitrary bytes is therefore not the specified raw-key
form. The binding must encode the 32-byte DBK as an in-memory `x'<64 hex>'`
blob literal, call `sqlite3_key` before the first database operation, verify the
schema and cipher integrity, and zeroize both the DBK and encoded buffer. This
avoids SQL interpolation while preserving SQLCipher's defined no-KDF raw-key
semantics.

## Layered decisions

The matrix deliberately assigns different mechanisms to different layers:

- **At rest:** RFC 0005 uses pinned SQLCipher for transactional business data;
  XChaCha20-Poly1305 only for closed DBK/Recovery-KEK wrappers; Argon2id for the
  recovery password path; explicitly labelled platform device protection; and age
  recipient-mode backups. None is a rollback anchor.
- **In transit:** Noise is the one-to-one `Research` candidate;
  MLS/OpenMLS remains group-communication `Research`. TLS, localhost, VPNs, storage encryption,
  and broker transport are not described as E2EE.
- **Execution:** the import-free Wasmtime slice is the `Current` pure-compute
  foundation. Effectful plugins remain `Target` and require broker mediation;
  Capsicum and seL4 are not assumed on ordinary deployments.
- **Supply chain:** TUF is the `Target` update trust system; in-toto is adapted
  for provenance. Neither grants runtime authority.
- **Interoperability:** MCP is a protocol adapter behind the broker, never a
  policy, trust, or declassification boundary.

## Claim discipline and validation gates

For every adopted or adapted mechanism, the implementation plan MUST record:

1. exact version, configuration, enabled features, and upstream source;
2. Sovereign-specific trust boundary and assumptions;
3. official/upstream interoperability or conformance vectors where available;
4. negative, downgrade, replay, resource-exhaustion, and failure-path tests;
5. dependency provenance, vulnerability response, and update owner;
6. what remains visible to hosts, relays, operators, plugins, endpoints, and
   local attackers; and
7. the evidence required before documentation changes from `Target` or
   `Research` to `Current`.

No project name, RFC number, formal proof, upstream audit, use of “capability,”
hardware feature, encryption primitive, or E2EE protocol may be used as a
blanket security claim. Claims attach to an exact implemented path and its
tested boundary.
