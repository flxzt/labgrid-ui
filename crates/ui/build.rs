// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::process::Command;

fn main() {
    // Set and propagate the `PROJECT_GIT_COMMIT_ID` environment variable to the build
    // for usage by utility function [utils::project_git_commit_id].
    match Command::new("git")
        .args(["show", "-s", "--format=%h"])
        .output()
    {
        Ok(output) => {
            let commit_id = String::from_utf8(output.stdout).unwrap();
            println!("cargo:rustc-env=PROJECT_GIT_COMMIT_ID={}", commit_id);
        }
        Err(e) => {
            eprintln!(
                "Fetching git commit ID unsuccessful, can't set PROJECT_GIT_COMMIT_ID, Err: {e:?}"
            );
        }
    }
}
