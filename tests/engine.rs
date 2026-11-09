//! Pure engine tests: pathfinding, travel, resolution rules, escalation,
//! pranks, scoring, endings and determinism.

use dispatch112::bot::{play_week, BotProfile};
use dispatch112::sim::catalog;
use dispatch112::sim::map::{MAP_H, MAP_W};
use dispatch112::sim::path::{distance, find_path};
use dispatch112::sim::schedule::{self, Scheduled};
use dispatch112::sim::score::{grade, score};
use dispatch112::sim::week::select_ending;
use dispatch112::sim::*;

/// Bazaar street two cells north of the main avenue.
const BAZAAR: Pos = Pos::new(21, 2);
/// A street in the Industrial Zone.
const INDUSTRIAL: Pos = Pos::new(28, 11);

fn sched(id: &str, tick: u32, pos: Pos, severity: u8) -> Scheduled {
    Scheduled { tick, template: catalog::find(id), variant: 0, pos, severity, story: false }
}

fn params(d: Difficulty) -> ShiftParams {
    // Tuesday: no storm, no tutorial tips.
    let mut p = ShiftParams::new(1, d, 42);
    p.tutorial = false;
    p
}

fn shift_with(calls: Vec<Scheduled>) -> Shift {
    Shift::with_schedule(params(Difficulty::Dispatcher), calls)
}

/// Process ticks up to and including `t`.
fn run_through(s: &mut Shift, t: u32) {
    while s.tick <= t && !s.is_over() {
        s.tick();
    }
}

fn first_unit(s: &Shift, k: UnitKind) -> usize {
    s.units.iter().find(|u| u.kind == k).map(|u| u.id).unwrap()
}

// ------------------------------------------------------------- pathfinding

#[test]
fn path_is_contiguous_shortest_and_on_roads() {
    let m = CityMap::get();
    let p = find_path(m, m.hospital, m.fire_station, &[]).expect("reachable");
    assert_eq!(*p.last().unwrap(), m.fire_station);
    let mut prev = m.hospital;
    for &c in &p {
        assert_eq!(prev.manhattan(c), 1, "path must move one cell at a time");
        assert!(m.is_passable(c));
        prev = c;
    }
    // BFS is optimal on a unit grid: never shorter than Manhattan distance.
    assert!(p.len() as i32 >= m.hospital.manhattan(m.fire_station));
    assert_eq!(find_path(m, m.hospital, m.hospital, &[]), Some(vec![]));
}

#[test]
fn path_respects_water_and_blocked_bridges() {
    let m = CityMap::get();
    // Water is never a destination.
    assert!(find_path(m, m.hospital, Pos::new(10, 8), &[]).is_none());
    // Closing the Bazaar bridge can only make a crossing longer.
    let open = distance(m, BAZAAR, INDUSTRIAL, &[]).unwrap();
    let closed = distance(m, BAZAAR, INDUSTRIAL, &[m.flood_bridge]).unwrap();
    assert!(closed > open, "flooded bridge should force a detour ({open} vs {closed})");
    // Closing every bridge cuts the city in half.
    let all = [Pos::new(7, 8), Pos::new(21, 8), Pos::new(35, 8), Pos::new(49, 8)];
    assert!(find_path(m, BAZAAR, INDUSTRIAL, &all).is_none());
}

#[test]
fn whole_road_network_is_connected() {
    let m = CityMap::get();
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let p = Pos::new(x, y);
            if m.is_passable(p) {
                assert!(distance(m, m.hospital, p, &[]).is_some(), "{p:?} unreachable");
            }
        }
    }
}

// ------------------------------------------------------------- travel

#[test]
fn ambulance_travels_works_and_returns_to_base() {
    let m = CityMap::get();
    let mut s = shift_with(vec![sched("heart_attack", 0, BAZAAR, 2)]);
    s.tick();
    let call = s.calls[0].id;
    let amb = first_unit(&s, UnitKind::Ambulance);
    s.apply(&Command::Dispatch { call, units: vec![amb] }).unwrap();
    assert!(matches!(s.units[amb].state, UnitState::EnRoute { .. }));

    let len = distance(m, m.hospital, BAZAAR, &[]).unwrap() as u32;
    let expected = (len * 100).div_ceil(s.speed(&s.units[amb], true));
    let mut ticks = 0;
    while !matches!(s.units[amb].state, UnitState::OnScene { .. }) {
        s.tick();
        ticks += 1;
        assert!(ticks < 100);
    }
    assert_eq!(ticks, expected, "travel time = path length / siren speed");
    assert_eq!(s.units[amb].pos, BAZAAR);

    while s.calls[0].status == CallStatus::Waiting {
        s.tick();
    }
    assert_eq!(s.calls[0].status, CallStatus::Resolved);
    assert_eq!(s.stats.resolved, 1);
    assert_eq!(s.units[amb].state, UnitState::Returning);
    let mut guard = 0;
    while s.units[amb].state != UnitState::AtBase {
        s.tick();
        guard += 1;
        assert!(guard < 100);
    }
    assert_eq!(s.units[amb].pos, m.hospital);
    assert!(s.units[amb].is_available());
}

