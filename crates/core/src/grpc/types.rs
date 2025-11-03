// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::proto;
use core::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
#[error("Conversion failed, {msg}")]
pub struct ConversionError {
    msg: String,
}

impl ConversionError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }
}

// Stream messages

#[derive(Debug, Clone)]
pub enum ClientInMsg {
    Sync(Sync),
    StartupDone(StartupDone),
    Subscribe(Subscribe),
}

impl TryFrom<proto::ClientInMessage> for ClientInMsg {
    type Error = ConversionError;

    fn try_from(value: proto::ClientInMessage) -> Result<Self, Self::Error> {
        let kind = value
            .kind
            .ok_or_else(|| ConversionError::new("ClientInMessage kind is None"))?;
        let res = match kind {
            proto::client_in_message::Kind::Sync(val) => Self::Sync(val.try_into()?),
            proto::client_in_message::Kind::Startup(val) => Self::StartupDone(val.try_into()?),
            proto::client_in_message::Kind::Subscribe(val) => Self::Subscribe(val.try_into()?),
        };
        Ok(res)
    }
}

impl TryFrom<ClientInMsg> for proto::ClientInMessage {
    type Error = ConversionError;

    fn try_from(value: ClientInMsg) -> Result<Self, Self::Error> {
        let kind = match value {
            ClientInMsg::Sync(val) => proto::client_in_message::Kind::Sync(val.try_into()?),
            ClientInMsg::StartupDone(val) => {
                proto::client_in_message::Kind::Startup(val.try_into()?)
            }
            ClientInMsg::Subscribe(val) => {
                proto::client_in_message::Kind::Subscribe(val.try_into()?)
            }
        };
        Ok(Self { kind: Some(kind) })
    }
}

#[derive(Debug, Clone)]
pub struct ClientOutMsg {
    pub sync: Option<Sync>,
    pub updates: Vec<UpdateResponse>,
}

impl TryFrom<proto::ClientOutMessage> for ClientOutMsg {
    type Error = ConversionError;

    fn try_from(value: proto::ClientOutMessage) -> Result<Self, Self::Error> {
        let sync = value.sync.map(Sync::try_from).transpose()?;
        let updates = value
            .updates
            .into_iter()
            .map(|v| v.try_into())
            .collect::<Result<Vec<UpdateResponse>, ConversionError>>()?;
        Ok(Self { sync, updates })
    }
}

#[derive(Debug, Clone)]
pub enum ExporterInMessage {
    Resource(Resource),
    StartupDone(StartupDone),
    ExporterResponse(ExporterResponse),
}

impl TryFrom<proto::ExporterInMessage> for ExporterInMessage {
    type Error = ConversionError;

    fn try_from(value: proto::ExporterInMessage) -> Result<Self, Self::Error> {
        let kind = value
            .kind
            .ok_or_else(|| ConversionError::new("ExporterInMessage kind is None"))?;
        let res = match kind {
            proto::exporter_in_message::Kind::Resource(val) => Self::Resource(val.try_into()?),
            proto::exporter_in_message::Kind::Startup(val) => Self::StartupDone(val.try_into()?),
            proto::exporter_in_message::Kind::Response(val) => {
                Self::ExporterResponse(val.try_into()?)
            }
        };
        Ok(res)
    }
}

impl TryFrom<ExporterInMessage> for proto::ExporterInMessage {
    type Error = ConversionError;

    fn try_from(value: ExporterInMessage) -> Result<Self, Self::Error> {
        let kind = match value {
            ExporterInMessage::Resource(val) => {
                proto::exporter_in_message::Kind::Resource(val.try_into()?)
            }
            ExporterInMessage::StartupDone(val) => {
                proto::exporter_in_message::Kind::Startup(val.try_into()?)
            }
            ExporterInMessage::ExporterResponse(val) => {
                proto::exporter_in_message::Kind::Response(val.try_into()?)
            }
        };
        Ok(Self { kind: Some(kind) })
    }
}

#[derive(Debug, Clone)]
pub enum ExporterOutMessage {
    Hello {
        version: String,
    },
    ExporterSetAcquiredRequest {
        group_name: String,
        resource_name: String,
        place_name: Option<String>,
    },
}

impl TryFrom<proto::ExporterOutMessage> for ExporterOutMessage {
    type Error = ConversionError;

