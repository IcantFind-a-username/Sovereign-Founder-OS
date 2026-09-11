# Using the workbench as a founder would

- **Outcome:** three pull requests (#110–#112, merged), four backlog
  entries
- **Base:** `caa1bb8` · **Machine:** the founder's Mac, macOS 26.5 arm64,
  Ollama with `qwen2.5:7b`
- **Method:** an empty workspace, the Chinese UI, a real local model, and the
  whole path a consultant takes — name the company, add a lead with notes,
  hire AI employees, approve their proposals, send an offer, record the
  acceptance, plan and finish the project, invoice, record payment, run a
  compliance check, read the Security Center — clicking through the pages
  rather than calling the API.

## The one thing to read first

**The invoice an AI employee drafted reached the outbox asking the customer
to pay nothing, by no date.** The model wrote a one-line description as the
body; the amount and due date lived only in fields, and the composed `.eml`
carried the body alone. Every screen the founder saw showed `SGD 8,000.00`
and a due date on the card. The message did not.

Nothing in the test suite could have caught it: the tests checked that the
message was well-formed and injection-safe, and it was. What it said to the
customer was never read. #110 makes the system state the amount, due date,
issuer and a reference from the approved record, in the document's
language, and never from prose — a body still quoting an old figure cannot
override the field.

## What landed

| PR | What it changes |
| --- | --- |
| #110 | The composed message states what is owed. Subject names the kind; non-ASCII headers are RFC 2047 words; `8bit` declared; currency from the profile instead of `$`. A company compliance check no longer records its model call against a nil customer on the signed chain; discovery summaries are dated locally, with headings in the content's language. |
| #111 | What the founder reads: a "sent" count that counted unsent files, a model-status failure that read as "no model configured", a finish button that could only fail, pending proposals with no buttons, role cards half in English, internal codes shown as words, and the layout defects (grey script-built inputs, wrapped amounts, a section-gap rule that removed the gap after every panel). |
| #112 | The founder reads the exact message before approving it and downloads the file after — served only when a signed receipt names it and its bytes still match that receipt. |

## Measured, not assumed

- Model runs took 4–14 s per employee on this machine; the page had said
  "…" for all of it. It now says 5–30 s.
- Adding a customer took 17 ms. It *looked* like a second because the
  in-app browser's screenshots lag a frame — see below.
- Python's `email` parser renders a display name split across two RFC 2047
  words with a space in the middle (`翠林饮品有 限公司`), although the RFC says
  that space is to be ignored. Base64 words fit a Chinese name in one word,
  which is why #110 uses `B`, not `Q`.

## Patterns worth reusing

**Read the artefact the way its reader will.** The `.eml` was tested as
bytes and never as a message. Parsing the composed file with an independent
reader (Python's `email`, policy `default`) found the split name; reading
its body as a customer would found the missing amount.

**Serve evidence-backed files only while they match the evidence.** The
download route could have served any well-formed name in the outbox. It
serves a file a signed receipt names, and refuses one whose hash no longer
matches (409). A file changed after approval is not what was approved.

**A check must say whether it inspected anything.** The header-length test
first asserted every line, then only lines holding encoded-words — and then
that there were more than four of those, so a title that stopped folding
would fail it rather than pass vacuously.

## Not the product's fault (for the next person driving the pane)

- The in-app browser's screenshots lag one frame: the sidebar highlight, a
  toast, an open dialog can show the *previous* state. Confirm with a DOM
  read before calling it a bug — three "bugs" this session were this.
- `ref`-based clicks land in the 1024-wide viewport frame while screenshots
  are 800 wide; click by screenshot coordinates or by script.
- Starting the server and navigating in the same batch races: the first load
  can fail its first status request. Restarting the server under an open tab
  can leave a request pending on a dead socket (a blank page). Reload.

## What is still not verified

- The download in a real mail client. The route and bytes are tested; the
  file was not opened in Mail.app, to avoid acting on the owner's desktop.
- The Tauri shell: downloads from its webview were not exercised.
- Everything in the backlog entries this session added — English backend
  refusals, the placeholder sender address, template drafts that call
  themselves drafts, the English pack review note.
