# Orchestrated handoff rounds

**Codex automated lane:** start at the [Luna scaffold entry](codex/README.md).
It owns the Codex task cards, model roles, review/escalation protocol and
controller-managed queue updates. The human-relayed procedure below remains
the legacy lane; do not mix its worker/owner responsibilities into a Codex card.

How a human-relayed **orchestrator** session dispatches bounded work to
**worker** sessions and receives a report back. The worker never decides
scope; the orchestrator never types implementation code; the owner relays
cards and reports between the two and stays the only one who pushes.

This complements, and does not replace, the queue rules in
[`docs/backlog.md`](../backlog.md), the conventions in
[`CLAUDE.md`](../../CLAUDE.md), and the `/iterate` skill. A task card is a
backlog item plus the orchestrator's guidance for landing it.

## Roles

| Role | Session | Does | Never does |
| --- | --- | --- | --- |
| **Orchestrator** | a larger-model session the owner talks to | chooses the next backlog item, writes the task card, reviews the report and the diff, runs the full gate, merges into `main`, dispatches the next card | writes implementation code, merges unreviewed work |
| **Worker** | one fresh Claude Code session per card (`.claude/settings.json` selects Sonnet and arms the Stop-hook gate) | follows the card, test-first, lands one commit series on its own branch, writes the handoff report | merges into `main`, pushes, opens PRs, widens or narrows scope, starts a second item |
| **Owner** | the human | pastes cards into worker windows and reports back to the orchestrator; runs `git push` when they choose | — |

## The task card

Every card carries, in this order:

1. **Id** `HO-NNN` and a short title.
2. **Base** — the `main` commit the worker must start from.
3. **Backlog item** — the exact entry title in `docs/backlog.md` (the item's
   done criteria are authoritative; the card may tighten them, never loosen).
4. **Goal** in two or three sentences, and why it is next.
5. **Scope** — directories that may change; **Out of scope** — what must
   not, even if tempting.
6. **Design guidance** — the orchestrator's reading of the code: functions
   to touch, shapes to keep, pitfalls, the RFC text that governs.
7. **Done when** — named tests and observable results.
8. **Gate** — the commands that must be green.
9. **Handoff** — branch name, report file name.

Cards live in the chat; the durable record is the backlog entry, the
commits, and the report file.

## Worker procedure (one card = one session = one branch)

1. **Verify the base.** `git status` must be clean and `main` checked out;
   `git rev-parse --short HEAD` must equal the card's base or be a descendant
   of it. If it is not, stop and report — do not guess.
2. **Branch.** `git checkout -b <prefix>/<slug>` with a prefix allowed by
   `CLAUDE.md` (`feature/`, `fix/`, `test/`, `docs/`, `refactor/`, `chore/`).
3. **Claim.** Mark the backlog entry `IN PROGRESS (<date>)` and commit that
   one-line edit first (`chore(backlog): claim <item>`).
4. **Test first, then implement** inside the card's scope. Anything found
   outside it becomes a new backlog entry, not a fix in passing.
5. **Gate.** `./scripts/test_changed.sh` must print `ALL GREEN` before every
   commit. Conventional commit messages; the owner's identity; no AI
   attribution of any kind (see `CLAUDE.md`, Git).
6. **Book-keep.** Check the entry off; add discovered problems as
   prioritized entries; add a `CLAUDE.md` lesson only for a durable,
   repo-general rule (cap 15).
7. **Report.** Write `docs/handoff/reports/HO-NNN-<slug>.md` from the
   template below, commit it (`docs(handoff): report HO-NNN`), then print
   the same report verbatim as the final chat message so the owner can relay
   it.
8. **Stop.** No merge, no push, no PR, no second item.

If the gate cannot go green this round: revert to the last green state,
remove the `IN PROGRESS` mark, write the diagnosis under the entry (keep
earlier diagnoses), and still write the report with `Outcome: blocked`.

## Handoff report template

Fixed headings, in this order; keep each section short and factual.

```markdown
# HO-NNN — <title>

- **Outcome:** landed | partial | blocked
- **Branch:** <name> · **Base:** <short hash> · **Head:** <short hash>
- **Commits:** <hash> <subject> (one per line)
- **Backlog entry:** <title> — checked off: yes | no

## What changed
<one line per file: path — what and why>

## Tests
<one line per test: name — the behavior it pins>

## Gate
<the exact `ALL GREEN` summary line, or the last lines of the failure>

## Deviations from the card
<none | what differed and why>

## Discovered problems
<none | each new backlog entry title, with its priority>

## Open questions for the orchestrator
<none | numbered>

## Suggested next item
<the backlog entry the worker thinks is now unblocked, with one reason>
```

## Orchestrator procedure

1. Read the report, then the diff (`git diff main...<branch> --stat`, the
   tests, then the implementation). Check the report against the diff — a
   report that claims a test that is not in the diff is a blocked round.
2. Run the full gate on the branch: `cargo test --workspace --locked`,
   `cargo clippy --workspace --all-targets --locked -- -D warnings`,
   `cargo fmt --all --check`, `./scripts/check-file-size.sh`.
3. Merge with `git merge --ff-only <branch>` (rebase the branch first if
   `main` moved) and delete the branch. Fix nothing during the merge — send
   it back as a follow-up card instead.
4. If a merged change moves a maturity claim, update `ROADMAP.md`,
   `README.md`, or the RFC in a separate `docs(...)` commit.
5. Write the next card.

## Concurrency

One worker per checkout. For parallel workers use
`git worktree add ../<name> -b <branch> main`; each worktree builds its own
`target/` (first build is slow) and claims its own entry. Two cards must
never name the same backlog entry.