#[test]
fn busy_unit_cannot_be_dispatched_twice() {
    let mut s = shift_with(vec![sched("heart_attack", 0, BAZAAR, 2), sched("stroke", 0, INDUSTRIAL, 3)]);
    s.tick();
    let amb = first_unit(&s, UnitKind::Ambulance);
    s.apply(&Command::Dispatch { call: 1, units: vec![amb] }).unwrap();
    assert_eq!(s.apply(&Command::Dispatch { call: 2, units: vec![amb] }), Err(CmdError::UnitBusy(amb)));
    assert_eq!(s.apply(&Command::Dispatch { call: 2, units: vec![] }), Err(CmdError::NoUnits));
    assert_eq!(s.apply(&Command::Dispatch { call: 99, units: vec![1] }), Err(CmdError::NoSuchCall));
}

// ------------------------------------------------------------- resolution

#[test]
fn wrong_unit_type_does_not_resolve_but_reveals_the_truth() {
    let mut s = shift_with(vec![sched("heart_attack", 0, BAZAAR, 2)]);
    s.tick();
    let police = first_unit(&s, UnitKind::Police);
    s.apply(&Command::Dispatch { call: 1, units: vec![police] }).unwrap();
    let mut guard = 0;
    while s.stats.wrong_units == 0 {
        s.tick();
        guard += 1;
        assert!(guard < 100);
    }
    let c = &s.calls[0];
    assert_eq!(c.status, CallStatus::Waiting, "a patrol car cannot treat a heart attack");
    assert!(c.revealed);
    assert_eq!(c.report, Some(Report::WrongUnit));
    assert_eq!(c.served, Needs::NONE);
    // The crew leaves after a short look.
    for _ in 0..3 {
        s.tick();
    }
    assert_eq!(s.units[police].state, UnitState::Returning);
    assert!(s.log.iter().any(|e| matches!(e.event, Event::WrongUnit { .. })));
}

#[test]
fn multi_service_call_needs_every_service() {
    let mut s = shift_with(vec![sched("car_crash", 0, BAZAAR, 2)]);
    s.tick();
    let amb = first_unit(&s, UnitKind::Ambulance);
    s.apply(&Command::Dispatch { call: 1, units: vec![amb] }).unwrap();
    // Let the ambulance finish its work.
    let mut guard = 0;
    while s.calls[0].served.amb == 0 {
        s.tick();
        guard += 1;
        assert!(guard < 200);
    }
    assert_eq!(s.calls[0].status, CallStatus::Waiting, "police still missing");
    assert_eq!(s.calls[0].report, Some(Report::NeedMore));
    let police = first_unit(&s, UnitKind::Police);
    s.apply(&Command::Dispatch { call: 1, units: vec![police] }).unwrap();
    while s.calls[0].status == CallStatus::Waiting {
        s.tick();
    }
    assert_eq!(s.calls[0].status, CallStatus::Resolved);
}

// ------------------------------------------------------------- escalation

#[test]
fn unattended_call_escalates_on_schedule_then_fails() {
    let p = Difficulty::Dispatcher.params();
    let mut s = shift_with(vec![sched("elderly_fall", 0, BAZAAR, 1)]);
    run_through(&mut s, p.esc_interval - 1);
    assert_eq!(s.calls[0].severity, 1);
    run_through(&mut s, p.esc_interval);
    assert_eq!(s.calls[0].severity, 2);
    run_through(&mut s, 2 * p.esc_interval);
    assert_eq!(s.calls[0].severity, 3);
    let deadline = 2 * p.esc_interval + p.crit_window;
    assert_eq!(s.calls[0].deadline, Some(deadline));
    run_through(&mut s, deadline - 1);
    assert_eq!(s.calls[0].status, CallStatus::Waiting);
    let before = s.trust;
    run_through(&mut s, deadline);
    assert_eq!(s.calls[0].status, CallStatus::Failed);
    assert_eq!(before - s.trust, p.fail_trust, "a lost call costs exactly the failure penalty");
    assert_eq!(s.stats.lives_lost, 1);
}

#[test]
fn covered_call_does_not_escalate() {
    let p = Difficulty::Dispatcher.params();
    let mut s = shift_with(vec![sched("apartment_fire", 0, Pos::new(35, 7), 3)]);
    s.tick();
    let fire = first_unit(&s, UnitKind::Fire);
    s.apply(&Command::Dispatch { call: 1, units: vec![fire] }).unwrap();
    // Fire station is two cells away: the crew is working long before the deadline.
    run_through(&mut s, p.crit_window + 5);
    assert_ne!(s.calls[0].status, CallStatus::Failed);
}

