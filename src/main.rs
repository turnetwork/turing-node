//! Substrate Node Template CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod chain_spec;
mod cli;
mod service;

pub use substrate_cli::{error, IntoExit, VersionInfo};

fn run() -> cli::error::Result<()> {
    let version = VersionInfo {
        name: "Turnetwork Node",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        executable_name: "turing-node",
        author: "Turing Network",
        description: "A Parity Substrate node implementing Turnetwork.",
        support_url: "https://github.com/turnetwork/turing-node/issues/new",
    };
    cli::run(::std::env::args(), cli::Exit, version)
}

error_chain::quick_main!(run);
