//! Pure simulation core: no terminal I/O, deterministic given seed + inputs.
//! One tick is one game minute; a shift is 480 ticks (22:00-06:00).

pub mod call;
pub mod catalog;
pub mod config;
pub mod map;
pub mod path;
pub mod rng;
pub mod schedule;
pub mod score;
pub mod shift;
pub mod units;
pub mod week;

pub use call::{Call, CallStatus, Report};
pub use catalog::{Needs, Template, TEMPLATES};
pub use config::{Difficulty, UpgradeKind, Upgrades, NIGHTS, SHIFT_TICKS};
pub use map::{CityMap, District, Pos, Tile};
pub use score::{Grade, ScoreBreakdown, ShiftStats};
pub use shift::{CmdError, Command, Event, LogEntry, Shift, ShiftEnd, ShiftParams, Tip};
pub use units::{Unit, UnitKind, UnitState};
pub use week::{Ending, NightRecord, Week};
