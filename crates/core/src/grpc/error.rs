// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::types;

#[derive(Debug, thiserror::Error)]
pub enum GrpcClientError {
    #[error("tonic reported transport error")]
    TonicTransport(#[from] tonic::transport::Error),
    #[error("tonic reported error status")]
    TonicStatus(#[from] tonic::Status),
    #[error("Message could not be converted")]
    MsgConversion(#[from] types::ConversionError),
}
