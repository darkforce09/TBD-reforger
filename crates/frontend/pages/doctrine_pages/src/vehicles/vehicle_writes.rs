//! The administrator's vehicle writes: the request each one sends, and the call that sends it.
//!
//! **Role:** describes the create (`POST /vehicle-database`), replace
//! (`PUT /vehicle-database/{id}`) and delete (`DELETE /vehicle-database/{id}`) requests as data
//! ([`VehicleRequest`]), and sends one through the API client's refusal-keeping verbs.
//! **Position:** between the page's form dialog and delete confirmation, which build a request,
//! and the request verbs of [`frontend_transport::client`].
//! **Signals & state:** none. The builders are pure; the send reads the `AuthStore` it is handed.
//! **Invariants:** the id travels as one percent-encoded path segment
//! ([`frontend_transport::endpoints::encode_path_segment`]), so no stored id can name another
//! route. Every write answers the stored row ([`Vehicle`]) or the refusal the backend sent, whole
//! ([`frontend_transport::Error`]). The send runs on `wasm32` only.

#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::vehicles::VehicleWrite;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_transport::endpoints::encode_path_segment;

#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::vehicles::Vehicle;

/// The collection path of the vehicle database, under the API root.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const VEHICLE_DATABASE_PATH: &str = "/vehicle-database";

/// One administrator write to the vehicle database.
#[cfg(any(target_arch = "wasm32", test))]
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

#[cfg(any(target_arch = "wasm32", test))]
impl VehicleRequest {
    /// The request that adds a vehicle with `body`.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn create(body: VehicleWrite) -> Self {
        Self::Create { body }
    }

    /// The request that replaces every field of vehicle `id` with `body`.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn replace(id: &str, body: VehicleWrite) -> Self {
        Self::Replace {
            id: id.to_owned(),
            body,
        }
    }

    /// The request that deletes vehicle `id`.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn delete(id: &str) -> Self {
        Self::Delete { id: id.to_owned() }
    }

    /// The HTTP method the request is sent with.
    #[cfg(test)]
    pub(super) fn method(&self) -> &'static str {
        match self {
            Self::Create { .. } => "POST",
            Self::Replace { .. } => "PUT",
            Self::Delete { .. } => "DELETE",
        }
    }

    /// The path under the API root, with the id percent-encoded as one segment.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn path(&self) -> String {
        match self {
            Self::Create { .. } => VEHICLE_DATABASE_PATH.to_owned(),
            Self::Replace { id, .. } | Self::Delete { id } => {
                format!("{VEHICLE_DATABASE_PATH}/{}", encode_path_segment(id))
            }
        }
    }

    /// The JSON body, for the writes that carry one.
    #[cfg(test)]
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
    store: frontend_session::AuthStore,
    request: VehicleRequest,
) -> Result<Vehicle, frontend_transport::Error> {
    use frontend_transport::Error;
    use frontend_transport::client::{
        api_delete_keeping_refusal, api_post_keeping_refusal, api_put_keeping_refusal,
    };
    // A body of plain strings always serialises; the failure arm reports the unreadable status
    // rather than sending a body nobody built.
    let json = |body: &VehicleWrite| serde_json::to_value(body).map_err(|_| Error::Transport);
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
