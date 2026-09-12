//! The vault v2 engine process.
//!
//! RFC 0005 puts raw key material in a dedicated process, and this binary is
//! that process. Every type that can hold or name a key is declared in the
//! private `engine` module here and nowhere else: the library target carries
//! protocol constants only, so no downstream crate can construct a key, reach
//! a raw handle, or open a database.
//!
//! The request dispatcher is a later Program 1A item. Until it lands, the
//! engine is exercised by its own unit tests
//! (`cargo test --bin sovereign-vault-v2-engine`), which is where the plan
//! puts that evidence.

mod engine;

fn main() {
    // The first project-controlled crypto action in this process: initialise
    // OpenSSL with configuration loading switched off, before anything can
    // ask for a database. A process that cannot do that stops here rather
    // than continuing on whatever configuration its environment chose.
    if engine::process::bootstrap_crypto_process().is_err() {
        eprintln!("vault-v2-engine: OpenSSL refused to initialise; not continuing");
        std::process::exit(1);
    }

    // Otherwise deliberately inert: a process that holds keys should not do
    // anything until the dispatcher that authorises each request exists.
}
