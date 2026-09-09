// TODO(Task 2): the next slice consumes this private teaching domain.
#[cfg_attr(not(test), allow(dead_code))]
mod domain;
// Keep the catalog declaration paired with the domain declaration.
#[cfg_attr(not(test), allow(dead_code))]
mod catalog;
// The pure handler follows domain and catalog.
#[cfg_attr(not(test), allow(dead_code))]
mod http;
// Compile-time assets follow the pure handler.
#[cfg_attr(not(test), allow(dead_code))]
mod assets;
mod server;

pub fn run(port: u16) -> std::io::Result<()> {
    server::run(port)
}
