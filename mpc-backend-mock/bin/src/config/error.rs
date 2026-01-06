use std::path::PathBuf;

use snafu::Snafu;

use crate::kms_client;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Could not open config from {}, error: {source}", filename.display()))]
    OpenConfig { filename: PathBuf, source: std::io::Error },

    #[snafu(display("Count not parse config from {}, error: {source}", filename.display()))]
    ParseConfig { filename: PathBuf, source: serde_yaml::Error },

    #[snafu(display("Could not resolve file path {}, error: {source}", file_path.display()))]
    ResolveFilePath { file_path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to parse bitcoin network, value: {value}",))]
    ParseBitcoinNetwork { value: String },

    #[snafu(display("Could not read file {}, error: {source}", path.display()))]
    ReadFile { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to restore solana private key, error: {source}"))]
    RestoreSolanaKeypair { source: Box<dyn std::error::Error> },

    #[snafu(display("Key Management Service client is required"))]
    KmsClientRequired,

    #[snafu(display(
        "Failed to initialize Google Cloud Platform Key Management Service client, error: {source}"
    ))]
    InitializeGcpKms { source: kms_client::Error },

    #[snafu(display(
        "Failed to decrypt value: {value} from Google Cloud Platform Key Management Service, \
         error: {source}"
    ))]
    GcpKmsDecrypt { value: String, source: kms_client::Error },
}
