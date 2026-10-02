//! Event authoring: creating an event and attaching a mission with its ORBAT snapshot.
//!
//! **Role:** the write paths that bring an event and its seats into being, shared by the
//! administrator routes and the `staging-fixtures` host tool.
//!
//! **Position:** `operations::handlers::event_create_update` and
//! `operations::handlers::event_mission_attachment` validate the request, settle the
//! administrator's authority and call these services on their transaction; the host tool's
//! `seed-load-fixture-events` calls the same services for its reserved fixture author.
//!
//! **Signals & state:** none; every write runs on the caller's transaction.
//!
//! **Invariants:** an event row commits with its `event.created` audit row, and an attachment with
//! its slots and its `event.mission_attached` or `event.mission_restored` audit row; neither path
//! has a second writer.

pub mod event_creation;
pub mod mission_attachment;
