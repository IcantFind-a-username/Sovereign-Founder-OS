# Agent memory — Sovereign Founder OS

Shared memory for AI-assisted sessions (Claude Code and Codex read the same
file; `AGENTS.md` is a symlink here). Keep this file lean: durable rules only.

## What this project is

A local-first, open-source AI operating system for running a one-person
company without surrendering data, decisions, or authority to any model,
plugin, or provider ("Mutually Constrained Autonomy"). Rust workspace,
Developer Preview maturity.

## Repo map

- `crates/` — 14 runtime crates, package name = `sovereign-<dir>`:
  `contracts` (canonical signed types), `policy`, `capability`, `authority`,
  `identity`, `vault`, `audit-ledger`, `sandbox` (wasmtime), `model`
  (gateway), `effects`, `execution`, `artifact`, `workflow`,
  `consultant-playground`.
- `apps/cli` — `sovereign-cli` binary + zero-dependency web frontend under
  `apps/cli/assets/` (JSDoc-typed JS, checked with `tsc --checkJs`).
- `tests/adversarial` — cross-crate security-invariant suite
  (`sovereign-adversarial-tests`).
- `rfcs/` — accepted designs; with current code, the source of truth over
  all other docs. `docs/INDEX.md` maps the rest.
- `scripts/` — CI-enforced guardrails; `docs/backlog.md` — the task queue.

## Verified commands (all green as of 2026-08-14)

```bash
./scripts/test_changed.sh        # scoped gate: run this one during iteration
cargo test --workspace --locked  # full suite (~198 tests, 39 targets)
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
./scripts/check-file-size.sh     # god-file limit: 1200 rs / 800 frontend
npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/tsconfig.json  # frontend
```

CI (`.github/workflows/ci.yml`) runs all of the above plus
`cargo build -p sovereign-cli --release --locked`, dependency audit, and
secret scanning. Toolchain is pinned: 1.97.0.

## Non-negotiable principles (full text: MANIFESTO.md, THREAT_MODEL.md)

- AI must not authorize itself: "what the model suggests" and "what the
  system allows" stay separated; sensitive authority comes from policy code
  and/or explicit human approval.
- Policy is code, not a prompt — deterministic, testable, enforced outside
  the model. Authority must expire (narrow, time-bound, revocable).
- Every important action leaves durable, tamper-evident evidence.
- Plugins/tools/model output are untrusted by default; local-first,
  user-controlled state; no single point of failure; recovery before
  autonomy.
- Never claim absolute security. Keep maturity labels honest: current vs.
  target vs. simulated.

## Git

- Branch prefixes: `feature/`, `hotfix/`, `fix/`, `chore/`, `docs/`,
  `refactor/` only. Never `claude/`, `ai/`, or other tool-named prefixes.
- No AI attribution anywhere in commits: no `Co-Authored-By` trailers, no AI
  names/emails as author or committer, no session links. Author and
  committer are the repository owner's identity.
- Conventional commit messages (`feat(scope): …`, `fix(scope): …`).
- No pull requests unless explicitly asked.

## Iteration rules

- Work comes from `docs/backlog.md` — one item per session, claim it first,
  test-first, stay in the item's stated scope (see `/iterate`).
- Features too big for one medium commit go through `/plan-feature` before
  any code is written.
- `./scripts/test_changed.sh` must be green before any commit; the Stop hook
  re-runs it and blocks red sessions from ending.
- Exploration beyond the item's scope goes to subagents that return
  conclusions only; never paste large logs or whole files into context.
- Split modules before they hit the size limit — the limit is a ceiling,
  not a target.

## Lessons learned (cap 15 — add only durable rules, prune stale ones)

1. `git ls-files --others --exclude-standard` is the only thing that catches
   brand-new untracked files in a change set; plain `git diff` misses them.
2. Test files count toward the 1200-line god-file limit too
   (`check-file-size.sh` globs `crates/**/*.rs`) — split big test files
   per-concern before adding cases.
3. Serde field names of signed types (`crates/contracts`) are load-bearing
   for signature verification — renaming a field is a breaking change even
   when the compiler is happy.
4. Clippy runs with `-D warnings` including on test targets; even a
   scratch test with `assert!(false)` fails the gate at compile time.
