//! The ballistics catalogs: uploading a catalog version with its calibration bundle, and the list
//! of stored versions.
//!
//! **Role:** declares the route component, the upload form, the validation report, the version
//! list and the pure view model behind them.
//! **Position:** the `/admin/ballistics-catalogs` route, in the administration hub.
//! **Signals & state:** none at this level; the route owns the form state and the list's reload
//! counter.
//! **Invariants:** stored versions are immutable; this screen only adds versions, and only through
//! the API's calibration check.

pub mod page;
mod upload_form;
mod validation_report;
mod version_list;
mod view_model;

#[cfg(target_arch = "wasm32")]
pub use page::BallisticsCatalogsPage;

#[cfg(test)]
use view_model::{
    OutcomeTone, PickedFile, UploadOutcome, is_json_name, outcome_failures, outcome_headline,
    outcome_key, report_from_details, upload_blocker, upload_outcome, version_rows,
};

#[cfg(test)]
#[path = "tests/ballistics_catalogs.rs"]
mod tests;
