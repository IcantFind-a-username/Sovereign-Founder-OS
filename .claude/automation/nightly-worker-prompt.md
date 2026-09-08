# Nightly iteration worker prompt

This file is the canonical spawn prompt for nightly automated iteration
sessions. The dispatch routine reads it from `origin/feature/auto-iterate`
and passes everything below the `---` separator verbatim to
`create_session`. It is deliberately minimal: it only gets the session to a
checked-out working copy, then hands over to the repository's own operating
documents — `.claude/automation/nightly-worker-runbook.md` (the round's
procedure), `docs/backlog.md` (queue rules), `CLAUDE.md` (conventions) —
which are versioned and reviewed here like any other change.

---

Scheduled maintenance round for the repository
IcantFind-a-username/sovereign-founder-os, set up by the repository owner as
a nightly routine.

Setup: attach the repository with the add_repo tool (owner
"IcantFind-a-username", repo "sovereign-founder-os", access "push"), run the
clone command it returns, then call register_repo_root. The clone is a
single-branch fetch of the default branch; widen it:

    git fetch origin +refs/heads/main:refs/remotes/origin/main +refs/heads/feature/auto-iterate:refs/remotes/origin/feature/auto-iterate

(add `--unshallow` and retry if git reports a shallow clone), then

    git checkout -B feature/auto-iterate origin/feature/auto-iterate

If attach, clone, fetch, or checkout fails, state the exact failing command
and its error output in your final message and end the session.

Then read `.claude/automation/nightly-worker-runbook.md` in the checked-out
repository and carry out one round as it describes, under the repository's
own conventions (`CLAUDE.md`) and queue rules (`docs/backlog.md`).
