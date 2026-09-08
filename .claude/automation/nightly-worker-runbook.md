# Nightly iteration runbook

Operating procedure for the scheduled nightly iteration round. The spawn
prompt (see `nightly-worker-prompt.md`) intentionally carries only the clone
and checkout steps; everything about how a round runs lives here, in the
repository, where it is versioned and reviewed like any other change.

## The round

1. **Finish setup.** With `feature/auto-iterate` checked out: merge
   `origin/main`. If conflicts are non-trivial, abort the merge, append a
   note under "## Run log" in `docs/backlog.md`, push, and end the session.
   Set the commit identity required by CLAUDE.md's Git section:
   `git config user.name "Franz Xu"` and
   `git config user.email "117125368+IcantFind-a-username@users.noreply.github.com"`.
2. **Pick one item.** Read `docs/backlog.md` — its "Rules" section governs.
   Take the top-most unchecked entry that is not marked IN PROGRESS and not
   tagged `needs:fable`. No eligible entry → end the session without
   inventing work. An IN PROGRESS mark older than 3 days with no commits
   referencing the item is stale: remove it, note the reclaim, and the item
   is eligible again.
3. **Claim.** Mark the item `IN PROGRESS (<date>)`, commit that one-line
   edit as `chore(backlog): claim <item>`, push. If the push is rejected,
   quote the exact error in the final message and end — never work
   unclaimed.
4. **Test first.** Write the failing test(s) that encode the item's done
   criteria, confirm they fail for the expected reason, then implement the
   smallest change that makes them pass, inside the item's stated directory
   scope. Anything discovered outside that scope becomes a new backlog
   entry — it does not get fixed in passing.
5. **Gate.** `./scripts/test_changed.sh` must print ALL GREEN before any
   commit. The repo's Stop hook re-runs it.
6. **Land and book-keep.** Conventional commit message; commit conventions
   (identity, no AI attribution, no PRs) are CLAUDE.md's Git section, which
   governs. Push to `feature/auto-iterate` (on network failure retry up to 4
   times, backoff 2s/4s/8s/16s). Check the item off, append one Run-log
   line, add newly discovered problems as prioritized entries, and add a
   CLAUDE.md lessons-learned line only for a durable repo-general rule
   (cap 15 — prune first if full). Commit and push the book-keeping.
7. **If the gate cannot go green:** revert to the last green state, remove
   the IN PROGRESS mark, append the diagnosis under the item (keep earlier
   diagnoses), push, end. If the item already carried a failed-round
   diagnosis, also tag it `needs:fable`. Never retry indefinitely. One item
   per round — after landing or diagnosing, the session is over.

## Standing notes

- **Environment failure ≠ empty queue.** If setup fails (fetch, checkout,
  missing `docs/backlog.md` or `scripts/test_changed.sh`), quote the exact
  failing command and its full error in the final message and end. "Queue is
  empty" may only be concluded from an existing, readable backlog with no
  eligible entry.
- **Commit identity.** Author and committer follow CLAUDE.md's Git section
  (repository owner's identity). A known cosmetic consequence is that GitHub
  may show these commits as "Unverified"; the owner accepted this trade-off
  (Run log, 2026-08-23). Already-pushed history is never rewritten over a
  verification badge.
