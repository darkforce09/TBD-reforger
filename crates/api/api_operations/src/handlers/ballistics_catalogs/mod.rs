//! The game ballistics catalog routes: the administrator upload and the public reads.
//!
//! **Role:** HTTP handlers of `/api/v1/ballistics-catalogs`: [`upload`] reads the two multipart
//! parts and answers the calibration report; [`reads`] answers the catalog list and one stored
//! catalog version.
//!
//! **Position:** registered by [`crate::routes::routes`]; the upload route carries
//! its own body limit [`upload::MAX_CATALOG_UPLOAD_BODY_BYTES`]. Both call
//! [`crate::services::ballistics_catalogs`].
//!
//! **Signals & state:** none.
//!
//! **Invariants:** the reads take no extractor of identity, so they answer anonymous callers; the
//! upload takes `AdminUser`, so every other caller is refused before the body is read.

pub mod reads;
pub mod upload;
