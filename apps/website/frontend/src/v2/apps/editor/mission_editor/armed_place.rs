//! Armed place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The action selected for a pointer release while placement is armed.
pub enum ArmedUp {
    Place,
    KeepArmed,
    Disarm,
    FallThroughPan,
    Ignore,
}

/// Selects placement behavior from button and canvas hit state.
pub fn decide_armed_pointerup(button: i16, on_canvas: bool) -> ArmedUp {
    match button {
        0 if on_canvas => ArmedUp::Place,
        0 => ArmedUp::KeepArmed,
        1 => ArmedUp::FallThroughPan,
        2 => ArmedUp::Disarm,
        _ => ArmedUp::Ignore,
    }
}

/// Reports whether a held button can promote an armed gesture.
pub fn may_promote(buttons: u16) -> bool {
    buttons != 0
}

/// Reports whether the left gesture starts while placement is armed.
pub fn open_left_gesture_while_armed(armed: bool) -> bool {
    !armed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The active left-button gesture kind.
pub enum LeftKind {
    Pending,
    Ruler,
    Move,
    Marquee,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The command emitted by an armed placement transition.
pub enum Effect {
    Place,
    Disarm,
    PromoteMove,
    CommitMove,
    CommitRulerVertex,
    PanDelta,
    ClearLeft,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// State tracked by the armed placement transition model.
pub struct Host {
    pub armed: bool,
    pub left: Option<LeftKind>,
    pub pan: bool,
}

#[derive(Debug, Clone, Copy)]
/// An event consumed by the armed placement transition model.
pub enum Ev {
    Arm,
    PointerDown { button: i16, on_canvas: bool },
    PointerMove { buttons: u16, past_threshold: bool },
    PointerUp { button: i16, on_canvas: bool },
    Escape,
}

/// Applies one event and returns commands for its transition.
pub fn step(host: &mut Host, ev: Ev) -> Vec<Effect> {
    let mut out = Vec::new();
    match ev {
        Ev::Arm => {
            host.armed = true;
        }
        Ev::PointerDown {
            button,
            on_canvas: _,
        } => {
            if button == 1 {
                host.pan = true;
            } else if button == 0 {
                if open_left_gesture_while_armed(host.armed) {
                    if host.left.is_none() {
                        host.left = Some(LeftKind::Pending);
                    }
                }
            }
        }
        Ev::PointerMove {
            buttons,
            past_threshold,
        } => {
            if host.pan {
                out.push(Effect::PanDelta);
                return out;
            }
            if host.armed {
                return out;
            }
            if let Some(LeftKind::Pending) = host.left {
                if past_threshold {
                    if may_promote(buttons) {
                        host.left = Some(LeftKind::Move);
                        out.push(Effect::PromoteMove);
                    } else {
                        host.left = None;
                        out.push(Effect::ClearLeft);
                    }
                }
            }
        }
        Ev::PointerUp { button, on_canvas } => {
            if host.armed {
                if host.left.take().is_some() {
                    out.push(Effect::ClearLeft);
                }
                match decide_armed_pointerup(button, on_canvas) {
                    ArmedUp::Place => {
                        out.push(Effect::Place);
                        host.armed = false;
                    }
                    ArmedUp::KeepArmed => {}
                    ArmedUp::Disarm => {
                        host.armed = false;
                        out.push(Effect::Disarm);
                    }
                    ArmedUp::FallThroughPan => {
                        if host.pan {
                            host.pan = false;
                        }
                    }
                    ArmedUp::Ignore => {}
                }
                return out;
            }
            if host.pan && button == 1 {
                host.pan = false;
                return out;
            }
            if button != 0 {
                if let Some(k) = host.left.take() {
                    out.push(Effect::ClearLeft);
                    let _ = k;
                }
                return out;
            }
            match host.left.take() {
                Some(LeftKind::Move) => out.push(Effect::CommitMove),
                Some(LeftKind::Ruler) => out.push(Effect::CommitRulerVertex),
                Some(LeftKind::Pending) | Some(LeftKind::Marquee) | None => {}
            }
        }
        Ev::Escape => {
            if host.armed {
                host.armed = false;
                out.push(Effect::Disarm);
            }
            if host.left.take().is_some() {
                out.push(Effect::ClearLeft);
            }
        }
    }
    out
}

/// Applies an event sequence and returns its final state and commands.
pub fn run(mut host: Host, events: &[Ev]) -> (Host, Vec<Effect>) {
    let mut all = Vec::new();
    for &ev in events {
        all.extend(step(&mut host, ev));
    }
    (host, all)
}
