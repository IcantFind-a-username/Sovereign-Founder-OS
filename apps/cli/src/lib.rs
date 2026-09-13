//! Library surface for `sovereign-cli`. Integration tests and the binary
//! share the workspace implementation through this crate.

/// Hidden subcommand the runtime spawns for out-of-process compilation.
pub const COMPILE_WORKER_SUBCOMMAND: &str = "__compile-worker";

pub mod workspace;
