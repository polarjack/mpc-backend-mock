pub mod config;
pub mod error;
pub mod model;

use std::{
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const PROJECT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub static PROJECT_SEMVER: LazyLock<semver::Version> = LazyLock::new(|| {
    semver::Version::parse(PROJECT_VERSION).unwrap_or(semver::Version {
        major: 0,
        minor: 0,
        patch: 0,
        pre: semver::Prerelease::EMPTY,
        build: semver::BuildMetadata::EMPTY,
    })
});

pub const PROJECT_NAME: &str = "mpc-backend-mock";
pub const PROJECT_NAME_WITH_INITIAL_CAPITAL: &str = "MPC Backend Mock";

pub const PROGRAM_NAME: &str = "mpc-backend-mock";
pub const CONFIG_NAME: &str = "mpc-backend-mock.yaml";

pub const DEFAULT_WEB_PORT: u16 = 14444;
pub const DEFAULT_WEB_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

pub const DEFAULT_METRICS_PORT: u16 = 14446;
pub const DEFAULT_METRICS_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

pub const DEFAULT_HEALTH_CHECK_PORT: u16 = 14447;
pub const DEFAULT_HEALTH_CHECK_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

pub static PROJECT_CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    ProjectDirs::from("", PROJECT_NAME, PROJECT_NAME)
        .expect("Creating `ProjectDirs` should always success")
        .config_dir()
        .to_path_buf()
});

#[must_use]
pub fn fallback_project_config_directories() -> Vec<PathBuf> {
    let Some(user_dirs) = directories::UserDirs::new() else {
        return Vec::new();
    };
    vec![
        [user_dirs.home_dir(), (Path::new(".config")), (Path::new(PROJECT_NAME))].iter().collect(),
        [user_dirs.home_dir(), (Path::new(&format!(".{PROJECT_NAME}")))].iter().collect(),
        [&Path::new("/"), &Path::new("etc"), &Path::new(PROJECT_NAME)].iter().collect(),
    ]
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub version: String,
    pub branch: String,
    pub commit_hash: String,
    pub bitcoin_network: String,
    pub solana_cluster: String,
    pub start_time: DateTime<Utc>,
}
