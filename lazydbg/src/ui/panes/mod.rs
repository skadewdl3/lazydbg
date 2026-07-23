use std::range::Range;

use num_enum::TryFromPrimitive;
use reactatui::hooks::{State, use_global, use_global_with};
use strum::EnumCount;

pub mod disassembly;
pub mod frame;
pub mod logs;
pub mod stack;
pub mod status;

#[derive(Copy, Clone, EnumCount, TryFromPrimitive, Eq, PartialEq)]
#[repr(i16)]
pub enum Pane {
    Frame = 0,
    Disassembly = 1,
    Stack = 2,
}

impl Pane {
    fn state() -> State<Pane> {
        use_global_with("active-pane", || Pane::Frame)
    }

    pub fn is_active(&self) -> bool {
        let state = Pane::state();
        let current_pane = state.get();
        current_pane == *self
    }

    pub fn next() {
        let state = Pane::state();
        // eprintln!("state rn: {}", state.get() as u8);
        let current_pane = state.get();
        let next_pane = (current_pane as i16 + 1).rem_euclid(Pane::COUNT as i16);
        // eprintln!("setting state to: {}", next_pane);
        state.set(Pane::try_from(next_pane).expect("Invalid pane!"));
    }
    pub fn prev() {
        let state = Pane::state();
        let current_pane = state.get();
        let prev_pane = (current_pane as i16 - 1).rem_euclid(Pane::COUNT as i16);
        state.set(Pane::try_from(prev_pane).expect("Invalid pane!"));
    }
}
