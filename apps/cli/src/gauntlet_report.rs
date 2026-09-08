//! How Security Center gauntlet findings are turned into a pass/fail verdict
//! and the sentence shown next to it.
//!
//! Separated from `ui.rs` so the wording of a security claim is testable on
//! its own. The rule this module exists to enforce: a check may only claim
//! that a defense worked when the run actually exercised that defense.

use sovereign_sandbox::AddressSpaceEnforcement;

pub(crate) struct Verdict {
    pub pass: bool,
    pub detail: String,
}

/// Verdict for the compile-isolation check.
///
/// `worker_started` must come from an independent, *successful* compile
/// through the same worker this run — not from the hostile compile itself.
/// A worker that fails to launch returns the same `CompileWorkerFailed`
/// variant as a worker that ran and died containing a hostile compilation, so
/// the failure alone cannot tell "contained" from "never happened".
pub(crate) fn compile_isolation(
    worker_started: bool,
    hostile_compile_failed_closed: bool,
    enforcement: AddressSpaceEnforcement,
) -> Verdict {
    if !worker_started {
        return Verdict {
            pass: false,
            detail: "the compile worker process never started on this host, so nothing was \
                     compiled out of process and this check proves nothing about isolation — \
                     the hostile artifact failed for the same reason every artifact would"
                .to_string(),
        };
    }
    if !hostile_compile_failed_closed {
        return Verdict {
            pass: false,
            detail: "compiling a malformed artifact did not fail closed in the parent".to_string(),
        };
    }
    let containment = "an artifact that fails compilation was compiled in a killable worker \
                       process; the failure was contained in the child and the host stayed up";
    let detail = match enforcement {
        AddressSpaceEnforcement::Enforced(bytes) => format!(
            "{containment}, under a hard address-space ceiling of {} MiB",
            bytes / (1024 * 1024)
        ),
        // Honest maturity label: process containment is real here, the memory
        // cap is not. Saying "memory-limited" anyway would be a claim the
        // platform cannot back.
        AddressSpaceEnforcement::Unavailable => format!(
            "{containment}. This platform applies no address-space ceiling to the worker, so a \
             compile-time memory blow-up is contained by process isolation and the wall-clock \
             kill, not capped"
        ),
    };
    Verdict { pass: true, detail }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn a_worker_that_never_started_is_not_evidence_of_containment() {
        // The exact false green this module exists to prevent: on a host where
        // the worker cannot launch, every compile fails closed, so the hostile
        // compile "fails" too — while nothing was compiled out-of-process at
        // all.
        let verdict = compile_isolation(false, true, AddressSpaceEnforcement::Enforced(ONE_GIB));
        assert!(
            !verdict.pass,
            "a check reported the worker contained a hostile compile that never ran in it"
        );
        assert!(
            verdict.detail.contains("never started"),
            "the reader is not told why this proves nothing: {}",
            verdict.detail
        );
    }

    #[test]
    fn a_hostile_compile_that_did_not_fail_closed_is_a_failure() {
        let verdict = compile_isolation(true, false, AddressSpaceEnforcement::Enforced(ONE_GIB));
        assert!(!verdict.pass);
    }

    #[test]
    fn a_passing_check_names_the_ceiling_it_actually_enforced() {
        let verdict = compile_isolation(true, true, AddressSpaceEnforcement::Enforced(ONE_GIB));
        assert!(verdict.pass);
        assert!(
            verdict.detail.contains("1024 MiB"),
            "a claimed memory cap must state its size: {}",
            verdict.detail
        );
    }

    #[test]
    fn a_passing_check_does_not_claim_a_cap_the_platform_cannot_apply() {
        let verdict = compile_isolation(true, true, AddressSpaceEnforcement::Unavailable);
        assert!(
            verdict.pass,
            "process containment still held; only the memory ceiling is missing"
        );
        assert!(
            !verdict.detail.contains("memory-limited"),
            "the detail claims a limit this platform does not apply: {}",
            verdict.detail
        );
        assert!(
            verdict.detail.contains("no address-space ceiling"),
            "the missing ceiling must be stated, not omitted: {}",
            verdict.detail
        );
    }
}
