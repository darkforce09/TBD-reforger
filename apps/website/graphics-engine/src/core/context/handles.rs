//! Role: handles.
//! Position: `core/context` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::state::RenderEngine;
use std::{cell::RefCell, rc::Rc};

/// A mounted engine, or an empty slot during initialization and teardown.
pub type EngineHandle = Rc<RefCell<Option<RenderEngine>>>;
