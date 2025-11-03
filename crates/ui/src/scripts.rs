// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::util;
use anyhow::Context;
use core::fmt::Display;
use core::ops::{Deref, DerefMut};
use notify::Watcher;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::sync::mpsc;
use tracing::error;

/// A specific environment entry.
///
/// Used to let users change specific environment values which will be passed to the executed script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum EnvEntry {
    LgPlace,
    LgEnv,
}

impl EnvEntry {
    pub(crate) fn as_env_var(&self) -> String {
        match self {
            Self::LgPlace => "LG_PLACE",
            Self::LgEnv => "LG_ENV",
        }
        .to_string()
    }
}

/// The environment that will be passed to the executed script.
#[derive(Debug, Clone, Default)]
pub(crate) struct Env(HashMap<EnvEntry, String>);

impl Deref for Env {
    type Target = HashMap<EnvEntry, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Env {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (entry, value) in self.0.iter() {
            writeln!(f, "- {}={}", entry.as_env_var(), value)?;
        }
        Ok(())
    }
}

impl Env {
    /// Create a new environment, picking up values from environment variables like `LG_ENV` or `LG_PLACE`.
    pub(crate) fn with_env() -> Self {
        let mut env = Self::default();
        if let Ok(val) = std::env::var("LG_ENV") {
            env.insert(EnvEntry::LgEnv, val);
        }
        if let Ok(val) = std::env::var("LG_PLACE") {
            env.insert(EnvEntry::LgPlace, val);
        }
        env
    }

    pub(crate) fn env_vars(&self) -> impl Iterator<Item = (String, &'_ str)> {
        self.0
            .iter()
            .map(|(entry, val)| (entry.as_env_var(), val.as_str()))
    }
}

/// Holds information for found scripts in the specified directory.
///
/// Is also responsible for holding a file watcher that looks for changes in this directory.
#[derive(Debug)]
pub(crate) struct Scripts {
    /// The path to the script directory.
    pub(crate) dir: PathBuf,
    /// The found scripts found in the specified directory.
    pub(crate) scripts: Vec<Script>,
    /// The environment that will be passed when executing a script.
    pub(crate) env: Env,
    /// Watches the script directory while it is held.
    ///
    /// It its drop-guarded, so will stop watching and calling the specified closure defined in `watch()`
    /// as soon as it dropped.
    #[allow(unused)]
    watcher: Option<notify::RecommendedWatcher>,
}

impl Default for Scripts {
    fn default() -> Self {
        Self {
            dir: util::default_scripts_dir(),
            scripts: Vec::default(),
            watcher: None,
            env: Env::default(),
        }
    }
}

impl Scripts {
    /// Finds scripts in the supplied directory.
    pub(crate) fn from_dir(dir: PathBuf) -> anyhow::Result<Self> {
        if !dir.exists() || !dir.is_dir() {
            return Err(anyhow::anyhow!("Path must point to a directory"));
        }
        let scripts = scripts_in_dir(&dir)?;
        Ok(Self {
            dir,
            scripts,
            watcher: None,
            env: Env::with_env(),
        })
    }

    /// Performs a rescan of the scripts directory.
    pub(crate) fn rescan(&mut self) -> anyhow::Result<()> {
        let scripts = scripts_in_dir(&self.dir)?;
        self.scripts = scripts;
        Ok(())
    }

    /// Starts watching the scripts directory by registering a file watcher.
    ///
    /// the file watcher will send events through the channel which can be received
    /// by the returned channel receiver.
    #[allow(unused)]
    pub(crate) fn watch(&mut self) -> anyhow::Result<mpsc::UnboundedReceiver<()>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut watcher = notify::recommended_watcher(
            move |res: Result<notify::Event, notify::Error>| match res {
                Ok(_) => {
                    // Nothing to do if sending fails
                    let _ = tx.send(());
                }
                Err(err) => {
                    error!(?err, "Watch error");
                }
            },
        )
        .context("Creating watcher")?;
        watcher
            .watch(&self.dir, notify::RecursiveMode::NonRecursive)
            .context("Start watching dir")?;
        self.watcher = Some(watcher);
        Ok(rx)
    }

    /// Stops watching the scripts directory by taking the register file watcher and dropping it.
    #[allow(unused)]
    pub(crate) fn unwatch(&mut self) {
        self.watcher.take();
    }

    /// Returns the current scripts directory.
    pub(crate) fn dir(&self) -> PathBuf {
        self.dir.clone()
    }

    /// Returns an iterator of all current found scripts in the scripts directory.
    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = &Script> {
        self.scripts.iter()
    }
}

