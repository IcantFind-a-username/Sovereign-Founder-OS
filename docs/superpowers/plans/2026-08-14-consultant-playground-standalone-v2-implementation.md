# Consultant Playground Standalone v2

**Status:** Target; additive product-learning slice.

**Goal:** Let an independent consultant practice one fixed
Company → Offer → Lead/Customer → Discovery thread, two fixed changes,
compiled search, and deterministic guidance without reading, accepting,
persisting, or exporting real business data.

This plan adds `sovereign playground --port 7788`. It preserves the current
Experimental `sovereign ui` product skeleton and all of its routes, assets,
Workspace behavior, export, disclosure, integrity, and offline verification
paths byte- and behavior-compatibly. That preservation is continuity, not a
security endorsement.

## Sources and boundary

- [Security Architecture Program](2026-08-13-security-architecture-program.md)
- [Architecture](../../../ARCHITECTURE.md)
- [Roadmap](../../../ROADMAP.md)
- [Threat model](../../../THREAT_MODEL.md)

```text
compiled fixture constants
        |
        v
publish=false sovereign-consultant-playground leaf
  private non-serializable graph
        |
        v
process-local session -> one-way serializable read model
        |
        v
exact loopback routes and embedded assets

sovereign playground --port 7788 -> leaf run(7788)
```

The leaf has no dependency on Workspace, Vault, `dirs`, authority,
capability, policy, model, effects, audit, workflow, or the CLI. It has no
filesystem, environment, process, clock, randomness, outbound-network, import,
export, or generic backend API. `tiny_http` is used only for one inbound server
bound to literal `127.0.0.1`. It has no product authority and cannot read or
mutate product state.

The command does not open a browser. It accepts only built-in Clap parsing of
`--port <u16>` with default `7788`; it accepts no root, path, Workspace,
business value, configuration file, hidden flag, custom parser, or environment
backend selector.

## Exact synthetic domain

`ConsultantPlaygroundGraph` is private and never implements `Serialize`,
`Deserialize`, `save`, `load`, `export`, or a conversion to Workspace. Its
compiled fixture is exactly:

| Entity | Fixed value |
| --- | --- |
| Company | North Star Operations |
| Offer | Reporting clarity sprint; initial price $2,500 |
| Relationship | Acme Ltd; Alex Chen; `alex.chen@example.test`; Lead |
| Discovery | Weekly reporting takes six hours; budget $3,000–$5,000; finance must approve; next step is a 30-minute scoping call |

Human-language teaching facts are closed semantic keys with complete compiled
English and Simplified Chinese catalogs. Names, the `.example.test` address,
and numeric money are the only shared literals.

Allowed actions are exactly:

```text
CorrectOfferPrice       $2,500 -> $3,500
PromoteAcmeToCustomer   Lead -> Customer
ShowReportingSearch     pure read
Reset                   reconstruct exact fixture
```

No action accepts a string, ID, timestamp, price, status, query, patch, or
arbitrary JSON value. Search and guidance are read-only. Restart reconstructs
the exact fixture; it is not persistence, recovery, or resume.

`PlaygroundReadModel` is a one-way serializable projection. Responses always
state `profile = synthetic_playground`, `real_data_enabled = false`, and
`persistence = none`. There is no constructor or conversion from a response
DTO back into the graph or any product type.

## Exact HTTP and asset surface

The leaf serves only:

| Method | Route |
| --- | --- |
| GET | `/` |
| GET | `/assets/styles.css` |
| GET | `/assets/i18n.js` |
| GET | `/assets/app.js` |
| GET | `/assets/consultant-ui.js` |
| GET | `/favicon.svg` |
| GET | `/api/playground/consultant` |
| POST | `/api/playground/consultant/action` |

There are no wildcard routes, aliases, query actions, uploads, forms for
business data, cookies, CORS, telemetry, analytics, remote resources, or
browser storage for exercise state. The POST body is a deny-unknown-fields
object containing one closed unit action and is limited to 256 bytes. JSON and
404 responses are typed and `Cache-Control: no-store`.

The browser uses `textContent`, literal same-origin endpoints, local embedded
assets, semantic HTML, keyboard-visible focus, 44-pixel targets, reduced-motion
support, and a single-column 375-pixel layout. Beginner copy says plainly:

> Practice with this example only. You cannot enter or save your own business
> or customer data here. Real-data setup is unavailable in this preview.

The Simplified Chinese catalog carries the same meaning. Trust-program names
appear only in optional advanced details.

## Product compatibility contract

The implementation may add the leaf package, its assets/tests, one CLI
dependency, and one additive `Playground` command/arm. It must not modify:

- `Commands::Ui`, its help, port/default, `--no-open` flag, or match arm;
- `apps/cli/src/ui.rs` or `apps/cli/assets/**`;
- `apps/cli/src/workspace/**` or Workspace bytes and behavior;
- `/api/export`, `/api/verify-export`, `/api/state`, disclosure browsing,
  integrity, or Workspace routes;
- `verify-export`, `integrity`, or any other existing command behavior.

The top-level help changes only by listing the new `playground` command.
Snapshot and process regressions pin the existing `ui` help/flags and current
route transcripts before the additive change.

