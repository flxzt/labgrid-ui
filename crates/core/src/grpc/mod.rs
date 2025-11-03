// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod error;
pub mod types;

pub mod proto {
    #![allow(clippy::enum_variant_names)]

    tonic::include_proto!("labgrid");
}

use error::GrpcClientError;
use std::collections::HashMap;
use tokio_stream::StreamExt;
use tonic::Request;
use tracing::{error, instrument};
use types::{ClientInMsg, ExporterInMessage, Filter, Place, Reservation};

#[derive(Debug)]
pub struct LabgridGrpcClient {
    client: proto::coordinator_client::CoordinatorClient<tonic::transport::Channel>,
}

impl LabgridGrpcClient {
    #[instrument]
    pub async fn new(address: &str) -> Result<Self, GrpcClientError> {
        let client =
            proto::coordinator_client::CoordinatorClient::connect(format!("http://{address}"))
                .await
                .map_err(GrpcClientError::from)?;
        Ok(Self { client })
    }

    #[instrument(skip(in_stream))]
    pub async fn client_stream(
        &mut self,
        in_stream: impl tokio_stream::Stream<Item = ClientInMsg> + Send + 'static,
    ) -> Result<tonic::Streaming<proto::ClientOutMessage>, GrpcClientError> {
        let in_stream = in_stream.filter_map(|m| match proto::ClientInMessage::try_from(m) {
            Ok(m) => Some(m),
            Err(error) => {
                error!(
                    ?error,
                    "Convert client in message to protobuf representation"
                );
                None
            }
        });
        Ok(self.client.client_stream(in_stream).await?.into_inner())
    }

    #[instrument(skip(in_stream))]
    pub async fn exporter_stream(
        &mut self,
        in_stream: impl tokio_stream::Stream<Item = ExporterInMessage> + Send + 'static,
    ) -> Result<tonic::Streaming<proto::ExporterOutMessage>, GrpcClientError> {
        let in_stream = in_stream.filter_map(|m| match proto::ExporterInMessage::try_from(m) {
            Ok(m) => Some(m),
            Err(error) => {
                error!(?error, "Convert ExporterInMessage to protobuf repr");
                None
            }
        });
        Ok(self.client.exporter_stream(in_stream).await?.into_inner())
    }

    #[instrument]
    pub async fn add_place(&mut self, name: String) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::AddPlaceRequest { name });
        let _response = self
            .client
            .add_place(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn delete_place(&mut self, name: String) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::DeletePlaceRequest { name });
        let _response = self
            .client
            .delete_place(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn get_places(&mut self) -> Result<Vec<Place>, GrpcClientError> {
        let request = Request::new(proto::GetPlacesRequest {});
        let response = self
            .client
            .get_places(request)
            .await
            .map_err(GrpcClientError::from)?;
        response
            .into_inner()
            .places
            .into_iter()
            .map(|p| Place::try_from(p).map_err(GrpcClientError::from))
            .collect()
    }

    #[instrument]
    pub async fn add_place_alias(
        &mut self,
        place_name: String,
        alias: String,
    ) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::AddPlaceAliasRequest {
            placename: place_name,
            alias,
        });
        let _response = self
            .client
            .add_place_alias(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn delete_place_alias(
        &mut self,
        place_name: String,
        alias: String,
    ) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::DeletePlaceAliasRequest {
            placename: place_name,
            alias,
        });
        let _response = self
            .client
            .delete_place_alias(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn set_place_tags(
        &mut self,
        place_name: String,
        tags: HashMap<String, String>,
    ) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::SetPlaceTagsRequest {
            placename: place_name,
            tags,
        });
        let _response = self
            .client
            .set_place_tags(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn add_place_match(
        &mut self,
        place_name: String,
        pattern: String,
        rename: Option<String>,
    ) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::AddPlaceMatchRequest {
            placename: place_name,
            pattern,
            rename,
        });
        let _response = self
            .client
            .add_place_match(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn delete_place_match(
        &mut self,
        place_name: String,
        pattern: String,
        rename: Option<String>,
    ) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::DeletePlaceMatchRequest {
            placename: place_name,
            pattern,
            rename,
        });
        let _response = self
            .client
            .delete_place_match(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn acquire_place(&mut self, place_name: String) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::AcquirePlaceRequest {
            placename: place_name,
        });
        let _response = self
            .client
            .acquire_place(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn release_place(
        &mut self,
        place_name: String,
        from_user: Option<String>,
    ) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::ReleasePlaceRequest {
            placename: place_name,
            fromuser: from_user,
        });
        let _response = self
            .client
            .release_place(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn allow_place(
        &mut self,
        place_name: String,
        user: String,
    ) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::AllowPlaceRequest {
            placename: place_name,
            user,
        });
        let _response = self
            .client
            .allow_place(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn create_reservation(
        &mut self,
        filters: HashMap<String, Filter>,
        prio: f64,
    ) -> Result<Reservation, GrpcClientError> {
        let request = Request::new(proto::CreateReservationRequest {
            filters: filters
                .into_iter()
                .map(|f| Ok((f.0, f.1.try_into()?)))
                .collect::<Result<HashMap<String, proto::reservation::Filter>, GrpcClientError>>(
                )?,
            prio,
        });
        let response = self
            .client
            .create_reservation(request)
            .await
            .map_err(GrpcClientError::from)?;
        Reservation::try_from(response.into_inner().reservation.ok_or_else(|| {
            GrpcClientError::MsgConversion(types::ConversionError::new(
                "Response not holding a reservation",
            ))
        })?)
        .map_err(GrpcClientError::from)
    }

    #[instrument]
    pub async fn cancel_reservation(&mut self, token: String) -> Result<(), GrpcClientError> {
        let request = Request::new(proto::CancelReservationRequest { token });
        let _response = self
            .client
            .cancel_reservation(request)
            .await
            .map_err(GrpcClientError::from)?;
        Ok(())
    }

    #[instrument]
    pub async fn poll_reservation(
        &mut self,
        token: String,
    ) -> Result<Reservation, GrpcClientError> {
        let request = Request::new(proto::PollReservationRequest { token });
        let response = self
            .client
            .poll_reservation(request)
            .await
            .map_err(GrpcClientError::from)?;
        Reservation::try_from(
            response
                .into_inner()
                .reservation
                .ok_or_else(|| types::ConversionError::new("Response not holding a reservation"))?,
        )
        .map_err(GrpcClientError::from)
    }

    #[instrument]
    pub async fn get_reservations(&mut self) -> Result<Vec<Reservation>, GrpcClientError> {
        let request = Request::new(proto::GetReservationsRequest {});
        let response = self
            .client
            .get_reservations(request)
            .await
            .map_err(GrpcClientError::from)?;
        response
            .into_inner()
            .reservations
            .into_iter()
            .map(|r| Reservation::try_from(r).map_err(GrpcClientError::from))
            .collect()
    }
}
