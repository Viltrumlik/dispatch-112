//! The week: five night shifts, carried-over meters, upgrades, the arsonist
//! investigation and the five endings.

use super::config::*;
use super::rng::{mix, Rng};
use super::score::{self, Grade, ScoreBreakdown, ShiftStats};
use super::shift::{build_fleet, Shift, ShiftEnd, ShiftParams};
use serde::{Deserialize, Serialize};

/// Correct flags needed (by the end of Thursday) for Inspector Dilshod's trap.
pub const TRAP_FLAGS: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ending {
    /// E1: all five shifts B or better and the arsonist caught.
    GoldenHeadset,
    /// E2: week completed, arsonist not caught.
    SteadyHand,
    /// E3: arsonist caught, but at least one shift graded C or D.
    NightDetective,
    /// E4: city trust fell to zero — fired.
    BadgeReturned,
    /// E5: stress reached 100 — walked out mid-shift.
    Burnout,
}

impl Ending {
    pub fn code(self) -> &'static str {
        match self {
            Ending::GoldenHeadset => "E1",
            Ending::SteadyHand => "E2",
            Ending::NightDetective => "E3",
            Ending::BadgeReturned => "E4",
            Ending::Burnout => "E5",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Ending::GoldenHeadset => "ending.e1",
            Ending::SteadyHand => "ending.e2",
            Ending::NightDetective => "ending.e3",
            Ending::BadgeReturned => "ending.e4",
            Ending::Burnout => "ending.e5",
        }
    }

    pub fn is_loss(self) -> bool {
        matches!(self, Ending::BadgeReturned | Ending::Burnout)
    }
}

