// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context;
use clap::Parser;
use labgrid_ui_core::LabgridGrpcClient;
use std::collections::HashMap;
use std::error::Error;
use tokio_util::sync::CancellationToken;
use tracing::debug;

#[derive(Debug, clap::Parser)]
pub struct Cli {
    /// Coordinator host and port.
    #[arg(short = 'c', long, env = "LG_COORDINATOR")]
    coordinator: String,
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Debug, clap::Subcommand)]
#[non_exhaustive]
pub enum Command {
    ClientStream,
    ExporterStream,
    AddPlace {
        #[arg(short, long)]
        name: String,
    },
    DeletePlace {
        #[arg(short, long)]
        name: String,
    },
    GetPlaces,
    AddPlaceAlias {
        #[arg(short, long)]
        place_name: String,
        #[arg(short, long)]
        alias: String,
    },
    DeletePlaceAlias {
        #[arg(short, long)]
        place_name: String,
        #[arg(short, long)]
        alias: String,
    },
    SetPlaceTags {
        #[arg(short, long)]
        place_name: String,
        /// Specify the place tags. Allows repeated argument invokations.{n}
        /// e.g. `set-place-tags -t "board=foo" -t "category=bar" ..`.
        #[arg(short = 't', long = "tag", value_parser = parse_key_val::<String, String>)]
        tags: Vec<(String, String)>,
    },
    AddPlaceMatch {
        #[arg(short, long)]
        place_name: String,
        #[arg(short, long)]
        pattern: String,
        #[arg(short, long)]
        rename: Option<String>,
    },
    DeletePlaceMatch {
        #[arg(short, long)]
        place_name: String,
        #[arg(short, long)]
        pattern: String,
        #[arg(short, long)]
        rename: Option<String>,
    },
    AcquirePlace {
        #[arg(short, long)]
        place_name: String,
    },
    ReleasePlace {
        #[arg(short, long)]
        place_name: String,
        #[arg(short, long)]
        from_user: Option<String>,
    },
    AllowPlace {
        #[arg(short, long)]
        place_name: String,
        #[arg(short, long)]
        user: String,
    },
    CreateReservation {
        // TODO: filters parsing
        #[arg(short, long)]
        prio: f64,
    },
    CancelReservation {
        #[arg(short, long)]
        token: String,
    },
    PollReservation {
        #[arg(short, long)]
        token: String,
    },
    GetReservations,
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing_subscriber()?;
    let cli = Cli::parse();
    let addr = cli.coordinator;
    let mut grpc_client = LabgridGrpcClient::new(&addr).await?;
    let quit_token = CancellationToken::new();

    let quit_token_c = quit_token.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        quit_token_c.cancel();
    });

    debug!(addr, "Successfully connected to coordinator");
    match cli.cmd {
        Command::ClientStream => {
            println!("Client stream");

            todo!()
        }
        Command::ExporterStream => {
            println!("Exporter stream");

            todo!()
        }
        Command::AddPlace { name } => {
            println!("Add place");
            tokio::select! {
                res = grpc_client.add_place(name) => {
                    res.context("Add place result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::DeletePlace { name } => {
            println!("Delete place");
            tokio::select! {
                res = grpc_client.delete_place(name) => {
                    res.context("Delete place result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::GetPlaces => {
            println!("Get Places");
            tokio::select! {
                places = grpc_client.get_places() => {
                    println!("Got places:");
                    for place in places? {
                        println!("  - {place:#?}");
                    }
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::AddPlaceAlias { place_name, alias } => {
            println!("Add place alias");
            tokio::select! {
                res = grpc_client.add_place_alias(place_name, alias) => {
                    res.context("Add place alias result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::DeletePlaceAlias { place_name, alias } => {
            println!("Delete place alias");
            tokio::select! {
                res = grpc_client.delete_place_alias(place_name, alias) => {
                    res.context("Delete place alias result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::SetPlaceTags { place_name, tags } => {
            println!("Set place tags");

            tokio::select! {
                res = grpc_client.set_place_tags(place_name, tags.into_iter().collect()) => {
                    res.context("Set place tags result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::AddPlaceMatch {
            place_name,
            pattern,
            rename,
        } => {
            println!("Add place match");

            tokio::select! {
                res = grpc_client.add_place_match(place_name, pattern, rename) => {
                    res.context("Add place match result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::DeletePlaceMatch {
            place_name,
            pattern,
            rename,
        } => {
            println!("Delete place match");

            tokio::select! {
                res = grpc_client.delete_place_match(place_name, pattern, rename) => {
                    res.context("Delete place match result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::AcquirePlace { place_name } => {
            println!("Acquire place");

            tokio::select! {
                res = grpc_client.acquire_place(place_name) => {
                    res.context("Acquire place result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::ReleasePlace {
            place_name,
            from_user,
        } => {
            println!("Release place");

            tokio::select! {
                res = grpc_client.release_place(place_name, from_user) => {
                    res.context("Release place result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::AllowPlace { place_name, user } => {
            println!("Allow place");

            tokio::select! {
                res = grpc_client.allow_place(place_name, user) => {
                    res.context("Allow place result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::CreateReservation { prio } => {
            println!("Create reservation");
            let filters = HashMap::default();

            tokio::select! {
                res = grpc_client.create_reservation(filters, prio) => {
                    res.context("Create reservation result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::CancelReservation { token } => {
            println!("Cancel reservation");

            tokio::select! {
                res = grpc_client.cancel_reservation(token) => {
                    res.context("Cancel reservation result")?;
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::PollReservation { token } => {
            println!("Poll Reservation");
            tokio::select! {
                reservation = grpc_client.poll_reservation(token) => {
                    let reservation = reservation?;
                    println!("Got reservation: {reservation:#?}");
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
        Command::GetReservations => {
            println!("Cancel reservation");

            tokio::select! {
                res = grpc_client.get_reservations() => {
                    let reservations = res.context("Get reservation result")?;
                    println!("Got reservations:");
                    for reservation in reservations {
                        println!("  - {reservation:#?}");
                    }
                },
                _ = quit_token.cancelled() => {
                }
            }
        }
    }
    Ok(())
}

fn setup_tracing_subscriber() -> anyhow::Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )?;
    debug!(".. tracing subscriber initialized");
    Ok(())
}
