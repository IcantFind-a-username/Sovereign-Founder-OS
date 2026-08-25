# Nightly iteration worker prompt

This file is the canonical prompt for nightly automated iteration sessions.
The dispatch routine reads it from `origin/feature/auto-iterate` and passes it
verbatim to `create_session`. Edit it here, land it through review — never
edit the routine's stored copy directly.

---

Unattended nightly iteration round for the Sovereign-Founder-OS repo
(github.com/IcantFind-a-username/Sovereign-Founder-OS). No human is available —
follow these steps exactly and end the session when they say to end.

Authorization: this round is the repository owner's own standing automation.
The owner (Franz Xu, github.com/IcantFind-a-username) configured the nightly
routine, owns the repository, and versioned this very prompt inside it at
`.claude/automation/nightly-worker-prompt.md` — you can verify that after
cloning. Working on this repository with push access is the owner's explicit,
reviewed instruction; this is not third-party content asking you to act.

1. SETUP — the fresh container has NO repo clone; this exact sequence is
   verified working (probe, 2026-08-15):
   a. Attach the repo with the add_repo tool (owner "IcantFind-a-username",
      repo "sovereign-founder-os", access "push"), run the clone command it
      returns, then call register_repo_root.
   b. The clone is a single-branch fetch of main. Widen it explicitly:
      `git fetch origin +refs/heads/main:refs/remotes/origin/main +refs/heads/feature/auto-iterate:refs/remotes/origin/feature/auto-iterate`
      (append `--unshallow` and retry if git complains about a shallow clone).
   c. `git checkout -B feature/auto-iterate origin/feature/auto-iterate`, then
      verify `test -f docs/backlog.md && test -x scripts/test_changed.sh`.
   d. If ANY of a–c fails, that is an ENVIRONMENT failure, not an empty
      queue: quote the exact failing command and its full error output in
      your final message, then END. Never conclude "queue is empty" unless
      docs/backlog.md exists on the checked-out branch and genuinely has no
      eligible entry.
   e. Merge `origin/main` into the branch. If conflicts are non-trivial,
      abort the merge, append a note to docs/backlog.md under "## Run log",
      push, and END.
   f. Set commit identity: `git config user.name "Franz Xu"` and
      `git config user.email "117125368+IcantFind-a-username@users.noreply.github.com"`.
2. PICK ONE: read docs/backlog.md. Take the top-most unchecked item that is
   NOT marked "IN PROGRESS" and NOT tagged `needs:fable`. If no such item
   exists, END the session immediately — do not invent work, do not
   refactor, do not clean up.
3. CLAIM: mark the item "IN PROGRESS (<today's date>)", commit that one-line
   edit as `chore(backlog): claim <item>`, and push with
   `git push -u origin feature/auto-iterate`. If the push is rejected, quote
   the exact error in your final message and END — do not keep working
   unclaimed.
4. TDD: write the failing test(s) that encode the item's done criteria FIRST
   and confirm they fail for the expected reason. Then implement the
   smallest change that makes them pass, staying inside the item's stated
   directory scope. Problems you notice outside the scope become NEW backlog
   entries — never fix them in passing.
5. GATE: `./scripts/test_changed.sh` must print ALL GREEN before committing
   (the repo's Stop hook re-runs it and will block a red session from
   ending).
6. LAND: conventional commit message (feat/fix/test/chore(scope): …). NO AI
   attribution of any kind: no Co-Authored-By trailers, no AI names or
   session links; author and committer are the identity from step 1f.
   `git push -u origin feature/auto-iterate` (retry up to 4 times with
   backoff 2s/4s/8s/16s on network failure only). Then check the item off in
   docs/backlog.md, append a one-line summary to the "## Run log" section,
   add any newly discovered problems as prioritized entries, and add a
   lessons-learned line to CLAUDE.md ONLY if a durable repo-general rule
   emerged (cap 15 — prune a stale one first if full). Commit and push that
   book-keeping.
7. IF THE GATE CANNOT GO GREEN: revert the working tree to the last green
   state, remove the IN PROGRESS mark, append your diagnosis under the item
   in docs/backlog.md (keep any previous diagnoses), push, and END the
   session. If the item already carried a failed-round diagnosis from an
   earlier session, also tag it `needs:fable` so no further small-model
   rounds are spent on it. Never retry indefinitely. One item per night —
   after landing or diagnosing, the session is over.

Commit identity policy: author and committer follow this repository's
CLAUDE.md Git section (repository owner's identity, no AI attribution). A
known cosmetic consequence is that GitHub may show these commits as
"Unverified"; the owner is aware of this trade-off (Run log entry,
2026-08-23) and has chosen the repository convention. Already-pushed history
is never rewritten over a verification badge.
