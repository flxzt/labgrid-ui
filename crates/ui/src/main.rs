// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Turns off console window on Windows, but not when building with dev profile.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

/// Core app logic and state.
pub(crate) mod app;
/// Persistent application configuration.
pub(crate) mod config;
/// Connection subscription and state for communicating with the coordinator through grpc.
pub(crate) mod connection;
/// Utilities for changing the application language, retreive translations, and so on.
pub(crate) mod i18n;
/// State and logic related to the scripts tab of the application.
pub(crate) mod scripts;
/// Miscellaneous utilities.
pub(crate) mod util;
/// Application UI views derived from the application state.
pub(crate) mod views;

use clap::Parser;
use tracing::debug;

/// Command line arguments for additional options.
///
/// Can be used to overwrite the app defaults
/// or restored values from settings (persistent settings not yet implemented).
#[derive(Debug, clap::Parser)]
pub(crate) struct Args {
    /// Labgrid coordinator host and port.
    #[arg(short = 'c', long, env = "LG_COORDINATOR")]
    coordinator: Option<String>,
    /// Optimize the UI for touch screens.
    #[arg(long, default_value_t = false)]
    optimize_touch: bool,
    // Use a internal-only clipboard implementation.{n}
    // Useful when the app is started on a wayland/X11 server that does not implement a clipboard.
    #[arg(long, default_value_t = false)]
    internal_clipboard: bool,
}

fn main() -> anyhow::Result<()> {
    setup_tracing_subscriber()?;
    let args = Args::parse();
    app::run(args)?;
    Ok(())
}

/// Sets up a tracing subscriber that logs to the console.
///
/// Picks up values of environment variable `RUST_LOG` to determine event emission levels
/// (error, warn, info, debug, ..).
fn setup_tracing_subscriber() -> anyhow::Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )?;
    debug!(".. tracing subscriber initialized");
    Ok(())
}
