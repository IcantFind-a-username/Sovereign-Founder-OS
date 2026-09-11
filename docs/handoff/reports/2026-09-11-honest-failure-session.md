# Making the product unable to fail quietly

- **Outcome:** ten pull requests (#94–#103), nine merged; one backlog premise
  disproved and withdrawn, one stale P1 closed, two god-file splits
- **Base:** `02508dd` · **Head at writing:** `7562a0b`
- **Machine:** the founder's Mac, macOS 26.5 arm64, Ollama 0.33.3 with
  `qwen2.5:7b`

## The one thing to read first

**A correctly configured local model on this machine had never produced a
single word, and every surface in the product said it was working.** The
provider showed `healthy`, the model page showed `real_model: true`, and each
AI-employee card came back from the template drafter without saying why.

One line caused it: the Ollama adapter refused `Transfer-Encoding: chunked`.
Ollama sets a content length only when the body fits its write buffer, so the
health probe — a short `GET /api/tags` — passed while every real answer came
back chunked and failed. The gateway then did exactly what it was designed to
do: skip the provider and fall back. The fallback is silent by design, and
nothing surfaced the one field that knew (`failover_from`).

That is the shape of failure this whole session is about. **A provider that is
reachable, healthy and never usable is worse than one that is down**, because
"down" is visible and this is not. Everything merged after #94 is an answer to
the same question: when the system declines to do what it appeared to do, who
is told, and what are they told?

## What landed

| PR | What it changes |
| --- | --- |
| #94 | The chunked-transfer defect. The model produces output for the first time. |
| #95 | Three prompt/parser defects that were ours, not the model's. |
| #96 | A skipped provider says *why* it was skipped. |
| #97 | A refused model answer says *which kind* of wrong it was. |
| #98 | All 40 HTTP routes under test, with a scan that fails if a route is added without one. |
| #99 | The export report counts every kind the bundle holds — named an inventory, not a verification. |
| #100 | Decisions, payments and compliance reports reconcile against the signed chain. |
| #101 | Closes a P1 that #90 had already resolved. |
| #102 | Expired authority claims are purged; the revocation half withdrawn on evidence. |
| #103 | The security gauntlet moves out of `ui.rs` (in review at writing). |

## What was measured, not assumed

Every role, five times per language, against identical facts on a fixed
workspace, with runs left unapproved so state never moved between trials:

| | en before | en after | zh before | zh after |
| --- | --- | --- | --- | --- |
| answers that validated | 27/30 | **30/30** | 27/30 | **29/30** |

Three of the six original failures were the product's fault:

- A schema written as `"kind":"gap"|"contradiction"|…` reads to a model as a
  list of bare values, and it copied that shape back with the key name
  dropped. Four of six failures. The allowed words moved into prose.
- The model answered, then repeated the identical object inside a code fence.
  Both objects valid; the first-`{`-to-last-`}` span is not.
- `"amount":"4.000.000"` for a 5,000.00 offer — refused correctly, so the fix
  went in the prompt, not the parser.

The remaining two were the model being wrong and the guard holding: literal
newlines inside a JSON string, and a truncated answer. Both must stay refused.

**The guard held in every one of the six cases.** A founder got a template
draft labelled as one rather than a plausible invoice with a wrong number.
That property is now observed rather than assumed.

## Two premises that turned out to be false

Worth recording because both would have produced work that looked like
progress.

**A backlog P2 asked to revoke a delivery's capability and approval when the
founder revokes the send**, on the stated grounds that they "stay consumable".
They do not — the bundle transaction (#90) consumes them at execution. Probed
on a real delivery: one token, one approval, one idempotency key and two
bundle records, all consumed. Implementing it would have been a
security-shaped change that secures nothing, which is worse than no change
because it invites the reader to believe there is protection. It would also
now be the wrong gesture: since #93 a revoked document reopens as a new draft,
so the founder is withdrawing one send, not their authority to make another.
Withdrawn, with the one case that still bites (an interrupted dispatch resumed
after a revocation) moved to the execution-journal entry.

**A P1 at the top of the queue described work that #90 had already done.** It
had been diagnosed against an uncommitted working tree, and that tree is what
landed. Closed after running the entry's own verification commands rather than
asserting it — and recording the one place the implementation deviates from
how the entry worded it, plus a re-read of the THREAT_MODEL line it cited,
which still holds as written.

## Patterns worth reusing

**Tests that refuse to rot.** Three landed this session and each has teeth in
*both* directions: the route scan fails if a route is served and untested *or*
tested and no longer served; the export inventory fails the **build** if a
collection is added and not counted, and fails the **test** if counted but
never created; the crew prompt test forbids the shape that misled the model
from coming back.

**Counting, not existence.** Payments and compliance reports have signed
events that name the *subject* — the invoice, the venture — not the record. So
two payments against one invoice share a resource, and asking "is there an
event" lets the second hide behind the first one's evidence. Softening that
back to existence is one of the sabotages the test catches.

**Purge in both directions.** A purge that never runs and a purge that ignores
its horizon are both wrong, and the second is worse: deleting a live claim
deletes the evidence that makes a replay recognisable. The test asks for both.

**Verify a pure move by running it, not by compiling it.** A refactor that
builds can still have stopped doing anything. The gauntlet was exercised
through its real HTTP route after the split: 11 results, all passing.

## Mistakes made here, so the next session does not repeat them

- **Features added to files already near the size limit**, twice, so the gate
  reported it after the work rather than before. The rule already existed in
  `CLAUDE.md`; it was not followed. Both files were split; `ui.rs` was split
  pre-emptively for the same reason, and `compliance.rs` is queued at 1143.
- **An unconditional `git stash` on a clean tree**, which then popped an
  unrelated old stash and conflicted three files. Recovered with no loss; the
  old stash was left untouched. Check `git status` before stashing, or commit
  instead.
- **Switching branches with uncommitted work.** Git refused, so nothing was
  lost — but it is the same root cause as the stash: not looking at the tree
  before moving.

## What is still not verified

- **Model quality**, as distinct from model *validity*. Nothing here says a
  proposal is any good, only that its shape was checked before a founder saw
  it.
- **Any model but `qwen2.5:7b`, on any machine but this one.** Both prompt
  defects are the kind a different model hits at a different rate.
- **The compliance pack.** Every rule still needs a licensed professional to
  confirm it against its cited source; rules that cannot be confirmed should
  be demoted to `demo_rule` or removed. This is the one thing standing between
  the compliance feature and being shown to anyone outside.
- **The five-consultant usability protocol**, which needs five people.
