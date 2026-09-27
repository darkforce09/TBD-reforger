//! The administrator's vehicle writes: the request each one sends, and the call that sends it.
//!
//! **Role:** describes the create (`POST /vehicle-database`), replace
//! (`PUT /vehicle-database/{id}`) and delete (`DELETE /vehicle-database/{id}`) requests as data
//! ([`VehicleRequest`]), and sends one through the API client's refusal-keeping verbs.
//! **Position:** between the page's form dialog and delete confirmation, which build a request,
//! and the request verbs of [`crate::v2::core::api::client`].
//! **Signals & state:** none. The builders are pure; the send reads the `AuthStore` it is handed.
//! **Invariants:** the id travels as one percent-encoded path segment
//! ([`crate::v2::core::api::endpoints::encode_path_segment`]), so no stored id can name another
//! route. Every write answers the stored row ([`Vehicle`]) or the refusal the backend sent, whole
//! ([`crate::v2::core::api::client::ApiRefusal`]). The send runs on `wasm32` only.

use crate::v2::core::api::dto::vehicles::VehicleWrite;
use crate::v2::core::api::endpoints::encode_path_segment;

#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::dto::vehicles::Vehicle;

/// The collection path of the vehicle database, under the API root.
pub(super) const VEHICLE_DATABASE_PATH: &str = "/vehicle-database";

/// One administrator write to the vehicle database.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum VehicleRequest {
    /// `POST /vehicle-database`: add a vehicle; answered 201 with the new row.
    Create {
        /// Every field of the new vehicle.
        body: VehicleWrite,
    },
    /// `PUT /vehicle-database/{id}`: replace every field of a stored vehicle; answered 200 with
    /// the stored row.
    Replace {
        /// The stored vehicle's id, as the row carries it.
        id: String,
        /// Every field the vehicle is stored with afterwards.
        body: VehicleWrite,
    },
    /// `DELETE /vehicle-database/{id}`: take a vehicle out of the database; answered 200 with the
    /// row as it was stored.
    Delete {
        /// The stored vehicle's id, as the row carries it.
        id: String,
    },
}

impl VehicleRequest {
    /// The request that adds a vehicle with `body`.
    pub(super) fn create(body: VehicleWrite) -> Self {
        Self::Create { body }
    }

    /// The request that replaces every field of vehicle `id` with `body`.
    pub(super) fn replace(id: &str, body: VehicleWrite) -> Self {
        Self::Replace {
            id: id.to_owned(),
            body,
        }
    }

    /// The request that deletes vehicle `id`.
    pub(super) fn delete(id: &str) -> Self {
        Self::Delete { id: id.to_owned() }
    }

    /// The HTTP method the request is sent with.
    pub(super) fn method(&self) -> &'static str {
        match self {
            Self::Create { .. } => "POST",
            Self::Replace { .. } => "PUT",
            Self::Delete { .. } => "DELETE",
        }
    }

    /// The path under the API root, with the id percent-encoded as one segment.
    pub(super) fn path(&self) -> String {
        match self {
            Self::Create { .. } => VEHICLE_DATABASE_PATH.to_owned(),
            Self::Replace { id, .. } | Self::Delete { id } => {
                format!("{VEHICLE_DATABASE_PATH}/{}", encode_path_segment(id))
            }
        }
    }

    /// The JSON body, for the writes that carry one.
    pub(super) fn body(&self) -> Option<&VehicleWrite> {
        match self {
            Self::Create { body } | Self::Replace { body, .. } => Some(body),
            Self::Delete { .. } => None,
        }
    }
}

/// Sends `request` as the signed-in administrator and answers the row the backend stored, or the
/// refusal it sent.
#[cfg(target_arch = "wasm32")]
pub(super) async fn send_vehicle_request(
    store: crate::v2::core::auth::AuthStore,
    request: VehicleRequest,
) -> Result<Vehicle, crate::v2::core::api::client::ApiRefusal> {
    use crate::v2::core::api::client::{
        api_delete_keeping_refusal, api_post_keeping_refusal, api_put_keeping_refusal, ApiRefusal,
    };
    // A body of plain strings always serialises; the failure arm reports the unreadable status
    // rather than sending a body nobody built.
    let json =
        |body: &VehicleWrite| serde_json::to_value(body).map_err(|_| ApiRefusal::unreadable());
    let path = request.path();
    match &request {
        VehicleRequest::Create { body } => {
            api_post_keeping_refusal(store, &path, json(body)?).await
        }
        VehicleRequest::Replace { body, .. } => {
            api_put_keeping_refusal(store, &path, json(body)?).await
        }
        VehicleRequest::Delete { .. } => api_delete_keeping_refusal(store, &path).await,
    }
}

#[cfg(test)]
#[path = "tests/vehicle_writes.rs"]
mod tests;
