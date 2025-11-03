// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::AppMsg;
use crate::i18n::AppLanguage;
use crate::util;
use anyhow::Context;
use core::time::Duration;
use iced::futures;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    pub(crate) coordinator_address: String,
    pub(crate) language: AppLanguage,
    pub(crate) optimize_touch: bool,
    pub(crate) venv_dir: PathBuf,
    pub(crate) scripts_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            coordinator_address: String::default(),
            language: AppLanguage::default(),
            optimize_touch: false,
            venv_dir: util::default_venv_dir(),
            scripts_dir: util::default_scripts_dir(),
        }
    }
}

impl Config {
    /// Attempts to load the configuration the file.
    ///
    /// Returns `Ok(Some(Self))` if loading was successful, Ok(None) if the path did not point to a existing json file,
    /// `Err(error)` if loading failed.
    pub(crate) fn load_from_path(path: impl AsRef<Path>) -> anyhow::Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(None);
        }
        let file = File::open(path).context("Open file for reading")?;
        let reader = BufReader::new(file);
        let config = serde_json::from_reader(reader).context("Read configuration from file")?;
        Ok(config)
    }

    /// Saves the configuration to a path.
    ///
    /// Returns `Ok(())` if saving was successful, `Err(error)` if it failed.
    pub(crate) fn save_to_path(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let file = File::create(path).context("Open/Create file for writing")?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).context("Write configuration to file")
    }
}

/// An iced subscription that triggers periodic `AppMsg::SaveConfig` messages,
/// causing the application configuration to be saved.
pub(crate) fn periodic_save_subscription() -> impl futures::Stream<Item = AppMsg> {
    const SAVE_INTERVAL: Duration = Duration::from_secs(120);

    IntervalStream::new(time::interval(SAVE_INTERVAL)).map(|_| AppMsg::SaveConfig)
}
