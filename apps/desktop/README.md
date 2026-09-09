# Sovereign Founder OS — desktop app

A native window around the local runtime. Download it, open it, and your
workspace is there; no terminal, no browser tab, no account.

## What it is, precisely

The shell owns a window and a child process, and nothing else:

- it launches the audited `sovereign` runtime binary as a child
  (`ui --port 0 --no-open --supervised`), so the vault, keys, policy engine,
  and audit chain keep running in the same reviewed process they run in from
  the terminal;
- the runtime takes an ephemeral loopback port and prints the address it
  bound; the shell reads that line rather than guessing a port or racing
  whatever already holds 7787;
- the window then shows that local page. The page uses no Tauri IPC and this
  shell grants none, so the webview has no more authority than a browser tab
  pointed at the same address;
- the runtime's stdin is a pipe the shell holds open. If the shell exits for
  any reason — quit, crash, or `kill -9` — the OS closes the pipe and the
  runtime stops with it. A window can never leave a server running against
  your vault.

**This is packaging, not a new trust boundary.** The runtime still binds
`127.0.0.1` only, still performs no network send, and still has no
authenticated owner session. An installer and a desktop icon prove neither
isolation nor owner identity.

## Build it

```bash
./apps/desktop/build-bundle.sh          # Sovereign Founder OS.app
./apps/desktop/build-bundle.sh --dmg    # ...and a drag-to-install disk image
./apps/desktop/build-bundle.sh --debug  # faster, unoptimized
```

The script builds the runtime from the core workspace, stages it as the
bundle's sidecar under the target-triple name Tauri expects, then bundles.

The `.app` **is** the application: copy it into `/Applications` and it is
installed. The `.dmg` only wraps it for download — the app plus a shortcut to
`/Applications` to drag it across — so it is off by default. It is built with
`hdiutil` rather than Tauri's bundler, which styles the disk-image window
through Finder; that AppleScript step hangs indefinitely unless the calling
shell already holds Automation access to Finder, and it buys nothing but
window layout.

Running from a checkout without bundling works too: `cargo run` inside
`apps/desktop` falls back to `target/debug/sovereign` (or `release`) from the
core workspace.

## Why it is a separate workspace

The shell pulls in a webview stack, and on Linux the system GTK/WebKit
headers with it. Keeping it out of the core workspace means
`cargo clippy --workspace --locked`, `cargo test --workspace --locked`, and
the dependency audit in CI keep operating on the runtime crates alone. CI
does not build this crate; it is built on the machine that ships a release.

## Signing: ad-hoc today, not notarized

The build script ad-hoc signs the bundle, sealing its resources and binding
`Info.plist`. That is not cosmetic. Without it the only signature present is
the one the linker leaves on the inner executable, and macOS reports the
bundle as **damaged** — an alarming and untrue message for what is really an
unsigned app. `codesign --verify --deep --strict` now passes.

What remains is distribution trust, and no amount of local signing supplies
it: the bundle carries no Developer ID and no notarization ticket, so a copy
that arrived over the network is quarantined and Gatekeeper refuses it until
you open it once by hand:

```bash
# either right-click → Open the first time, or:
xattr -dr com.apple.quarantine "/Applications/Sovereign Founder OS.app"
```

Developer ID signing, notarization, stapling, updates, and uninstall are
separate release work with their own acceptance; see `ROADMAP.md`. Note also
that the DMG is staged with `ditto` rather than `cp -R`: the latter does not
preserve the attributes a signed bundle depends on and silently invalidates
the signature it was just given.
