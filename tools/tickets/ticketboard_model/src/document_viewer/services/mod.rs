//! The document reads behind the viewer.
//!
//! **Role:** declares `document_loading`: the viewer click predicate, the repository fence and the
//! bounded read on a worker thread.
//! **Position:** called by the desktop application and its ticket details.
//! **Signals & state:** none here; `document_loading` spawns one worker thread per read.
//! **Invariants:** nothing outside the repository root is read, and no read passes 512 KB.

pub mod document_loading;
