# Live verification against a real local model

- **Outcome:** the configured model never produced a word until today; one
  transport defect and two prompt/parsing defects fixed, each measured before
  and after
- **Model:** `qwen2.5:7b` via Ollama 0.33.3 on the founder's Mac (18 GB, arm64)
- **Base:** `b0ada96` · **Merged on the way:** #94 (chunked transfer)
- **Backlog item:** "Live verification with a real Ollama model and the
  five-consultant usability protocol" — the model half; the five-consultant
  protocol still needs five people and is not addressed here

## The one thing to read first

**Before today, a correctly configured local model on this machine produced
nothing, and the product said everything was fine.** The Security page showed
the provider `healthy`, the model status page showed `real_model: true`, and
every AI-employee proposal came back from the template drafter. The only
honest signal was one field on each disclosure record —
`failover_from: ["ollama:qwen2.5:7b"]` — and nothing surfaced it.

The cause was one line in `crates/model/src/ollama.rs`: the HTTP response
parser refused `Transfer-Encoding: chunked`. Ollama sets a content length only
when the whole body fits its write buffer, so the health probe (a short
`GET /api/tags`) got a content length and passed, while every real answer came
back chunked and failed. A provider that is reachable, healthy, and never
usable is worse than one that is down, because the fallback is silent by
design and nothing distinguished the two. Fixed in #94.

## What was measured

All numbers below: one fixed workspace — a company profile, one customer with
discovery notes, an accepted offer, a finished project, an invoice — with each
role re-run five times against **identical facts**, so the only variable is the
model. Runs were not approved, so the state never moved between trials.

| Role | en before | en after | zh before | zh after | median |
| --- | --- | --- | --- | --- | --- |
| Requirements Analyst | 5/5 | 5/5 | 5/5 | 5/5 | 6–8 s |
| Proposal Writer | 5/5 | 5/5 | 5/5 | 5/5 | 15–18 s |
| Delivery Planner | 5/5 | 5/5 | 5/5 | 5/5 | 6–9 s |
| Invoice Clerk | 4/5 | 5/5 | 5/5 | 5/5 | 1.5–3 s |
| Quality Checker | 4/5 | 5/5 | **2/5** | 4/5 | 4–9 s |
| Compliance Checker | 4/5 | 5/5 | 5/5 | 5/5 | 7–10 s |
| **Total** | **27/30** | **30/30** | **27/30** | **29/30** | |

"Validated" means the model's own output survived the typed struct, the field
guards, and the role's checks, and became the proposal. Anything else falls
back to the deterministic template and says so on the card.

## The six failures, by cause

Every failing response was captured verbatim through a loopback logging proxy
in front of Ollama, so these are transcripts, not inferences.

**1. The schema taught the model to answer wrongly — four of six.** The
Quality Checker's schema line read
`{"findings":[{"kind":"gap"|"contradiction"|"placeholder"|"risk","detail":string}]}`.
A model reads that alternation as a list of bare values, and copied the shape
back: `{"placeholder","detail":"…"}`, with the key name dropped. One malformed
finding voids the whole list. The four allowed words moved into the task text,
where they cannot be mistaken for syntax, and the schema now shows one typed
field per key. zh went 2/5 → 4/5 on this alone.

**2. The same answer, twice — two of the remaining.** After fix 1, a new shape
surfaced: the model answered, then repeated the byte-identical object inside a
```` ```json ```` fence. Both objects are valid; the extraction spanned from
the first `{` to the *last* `}`, which is not. The parser now reads the first
complete JSON value and ignores what follows. This changes only which text
reaches the checks — every field is still validated, and the founder still
approves every proposal.

**3. An amount with thousands separators — one.** `"amount":"4.000.000"` for a
5,000.00 offer. `parse_amount_cents` refused it, correctly: had it been
accepted by a looser parser it would have been an invoice off by three orders
of magnitude. The prompt now states the exact format (digits, at most one
decimal point, no separators, no currency symbol). Not observed again.

**4. Raw newlines inside a JSON string — one.** The Compliance Checker's
summary contained literal line breaks inside the quoted value, which is not
valid JSON. Refused, template used. Nothing to fix here: the model was wrong
and the guard did its job.

**5. A truncated answer — one, still present.** One Chinese Quality Checker
response ended after the findings array with no closing brace. An incomplete
answer must be refused. This is the failure mode that remains at 1/5 for that
one role in Chinese, and it is the correct behaviour.

Causes 4 and 5 are the model being wrong and the system refusing it. Causes 1,
2 and 3 were the product being wrong about how it asked.

## What this says about the design

The guard held in every case. Six times the model produced something the
system could not verify, and six times a founder got a template draft labelled
"not a model's" rather than a plausible-looking invoice with a wrong number.
That is the property the whole crew design exists for, and it is now observed
rather than assumed.

The weakness it exposes is a different one: **a silent fallback is only honest
if someone can see it.** `model_backed: false` appears on the proposal card,
but the reason does not, and the disclosure record lists which providers were
skipped without saying why — `SkipReason.reason` exists in `crates/model` and
the workspace disclosure drops it on the floor. A founder who configures a
model and gets template output has no way to tell "the model was wrong this
time" from "the model has never once worked". That is the same gap the
chunked-transfer defect hid behind, and it is queued as a P2.

## What is still not verified

- **The five-consultant usability protocol.** It needs five people.
- **Model quality**, as opposed to model *validity*. Nothing here says whether
  a proposal is any good — only that its shape was checked before a founder
  was shown it. The summaries read plausibly; that is not a measurement.
- **Any model other than `qwen2.5:7b`**, and any machine other than this one.
  Both prompt defects are the kind that a different model would hit at a
  different rate.

## Reproducing this

`ollama serve`, then a fresh `HOME` whose
`Library/Application Support/sovereign-founder-os/model.json` is
`{"version":1,"ollama":{"enabled":true,"base_url":"http://127.0.0.1:11434","model":"qwen2.5:7b"}}`,
then `sovereign ui --no-open --port <port>`. Drive it over the loopback API
with `Host: 127.0.0.1:<port>`; the route sequence is in
`docs/handoff/reports/2026-09-10-mvp-walkthrough-without-a-model.md`. To read
what the model actually returned, put a logging proxy on another loopback port
and point `base_url` at it — the provider accepts any `127.0.0.1` port, and
without that the failing responses are unrecoverable.
