// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::{self, ErrorCriticality, ErrorReport};
use crate::i18n::fl;
use crate::util;
use core::fmt::Display;
use core::time::Duration;
use futures_util::stream::Fuse;
use iced::futures::channel::mpsc;
use iced::futures::{self, SinkExt, StreamExt};
use iced::stream;
use labgrid_ui_core::error::GrpcClientError;
use labgrid_ui_core::types::{
    self, ClientInMsg, ClientOutMsg, Place, Reservation, Resource, StartupDone, Subscribe,
    SubscribeKind, UpdateResponse,
};
use labgrid_ui_core::LabgridGrpcClient;
use labgrid_ui_core::{proto, tonic};
use std::collections::HashMap;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tracing::{debug, error, instrument, warn};

/// Channel size for connection messages.
const CHANNEL_SIZE: usize = 100;
/// The timeout that determines failure of a connecting attempt.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// The interval for periodically fetching the current reservations.
///
/// Needed because reservation information currently unfortunately is not part of the client stream.
const GET_RESERVATIONS_INTERVAL: Duration = Duration::from_secs(30);

/// A connection message emitted by the UI and received by the connection subscription.
#[derive(Debug, Clone)]
pub(crate) enum ConnectionMsg {
    Connect {
        address: String,
    },
    Disconnect,
    Sync,
    // Unused for now, maybe needed later
    #[allow(unused)]
    GetPlaces,
    AcquirePlace {
        name: String,
    },
    ReleasePlace {
        name: String,
    },
    AddPlace {
        name: String,
    },
    DeletePlace {
        name: String,
    },
    AddPlaceMatch {
        place_name: String,
        pattern: String,
    },
    DeletePlaceMatch {
        place_name: String,
        pattern: String,
    },
    AddPlaceTag {
        place_name: String,
        tag: (String, String),
    },
    DeletePlaceTag {
        place_name: String,
        tag: String,
    },
    GetReservations,
    CancelReservation {
        token: String,
    },
}

/// A connection event that is produced by the connection and sent to the UI through iced's message passing.
///
/// It can be a response to a connection message or produced on it's own by for example event streams.
#[derive(Debug, Clone)]
pub(crate) enum ConnectionEvent {
    ReceiveReady(ConnectionSender),
    Connected { address: String },
    Disconnected { error: Option<app::ErrorReport> },
    NonCriticalError { error: app::ErrorReport },
    Place(Place),
    DeletePlace(String),
    Places(Vec<Place>),
    Resource(Resource),
    DeleteResource(types::Path),
    Reservations(Vec<Reservation>),
}

/// A synchronization ID which needs to be always incrementing when sending sync messages to the labgrid coordinator.
#[derive(Debug)]
struct SyncId {
    id: u64,
}

impl Default for SyncId {
    fn default() -> Self {
        Self { id: 1 }
    }
}

impl SyncId {
    /// Retreive the next ID.
    fn next(&mut self) -> u64 {
        let id = self.id;
        self.id = self.id.saturating_add(1);
        id
    }
}

/// The sender that gets used by the UI to send connection messages to the connection subscription.
#[derive(Debug, Clone)]
pub(crate) struct ConnectionSender(mpsc::Sender<ConnectionMsg>);

impl ConnectionSender {
    pub(crate) fn send(&mut self, msg: ConnectionMsg) {
        if let Err(error) = self.0.try_send(msg) {
            error!(?error, "Send connection message");
        }
    }
}

/// Represents the current connection state.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum State {
    Disconnected,
    Connected {
        // TODO: periodic connected check
        client: LabgridGrpcClient,
        client_in_sender: mpsc::UnboundedSender<ClientInMsg>,
        client_out_stream: Fuse<tonic::Streaming<proto::ClientOutMessage>>,
        sync_id: SyncId,
    },
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connected { .. } => write!(f, "Connected"),
        }
    }
}

