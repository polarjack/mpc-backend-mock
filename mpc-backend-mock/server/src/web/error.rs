use snafu::Snafu;

use crate::web::controller::Error as ControllerError;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Error occurs while binding TCP server, error: {source}"))]
    BindTcpServer { source: std::io::Error },

    #[snafu(display("Error occurs while serving HTTP server, error: {message}"))]
    ServeHttpServer { message: String },

    #[snafu(display("{source}"))]
    Controller { source: ControllerError },
}

impl From<ControllerError> for Error {
    fn from(source: ControllerError) -> Self { Self::Controller { source } }
}
