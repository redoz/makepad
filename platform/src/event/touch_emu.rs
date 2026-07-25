//! Single-touch -> mouse emulation.
//!
//! The web backend hands an app `TouchUpdate` and nothing else on a touch
//! device: it `preventDefault()`s every touch event, which also suppresses the
//! browser's own compatibility mouse events. `Event::hits` maps touches onto
//! `Hit::FingerDown/Move/Up`, so widgets driven through the hit API keep
//! working -- but anything routing on raw `Event::MouseDown/MouseMove/MouseUp`
//! (hover washes, overlay/popup authorities, hand-drawn hit lists) is simply
//! dead under touch.
//!
//! This is the state machine behind the opt-in fix: a lone finger drives the
//! mouse stream instead of the touch stream, and a second finger hands the
//! gesture back to `TouchUpdate` so an app can still implement pinch. The
//! emulator is pure -- it decides *what* to dispatch; the platform layer does
//! the dispatching and the `CxFingers` bookkeeping that goes with it.

use {
    crate::{
        event::finger::{TouchPoint, TouchState},
        makepad_math::Vec2d,
    },
    std::default::Default,
};

/// One mouse event the platform should synthesize, in dispatch order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EmulatedPointer {
    Down(Vec2d),
    Move(Vec2d),
    Up(Vec2d),
}

/// What one `TouchUpdate` turns into.
#[derive(Clone, Debug, PartialEq)]
pub struct TouchEmuStep {
    pub pointers: Vec<EmulatedPointer>,
    /// Whether the `TouchUpdate` itself should still reach the app. False while
    /// a finger is being emulated, so no widget sees the same press twice.
    pub deliver_touch_update: bool,
}

/// Tracks the one touch currently standing in for the mouse.
#[derive(Clone, Debug, Default)]
pub struct TouchMouseEmulator {
    /// `(uid, last known position)`. The position is remembered so a touch that
    /// disappears without a `Stop` (cancel/leave) can still release the digit.
    active: Option<(u64, Vec2d)>,
}

impl TouchMouseEmulator {
    pub fn is_emulating(&self) -> bool {
        self.active.is_some()
    }