/// Start/create the connection subscription.
///
/// Once the connection is ready to receive messages the connection event [ConnectionEvent::ReceiveReady] is emitted.
pub(crate) fn kickoff() -> impl futures::Stream<Item = ConnectionEvent> {
    stream::channel(CHANNEL_SIZE, |mut output| async move {
        let mut state = State::Disconnected;
        let (sender, ref mut receiver) = mpsc::channel(CHANNEL_SIZE);
        output_send(
            &mut output,
            ConnectionEvent::ReceiveReady(ConnectionSender(sender)),
        )
        .await;
        let mut get_reservations_interval =
            IntervalStream::new(time::interval(GET_RESERVATIONS_INTERVAL)).fuse();

        loop {
            debug!(%state);
            match &mut state {
                State::Disconnected => {
                    futures::select! {
                        msg = receiver.select_next_some() => {
                            debug!(?msg, "Received connection message");
                            #[allow(clippy::single_match)]
                            match msg {
                                ConnectionMsg::Connect { address } => {
                                    if address.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::Disconnected {
                                                error: Some(ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                })
                                            }
                                        ).await;
                                        state = State::Disconnected;
                                        continue;
                                    }
                                    let timeout_sleep = tokio::time::sleep(CONNECT_TIMEOUT);
                                    tokio::pin!(timeout_sleep);

                                    // For visually debugging UI 'connecting' state
                                    //tokio::time::sleep(Duration::from_secs(5)).await;

                                    tokio::select!{
                                        res = connect(address.clone()) => {
                                            let (client, client_in_sender, client_out_stream, sync_id) = match res {
                                                Ok(res) => res,
                                                Err(e) => {
                                                    output_send(&mut output,
                                                        ConnectionEvent::Disconnected{
                                                            error: Some(
                                                                ErrorReport {
                                                                    criticality: ErrorCriticality::Critical,
                                                                    short: "Connecting failed".to_string(),
                                                                    detailed: format!("{e:?}")
                                                                }
                                                            )
                                                        }
                                                    ).await;
                                                    state = State::Disconnected;
                                                    continue;
                                                }
                                            };
                                            output_send(&mut output, ConnectionEvent::Connected { address }).await;
                                            state = State::Connected {
                                                client,
                                                client_in_sender,
                                                client_out_stream: client_out_stream.fuse(),
                                                sync_id,
                                            };
                                        },
                                        _ = &mut timeout_sleep => {
                                            output_send(&mut output,
                                                ConnectionEvent::Disconnected{
                                                    error: Some(
                                                        ErrorReport {
                                                            criticality: ErrorCriticality::Critical,
                                                            short: "Timeout reached while trying to connect".to_string(),
                                                            detailed: "".to_string()
                                                        }
                                                    )
                                                }
                                            ).await;
                                            state = State::Disconnected;
                                        }
                                    };
                                }
                                _ => {}
                            }
                        }
                            // TODO: cancellation?
                    }
                }
                State::Connected {
                    client,
                    client_in_sender,
                    client_out_stream,
                    sync_id,
                } => {
                    futures::select! {
                        msg = receiver.select_next_some() => {
                            debug!(?msg, "Received connection message");
                            match msg {
                                ConnectionMsg::Connect { address } => {
                                    if address.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: format!("Input: '{address}")
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    let timeout_sleep = tokio::time::sleep(CONNECT_TIMEOUT);
                                    tokio::pin!(timeout_sleep);

                                    tokio::select!{
                                        res = connect(address.clone()) => {
                                            let (client, client_in_sender, client_out_stream, sync_id) = match res {
                                                Ok(res) => res,
                                                Err(e) => {
                                                    output_send(&mut output, ConnectionEvent::Disconnected {
                                                        error: Some(
                                                                ErrorReport {
                                                                criticality: ErrorCriticality::Critical,
                                                                short: "Connecting failed".to_string(),
                                                                detailed: format!("{e:?}")
                                                            }
                                                        )
                                                    }).await;
                                                    state = State::Disconnected;
                                                    continue;
                                                }
                                            };
                                            output_send(&mut output, ConnectionEvent::Connected { address }).await;
                                            state = State::Connected {
                                                client,
                                                client_in_sender,
                                                client_out_stream: client_out_stream.fuse(),
                                                sync_id,
                                             };
                                        },
                                        _ = &mut timeout_sleep => {
                                            warn!("Timeout reached while trying to connect");
                                            output_send(&mut output, ConnectionEvent::Disconnected {
                                                error: Some(
                                                        ErrorReport {
                                                        criticality: ErrorCriticality::Critical,
                                                        short: "Timeout reached while trying to connect".to_string(),
                                                        detailed: "".to_string()
                                                    }
                                                )
                                            }).await;
                                            state = State::Disconnected;
                                        }
                                    }
                                }
                                ConnectionMsg::Disconnect => {
                                    output_send(&mut output, ConnectionEvent::Disconnected{error: None}).await;
                                    state = State::Disconnected;
                                }
                                ConnectionMsg::Sync => {
                                    client_stream_send(client_in_sender, ClientInMsg::Sync(types::Sync {id: sync_id.next()})).await;
                                }
                                ConnectionMsg::GetPlaces => {
                                    match client.get_places().await {
                                        Ok(places) => output_send(&mut output, ConnectionEvent::Places(places)).await,
                                        Err(error) => handle_grpc_client_error(&mut state, &mut output, error).await
                                    }
                                }
                                ConnectionMsg::AcquirePlace {name} => {
                                    if name.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    if let Err(error) = client.acquire_place(name).await {
                                        handle_grpc_client_error(&mut state, &mut output, error).await;
                                    };
                                },
                                ConnectionMsg::ReleasePlace {name} => {
                                    if name.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    if let Err(error) = client.release_place(name, None).await {
                                        handle_grpc_client_error(&mut state, &mut output, error).await;
                                    };
                                },
                                ConnectionMsg::AddPlace {name} => {
                                    if name.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    if let Err(error) = client.add_place(name).await {
                                        handle_grpc_client_error(&mut state, &mut output, error).await;
                                    };
                                },
                                ConnectionMsg::DeletePlace {name} => {
                                    if name.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    if let Err(error) = client.delete_place(name).await {
                                        handle_grpc_client_error(&mut state, &mut output, error).await;
                                        continue;
                                    };
                                },
                                ConnectionMsg::AddPlaceMatch {place_name, pattern} => {
                                    if place_name.trim().is_empty() || pattern.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    if let Err(error) = client.add_place_match(place_name, pattern, None).await {
                                        handle_grpc_client_error(&mut state, &mut output, error).await;
                                        continue;
                                    };
                                },
                                ConnectionMsg::DeletePlaceMatch {place_name, pattern} => {
                                    if place_name.trim().is_empty() | pattern.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    if let Err(error) = client.delete_place_match(place_name, pattern, None).await {
                                        handle_grpc_client_error(&mut state, &mut output, error).await;
                                        continue;
                                    };
                                },
                                ConnectionMsg::AddPlaceTag {
                                    place_name,
                                    tag
                                } => {
                                    if place_name.trim().is_empty() || tag.0.trim().is_empty() || tag.1.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    if let Err(error) = client.set_place_tags(place_name, HashMap::from([tag])).await {
                                        handle_grpc_client_error(&mut state, &mut output, error).await;
                                        continue;
                                    };
                                }
                                ConnectionMsg::DeletePlaceTag {
                                    place_name,
                                    tag
                                } => {
                                    if place_name.trim().is_empty() || tag.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    if let Err(error) = client.set_place_tags(place_name, HashMap::from([(tag, String::default())])).await {
                                        handle_grpc_client_error(&mut state, &mut output, error).await;
                                        continue;
                                    };
                                },
                                ConnectionMsg::GetReservations => {
                                    match client.get_reservations().await {
                                        Ok(reservations) => output_send(&mut output, ConnectionEvent::Reservations(reservations)).await,
                                        Err(error) => handle_grpc_client_error(&mut state, &mut output, error).await
                                    }
                                },
                                ConnectionMsg::CancelReservation {
                                    token
                                } => {
                                    if token.trim().is_empty() {
                                        output_send(&mut output,
                                            ConnectionEvent::NonCriticalError {
                                                error: ErrorReport {
                                                    criticality: ErrorCriticality::NonCritical,
                                                    short: fl!("connection-msg-invalid-input"),
                                                    detailed: "Input must not be empty".to_string()
                                                }
                                            }
                                        ).await;
                                        continue;
                                    }
                                    if let Err(error) = client.cancel_reservation(token).await {
                                        handle_grpc_client_error(&mut state, &mut output, error).await;
                                        continue;
                                    };
                                    match client.get_reservations().await {
                                        Ok(reservations) => output_send(&mut output, ConnectionEvent::Reservations(reservations)).await,
                                        Err(error) => handle_grpc_client_error(&mut state, &mut output, error).await
                                    }
                                },
                            }
                        },
                        client_out_msg = client_out_stream.select_next_some() => {
                            let Ok(msg) = client_out_msg.inspect_err(|error| error!(?error, "Received error as client out message")) else {
                                continue;
                            };
                            let Ok(msg) = ClientOutMsg::try_from(msg).inspect_err(|error| error!(?error, "Converting proto client out message")) else{
                                continue;
                            };
                            if let Err(error) = handle_out_msg(&mut output, msg).await {
                                error!(?error, "Handling received client out message");
                                continue;
                            }
                        },
                        _ = get_reservations_interval.select_next_some() => {
                            match client.get_reservations().await {
                                Ok(reservations) => output_send(&mut output, ConnectionEvent::Reservations(reservations)).await,
                                Err(error) => handle_grpc_client_error(&mut state, &mut output, error).await
                            }
                        }
                        // TODO: cancellation?
                    }
                }
            }
        }
    })
}

