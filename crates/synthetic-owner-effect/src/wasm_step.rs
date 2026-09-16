//! Fixed import-free Core Wasm step. The guest never publishes.

use sovereign_sandbox::{SandboxError, WasmSandbox, WasmSandboxLimits};

use crate::closed_profile::{canonical_guest_input, expected_closed_exit, fixed_core_wasm};
use crate::outcome::PublishError;
use crate::sealed::EffectIntentId;

/// Run an import-free Core Wasm v2 module with authenticated canonical input.
pub fn run_core_wasm_module(module: &[u8], input: &[u8]) -> Result<i32, PublishError> {
    let sandbox = WasmSandbox::new(WasmSandboxLimits::default()).map_err(map_sandbox)?;
    let result = sandbox
        .execute_import_free_core_v2(module, input)
        .map_err(map_sandbox)?;
    Ok(result.exit_code)
}

/// Accept only the closed checksum of the authenticated input.
pub fn accept_closed_guest_output(exit: i32, input: &[u8]) -> Result<(), PublishError> {
    if exit != expected_closed_exit(input) {
        Err(PublishError::OutputRejected)
    } else {
        Ok(())
    }
}

/// Fixed guest, expected closed checksum. Changed output is rejected.
pub fn run_fixed_core_wasm(
    intent_id: EffectIntentId,
    fixture_generation: u64,
) -> Result<(), PublishError> {
    let input = canonical_guest_input(intent_id, fixture_generation);
    let exit = run_core_wasm_module(fixed_core_wasm(), &input)?;
    accept_closed_guest_output(exit, &input)
}

fn map_sandbox(error: SandboxError) -> PublishError {
    match error {
        SandboxError::ForbiddenImport { .. } => PublishError::GuestImport,
        SandboxError::GuestInputRejected(_) => PublishError::OutputRejected,
        _ => PublishError::GuestUnavailable,
    }
}
