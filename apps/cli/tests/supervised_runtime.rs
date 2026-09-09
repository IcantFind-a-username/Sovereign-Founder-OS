//! The lifetime link the desktop shell depends on: a runtime started with
//! `--supervised` stops when the process that launched it goes away. Without
//! it, a force-killed window leaves a server running against the owner's
//! vault. Needs the real binary, so it lives here rather than in-crate.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Pinned in-crate by `ui_tests::the_ready_line_the_desktop_shell_parses_is_stable`
/// and repeated in `apps/desktop/src/main.rs`.
const READY_PREFIX: &str = "sovereign-ui listening on ";

/// Start a supervised runtime on an ephemeral port and wait until it reports
/// the address it bound, so the test acts on a server that is really serving.
fn start_supervised(home: &std::path::Path) -> (std::process::Child, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .args(["ui", "--port", "0", "--no-open", "--supervised"])
        .env("HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the runtime binary starts");

    let stdout = child.stdout.take().expect("stdout is piped");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(url) = line.strip_prefix(READY_PREFIX) {
                let _ = sender.send(url.trim().to_owned());
                return;
            }
        }
    });
    let url = receiver
        .recv_timeout(Duration::from_secs(60))
        .expect("the supervised runtime reports its address");
    (child, url)
}

fn wait_for_exit(
    child: &mut std::process::Child,
    within: Duration,
) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + within;
    loop {
        if let Some(status) = child.try_wait().expect("the child can be polled") {
            return Some(status);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn a_supervised_runtime_stops_when_its_supervisor_goes_away() {
    let home = tempfile::tempdir().unwrap();
    let (mut child, url) = start_supervised(home.path());
    assert!(url.starts_with("http://127.0.0.1:"), "{url}");
    assert!(
        !url.ends_with(":0"),
        "the reported port must be the real one: {url}"
    );

    // Closing stdin is exactly what the OS does when a supervisor dies,
    // including a force kill that runs no cleanup code.
    drop(child.stdin.take());
    let status = wait_for_exit(&mut child, Duration::from_secs(20))
        .expect("the runtime must not outlive its supervisor");
    assert!(status.success(), "it should stop cleanly, got {status}");
}

#[test]
fn an_unsupervised_runtime_keeps_serving_after_its_stdin_closes() {
    // The ordinary `sovereign ui` in a terminal must not exit because stdin
    // reached end of file — `nohup`, `< /dev/null`, and a closed pipe are all
    // normal there.
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .args(["ui", "--port", "0", "--no-open"])
        .env("HOME", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the runtime binary starts");
    let stdout = child.stdout.take().expect("stdout is piped");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line.starts_with(READY_PREFIX) {
                let _ = sender.send(());
                return;
            }
        }
    });
    receiver
        .recv_timeout(Duration::from_secs(60))
        .expect("the runtime reports its address");

    drop(child.stdin.take());
    assert!(
        wait_for_exit(&mut child, Duration::from_secs(5)).is_none(),
        "an unsupervised runtime must keep serving after stdin closes"
    );
    let _ = child.kill();
    let _ = child.wait();
}