/// Pure ending selection. `end` is how the last played shift ended.
pub fn select_ending(grades: &[Grade], end: ShiftEnd, nights_played: u8, caught: bool) -> Option<Ending> {
    match end {
        ShiftEnd::Fired => return Some(Ending::BadgeReturned),
        ShiftEnd::Burnout => return Some(Ending::Burnout),
        ShiftEnd::Completed => {}
    }
    if nights_played < NIGHTS {
        return None;
    }
    let all_good = grades.len() == NIGHTS as usize && grades.iter().all(|g| g.is_good());
    Some(match (caught, all_good) {
        (true, true) => Ending::GoldenHeadset,
        (true, false) => Ending::NightDetective,
        (false, _) => Ending::SteadyHand,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NightRecord {
    pub night: u8,
    pub stats: ShiftStats,
    pub score: ScoreBreakdown,
    pub grade: Grade,
    pub end: ShiftEnd,
    pub trust_end: i32,
    pub stress_end: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Week {
    pub difficulty: Difficulty,
    pub seed: u64,
    /// Index of the next shift to play (0 = Monday).
    pub night: u8,
    pub trust: i32,
    /// Stress carried into the next shift (hundredths).
    pub stress: i32,
    pub upgrades: Upgrades,
    pub records: Vec<NightRecord>,
    pub flags_correct: u32,
    pub flags_false: u32,
    pub trap_set: bool,
    pub arsonist_caught: bool,
    pub ending: Option<Ending>,
    /// Show tutorial tips on Monday.
    pub tutorial: bool,
}

impl Week {
    pub fn new(difficulty: Difficulty, seed: u64) -> Week {
        Week {
            difficulty,
            seed,
            night: 0,
            trust: TRUST_START,
            stress: 0,
            upgrades: Upgrades::default(),
            records: Vec::new(),
            flags_correct: 0,
            flags_false: 0,
            trap_set: false,
            arsonist_caught: false,
            ending: None,
            tutorial: true,
        }
    }

    pub fn is_over(&self) -> bool {
        self.ending.is_some()
    }

    pub fn shift_params(&self) -> ShiftParams {
        ShiftParams {
            night: self.night,
            difficulty: self.difficulty,
            seed: self.seed,
            upgrades: self.upgrades.clone(),
            trust: self.trust,
            stress: self.stress,
            trap_set: self.trap_set,
            tutorial: self.tutorial && self.night == 0,
        }
    }

    pub fn start_shift(&self) -> Shift {
        Shift::new(self.shift_params())
    }

    pub fn grades(&self) -> Vec<Grade> {
        self.records.iter().map(|r| r.grade).collect()
    }

    /// Whether the inspector's trap condition currently holds.
    pub fn trap_condition(correct: u32, false_flags: u32) -> bool {
        correct >= TRAP_FLAGS && false_flags <= correct
    }

    /// Record a finished shift; updates meters, the investigation and the ending.
    pub fn finish_shift(&mut self, shift: &Shift) -> NightRecord {
        let end = shift.end.unwrap_or(ShiftEnd::Completed);
        let breakdown = score::score(&shift.stats);
        let grade = score::grade(breakdown.total);
        let record = NightRecord {
            night: self.night,
            stats: shift.stats.clone(),
            score: breakdown,
            grade,
            end,
            trust_end: shift.trust,
            stress_end: shift.stress,
        };
        self.records.push(record.clone());
        self.trust = shift.trust;
        // A day's sleep takes most of the edge off.
        self.stress = shift.stress * 40 / 100;
        if self.night < schedule_finale_night() {
            self.flags_correct += shift.stats.flags_correct;
            self.flags_false += shift.stats.flags_false;
            self.trap_set = Week::trap_condition(self.flags_correct, self.flags_false);
        }
        if shift.arsonist_caught {
            self.arsonist_caught = true;
        }
        self.night += 1;
        self.ending = select_ending(&self.grades(), end, self.night, self.arsonist_caught);
        record
    }

    /// Three upgrade choices for the next night (deterministic per seed/night).
    pub fn upgrade_options(&self) -> Vec<UpgradeKind> {
        let units = build_fleet(self.difficulty, &self.upgrades).len();
        let mut pool: Vec<UpgradeKind> = UpgradeKind::ALL
            .iter()
            .copied()
            .filter(|u| {
                if u.adds_unit() {
                    units < MAX_UNITS && !(*u == UpgradeKind::FieldPost && self.upgrades.has(*u))
                } else {
                    !self.upgrades.has(*u)
                }
            })
            .collect();
        let mut rng = Rng::new(mix(self.seed, 500 + self.night as u64));
        let mut out = Vec::new();
        while out.len() < 3 && !pool.is_empty() {
            let i = rng.below(pool.len() as u32) as usize;
            out.push(pool.remove(i));
        }
        out
    }

    pub fn apply_upgrade(&mut self, u: UpgradeKind) {
        self.upgrades.taken.push(u);
    }
}

fn schedule_finale_night() -> u8 {
    super::schedule::FINALE_NIGHT
}

#[cfg(test)]
mod tests {
    use super::*;
    use Grade::*;

    #[test]
    fn ending_selection_table() {
        let good = [A, B, S, B, A];
        let mixed = [A, C, B, B, A];
        assert_eq!(select_ending(&good, ShiftEnd::Completed, 5, true), Some(Ending::GoldenHeadset));
        assert_eq!(select_ending(&mixed, ShiftEnd::Completed, 5, true), Some(Ending::NightDetective));
        assert_eq!(select_ending(&good, ShiftEnd::Completed, 5, false), Some(Ending::SteadyHand));
        assert_eq!(select_ending(&mixed, ShiftEnd::Completed, 5, false), Some(Ending::SteadyHand));
        assert_eq!(select_ending(&good[..2], ShiftEnd::Fired, 2, false), Some(Ending::BadgeReturned));
        assert_eq!(select_ending(&good[..3], ShiftEnd::Burnout, 3, true), Some(Ending::Burnout));
        assert_eq!(select_ending(&good[..3], ShiftEnd::Completed, 3, false), None);
    }

    #[test]
    fn trap_needs_four_correct_and_few_false_flags() {
        assert!(!Week::trap_condition(3, 0));
        assert!(Week::trap_condition(4, 0));
        assert!(Week::trap_condition(4, 4));
        assert!(!Week::trap_condition(4, 5));
    }

    #[test]
    fn upgrade_options_are_distinct_and_respect_fleet_cap() {
        let mut w = Week::new(Difficulty::Rookie, 9);
        for _ in 0..4 {
            let opts = w.upgrade_options();
            assert_eq!(opts.len(), 3);
            let mut d = opts.clone();
            d.dedup();
            assert_eq!(d.len(), 3);
            w.apply_upgrade(opts[0]);
            w.night += 1;
        }
        assert!(build_fleet(w.difficulty, &w.upgrades).len() <= MAX_UNITS);
    }
}
