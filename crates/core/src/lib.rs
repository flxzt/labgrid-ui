// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Modules
pub(crate) mod grpc;

// Re-Exports
/// Grpc client error types.
pub use grpc::error;
/// protobuf auto-generated code.
pub use grpc::proto;
/// Grpc rpc types that convert from/to protobuf auto-generated types.
pub use grpc::types;
/// Labgrid gRPC client implementation.
pub use grpc::LabgridGrpcClient;
pub use tonic;