// ------------------------------------------------------------- pranks

#[test]
fn dismissing_a_prank_is_rewarded() {
    let mut s = shift_with(vec![sched("prank_ufo", 0, BAZAAR, 2)]);
    s.tick();
    let before = s.trust;
    s.apply(&Command::Dismiss { call: 1 }).unwrap();
    assert_eq!(s.calls[0].status, CallStatus::Dismissed);
    assert_eq!(s.stats.prank_dismissed, 1);
    assert_eq!(s.trust - before, 50);
}

#[test]
fn dispatching_to_a_prank_wastes_the_unit_and_costs_trust() {
    let mut s = shift_with(vec![sched("prank_lion", 0, BAZAAR, 3)]);
    s.tick();
    let police = first_unit(&s, UnitKind::Police);
    s.apply(&Command::Dispatch { call: 1, units: vec![police] }).unwrap();
    let before = s.trust;
    let mut guard = 0;
    while s.calls[0].status == CallStatus::Waiting {
        s.tick();
        guard += 1;
        assert!(guard < 100);
    }
    assert_eq!(s.calls[0].status, CallStatus::PrankExposed);
    assert_eq!(s.stats.prank_dispatched, 1);
    assert_eq!(before - s.trust, 150);
}

#[test]
fn ignored_prank_expires_without_trust_loss() {
    let p = Difficulty::Dispatcher.params();
    let mut s = shift_with(vec![sched("prank_pizza", 0, BAZAAR, 1)]);
    s.tick();
    let before = s.trust;
    run_through(&mut s, p.prank_patience);
    assert_eq!(s.calls[0].status, CallStatus::Expired);
    assert_eq!(s.stats.prank_expired, 1);
    assert_eq!(s.trust, before);
}

#[test]
fn dismissing_a_real_call_is_punished_by_a_worse_callback() {
    let mut s = shift_with(vec![sched("burglary", 0, BAZAAR, 2)]);
    s.tick();
    s.apply(&Command::Dismiss { call: 1 }).unwrap();
    assert_eq!(s.stats.real_dismissed, 1);
    let before = s.trust;
    run_through(&mut s, 11);
    let c = &s.calls[0];
    assert_eq!(c.status, CallStatus::Waiting, "the caller calls back");
    assert!(c.callback);
    assert_eq!(c.severity, 3);
    assert!(c.deadline.is_some());
    assert!(before - s.trust >= 1200, "big trust penalty");
}

// ------------------------------------------------------------- coffee, flags

#[test]
fn coffee_break_pauses_ordinary_calls_but_defers_story_calls() {
    let mut story = sched("karim_lonely", 5, Pos::new(3, 2), 1);
    story.story = true;
    let mut s = shift_with(vec![sched("trash_fire", 4, BAZAAR, 1), story]);
    s.tick();
    let stress_before = s.stress;
    s.apply(&Command::Coffee).unwrap();
    assert_eq!(s.apply(&Command::Coffee), Err(CmdError::CoffeeUsed));
    assert!(s.stress <= stress_before);
    run_through(&mut s, 10);
    assert!(s.calls.is_empty(), "no calls during the break");
    assert!(s.log.iter().any(|e| e.event == Event::ZuhraTookCall));
    run_through(&mut s, 17);
    assert_eq!(s.calls.len(), 1, "the story call arrives after the break");
    assert_eq!(s.calls[0].template().id, "karim_lonely");
}

#[test]
fn flags_are_tallied_at_the_end_of_the_shift() {
    let mut s = shift_with(vec![sched("arson_kiosk", 0, INDUSTRIAL, 2), sched("decoy_transformer", 0, INDUSTRIAL, 2)]);
    s.tick();
    s.apply(&Command::Flag { call: 1 }).unwrap();
    s.apply(&Command::Flag { call: 2 }).unwrap();
    while !s.is_over() {
        s.tick();
    }
    assert_eq!(s.stats.arson_calls, 1);
    assert_eq!(s.stats.flags_correct, 1);
    assert_eq!(s.stats.flags_false, 1);
}

#[test]
fn finale_needs_police_and_fire_to_catch_the_arsonist() {
    let mut p = params(Difficulty::Dispatcher);
    p.night = 4;
    p.trap_set = true;
    let mut s = Shift::with_schedule(p.clone(), vec![sched("arson_finale", 0, INDUSTRIAL, 2)]);
    s.tick();
    let fire = first_unit(&s, UnitKind::Fire);
    let police = first_unit(&s, UnitKind::Police);
    s.apply(&Command::Dispatch { call: 1, units: vec![fire, police] }).unwrap();
    while !s.is_over() {
        s.tick();
    }
    assert!(s.arsonist_caught);

    let mut s = Shift::with_schedule(p, vec![sched("arson_finale", 0, INDUSTRIAL, 2)]);
    s.tick();
    s.apply(&Command::Dispatch { call: 1, units: vec![fire] }).unwrap();
    while !s.is_over() {
        s.tick();
    }
    assert!(!s.arsonist_caught, "fire alone puts out the fire but he walks away");
    assert_eq!(s.calls[0].status, CallStatus::Failed);
}

