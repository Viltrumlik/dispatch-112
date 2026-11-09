//! Shift statistics, score breakdown and letter grades.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShiftStats {
    pub calls: u32,
    pub real_calls: u32,
    pub pranks: u32,
    pub resolved: u32,
    pub failed: u32,
    pub lives_lost: u32,
    pub handed_over: u32,
    pub prank_dismissed: u32,
    pub prank_dispatched: u32,
    pub prank_expired: u32,
    pub real_dismissed: u32,
    pub wrong_units: u32,
    /// Sum of per-call response quality in permille.
    pub response_quality: u32,
    pub response_count: u32,
    pub response_ticks: u32,
    pub arson_calls: u32,
    pub flags_correct: u32,
    pub flags_false: u32,
    pub coffee: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Grade {
    D,
    C,
    B,
    A,
    S,
}

impl Grade {
    pub fn letter(self) -> &'static str {
        match self {
            Grade::S => "S",
            Grade::A => "A",
            Grade::B => "B",
            Grade::C => "C",
            Grade::D => "D",
        }
    }

    /// B or better.
    pub fn is_good(self) -> bool {
        self >= Grade::B
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Lives and property: share of real calls brought to a good end (0..=50).
    pub outcome: u32,
    /// Speed of the first useful unit versus the call's severity (0..=30).
    pub response: u32,
    /// Prank handling accuracy (0..=20).
    pub pranks: u32,
    /// Deductions for wrong-unit dispatches.
    pub penalty: u32,
    pub total: u32,
}

pub const GRADE_S: u32 = 92;
pub const GRADE_A: u32 = 84;
pub const GRADE_B: u32 = 72;
pub const GRADE_C: u32 = 56;

/// Target response time (ticks) for a call of a given base severity.
pub fn response_target(severity: u8) -> u32 {
    match severity {
        3 => 10,
        2 => 16,
        _ => 26,
    }
}

/// Quality of one response in permille: full marks up to the target, then a
/// linear fall to zero at three times the target.
pub fn response_quality(severity: u8, ticks: u32) -> u32 {
    let target = response_target(severity);
    if ticks <= target {
        1000
    } else {
        let over = ticks - target;
        1000u32.saturating_sub(over * 1000 / (2 * target))
    }
}

pub fn score(s: &ShiftStats) -> ScoreBreakdown {
    // Outcome: resolved vs. failed / wrongly dismissed; calls still open at the
    // end of the shift count half.
    let denom2 = 2 * (s.resolved + s.failed + s.real_dismissed) + s.handed_over;
    let ratio_permille = (2000 * s.resolved).checked_div(denom2).map_or(1000, |r| r + 500 * s.handed_over / denom2);
    // Squared so each loss hurts noticeably.
    let outcome = 50 * ratio_permille * ratio_permille / 1_000_000;

    // No useful unit ever arrived: full marks only if there was nothing to answer.
    let response = match s.response_count {
        0 if s.real_calls > 0 => 0,
        0 => 30,
        n => 30 * s.response_quality / (1000 * n),
    };

    // Pranks: correct dismissals vs. mistakes (dispatching to a prank, a prank left
    // hanging, dismissing a real emergency counts double).
    let good = s.prank_dismissed * 2;
    let bad = s.prank_dispatched * 2 + s.prank_expired + s.real_dismissed * 4;
    let pranks = (20 * good).checked_div(good + bad).unwrap_or(20);

    let penalty = s.wrong_units.min(15);
    let total = (outcome + response + pranks).saturating_sub(penalty);
    ScoreBreakdown { outcome, response, pranks, penalty, total }
}

pub fn grade(total: u32) -> Grade {
    match total {
        t if t >= GRADE_S => Grade::S,
        t if t >= GRADE_A => Grade::A,
        t if t >= GRADE_B => Grade::B,
        t if t >= GRADE_C => Grade::C,
        _ => Grade::D,
    }
}
