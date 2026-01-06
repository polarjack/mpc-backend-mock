mod cli;
mod command;
mod config;
mod error;
mod keycloak_client;
mod kms_client;
mod shadow {
    #![allow(clippy::needless_raw_string_hashes)]
    use shadow_rs::shadow;
    shadow!(build);

    pub use self::build::*;
}

use clap::Parser;

use self::{cli::Cli, error::CommandError};

fn main() {
    if let Err(err) = Cli::parse().run() {
        eprintln!("Error: {err}");
        std::process::exit(err.exit_code());
    }
}
