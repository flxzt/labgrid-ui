// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

fn main() -> anyhow::Result<()> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["proto/labgrid-coordinator.proto"], &["proto/"])?;
    Ok(())
}
