//! `--headless`: run the bot through a whole week without a terminal UI and
//! print a summary; with `--seeds N`, print a balance table.

use crate::args;
use crate::bot::{play_week, BotProfile};
use crate::i18n::{t, tf, Lang};
use crate::sim::{Difficulty, Ending, Grade, Week};
use std::fmt::Write as _;

fn night_name(lang: Lang, n: u8) -> &'static str {
    t(lang, &format!("night.{n}"))
}

/// Human-readable report of one simulated week.
pub fn week_summary(lang: Lang, week: &Week, bot_name: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{}",
        tf(
            lang,
            "headless.title",
            args!(seed = week.seed, difficulty = t(lang, week.difficulty.key()), bot = bot_name)
        )
    );
    let col = |k: &str| t(lang, &format!("headless.col.{k}"));
    let _ = writeln!(
        s,
        "{:<12} {:>5} {:>6} {:>6} {:>7} {:>6} {:>6} {:>6} {:>5} {:>6} {:>6}",
        col("night"),
        col("calls"),
        col("resolved"),
        col("failed"),
        col("pranks"),
        col("wrong"),
        col("resp"),
        col("score"),
        col("grade"),
        col("trust"),
        col("stress")
    );
    for r in &week.records {
        let resp = if r.stats.response_count > 0 {
            format!("{:.1}", r.stats.response_ticks as f64 / r.stats.response_count as f64)
        } else {
            "-".into()
        };
        let _ = writeln!(
            s,
            "{:<12} {:>5} {:>6} {:>6} {:>7} {:>6} {:>6} {:>6} {:>5} {:>6} {:>6}",
            night_name(lang, r.night),
            r.stats.calls,
            r.stats.resolved,
            r.stats.failed,
            format!("{}/{}", r.stats.prank_dismissed, r.stats.pranks),
            r.stats.wrong_units,
            resp,
            r.score.total,
            r.grade.letter(),
            r.trust_end / 100,
            r.stress_end / 100,
        );
    }
    let _ = writeln!(
        s,
        "{}",
        tf(
            lang,
            "headless.flags",
            args!(
                correct = week.flags_correct,
                wrong = week.flags_false,
                trap = t(lang, if week.trap_set { "headless.yes" } else { "headless.no" }),
                caught = t(lang, if week.arsonist_caught { "headless.yes" } else { "headless.no" })
            )
        )
    );
    match week.ending {
        Some(e) => {
            let _ = writeln!(s, "{}", tf(lang, "headless.ending", args!(code = e.code(), name = t(lang, e.key()))));
        }
        None => {
            let _ = writeln!(s, "{}", t(lang, "headless.unfinished"));
        }
    }
    s
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BalanceRow {
    pub weeks: u32,
    pub completed: u32,
    pub endings: [u32; 5],
    pub caught: u32,
    pub shifts: u32,
    pub good_shifts: u32,
    pub score_sum: u32,
    pub trust_sum: i64,
    pub failed_sum: u32,
}

impl BalanceRow {
    pub fn add(&mut self, w: &Week) {
        self.weeks += 1;
        if let Some(e) = w.ending {
            let i = match e {
                Ending::GoldenHeadset => 0,
                Ending::SteadyHand => 1,
                Ending::NightDetective => 2,
                Ending::BadgeReturned => 3,
                Ending::Burnout => 4,
            };
            self.endings[i] += 1;
            if !e.is_loss() {
                self.completed += 1;
            }
        }
        if w.arsonist_caught {
            self.caught += 1;
        }
        for r in &w.records {
            self.shifts += 1;
            if r.grade >= Grade::B {
                self.good_shifts += 1;
            }
            self.score_sum += r.score.total;
            self.failed_sum += r.stats.failed;
        }
        self.trust_sum += (w.trust / 100) as i64;
    }

    pub fn avg_score(&self) -> f64 {
        if self.shifts == 0 {
            0.0
        } else {
            self.score_sum as f64 / self.shifts as f64
        }
    }

    pub fn good_share(&self) -> f64 {
        if self.shifts == 0 {
            0.0
        } else {
            100.0 * self.good_shifts as f64 / self.shifts as f64
        }
    }
}

pub fn balance_row(difficulty: Difficulty, seeds: u32, profile: BotProfile) -> BalanceRow {
    let mut row = BalanceRow::default();
    for seed in 1..=seeds as u64 {
        row.add(&play_week(difficulty, seed, profile));
    }
    row
}

/// Balance table across difficulties (seeds 1..=N each).
pub fn balance_table(
    lang: Lang,
    difficulties: &[Difficulty],
    seeds: u32,
    profile: BotProfile,
    bot_name: &str,
) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "{}", tf(lang, "headless.table_title", args!(seeds = seeds, bot = bot_name)));
    let _ = writeln!(s);
    let tc = |k: &str| t(lang, &format!("headless.tcol.{k}"));
    let _ = writeln!(
        s,
        "| {} | {} | {} | E1 | E2 | E3 | E4 | E5 | {} | {} | {} | {} |",
        tc("difficulty"),
        tc("weeks"),
        tc("completed"),
        tc("caught"),
        tc("good"),
        tc("score"),
        tc("failed")
    );
    let _ = writeln!(s, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for &d in difficulties {
        let r = balance_row(d, seeds, profile);
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0}% | {:.1} | {:.1} |",
            t(lang, d.key()),
            r.weeks,
            r.completed,
            r.endings[0],
            r.endings[1],
            r.endings[2],
            r.endings[3],
            r.endings[4],
            r.caught,
            r.good_share(),
            r.avg_score(),
            if r.shifts == 0 { 0.0 } else { r.failed_sum as f64 / r.shifts as f64 },
        );
    }
    s
}
