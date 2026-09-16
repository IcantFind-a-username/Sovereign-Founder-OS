//! Fixture process entry. Not a product CLI and not a hidden sovereign-cli mode.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use sovereign_synthetic_owner_effect::{ProcessBoundary, RELEASE_EXCLUSION_NEEDLE};

fn main() -> ExitCode {
    let parsed = match parse_args(std::env::args().skip(1)) {
        Ok(parsed) => parsed,
        Err(()) => {
            let _ = writeln!(std::io::stderr(), "E-USAGE");
            return ExitCode::from(2);
        }
    };

    let boundary = match ProcessBoundary::acquire(&parsed.root) {
        Ok(boundary) => boundary,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error}");
            return ExitCode::from(exit_status(error));
        }
    };

    match boundary.hold_store(|_store| {
        // Touch the needle so a product binary that accidentally linked this
        // crate cannot claim the constant was eliminated as unused.
        std::hint::black_box(RELEASE_EXCLUSION_NEEDLE);
        let mut stdout = std::io::stdout();
        let _ = writeln!(stdout, "boundary-ready");
        let _ = stdout.flush();
        if parsed.hold {
            std::thread::sleep(Duration::from_secs(30));
        }
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error}");
            ExitCode::from(exit_status(error))
        }
    }
}

struct Args {
    root: PathBuf,
    hold: bool,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Args, ()> {
    let mut root = None;
    let mut hold = false;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                let value = args.next().ok_or(())?;
                root = Some(PathBuf::from(value));
            }
            "--hold" => hold = true,
            _ => return Err(()),
        }
    }
    Ok(Args {
        root: root.ok_or(())?,
        hold,
    })
}

fn exit_status(error: sovereign_synthetic_owner_effect::BoundaryError) -> u8 {
    use sovereign_synthetic_owner_effect::BoundaryError::*;
    match error {
        RootRejected => 3,
        AlreadyRunning => 4,
        LockUnavailable => 5,
        StoreUnavailable => 6,
    }
}
