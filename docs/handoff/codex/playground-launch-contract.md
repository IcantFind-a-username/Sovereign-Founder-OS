# Playground CLI launch and real browser acceptance

**Revision 1 · Independently reviewed and Frozen by controller. Dispatch only
after the dependencies below are accepted.**
Design baseline `8e1e840`; S1-06's [accepted evidence](reports/S1-06-supervised.md)
proves embedding, Node checks and static review, not a running browser.
[Server](playground-server-contract.md), [assets](playground-assets-contract.md)
and [HTTP](playground-http-contract.md) contracts retain their frozen behavior.
This is the launch/acceptance increment of [standalone-v2](../../superpowers/plans/2026-08-14-consultant-playground-standalone-v2-implementation.md),
not another product plan. RFC 0004/0005 boundaries and actual OwnerSession /
ActiveV2 prerequisites remain unchanged. The [full MVP goal](models-and-goals.md#131-当前推荐-goal交付全流程可演示-mvp)
continues beyond this fixed synthetic S1 leaf.

## Accepted dependency correction

Retain historical IDs; execute **S1-07 → S1-09 → S1-08 → S1-10 → S1-11**.
S1-09 requires accepted S1-07 and its stopped writer. S1-08 requires accepted
S1-09 and therefore the actual release CLI, not a temporary launcher or a test
child presented as product. S1-10 retains the independent two-root isolation
proof. S1-11 remains the final runnable-Playground review. Controller updates
the existing index/backlog links after review; these docs do not claim them done.

## Exact CLI interface and compatibility

Add this sole variant adjacent to `Ui` in `apps/cli/src/main.rs`:

```rust
/// Run the fixed synthetic consultant Playground on 127.0.0.1
Playground {
    /// Port to bind on loopback
    #[arg(long, default_value_t = 7788)]
    port: u16,
},
```

Add only this dispatch arm:

```rust
Commands::Playground { port } => sovereign_consultant_playground::run(port)?,
```

No root calculation, `data_dir()`, environment access, browser opening, extra
state construction or error-to-success wrapper occurs in that arm. Clap's
built-in u16 parser accepts 0 through 65535, including port 0 for ephemeral
binding. `--help` is Clap's normal help; the only application option is
`--port`, default 7788. Unknown flags/positional values, negative/non-integer/
overflow ports are parsing errors. No `--root`, `--no-open`, hidden selector,
configuration file, input value or custom parser is added to this command.

Add `sovereign-consultant-playground = { workspace = true }` to
`apps/cli/Cargo.toml` normal dependencies. The root workspace dependency already
exists. Cargo.lock changes only the `sovereign-cli` package's dependency list;
no new registry package/version/feature is required for this wiring.
S1-07 already owns httparse's package and leaf edge.

`Commands::Ui`, its doc comments/attributes/default 7787/`no_open`, and
`Commands::Ui { port, no_open } => ui::run(port, data_dir(), !no_open)?,`
remain byte-identical. No edits to `ui.rs`, old assets, Workspace, existing
command bodies or hidden compile-worker command. Top-level help gains the
Playground entry; existing subcommand help remains byte-identical.

Observed gate constraint: `tests/support/manifest.rs::manifest_boundary`
selects the leaf package and validates its outgoing dependencies and targets.
It does not prohibit the CLI depending on the leaf. No gate amendment is
needed for this edge. S1-G07's separate httparse/server amendment must already
be accepted; it is not authority to relax any check for CLI wiring.

## Test and evidence boundary

S1-09 adds terminal `#[cfg(test)] mod cli_tests;` and sibling
`apps/cli/src/cli_tests.rs`. Reuse Clap `Parser` / `CommandFactory` and standard
assertions for command/help/port matrices; no general CLI parser or snapshot
framework. Pin Ui variant/arm source blocks and run existing CLI/Workspace/
export/disclosure/integrity tests. Capture actual baseline and candidate
`sovereign ui --help` stdout/stderr/exit status and compare them byte-for-byte.
Protected legacy source/asset trees must have an empty diff against the dispatch
base. Existing route regressions plus identical source are compatibility
evidence, not a new live-route transcript; final standalone-v2 acceptance still
requires the process/route evidence in S1-10/S1-11.

S1-09 real process smoke calls the shipped `sovereign playground --port 0`,
reads S1-07's exact `Playground: http://127.0.0.1:<actual_port>` stdout line,
uses that actual authority for GET/assets/one action/GET/Reset, checks nonzero
exit on occupied port, and terminates/reaps every child. No user data root is
passed or modified; process isolation against fake roots remains S1-10.
Controller selected recorded terminal-tool smoke for this small CLI increment.
Use existing process/HTTP tools (for example exec sessions and curl), with bounded
requests and explicit shutdown/exit confirmation. This is manual process evidence,
not a new Rust test or helper library. Do not copy S1-07's test process/parser
framework across crates or add a production test backdoor. The three automated
Clap/compatibility tests plus inherited S1-07 transport tests remain required.

S1-08 uses real browser automation through the executor's preferred
`ego-browser` skill. Read its installed SKILL.md at execution. Browser tool
availability is an execution prerequisite; simulated DOM/Node cannot replace
viewport, focus or network evidence. No browser session has been run by this
design. Browser-only route interception may delay/abort or replace responses
for negative tests; do not add server endpoints/flags or change production
behavior for fault injection. Save actual observations, not expected outcomes.

## Local preview instructions after S1-09 acceptance

From the repository root:

```bash
cargo run -p sovereign-cli -- playground --port 7788
```

Open `http://127.0.0.1:7788` manually in a browser. The process prints the exact
URL and stays running. Stop with Ctrl-C in its terminal. If 7788 is occupied,
choose an unused port or `--port 0` and open the printed actual URL. The server
binds IPv4 loopback, not an external interface. Build/run the distributable
binary with:

```bash
cargo build -p sovereign-cli --release --locked
./target/release/sovereign playground --port 7788
```

Practice the fixed price correction, Lead → Customer, reporting search and
Reset in English or Simplified Chinese. Language changes make no request.
Reset restores the fixture and keeps the selected language; page reload uses
GET to reconcile server state and resets browser-only language/view state;
process restart restores the fixture. On a failed request, reload the page
before continuing; the UI cannot infer whether a disconnected POST applied.
There is no business input, saving, model call, real-data setup or export.
This preview is not acceptance of the full business/AI MVP.

## Remaining decisions and release control

Controller independently accepted the dependency reorder and terminal-tool smoke,
fixing S1-09 to four files. Bind each card to its accepted predecessor/base before
dispatch. Browser automation APIs are selected from the actual skill. No business
behavior is unresolved or delegated to a worker. Missing tool capability or an
out-of-scope defect must be recorded accurately; it does not authorize changing
production boundaries or inventing passing evidence.

Tool inventory: `Commands`/`main`/`Ui` dispatch; public leaf `run(u16)` and
literal startup URL; accepted HTTP/assets and S1-07 transport support; Clap;
existing RustLexer/source_closure/manifest boundary; Node UI tests and
`test_changed.sh`. No new production tool. Reuse these; do not reimplement
equivalents. Declare any new helper before adding it.