/// Used when the grpc client reported an error.
///
/// Sends different events based on the error's severity.
async fn handle_grpc_client_error(
    state: &mut State,
    output: &mut mpsc::Sender<ConnectionEvent>,
    error: GrpcClientError,
) {
    match &error {
        GrpcClientError::TonicTransport(error) => {
            error!(?error, "Transport failure");
            output_send(
                output,
                ConnectionEvent::Disconnected {
                    error: Some(ErrorReport {
                        criticality: ErrorCriticality::Critical,
                        short: "Transport failure".to_string(),
                        detailed: format!("{error:?}"),
                    }),
                },
            )
            .await;
            *state = State::Disconnected;
        }
        GrpcClientError::MsgConversion(msg) => {
            output_send(
                output,
                ConnectionEvent::NonCriticalError {
                    error: ErrorReport {
                        criticality: ErrorCriticality::NonCritical,
                        short: "Message conversion".to_string(),
                        detailed: format!("{msg:?}"),
                    },
                },
            )
            .await;
        }
        GrpcClientError::TonicStatus(status) => match status.code() {
            tonic::Code::Ok => warn!("Everything's fine?!"),
            tonic::Code::Unavailable | tonic::Code::DeadlineExceeded => {
                error!(?error, "Encountered non-recoverable tonic error status");
                output_send(
                    output,
                    ConnectionEvent::Disconnected {
                        error: Some(ErrorReport {
                            criticality: ErrorCriticality::Critical,
                            short: "Non-recoverable tonic error status".to_string(),
                            detailed: format!("{error:?}"),
                        }),
                    },
                )
                .await;
                *state = State::Disconnected;
            }
            _ => {
                error!(?error, "Encountered tonic error status");
                output_send(
                    output,
                    ConnectionEvent::NonCriticalError {
                        error: ErrorReport {
                            criticality: ErrorCriticality::NonCritical,
                            short: "Tonic error status".to_string(),
                            detailed: format!("{error:?}"),
                        },
                    },
                )
                .await;
            }
        },
    }
}

