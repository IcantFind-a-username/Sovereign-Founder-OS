---
name: plan-feature
description: Decompose a large feature into single-round backlog items in docs/backlog.md. Planning only — implementation is forbidden in this skill. Every feature too big for one medium commit MUST pass through this step before any code is written.
---

# Plan a feature into backlog items

This skill produces backlog entries, nothing else. Writing or editing any
source file, test, or config during this skill is a violation — if you catch
yourself implementing, stop and put it in the backlog instead.

## Steps

1. **Understand the ask.** Restate the feature in one paragraph: user-visible
   behavior, affected subsystems, what is explicitly out of scope.
2. **Scout the code.** Identify the crates/directories involved. Use
   subagents for exploration and have them return conclusions only. Check
   `rfcs/` and `ARCHITECTURE.md` — if the feature contradicts an accepted
   RFC, stop and surface that instead of planning around it.
3. **Slice.** Cut the feature into items where each item:
   - is completable in ONE `/iterate` round (≈ one medium commit);
   - names its directory-level scope (`crates/x`, `apps/cli/src/y`);
   - has done criteria that a test can verify (name the test you expect);
   - leaves the workspace green when it lands — no item may depend on a
     future item to compile or pass tests. Order items so each builds on
     landed work.
4. **Write the entries** into `docs/backlog.md` in dependency order, using
   the file's entry format, with priorities relative to the existing queue.
5. **Report** the slice list to the user for approval. Do not start
   implementing any of it — the next `/iterate` session picks up item one.
