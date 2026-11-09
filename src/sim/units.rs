//! Emergency vehicles: ambulances, fire engines and patrol cars.

use super::map::Pos;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UnitKind {
    Ambulance,
    Fire,
    Police,
}

impl UnitKind {
    pub const ALL: [UnitKind; 3] = [UnitKind::Ambulance, UnitKind::Fire, UnitKind::Police];

    pub fn idx(self) -> usize {
        match self {
            UnitKind::Ambulance => 0,
            UnitKind::Fire => 1,
            UnitKind::Police => 2,
        }
    }

    /// Language-neutral letter used in unit codes (A1, F2, P1).
    pub fn letter(self) -> char {
        match self {
            UnitKind::Ambulance => 'A',
            UnitKind::Fire => 'F',
            UnitKind::Police => 'P',
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            UnitKind::Ambulance => "unit.ambulance",
            UnitKind::Fire => "unit.fire",
            UnitKind::Police => "unit.police",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitState {
    AtBase,
    EnRoute {
        call: u32,
    },
    /// On scene until `until`. `working` is false when the crew only checks
    /// the address (wrong unit type or a prank) before leaving.
    OnScene {
        call: u32,
        until: u32,
        working: bool,
    },
    Returning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unit {
    /// Index in the fleet; the player's key is `id + 1`.
    pub id: usize,
    pub kind: UnitKind,
    /// Per-kind ordinal used in the code (A1, A2...).
    pub ordinal: u8,
    pub base: Pos,
    pub pos: Pos,
    pub state: UnitState,
    pub path: VecDeque<Pos>,
    /// Movement accumulator in hundredths of a cell.
    pub budget: u32,
    /// Number of jobs completed this shift.
    pub jobs: u32,
}

impl Unit {
    pub fn new(id: usize, kind: UnitKind, ordinal: u8, base: Pos) -> Unit {
        Unit { id, kind, ordinal, base, pos: base, state: UnitState::AtBase, path: VecDeque::new(), budget: 0, jobs: 0 }
    }

    pub fn code(&self) -> String {
        format!("{}{}", self.kind.letter(), self.ordinal)
    }

    /// Can be sent to a new call right now.
    pub fn is_available(&self) -> bool {
        matches!(self.state, UnitState::AtBase | UnitState::Returning)
    }

    /// The call this unit is committed to, if any.
    pub fn call(&self) -> Option<u32> {
        match self.state {
            UnitState::EnRoute { call } | UnitState::OnScene { call, .. } => Some(call),
            _ => None,
        }
    }

    pub fn is_working_on(&self, call_id: u32) -> bool {
        matches!(self.state, UnitState::OnScene { call, working: true, .. } if call == call_id)
    }
}