/// Sends an event through the connection event channel.
///
/// The sent event will be handled by iced's message passing and appear in the `update` routine of the UI.
async fn output_send(output: &mut mpsc::Sender<ConnectionEvent>, event: ConnectionEvent) {
    if let Err(error) = output.send(event).await {
        error!(?error, "Sending connection event");
    }
}

/// Sends an client in message through the client in channel.
///
/// It will be handled by the connection subscription and sent to the coordinator.
async fn client_stream_send(sender: &mut mpsc::UnboundedSender<ClientInMsg>, msg: ClientInMsg) {
    if let Err(error) = sender.send(msg).await {
        error!(?error, "Sending client in message to stream");
    }
}

/// Handles an incoming client out message sent by the coordinator.
///
/// This handler converts it to connection events that will be handled by the UI.
async fn handle_out_msg(
    output: &mut mpsc::Sender<ConnectionEvent>,
    msg: ClientOutMsg,
) -> anyhow::Result<()> {
    for update in msg.updates {
        match update {
            UpdateResponse::Resource(r) => output_send(output, ConnectionEvent::Resource(r)).await,
            UpdateResponse::DeleteResource(p) => {
                output_send(output, ConnectionEvent::DeleteResource(p)).await;
            }
            UpdateResponse::Place(p) => output_send(output, ConnectionEvent::Place(p)).await,
            UpdateResponse::DeletePlace(n) => {
                output_send(output, ConnectionEvent::DeletePlace(n)).await;
            }
        }
    }
    Ok(())
}

