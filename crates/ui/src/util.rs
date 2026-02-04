// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context;
use std::path::PathBuf;
use std::sync::LazyLock;
use tracing::debug;

pub(crate) static PROJECT_DIRS: LazyLock<directories::ProjectDirs> = LazyLock::new(|| {
    directories::ProjectDirs::from("com.duagon.labgrid-ui", "Duagon", "labgrid-ui")
        .expect("Initializing project directories")
});

/// Returns the project authors found in the crate `Cargo.toml` file.
pub(crate) fn project_authors() -> String {
    env!("CARGO_PKG_AUTHORS").to_string()
}

/// Returns the project version found in the crate `Cargo.toml` file.
pub(crate) fn project_version() -> String {
    let ref_suffix = project_git_commit_id().map(|id| " (ref ".to_string() + &id + ")");
    env!("CARGO_PKG_VERSION").to_string() + &ref_suffix.unwrap_or_default()
}

/// Returns the current project git commit id.
pub(crate) fn project_git_commit_id() -> Option<String> {
    // this environment variable is set and propagated to the build in `build.rs`
    option_env!("PROJECT_GIT_COMMIT_ID").map(|s| s.to_string())
}

/// Returns the default scripts directory in the default app data dir.
pub(crate) fn default_scripts_dir() -> PathBuf {
    PROJECT_DIRS.data_dir().join("scripts")
}

/// Returns the default python virtual environment directory.
pub(crate) fn default_venv_dir() -> PathBuf {
    PathBuf::from("/opt/labgrid/venv")
}

/// Returns the path to the app configuration file.
pub(crate) fn config_path() -> PathBuf {
    PROJECT_DIRS.config_dir().join("config.json")
}

/// Ensure that all default app directories are present.
///
/// If not, new directories will be created.
///
/// Returns error if any directory creation fails.
pub(crate) fn ensure_app_default_dirs() -> anyhow::Result<()> {
    let default_config_dir = PROJECT_DIRS.config_dir();
    std::fs::create_dir_all(default_config_dir).context("Create application config directory")?;
    debug!(
        dir = default_config_dir.display().to_string(),
        "Created default application config directory"
    );
    let default_scripts_dir = default_scripts_dir();
    std::fs::create_dir_all(&default_scripts_dir)
        .context("Create application scripts directory")?;
    debug!(
        dir = default_scripts_dir.display().to_string(),
        "Created default application scripts directory"
    );
    Ok(())
}

/// Get the hostname for usage by the labgrid grpc client.
///
/// First attempts to read out `LG_HOSTNAME` environment variable,
/// defaulting to the system hostname if not present.
pub(crate) fn get_lg_hostname() -> String {
    std::env::var("LG_HOSTNAME").unwrap_or_else(|_| whoami::hostname().unwrap_or_default())
}

/// Get the username for usage by the labgrid grpc client.
///
/// First attempts to read out `LG_USERNAME` environment variable,
/// defaulting to the system username if not present.
pub(crate) fn get_lg_username() -> String {
    std::env::var("LG_USERNAME").unwrap_or_else(|_| whoami::username().unwrap_or_default())
}
