// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0
//! Portable messages for the application projection provider route.
use chirograph::{CarrierNotice, CarrierRequest, CarrierResponse, EndpointDescriptor};
use ostiary::app_admission::{AppId, AppRouteId};
use serde::{Deserialize, Serialize};

pub const PROJECTION_PROVIDER_ROUTE: &str = "mere.projection-provider";

/// Read field by field, so a keyframe inside a completed
/// response stays its envelope on the CBOR door (D219, D225).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderCall {
    Offer { route: AppRouteId, label: String },
    Next { completed: Option<WorkResult> },
    Withdraw,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderAnswer {
    Registered { max_sessions: usize },
    Work { work: Work },
    Idle,
    Withdrawn,
}
/// Narrow facts asserted by the host after viewer admission. These are not
/// credentials and do not authenticate the host or viewer to the provider.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewerContext {
    pub session: String,
    pub subject: [u8; 32],
    pub application: Option<AppId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Work {
    pub id: u64,
    pub session: String,
    pub operation: WorkOperation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkOperation {
    Open { context: ViewerContext },
    Request { request: CarrierRequest },
    PollNotice,
    Close,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkResult {
    pub id: u64,
    pub session: String,
    pub result: WorkOutcome,
}
/// Read field by field, as [`ProviderCall`] is.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkOutcome {
    Opened { descriptor: EndpointDescriptor },
    Response { response: CarrierResponse },
    Notice { notice: Option<CarrierNotice> },
    Closed,
    Failed { message: String },
}

/// `Deserialize` with the derived shape. Serde's internally tagged enums
/// buffer the item and report it human-readable, which would hand a sealed
/// keyframe to the plain decoder; here each field reads from the format
/// itself. Keys the variant does not name are ignored, as the derive did.
mod read {
    use serde::Deserialize;
    use serde::de::{Deserializer, Error};

    use super::*;

    fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
        d: D,
    ) -> Result<Option<T>, D::Error> {
        T::deserialize(d).map(Some)
    }

    fn required<T, E: Error>(field: Option<T>, name: &'static str) -> Result<T, E> {
        field.ok_or_else(|| E::missing_field(name))
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum CallKind {
        Offer,
        Next,
        Withdraw,
    }

    #[derive(Deserialize)]
    struct CallFields {
        #[serde(rename = "type")]
        kind: CallKind,
        route: Option<AppRouteId>,
        label: Option<String>,
        #[serde(default, deserialize_with = "present")]
        completed: Option<Option<WorkResult>>,
    }

    impl<'de> Deserialize<'de> for ProviderCall {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let f = CallFields::deserialize(deserializer)?;
            Ok(match f.kind {
                CallKind::Offer => Self::Offer {
                    route: required(f.route, "route")?,
                    label: required(f.label, "label")?,
                },
                CallKind::Next => Self::Next {
                    completed: required(f.completed, "completed")?,
                },
                CallKind::Withdraw => Self::Withdraw,
            })
        }
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum OutcomeKind {
        Opened,
        Response,
        Notice,
        Closed,
        Failed,
    }

    #[derive(Deserialize)]
    struct OutcomeFields {
        #[serde(rename = "type")]
        kind: OutcomeKind,
        descriptor: Option<EndpointDescriptor>,
        response: Option<CarrierResponse>,
        #[serde(default, deserialize_with = "present")]
        notice: Option<Option<CarrierNotice>>,
        message: Option<String>,
    }

    impl<'de> Deserialize<'de> for WorkOutcome {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let f = OutcomeFields::deserialize(deserializer)?;
            Ok(match f.kind {
                OutcomeKind::Opened => Self::Opened {
                    descriptor: required(f.descriptor, "descriptor")?,
                },
                OutcomeKind::Response => Self::Response {
                    response: required(f.response, "response")?,
                },
                OutcomeKind::Notice => Self::Notice {
                    notice: required(f.notice, "notice")?,
                },
                OutcomeKind::Closed => Self::Closed,
                OutcomeKind::Failed => Self::Failed {
                    message: required(f.message, "message")?,
                },
            })
        }
    }
}