A future authenticated product router remains Target. It may accept real
Company/Offer/Relationship/Discovery values only when the actual landed
`OwnerSession` authorization and actual `ActiveV2` protected storage selector
are construction requirements. A Boolean, header, warning acknowledgement,
environment flag, fake grant, or fixture type cannot substitute for either.

## Implementation tasks

Every behavior task uses genuine RED → minimal GREEN → focused gate →
existing-product regression → independent review. Capture the failing test
before code. Keep commits small and do not mix the owner/effect fixture into
this branch.

### Task 1 — Add the physical leaf and non-serializable graph

Create `crates/consultant-playground` as `publish = false`. Register only the
new workspace member/dependency and lockfile local-package edge. Create the
private graph, closed semantic-key enum, exact fixture, and process-local
session; do not add actions, HTTP, or assets yet.

RED tests prove the exact fixture, deterministic reconstruction, graph
invariants, absence of serde/persistence/product types, and exact package
dependency surface.

Suggested commit: `feat(playground): add fixed consultant teaching graph`.

### Task 2 — Add closed actions, read models, localization, search, and guidance

Implement only the four actions above. Add one-way response DTOs, exhaustive
English/Chinese catalogs, fixed reporting search, and ordered guidance:
correct price, promote lead, follow the recorded scoping call, then complete.

RED tests compare every graph field before/after each action, prove search and
guidance do not mutate, reject arbitrary values, exercise every fixture state,
verify catalog/placeholder parity, and prove no response converts into a graph
or Workspace.

Suggested commit: `feat(playground): add fixed actions search and guidance`.

### Task 3 — Add typed leaf HTTP contracts

Add the deny-unknown-fields action request, stable error codes, bounded raw-body
parser, and a handler whose only field is `Mutex<PlaygroundSession>`. Keep it
inside the leaf; do not change CLI code in this task.

RED tests cover exact success/error bodies and headers, empty/malformed/
duplicate/unknown/oversized input, every disallowed method and Host, no CORS,
and construction without a root, Store, Vault, authority, or backend.

Suggested commit: `feat(playground): add typed synthetic http contracts`.

### Task 4 — Serve exact assets and add the standalone command

Add the exact server/route manifest and six embedded assets under the leaf.
Add `Commands::Playground { port }` and the direct
`sovereign_consultant_playground::run(port)?` arm. Preserve the existing `Ui`
variant and arm exactly and do not launch a browser.

RED tests prove:

- one literal loopback bind and the exact route/asset graph;
- browser calls only the two Playground endpoints;
- no business input, persistence, remote resource, or hidden network API;
- `sovereign playground --help` exposes only `--port` with default 7788;
- `sovereign ui --help`, flags, startup arguments, route behavior, assets,
  export, disclosure, integrity, and Workspace tests are unchanged;
- leaf production sources have no Vault/Workspace/`dirs`/authority,
  filesystem, environment, process, outbound-client, or include escape;
- two real Playground processes under two content-distinct fake platform-data
  roots return byte-identical complete HTTP transcripts and stdout/stderr,
  contain no canary, and leave both roots recursively unchanged.

Suggested commit: `feat(cli): add standalone consultant playground`.

### Task 5 — Attack, accessibility, and product handoff

Run an independent attack review over route closure, raw HTTP, source/dependency
inventory, two-root process isolation, asset references, hostile rendered
strings, action replay, and product compatibility. Record manual accessibility
and five-consultant usability protocols as Target until real observations
exist; never invent results.

Update Current product docs only after code lands, using at most:

> Synthetic consultant Playground: one fixed in-memory
> Company/Offer/Lead/Discovery example, two fixed changes, compiled bilingual
> search, and read-only guidance. It cannot accept, save, inspect, verify, or
> export your business data.

Suggested commit: `test(playground): review standalone isolation`.

## Verification gates

Focused gates must enumerate expected test names and reject zero/skipped tests.
The final branch gate is:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build -p sovereign-cli --release --locked
npx -y -p typescript@5.5.4 tsc \
  -p crates/consultant-playground/assets/tsconfig.json
./scripts/check-file-size.sh
git diff --check
```

Additionally compare the pre/post `sovereign ui --help` bytes and current UI
route transcript, run the full Workspace/export/disclosure/integrity tests, run
the two-root real-process test, inspect `cargo tree` for the leaf's exact
dependencies, and scan the product diff to prove `apps/cli/src/ui.rs`,
`apps/cli/assets/**`, and `apps/cli/src/workspace/**` are unchanged.

## Exit criteria

1. A non-specialist can understand and practice the fixed consultant thread in
   English or Simplified Chinese without Trust Layer terminology.
2. The graph is compiled, non-serializable, process-local, and accepts no real
   business value or storage selector.
3. Search/guidance cannot mutate, and the UI can request only four closed
   actions.
4. Two fake data roots remain byte-identical while real Playground processes
   return identical canary-free transcripts.
5. The current Experimental UI, export, disclosure, integrity, Workspace, and
   offline verifier remain behavior-compatible.
6. The Playground makes no AI, owner-authentication, Vault, E2EE, recovery,
   persistence, or production-safety claim.
7. Persistent real-data integration remains blocked on actual OwnerSession and
   `ActiveV2` types, not a substitute flag or fixture grant.
