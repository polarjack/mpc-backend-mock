use snafu::Snafu;

use crate::config;

/// Result type alias for the CLI.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error type for the CLI.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("{source}"))]
    Application { source: mpc_backend_mock_server::Error },

    #[snafu(display("Could not initialize tokio runtime, error: {source}"))]
    InitializeTokioRuntime { source: tokio::io::Error },

    #[snafu(display("{source}"))]
    Config { source: config::Error },
}

impl From<config::Error> for Error {
    fn from(source: config::Error) -> Self { Self::Config { source } }
}

impl From<mpc_backend_mock_server::Error> for Error {
    fn from(source: mpc_backend_mock_server::Error) -> Self { Self::Application { source } }
}

pub trait CommandError {
    fn exit_code(&self) -> exitcode::ExitCode;
}

impl CommandError for Error {
    fn exit_code(&self) -> exitcode::ExitCode {
        match self {
            Self::Application { .. } => exitcode::SOFTWARE,
            Self::Config { .. } => exitcode::CONFIG,
            Self::InitializeTokioRuntime { .. } => exitcode::IOERR,
        }
    }
}