// ------------------------------------------------------------- scoring

#[test]
fn grade_computation() {
    let perfect = ShiftStats {
        calls: 30,
        real_calls: 26,
        pranks: 4,
        resolved: 26,
        prank_dismissed: 4,
        response_quality: 26_000,
        response_count: 26,
        ..Default::default()
    };
    let b = score(&perfect);
    assert_eq!((b.outcome, b.response, b.pranks, b.total), (50, 30, 20, 100));
    assert_eq!(grade(b.total), Grade::S);

    let mut rough = perfect.clone();
    rough.resolved = 20;
    rough.failed = 6;
    rough.prank_dismissed = 1;
    rough.prank_dispatched = 3;
    rough.response_quality = 20 * 700;
    rough.response_count = 20;
    rough.wrong_units = 3;
    let r = score(&rough);
    assert!(r.total < b.total);
    assert!(grade(r.total) <= Grade::C, "six losses and bad prank handling: {}", r.total);

    assert_eq!(grade(100), Grade::S);
    assert_eq!(grade(score::GRADE_A), Grade::A);
    assert_eq!(grade(score::GRADE_B), Grade::B);
    assert_eq!(grade(score::GRADE_B - 1), Grade::C);
    assert_eq!(grade(score::GRADE_C - 1), Grade::D);
    assert!(Grade::B.is_good() && !Grade::C.is_good());
}

use dispatch112::sim::score;

#[test]
fn ending_selection() {
    use Grade::*;
    let e = |g: &[Grade], end, n, caught| select_ending(g, end, n, caught);
    assert_eq!(e(&[S, A, B, A, B], ShiftEnd::Completed, 5, true), Some(Ending::GoldenHeadset));
    assert_eq!(e(&[S, A, C, A, B], ShiftEnd::Completed, 5, true), Some(Ending::NightDetective));
    assert_eq!(e(&[S, A, B, A, B], ShiftEnd::Completed, 5, false), Some(Ending::SteadyHand));
    assert_eq!(e(&[D], ShiftEnd::Fired, 1, false), Some(Ending::BadgeReturned));
    assert_eq!(e(&[A, A], ShiftEnd::Burnout, 2, false), Some(Ending::Burnout));
    assert_eq!(e(&[A, A], ShiftEnd::Completed, 2, false), None);
}

#[test]
fn trap_is_set_only_with_enough_correct_flags() {
    let mut w = Week::new(Difficulty::Rookie, 3);
    // Play Monday..Thursday with a bot that never flags: no trap.
    for _ in 0..4 {
        let mut s = w.start_shift();
        let mut bot = dispatch112::bot::Bot::new(BotProfile::no_flags(), 1);
        dispatch112::bot::play_shift(&mut s, &mut bot);
        w.finish_shift(&s);
    }
    assert!(!w.trap_set);
    assert!(!schedule::build(4, Difficulty::Rookie, 3, false)
        .iter()
        .any(|c| c.template == catalog::find("arson_finale")));
}

// ------------------------------------------------------------- determinism

#[test]
fn same_seed_and_inputs_give_the_same_week() {
    let a = play_week(Difficulty::Dispatcher, 1234, BotProfile::good());
    let b = play_week(Difficulty::Dispatcher, 1234, BotProfile::good());
    assert_eq!(a, b);
    let c = play_week(Difficulty::Dispatcher, 1235, BotProfile::good());
    assert_ne!(a.records, c.records, "different seeds should differ");
}

#[test]
fn same_seed_and_manual_inputs_give_the_same_shift() {
    let run = || {
        let mut s = Shift::new(ShiftParams::new(2, Difficulty::Veteran, 77));
        for t in 0..480u32 {
            if t % 7 == 0 {
                let first = s.open_calls().next().map(|c| c.id);
                if let Some(c) = first {
                    let free: Vec<usize> = s.units.iter().filter(|u| u.is_available()).map(|u| u.id).take(1).collect();
                    let _ = s.apply(&Command::Dispatch { call: c, units: free });
                }
            }
            if t == 200 {
                let _ = s.apply(&Command::Coffee);
            }
            s.tick();
        }
        (s.stats.clone(), s.trust, s.stress, s.log.clone(), s.end)
    };
    assert_eq!(run(), run());
}