    pub fn step(&mut self, touches: &[TouchPoint]) -> TouchEmuStep {
        let live = touches
            .iter()
            .filter(|t| !matches!(t.state, TouchState::Stop))
            .count();
        let mut pointers = Vec::new();

        if let Some((uid, last)) = self.active {
            let current = touches.iter().find(|t| t.uid == uid);
            if live >= 2 {
                // A second finger landed: end the emulated press where the
                // first finger is now and hand the whole gesture to the app as
                // touch, so a pinch never starts halfway through a drag.
                pointers.push(EmulatedPointer::Up(current.map_or(last, |t| t.abs)));
                self.active = None;
                return TouchEmuStep {
                    pointers,
                    deliver_touch_update: true,
                };
            }
            match current {
                Some(t) => match t.state {
                    TouchState::Stop => {
                        pointers.push(EmulatedPointer::Up(t.abs));
                        self.active = None;
                    }
                    TouchState::Move => {
                        pointers.push(EmulatedPointer::Move(t.abs));
                        self.active = Some((uid, t.abs));
                    }
                    TouchState::Start | TouchState::Stable => {
                        self.active = Some((uid, t.abs));
                    }
                },
                // The browser dropped the touch without a `Stop` (touchcancel
                // that never listed it, a lost pointer). Release anyway --
                // otherwise the mouse digit stays captured forever.
                None => {
                    pointers.push(EmulatedPointer::Up(last));
                    self.active = None;
                }
            }
            return TouchEmuStep {
                pointers,
                deliver_touch_update: false,
            };
        }

        // Not emulating: adopt a finger only as it lands, and only if it is
        // alone. A finger left over from a pinch never carries `Start` again,
        // so a gesture that began as touch stays touch until the last finger
        // lifts.
        if live == 1 {
            if let Some(t) = touches
                .iter()
                .find(|t| matches!(t.state, TouchState::Start))
            {
                self.active = Some((t.uid, t.abs));
                pointers.push(EmulatedPointer::Down(t.abs));
                return TouchEmuStep {
                    pointers,
                    deliver_touch_update: false,
                };
            }
        }

        TouchEmuStep {
            pointers,
            deliver_touch_update: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{area::Area, makepad_math::dvec2};
    use std::cell::Cell;

    fn touch(uid: u64, state: TouchState, x: f64, y: f64) -> TouchPoint {
        TouchPoint {
            state,
            abs: dvec2(x, y),
            time: 0.0,
            uid,
            rotation_angle: 0.0,
            force: 0.0,
            radius: dvec2(1.0, 1.0),
            handled: Cell::new(Area::Empty),
            sweep_lock: Cell::new(Area::Empty),
        }
    }

    #[test]
    fn a_lone_finger_drives_the_mouse_stream_and_swallows_the_touch_update() {
        let mut emu = TouchMouseEmulator::default();

        let down = emu.step(&[touch(1, TouchState::Start, 10.0, 20.0)]);
        assert_eq!(down.pointers, vec![EmulatedPointer::Down(dvec2(10.0, 20.0))]);
        assert!(!down.deliver_touch_update);
        assert!(emu.is_emulating());

        let mv = emu.step(&[touch(1, TouchState::Move, 12.0, 24.0)]);
        assert_eq!(mv.pointers, vec![EmulatedPointer::Move(dvec2(12.0, 24.0))]);
        assert!(!mv.deliver_touch_update);

        let up = emu.step(&[touch(1, TouchState::Stop, 12.0, 24.0)]);
        assert_eq!(up.pointers, vec![EmulatedPointer::Up(dvec2(12.0, 24.0))]);
        assert!(!up.deliver_touch_update);
        assert!(!emu.is_emulating());
    }

    #[test]
    fn a_stable_finger_emits_nothing_but_stays_adopted() {
        let mut emu = TouchMouseEmulator::default();
        emu.step(&[touch(1, TouchState::Start, 0.0, 0.0)]);
        let step = emu.step(&[touch(1, TouchState::Stable, 0.0, 0.0)]);
        assert!(step.pointers.is_empty());
        assert!(!step.deliver_touch_update);
        assert!(emu.is_emulating());
    }

    #[test]
    fn a_second_finger_releases_the_emulated_press_and_hands_over_the_touch_update() {
        let mut emu = TouchMouseEmulator::default();
        emu.step(&[touch(1, TouchState::Start, 10.0, 10.0)]);

        let step = emu.step(&[
            touch(1, TouchState::Stable, 11.0, 10.0),
            touch(2, TouchState::Start, 90.0, 90.0),
        ]);
        assert_eq!(step.pointers, vec![EmulatedPointer::Up(dvec2(11.0, 10.0))]);
        assert!(step.deliver_touch_update);
        assert!(!emu.is_emulating());
    }

    #[test]
    fn a_finger_left_over_from_a_pinch_is_not_adopted() {
        let mut emu = TouchMouseEmulator::default();
        emu.step(&[touch(1, TouchState::Start, 10.0, 10.0)]);
        emu.step(&[
            touch(1, TouchState::Stable, 10.0, 10.0),
            touch(2, TouchState::Start, 90.0, 90.0),
        ]);

        // Second finger lifts; the first is still down but never `Start`s again.
        let step = emu.step(&[
            touch(1, TouchState::Stable, 10.0, 10.0),
            touch(2, TouchState::Stop, 90.0, 90.0),
        ]);
        assert!(step.pointers.is_empty());
        assert!(step.deliver_touch_update);
        assert!(!emu.is_emulating());

        // And once it lifts, the next lone finger is adopted again.
        emu.step(&[touch(1, TouchState::Stop, 10.0, 10.0)]);
        let step = emu.step(&[touch(3, TouchState::Start, 5.0, 5.0)]);
        assert_eq!(step.pointers, vec![EmulatedPointer::Down(dvec2(5.0, 5.0))]);
        assert!(emu.is_emulating());
    }

    #[test]
    fn a_vanished_touch_still_releases_the_digit() {
        let mut emu = TouchMouseEmulator::default();
        emu.step(&[touch(1, TouchState::Start, 7.0, 8.0)]);
        emu.step(&[touch(1, TouchState::Move, 9.0, 8.0)]);

        // A `touchcancel` that lists nothing at all.
        let step = emu.step(&[]);
        assert_eq!(step.pointers, vec![EmulatedPointer::Up(dvec2(9.0, 8.0))]);
        assert!(!step.deliver_touch_update);
        assert!(!emu.is_emulating());
    }
}