/// Attempts to connect to the coordinator with the supplied address (including port, delimited by `:` character).
///
/// Returns:
/// - the gRPC client that needs to be held to keep the connection alive.
/// - the client in message sender, which can be used to send client in messages to the coordinator event stream.
/// - a stream that emits client out messages incoming from the coordinator.
/// - the sync id that needs to be used whenever a sync event is sent to the coordinator.
#[instrument]
async fn connect(
    address: String,
) -> anyhow::Result<(
    LabgridGrpcClient,
    mpsc::UnboundedSender<ClientInMsg>,
    tonic::Streaming<proto::ClientOutMessage>,
    SyncId,
)> {
    let mut client = LabgridGrpcClient::new(address.as_str()).await?;
    debug!("Successfully connected with gRPC client");
    let (mut client_in_sender, client_in_receiver) = mpsc::unbounded::<ClientInMsg>();
    let mut sync_id = SyncId::default();

    client_stream_send(
        &mut client_in_sender,
        ClientInMsg::StartupDone(StartupDone {
            version: "1".to_string(),
            name: format!("{}/{}", util::get_lg_hostname(), util::get_lg_username()),
        }),
    )
    .await;
    client_stream_send(
        &mut client_in_sender,
        ClientInMsg::Subscribe(Subscribe {
            is_unsubscribe: None,
            kind: SubscribeKind::AllPlaces(true),
        }),
    )
    .await;
    client_stream_send(
        &mut client_in_sender,
        ClientInMsg::Subscribe(Subscribe {
            is_unsubscribe: None,
            kind: SubscribeKind::AllResources(true),
        }),
    )
    .await;
    client_stream_send(
        &mut client_in_sender,
        ClientInMsg::Sync(types::Sync { id: sync_id.next() }),
    )
    .await;

    // We need to send the messages first before initiating a client stream, otherwise it would never resolve.
    let client_out_stream = client.client_stream(client_in_receiver.fuse()).await?;
    debug!("Successfully initiated client stream");
    Ok((client, client_in_sender, client_out_stream, sync_id))
}
