---
name: iterate
description: Run exactly one iteration round from docs/backlog.md — claim one task, write a failing test first, implement, pass all gates, commit, update the books. Use for every autonomous or routine development session on this repo.
---

# One iteration round

Execute the steps below in order. One round = one backlog item = one session.
When the round is done, stop — do not start a second item in the same session.

## 1. Claim one task

- Read `docs/backlog.md`. Take the highest-priority unclaimed item
  (top-most unchecked entry not marked 进行中/IN PROGRESS).
- If you are an unattended session on a small/default model, skip entries
  tagged `needs:fable` — those wait for a human-driven session on a larger
  model. Take the next untagged item instead.
- Mark it `IN PROGRESS (<date>)` and commit that one-line edit immediately
  (`chore(backlog): claim <task-id>`), so parallel agents don't collide.
- If the backlog is empty: report that and end the session. Do not invent work.

## 2. Test first

- Write the failing test(s) that encode the item's done criteria BEFORE the
  implementation. Run them; confirm they fail for the expected reason.
- Scope exploration tightly: the backlog item names the directories in scope.
  If you must explore beyond them, send a subagent and have it return
  conclusions only — never pull whole files into this session's context.

## 3. Implement

- Smallest change that makes the tests pass. Stay inside the item's stated
  scope. No drive-by refactors, no renames "while you're there", no fixing of
  unrelated problems — new problems you notice go to `docs/backlog.md` as new
  entries instead.
- Respect the non-negotiable principles in `CLAUDE.md` (policy is code, no
  self-authorization, evidence for sensitive actions, honest maturity labels).

## 4. Gate

- Run `./scripts/test_changed.sh`. It must print ALL GREEN.
  (The Stop hook runs it again anyway — a session cannot end red.)
- If you cannot get it green this round: revert to the last green state,
  write the diagnosis under the backlog item, and end the session. Do not
  loop indefinitely. If the item already carries a diagnosis from a previous
  failed round, also tag it `needs:fable` (see backlog rules) so no further
  small-model rounds burn on it.

## 5. Commit and book-keep

- Conventional commit (`feat(scope): …` / `fix(scope): …` / `test(scope): …`).
  No AI attribution of any kind (see CLAUDE.md Git rules).
- In `docs/backlog.md`: check the item off; add any newly discovered problems
  as new prioritized entries.
- If this round taught a durable, repo-general rule, add it to the
  "Lessons learned" list in `CLAUDE.md` (cap 15 — remove a stale one first
  if full). Session-specific details do not qualify.
- Push with `git push -u origin <branch>` per the session's branch rules.
