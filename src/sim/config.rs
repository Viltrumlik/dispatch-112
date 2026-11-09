//! Difficulty parameters, unit speeds and between-shift upgrades.

use super::units::UnitKind;
use serde::{Deserialize, Serialize};

/// Ticks (game minutes) in one shift: 22:00 -> 06:00.
pub const SHIFT_TICKS: u32 = 480;
/// Number of night shifts in the week (Monday..Friday).
pub const NIGHTS: u8 = 5;
/// Maximum number of units (keys 1-9).
pub const MAX_UNITS: usize = 9;
/// Meters are kept in hundredths: 10_000 == 100.0.
pub const METER_MAX: i32 = 10_000;
pub const TRUST_START: i32 = 6_000;
/// Length of the coffee break in ticks.
pub const COFFEE_TICKS: u32 = 15;
/// A real call nobody has been sent to for this long starts draining trust...
pub const IGNORED_AFTER: u32 = 12;
/// ...by this much per tick (hundredths).
pub const IGNORED_DRAIN: i32 = 5;
/// Recently closed calls stay visible (and flaggable) in the queue this long.
pub const CLOSED_LINGER: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Difficulty {
    Rookie,
    Dispatcher,
    Veteran,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Rookie, Difficulty::Dispatcher, Difficulty::Veteran];

    pub fn key(self) -> &'static str {
        match self {
            Difficulty::Rookie => "difficulty.rookie",
            Difficulty::Dispatcher => "difficulty.dispatcher",
            Difficulty::Veteran => "difficulty.veteran",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Difficulty::Rookie => "rookie",
            Difficulty::Dispatcher => "dispatcher",
            Difficulty::Veteran => "veteran",
        }
    }

    pub fn from_code(s: &str) -> Option<Difficulty> {
        match s.to_ascii_lowercase().as_str() {
            "rookie" | "easy" | "1" => Some(Difficulty::Rookie),
            "dispatcher" | "normal" | "2" => Some(Difficulty::Dispatcher),
            "veteran" | "hard" | "3" => Some(Difficulty::Veteran),
            _ => None,
        }
    }

    pub fn next(self) -> Difficulty {
        match self {
            Difficulty::Rookie => Difficulty::Dispatcher,
            Difficulty::Dispatcher => Difficulty::Veteran,
            Difficulty::Veteran => Difficulty::Rookie,
        }
    }

    pub fn prev(self) -> Difficulty {
        self.next().next()
    }

    pub fn params(self) -> DiffParams {
        match self {
            Difficulty::Rookie => DiffParams {
                interval: 23,
                night_growth: 10,
                esc_interval: 34,
                crit_window: 30,
                units: [3, 2, 2],
                fail_trust: 800,
                stress_mult: 80,
                prank_share: 120,
                prank_patience: 40,
            },
            Difficulty::Dispatcher => DiffParams {
                interval: 16,
                night_growth: 12,
                esc_interval: 26,
                crit_window: 22,
                units: [3, 2, 2],
                fail_trust: 1000,
                stress_mult: 100,
                prank_share: 140,
                prank_patience: 35,
            },
            Difficulty::Veteran => DiffParams {
                interval: 14,
                night_growth: 14,
                esc_interval: 22,
                crit_window: 19,
                units: [2, 2, 2],
                fail_trust: 1000,
                stress_mult: 120,
                prank_share: 160,
                prank_patience: 30,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DiffParams {
    /// Mean ticks between ordinary calls on Monday.
    pub interval: u32,
    /// Extra call rate per night, in percent.
    pub night_growth: u32,
    /// Ticks for an unattended call to rise one severity level.
    pub esc_interval: u32,
    /// Ticks a severity-3 call survives without help.
    pub crit_window: u32,
    /// Starting ambulances, fire engines, patrol cars.
    pub units: [u8; 3],
    /// Trust lost when a call fails (hundredths).
    pub fail_trust: i32,
    /// Stress gain multiplier (percent).
    pub stress_mult: i32,
    /// Share of pranks among ordinary calls (permille).
    pub prank_share: u32,
    /// Ticks a prank caller waits before hanging up.
    pub prank_patience: u32,
}

/// Movement speed in hundredths of a cell per tick.
pub fn siren_speed(kind: UnitKind) -> u32 {
    match kind {
        UnitKind::Ambulance => 200,
        UnitKind::Fire => 160,
        UnitKind::Police => 240,
    }
}

pub const RETURN_SPEED: u32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpgradeKind {
    ExtraAmbulance,
    FieldPost,
    ExtraFire,
    ExtraPolice,
    FastFire,
    Screening,
    GreenWave,
    CalmTraining,
    RapidTriage,
}

impl UpgradeKind {
    pub const ALL: [UpgradeKind; 9] = [
        UpgradeKind::ExtraAmbulance,
        UpgradeKind::FieldPost,
        UpgradeKind::ExtraFire,
        UpgradeKind::ExtraPolice,
        UpgradeKind::FastFire,
        UpgradeKind::Screening,
        UpgradeKind::GreenWave,
        UpgradeKind::CalmTraining,
        UpgradeKind::RapidTriage,
    ];

    pub fn key(self) -> &'static str {
        match self {
            UpgradeKind::ExtraAmbulance => "upgrade.extra_ambulance",
            UpgradeKind::FieldPost => "upgrade.field_post",
            UpgradeKind::ExtraFire => "upgrade.extra_fire",
            UpgradeKind::ExtraPolice => "upgrade.extra_police",
            UpgradeKind::FastFire => "upgrade.fast_fire",
            UpgradeKind::Screening => "upgrade.screening",
            UpgradeKind::GreenWave => "upgrade.green_wave",
            UpgradeKind::CalmTraining => "upgrade.calm_training",
            UpgradeKind::RapidTriage => "upgrade.rapid_triage",
        }
    }

    /// Adds a vehicle to the fleet.
    pub fn adds_unit(self) -> bool {
        matches!(
            self,
            UpgradeKind::ExtraAmbulance | UpgradeKind::FieldPost | UpgradeKind::ExtraFire | UpgradeKind::ExtraPolice
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upgrades {
    pub taken: Vec<UpgradeKind>,
}

impl Upgrades {
    pub fn has(&self, u: UpgradeKind) -> bool {
        self.taken.contains(&u)
    }

    pub fn count(&self, u: UpgradeKind) -> usize {
        self.taken.iter().filter(|&&t| t == u).count()
    }

    pub fn extra_units(&self) -> usize {
        self.taken.iter().filter(|u| u.adds_unit()).count()
    }
}