/// Returns all found python scripts in the supplied directory.
fn scripts_in_dir(scripts_dir: impl AsRef<Path>) -> anyhow::Result<Vec<Script>> {
    let dir = std::fs::read_dir(scripts_dir).context("Enumerating files in scripts dir")?;
    Ok(dir
        .into_iter()
        .filter_map(|f| {
            let Ok(f) = f else { return None };
            Script::from_path(f.path()).ok()
        })
        .collect())
}

/// Represents a single found script.
#[derive(Debug, Clone)]
pub(crate) struct Script {
    pub(crate) path: PathBuf,
    pub(crate) _type: ScriptType,
}

impl PartialEq for Script {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

/// Represents the script type that can be executed/is supported by the application.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub(crate) enum ScriptType {
    Shell,
    Python,
}

impl ScriptType {
    /// Determines the script type from the file name extension.
    pub(crate) fn from_ext(ext: &OsStr) -> anyhow::Result<Self> {
        let ext = ext.to_string_lossy();
        let ext = ext.as_ref();
        match ext {
            "sh" => Ok(Self::Shell),
            "py" => Ok(Self::Python),
            _ => Err(anyhow::anyhow!(
                "Extention '{ext:?}' not a valid script type"
            )),
        }
    }
}

impl Script {
    /// Creates a new script from the supplied path to the script file.
    pub(crate) fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        // Follows symlinks, which we'll allow
        let Ok(path) = std::fs::canonicalize(path) else {
            return Err(anyhow::anyhow!("Unable to canonicalize path"));
        };
        if !path.is_file() {
            return Err(anyhow::anyhow!("Not a file"));
        }
        let Some(ext) = path.extension() else {
            return Err(anyhow::anyhow!("File does not have an extension"));
        };
        let _type = ScriptType::from_ext(ext)?;
        Ok(Self { path, _type })
    }

    //// Returns the path to the script file.
    pub(crate) fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Executes the script.
    ///
    /// It will pass the supplied environment to the execution environment
    /// And, if the script is python, run through it through the python interpreter
    /// found by the supplied virtual environment directory.
    ///
    /// Returns: `Result<(exit-code, stdout, stderr)>`
    pub(crate) async fn execute(
        &self,
        venv_dir: impl AsRef<Path>,
        env: &Env,
    ) -> anyhow::Result<(i32, String, String)> {
        let program = match self._type {
            ScriptType::Shell => PathBuf::from("/usr/bin/bash"),
            ScriptType::Python => venv_dir.as_ref().join("bin").join("python3"),
        };

        println!("### Executing Command ###\nEnv:\n{env}");
        let child = tokio::process::Command::new(program.as_os_str())
            .args([&self.path])
            .envs(env.env_vars())
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Script execution failed")?;
        let output = child
            .wait_with_output()
            .await
            .context("Failed to wait on spawned command child")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("### Command finished ###");
        println!("### Command stdout ###\n{stdout}\n");
        eprintln!("### Command stderr ###\n{stderr}\n");
        Ok((
            output.status.code().unwrap_or(0),
            stdout.to_string(),
            stderr.to_string(),
        ))
    }
}

/// Represents the current status of the script.
#[derive(Debug, Clone)]
pub(crate) enum ScriptStatus {
    None,
    Running {
        script: Script,
        /// Keep the handle to the task running the script around,
        /// because it aborts on drop.
        #[allow(unused)]
        handle: iced::task::Handle,
    },
    Finished {
        script: Script,
        exit_code: i32,
    },
}

/// Validate if the supplied path points to a valid python virtual environment directory.
pub(crate) fn validate_venv_dir(dir: impl AsRef<Path>) -> anyhow::Result<()> {
    let dir = dir.as_ref();
    if !dir.is_dir() {
        return Err(anyhow::anyhow!(
            "Path '{}' is not a directory",
            dir.display()
        ));
    }
    let venv_python = dir.join("bin").join("python3");
    if !venv_python.is_file() {
        return Err(anyhow::anyhow!(
            "Venv python interpreter does not exist at location '{}'",
            venv_python.display()
        ));
    }
    Ok(())
}