    fn try_from(value: proto::ExporterOutMessage) -> Result<Self, Self::Error> {
        let kind = value
            .kind
            .ok_or_else(|| ConversionError::new("ExporterOutMessage kind is None"))?;
        let res = match kind {
            proto::exporter_out_message::Kind::Hello(val) => Self::Hello {
                version: val.version,
            },
            proto::exporter_out_message::Kind::SetAcquiredRequest(val) => {
                Self::ExporterSetAcquiredRequest {
                    group_name: val.group_name,
                    resource_name: val.resource_name,
                    place_name: val.place_name,
                }
            }
        };
        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct Subscribe {
    pub is_unsubscribe: Option<bool>,
    pub kind: SubscribeKind,
}

impl TryFrom<proto::Subscribe> for Subscribe {
    type Error = ConversionError;

    fn try_from(value: proto::Subscribe) -> Result<Self, Self::Error> {
        let is_unsubscribe = value.is_unsubscribe;
        let kind = value
            .kind
            .ok_or_else(|| ConversionError::new("Subscribe kind is None"))?
            .try_into()?;
        Ok(Self {
            is_unsubscribe,
            kind,
        })
    }
}

impl TryFrom<Subscribe> for proto::Subscribe {
    type Error = ConversionError;

    fn try_from(value: Subscribe) -> Result<Self, Self::Error> {
        let is_unsubscribe = value.is_unsubscribe;
        let kind = Some(value.kind.try_into()?);
        Ok(Self {
            is_unsubscribe,
            kind,
        })
    }
}

#[derive(Debug, Clone)]
pub enum SubscribeKind {
    AllPlaces(bool),
    AllResources(bool),
}

impl TryFrom<proto::subscribe::Kind> for SubscribeKind {
    type Error = ConversionError;

    fn try_from(value: proto::subscribe::Kind) -> Result<Self, Self::Error> {
        let res = match value {
            proto::subscribe::Kind::AllPlaces(val) => Self::AllPlaces(val),
            proto::subscribe::Kind::AllResources(val) => Self::AllResources(val),
        };
        Ok(res)
    }
}

impl TryFrom<SubscribeKind> for proto::subscribe::Kind {
    type Error = ConversionError;

    fn try_from(value: SubscribeKind) -> Result<Self, Self::Error> {
        let res = match value {
            SubscribeKind::AllPlaces(val) => Self::AllPlaces(val),
            SubscribeKind::AllResources(val) => Self::AllResources(val),
        };
        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct ExporterResponse {
    pub success: bool,
}

impl TryFrom<proto::ExporterResponse> for ExporterResponse {
    type Error = ConversionError;

    fn try_from(value: proto::ExporterResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            success: value.success,
        })
    }
}

impl TryFrom<ExporterResponse> for proto::ExporterResponse {
    type Error = ConversionError;

    fn try_from(value: ExporterResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            success: value.success,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StartupDone {
    pub version: String,
    pub name: String,
}

impl TryFrom<proto::StartupDone> for StartupDone {
    type Error = ConversionError;

    fn try_from(value: proto::StartupDone) -> Result<Self, Self::Error> {
        Ok(Self {
            version: value.version,
            name: value.name,
        })
    }
}

impl TryFrom<StartupDone> for proto::StartupDone {
    type Error = ConversionError;

    fn try_from(value: StartupDone) -> Result<Self, Self::Error> {
        Ok(Self {
            version: value.version,
            name: value.name,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Sync {
    pub id: u64,
}

impl TryFrom<proto::Sync> for Sync {
    type Error = ConversionError;

    fn try_from(value: proto::Sync) -> Result<Self, Self::Error> {
        Ok(Self { id: value.id })
    }
}

impl TryFrom<Sync> for proto::Sync {
    type Error = ConversionError;

    fn try_from(value: Sync) -> Result<Self, Self::Error> {
        Ok(Self { id: value.id })
    }
}

#[derive(Debug, Clone)]
pub enum UpdateResponse {
    Resource(Resource),
    DeleteResource(Path),
    Place(Place),
    DeletePlace(String),
}

impl TryFrom<proto::UpdateResponse> for UpdateResponse {
    type Error = ConversionError;

    fn try_from(value: proto::UpdateResponse) -> Result<Self, Self::Error> {
        let kind = value
            .kind
            .ok_or_else(|| ConversionError::new("UpdateResponse kind is None"))?;
        let res = match kind {
            proto::update_response::Kind::Resource(val) => Self::Resource(val.try_into()?),
            proto::update_response::Kind::DelResource(val) => Self::DeleteResource(val.try_into()?),
            proto::update_response::Kind::Place(val) => Self::Place(val.try_into()?),
            proto::update_response::Kind::DelPlace(val) => Self::DeletePlace(val),
        };
        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub path: Path,
    pub cls: String,
    pub params: HashMap<String, MapValue>,
    pub extra: HashMap<String, MapValue>,
    pub acquired: String,
    pub available: bool,
}

impl TryFrom<proto::Resource> for Resource {
    type Error = ConversionError;

    fn try_from(value: proto::Resource) -> Result<Self, Self::Error> {
        let path = value
            .path
            .ok_or_else(|| ConversionError::new("Resoure path is None"))?
            .try_into()?;
        let cls = value.cls;
        let params = value
            .params
            .into_iter()
            .filter(|(_, v)| v.kind.is_some())
            .map(|p| Ok((p.0, p.1.try_into()?)))
            .collect::<Result<HashMap<String, MapValue>, ConversionError>>()?;
        let extra = value
            .extra
            .into_iter()
            .filter(|(_, v)| v.kind.is_some())
            .map(|p| Ok((p.0, p.1.try_into()?)))
            .collect::<Result<HashMap<String, MapValue>, ConversionError>>()?;
        let acquired = value.acquired;
        let available = value.avail;
        Ok(Self {
            path,
            cls,
            params,
            extra,
            acquired,
            available,
        })
    }
}

impl TryFrom<Resource> for proto::Resource {
    type Error = ConversionError;

    fn try_from(value: Resource) -> Result<Self, Self::Error> {
        let path = value.path.try_into()?;
        let cls = value.cls;
        let params = value
            .params
            .into_iter()
            .map(|p| Ok((p.0, p.1.try_into()?)))
            .collect::<Result<HashMap<String, proto::MapValue>, ConversionError>>()?;
        let extra = value
            .extra
            .into_iter()
            .map(|p| Ok((p.0, p.1.try_into()?)))
            .collect::<Result<HashMap<String, proto::MapValue>, ConversionError>>()?;
        let acquired = value.acquired;
        let available = value.available;
        Ok(Self {
            path: Some(path),
            cls,
            params,
            extra,
            acquired,
            avail: available,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path {
    pub exporter_name: Option<String>,
    pub group_name: String,
    pub resource_name: String,
}

impl TryFrom<proto::resource::Path> for Path {
    type Error = ConversionError;

    fn try_from(value: proto::resource::Path) -> Result<Self, Self::Error> {
        Ok(Self {
            exporter_name: value.exporter_name,
            group_name: value.group_name,
            resource_name: value.resource_name,
        })
    }
}

impl TryFrom<Path> for proto::resource::Path {
    type Error = ConversionError;

    fn try_from(value: Path) -> Result<Self, Self::Error> {
        Ok(Self {
            exporter_name: value.exporter_name,
            group_name: value.group_name,
            resource_name: value.resource_name,
        })
    }
}

impl Path {
    pub fn numeric_cmp(&self, other: &Self) -> Ordering {
        let name_ord = match (self.exporter_name.as_ref(), other.exporter_name.as_ref()) {
            (Some(first), Some(second)) => numeric_sort::cmp(first, second),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        };
        if name_ord != Ordering::Equal {
            return name_ord;
        }
        let group_ord = numeric_sort::cmp(&self.group_name, &other.group_name);
        if group_ord != Ordering::Equal {
            return group_ord;
        }
        numeric_sort::cmp(&self.resource_name, &other.resource_name)
    }
}

#[derive(Debug, Clone)]
pub enum MapValue {
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    Array(Vec<MapValue>),
}

impl TryFrom<proto::MapValue> for MapValue {
    type Error = ConversionError;

    fn try_from(value: proto::MapValue) -> Result<Self, Self::Error> {
        let kind = value
            .kind
            .ok_or_else(|| ConversionError::new("MapValue kind is None"))?;
        let res = match kind {
            proto::map_value::Kind::BoolValue(val) => Self::Bool(val),
            proto::map_value::Kind::IntValue(val) => Self::Int(val),
            proto::map_value::Kind::UintValue(val) => Self::UInt(val),
            proto::map_value::Kind::FloatValue(val) => Self::Float(val),
            proto::map_value::Kind::StringValue(val) => Self::String(val),
            proto::map_value::Kind::ArrayValue(val) => Self::Array(
                val.values
                    .into_iter()
                    .map(MapValue::try_from)
                    .collect::<Result<Vec<MapValue>, ConversionError>>()?,
            ),
        };
        Ok(res)
    }
}

impl TryFrom<MapValue> for proto::MapValue {
    type Error = ConversionError;

    fn try_from(value: MapValue) -> Result<Self, Self::Error> {
        let kind = match value {
            MapValue::Bool(val) => proto::map_value::Kind::BoolValue(val),
            MapValue::Int(val) => proto::map_value::Kind::IntValue(val),
            MapValue::UInt(val) => proto::map_value::Kind::UintValue(val),
            MapValue::Float(val) => proto::map_value::Kind::FloatValue(val),
            MapValue::String(val) => proto::map_value::Kind::StringValue(val),
            MapValue::Array(values) => proto::map_value::Kind::ArrayValue(proto::MapValueArray {
                values: values
                    .into_iter()
                    .map(proto::MapValue::try_from)
                    .collect::<Result<Vec<proto::MapValue>, ConversionError>>()?,
            }),
        };
        Ok(Self { kind: Some(kind) })
    }
}

// Other

#[derive(Debug, Clone)]
pub struct Filter(HashMap<String, String>);

impl TryFrom<proto::reservation::Filter> for Filter {
    type Error = ConversionError;

    fn try_from(value: proto::reservation::Filter) -> Result<Self, Self::Error> {
        Ok(Self(value.filter))
    }
}

impl TryFrom<Filter> for proto::reservation::Filter {
    type Error = ConversionError;

    fn try_from(value: Filter) -> Result<Self, Self::Error> {
        Ok(Self { filter: value.0 })
    }
}

#[derive(Debug, Clone)]
pub struct Reservation {
    pub owner: String,
    pub token: String,
    pub state: i32,
    pub prio: f64,
    pub filters: HashMap<String, Filter>,
    pub allocations: HashMap<String, String>,
    pub created: f64,
    pub timeout: f64,
}

impl TryFrom<proto::Reservation> for Reservation {
    type Error = ConversionError;

    fn try_from(value: proto::Reservation) -> Result<Self, Self::Error> {
        Ok(Self {
            owner: value.owner,
            token: value.token,
            state: value.state,
            prio: value.prio,
            filters: value
                .filters
                .into_iter()
                .map(|f| Ok((f.0, f.1.try_into()?)))
                .collect::<Result<HashMap<String, Filter>, ConversionError>>()?,
            allocations: value.allocations,
            created: value.created,
            timeout: value.timeout,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Place {
    pub name: String,
    pub aliases: Vec<String>,
    pub comment: String,
    pub tags: HashMap<String, String>,
    pub matches: Vec<ResourceMatch>,
    pub acquired: Option<String>,
    pub acquired_resources: Vec<String>,
    pub allowed: Vec<String>,
    pub created: f64,
    pub changed: f64,
    pub reservation: Option<String>,
}

impl TryFrom<proto::Place> for Place {
    type Error = ConversionError;

    fn try_from(value: proto::Place) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            aliases: value.aliases,
            comment: value.comment,
            tags: value.tags,
            matches: value
                .matches
                .into_iter()
                .map(ResourceMatch::try_from)
                .collect::<Result<Vec<ResourceMatch>, ConversionError>>()?,
            acquired: value.acquired.filter(|s| !s.is_empty()),
            acquired_resources: value.acquired_resources,
            allowed: value.allowed,
            created: value.created,
            changed: value.changed,
            reservation: value.reservation,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ResourceMatch {
    pub exporter: String,
    pub group: String,
    pub cls: String,
    pub name: Option<String>,
    pub rename: Option<String>,
}

impl TryFrom<proto::ResourceMatch> for ResourceMatch {
    type Error = ConversionError;

    fn try_from(value: proto::ResourceMatch) -> Result<Self, Self::Error> {
        Ok(Self {
            exporter: value.exporter,
            group: value.group,
            cls: value.cls,
            name: value.name,
            rename: value.rename,
        })
    }
}

impl ResourceMatch {
    pub fn numeric_cmp(&self, other: &Self) -> Ordering {
        let exporter_name_ord = numeric_sort::cmp(&self.exporter, &other.exporter);
        if exporter_name_ord != Ordering::Equal {
            return exporter_name_ord;
        }
        let group_ord = numeric_sort::cmp(&self.group, &other.group);
        if group_ord != Ordering::Equal {
            return group_ord;
        }
        let cls_ord = numeric_sort::cmp(&self.cls, &other.cls);
        if cls_ord != Ordering::Equal {
            return cls_ord;
        }
        match (self.name.as_ref(), other.name.as_ref()) {
            (Some(first), Some(second)) => numeric_sort::cmp(first, second),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        }
    }
}
