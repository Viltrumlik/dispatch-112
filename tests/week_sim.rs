//! Headless whole-week simulations with scripted bots.

use dispatch112::bot::{play_week, BotProfile};
use dispatch112::headless::{balance_row, BalanceRow};
use dispatch112::sim::*;

const SEEDS: u32 = 20;

#[test]
fn good_bot_finishes_rookie_week_with_good_grades_and_catches_the_arsonist() {
    let mut weeks = 0;
    let mut completed = 0;
    let mut caught = 0;
    let mut good = 0;
    let mut shifts = 0;
    for seed in 1..=SEEDS as u64 {
        let w = play_week(Difficulty::Rookie, seed, BotProfile::good());
        weeks += 1;
        if w.ending.is_some_and(|e| !e.is_loss()) {
            completed += 1;
        }
        if w.arsonist_caught {
            caught += 1;
            assert!(w.trap_set, "seed {seed}: caught without a trap");
            assert!(w.flags_correct >= dispatch112::sim::week::TRAP_FLAGS);
        }
        for r in &w.records {
            shifts += 1;
            if r.grade.is_good() {
                good += 1;
            }
        }
    }
    assert_eq!(completed, weeks, "a good dispatcher should survive Rookie");
    assert!(good * 100 >= shifts * 90, "grades >= B in {good}/{shifts} shifts");
    assert!(caught * 100 >= weeks * 80, "arsonist caught in only {caught}/{weeks} weeks");
}

#[test]
fn bot_that_never_flags_never_catches_the_arsonist() {
    for seed in 1..=8 {
        let w = play_week(Difficulty::Rookie, seed, BotProfile::no_flags());
        assert!(!w.trap_set);
        assert!(!w.arsonist_caught);
        if let Some(e) = w.ending.filter(|e| !e.is_loss()) {
            assert_eq!(e, Ending::SteadyHand);
        }
    }
}

#[test]
fn do_nothing_bot_is_fired_within_the_first_two_shifts() {
    for d in Difficulty::ALL {
        for seed in 1..=SEEDS as u64 {
            let w = play_week(d, seed, BotProfile::idle());
            assert_eq!(w.ending, Some(Ending::BadgeReturned), "{d:?} seed {seed}: {:?}", w.ending);
            assert!(w.records.len() <= 2, "{d:?} seed {seed}: survived {} shifts", w.records.len());
            assert_eq!(w.trust, 0);
        }
    }
}

#[test]
fn veteran_is_measurably_harder_than_dispatcher() {
    let disp: BalanceRow = balance_row(Difficulty::Dispatcher, SEEDS, BotProfile::good());
    let vet: BalanceRow = balance_row(Difficulty::Veteran, SEEDS, BotProfile::good());
    let rookie: BalanceRow = balance_row(Difficulty::Rookie, SEEDS, BotProfile::good());
    assert!(vet.completed < disp.completed, "completed: veteran {} vs dispatcher {}", vet.completed, disp.completed);
    assert!(vet.avg_score() + 2.0 < disp.avg_score(), "score: {} vs {}", vet.avg_score(), disp.avg_score());
    assert!(disp.avg_score() < rookie.avg_score() + 0.01);
    let lost = |r: &BalanceRow| r.failed_sum as f64 / r.shifts as f64;
    assert!(lost(&vet) > lost(&disp) && lost(&disp) > lost(&rookie));
}

#[test]
fn sloppy_play_is_punished_on_dispatcher() {
    let good = balance_row(Difficulty::Dispatcher, 12, BotProfile::good());
    let sloppy = balance_row(Difficulty::Dispatcher, 12, BotProfile::sloppy());
    assert!(sloppy.avg_score() + 3.0 < good.avg_score());
    assert!(sloppy.good_share() < good.good_share());
}
