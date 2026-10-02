//! Role: frame ids.
//! Position: `frame` in the graphics engine.
//! Signals & state: opaque keys handed down by the caller.
//! Invariants: this crate never interprets an id. It compares, sorts and indexes with them.

/// Draw-order key, assigned by the caller.
///
/// The renderer sorts ascending and **never interprets the value**. Distinct draw layers get
/// distinct ids; equal ids draw in the order they were handed over. The caller owns the meaning — which layer
/// is which, and which is painted over which — because that is cartography, not rendering.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct LaneId(pub u16);

/// Index into [`crate::frame::FramePacket::pipelines`].
///
/// Carrying the pipeline on the batch is what lets the encoder stay ignorant: it binds what it
/// is told instead of switching on what the lane means.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct PipelineId(pub u16);

/// Index into [`crate::frame::FramePacket::bind_groups`].
///
/// Group 0 is always the camera; these address groups 1 and 2.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct BindGroupId(pub u16);
