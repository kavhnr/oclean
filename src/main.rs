#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]

mod app;
mod cleanup;
mod process_tree;
mod resolve;

fn main() -> std::process::ExitCode {
    app::entrypoint()
}
