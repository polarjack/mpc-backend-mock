use std::{io, io::Write, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use mpc_backend_mock_server::ApiDoc;
use utoipa::OpenApi;

use crate::{command::run_server, config::Config, error, shadow};

#[derive(Debug, Parser)]
#[command(author,
    version,
    long_version = shadow::CLAP_LONG_VERSION,
    about,
    long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[clap(
        long = "config",
        short = 'c',
        env = "OLYMPUS_BACKEND_CONFIG_FILE_PATH",
        help = "Specify a configuration file"
    )]
    config_file_path: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[clap(about = "Print version information")]
    Version,

    #[clap(about = "Output shell completion code for the specified shell (bash, zsh, fish)")]
    Completion { shell: Shell },

    #[clap(about = "Output default configuration")]
    DefaultConfig,

    #[clap(about = "Run server")]
    #[command(visible_alias = "run")]
    Server,

    #[clap(about = "Output `OpenApi` document")]
    OpenApi,
}

impl Cli {
    pub fn run(self) -> Result<(), Box<error::Error>> {
        match self.command {
            Command::Version => {
                io::stdout()
                    .write_all(Self::command().render_long_version().as_bytes())
                    .expect("failed to write to stdout");
            }
            Command::Completion { shell } => {
                let mut command = Self::command();
                let bin_name = command.get_name().to_string();
                clap_complete::generate(shell, &mut command, bin_name, &mut io::stdout());
            }
            Command::DefaultConfig => {
                let config_text =
                    serde_yaml::to_string(&Config::default()).expect("`Config` is serializable");
                io::stdout().write_all(config_text.as_bytes()).expect("failed to write to stdout");
            }
            Command::Server => {
                let config = self.load_config()?;
                run_server(config)?;
            }
            Command::OpenApi => {
                io::stdout()
                    .write_all(
                        ApiDoc::openapi()
                            .to_yaml()
                            .expect("ApiDoc should be valid yaml")
                            .as_bytes(),
                    )
                    .expect("failed to write to stdout");
            }
        }

        Ok(())
    }

    #[allow(clippy::result_large_err)]
    fn load_config(&self) -> Result<Config, error::Error> {
        let config_file_path = &self.config_file_path.clone().unwrap_or_else(Config::default_path);
        Ok(Config::load(config_file_path)?)
    }
}
