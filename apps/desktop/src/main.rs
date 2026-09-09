//! Native desktop shell for Sovereign Founder OS.
//!
//! The shell is deliberately thin. It owns a window and a child process, and
//! nothing else:
//!
//! - the audited `sovereign` runtime binary is launched as a child with
//!   `ui --port 0 --no-open`, so the workspace, vault, keys, and audit chain
//!   stay in the same reviewed process they run in from the terminal;
//! - the runtime picks an ephemeral loopback port and prints it; the shell
//!   reads that line rather than guessing a port or racing a fixed one;
//! - the window then shows that local page. The page uses no Tauri IPC, and
//!   this shell grants none, so the webview has no more authority than a
//!   browser tab pointed at the same address;
//! - closing the window stops the runtime.
//!
//! This is a packaging change, not a trust-boundary change: the runtime still
//! binds `127.0.0.1` only and still has no authenticated owner session. A
//! desktop icon proves neither isolation nor owner identity.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{RunEvent, WebviewUrl, WebviewWindowBuilder};

/// The line the runtime prints once its listener is accepting. Kept in step
/// with `ui::READY_PREFIX` in `apps/cli`, which a test there pins verbatim.
const READY_PREFIX: &str = "sovereign-ui listening on ";
/// How long to wait for that line before giving up and explaining why.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(45);
const RUNTIME_BINARY: &str = "sovereign";

/// The runtime child, kept so the window's lifetime bounds the process's.
#[derive(Default)]
struct Runtime(Arc<Mutex<Option<Child>>>);

impl Runtime {
    fn stop(&self) {
        if let Ok(mut guard) = self.0.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn main() {
    let runtime = Runtime::default();
    let handle_for_exit = runtime.0.clone();

    tauri::Builder::default()
        .setup(move |app| {
            let window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("Sovereign Founder OS")
                    .inner_size(1280.0, 860.0)
                    .min_inner_size(900.0, 620.0)
                    .resizable(true)
                    .build()?;

            // Start the runtime off the UI thread: a slow first run (vault
            // creation, key generation) must not freeze the window.
            let child_slot = runtime.0.clone();
            let window_handle = window.clone();
            std::thread::spawn(move || match start_runtime(child_slot) {
                Ok(url) => {
                    if let Ok(parsed) = url.parse() {
                        let _ = window_handle.navigate(parsed);
                    }
                }
                Err(message) => show_error(&window_handle, &message),
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build the desktop shell")
        .run(move |_app, event| {
            if matches!(event, RunEvent::Exit) {
                Runtime(handle_for_exit.clone()).stop();
            }
        });
}

/// Launch the runtime and return the loopback URL it reported.
fn start_runtime(slot: Arc<Mutex<Option<Child>>>) -> Result<String, String> {
    let binary = resolve_runtime_binary()?;
    // `--supervised` plus a held-open stdin pipe is the lifetime link: if
    // this shell dies for any reason, including a force kill that never runs
    // the exit handler below, the OS closes the pipe and the runtime stops.
    let mut child = Command::new(&binary)
        .args(["ui", "--port", "0", "--no-open", "--supervised"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start {}:\n{error}", binary.display()))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "The runtime produced no output to read.".to_owned())?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(url) = line.strip_prefix(READY_PREFIX) {
                let _ = sender.send(url.trim().to_owned());
                return;
            }
        }
    });

    match receiver.recv_timeout(STARTUP_TIMEOUT) {
        Ok(url) => {
            if let Ok(mut guard) = slot.lock() {
                *guard = Some(child);
            }
            Ok(url)
        }
        Err(reason) => {
            // The runtime failed or hung: report its own words, not a guess.
            let detail = drain_stderr(&mut child);
            let _ = child.kill();
            let _ = child.wait();
            Err(match reason {
                RecvTimeoutError::Timeout => format!(
                    "The runtime did not report a ready address within {} seconds.{detail}",
                    STARTUP_TIMEOUT.as_secs()
                ),
                RecvTimeoutError::Disconnected => {
                    format!("The runtime stopped before it was ready.{detail}")
                }
            })
        }
    }
}

/// Whatever the child wrote to stderr, bounded, for the error page.
fn drain_stderr(child: &mut Child) -> String {
    let Some(stderr) = child.stderr.take() else {
        return String::new();
    };
    let text: String = BufReader::new(stderr)
        .lines()
        .map_while(Result::ok)
        .take(12)
        .collect::<Vec<_>>()
        .join("\n");
    if text.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n{text}")
    }
}

/// The runtime binary: bundled beside this executable in a packaged app,
/// or the workspace build output when running from a checkout.
fn resolve_runtime_binary() -> Result<PathBuf, String> {
    let mut tried = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            let bundled = directory.join(RUNTIME_BINARY);
            if is_executable(&bundled) {
                return Ok(bundled);
            }
            tried.push(bundled);
        }
    }
    // Development fallback: the sibling workspace's build output.
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    if let Some(workspace) = workspace {
        for profile in ["release", "debug"] {
            let candidate = workspace.join("target").join(profile).join(RUNTIME_BINARY);
            if is_executable(&candidate) {
                return Ok(candidate);
            }
            tried.push(candidate);
        }
    }
    Err(format!(
        "The `{RUNTIME_BINARY}` runtime binary was not found. Looked in:\n{}",
        tried
            .iter()
            .map(|path| format!("  {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Show a failure on the loading page. The message is carried as a query
/// parameter and rendered with `textContent`, never as markup.
fn show_error(window: &tauri::WebviewWindow, message: &str) {
    let encoded: String = message
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            b' ' => "+".to_owned(),
            other => format!("%{other:02X}"),
        })
        .collect();
    if let Ok(url) = format!("tauri://localhost/index.html?error={encoded}").parse() {
        let _ = window.navigate(url);
    }
}
