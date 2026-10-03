//! The frame hooks: registration order, both callbacks, and the do-nothing defaults.

use crate::frame_hook::{FrameHook, FrameHooks};

/// A stand-in renderer: the log every hook appends to.
type CallLog = Vec<String>;

struct NamedHook(&'static str);

impl FrameHook<CallLog> for NamedHook {
    fn camera_changed(&mut self, renderer: &mut CallLog) {
        renderer.push(format!("{}:camera", self.0));
    }

    fn before_encode(&mut self, renderer: &mut CallLog) {
        renderer.push(format!("{}:encode", self.0));
    }
}

/// Overrides only the camera callback.
struct CameraOnly {
    calls: u32,
}

impl FrameHook<CallLog> for CameraOnly {
    fn camera_changed(&mut self, renderer: &mut CallLog) {
        self.calls += 1;
        renderer.push(format!("camera-only:{}", self.calls));
    }
}

#[test]
fn hooks_run_in_registration_order_on_every_call() {
    let mut hooks: FrameHooks<CallLog> = FrameHooks::new();
    assert!(hooks.is_empty());
    hooks.register(Box::new(NamedHook("symbols")));
    hooks.register(Box::new(NamedHook("cull")));
    hooks.register(Box::new(NamedHook("textures")));
    assert_eq!(hooks.len(), 3);

    let mut log = CallLog::new();
    hooks.camera_changed(&mut log);
    hooks.before_encode(&mut log);
    hooks.before_encode(&mut log);
    assert_eq!(
        log,
        [
            "symbols:camera",
            "cull:camera",
            "textures:camera",
            "symbols:encode",
            "cull:encode",
            "textures:encode",
            "symbols:encode",
            "cull:encode",
            "textures:encode",
        ]
    );
}

#[test]
fn a_callback_a_hook_does_not_override_does_nothing() {
    let mut hooks: FrameHooks<CallLog> = FrameHooks::default();
    hooks.register(Box::new(CameraOnly { calls: 0 }));
    let mut log = CallLog::new();
    hooks.before_encode(&mut log);
    assert!(log.is_empty(), "the default before_encode is a no-op");
    hooks.camera_changed(&mut log);
    hooks.camera_changed(&mut log);
    assert_eq!(
        log,
        ["camera-only:1", "camera-only:2"],
        "the hook keeps its own state"
    );
}

#[test]
fn an_empty_list_runs_nothing() {
    let mut hooks: FrameHooks<CallLog> = FrameHooks::new();
    let mut log = CallLog::new();
    hooks.camera_changed(&mut log);
    hooks.before_encode(&mut log);
    assert!(log.is_empty());
    assert_eq!(hooks.len(), 0);
}
