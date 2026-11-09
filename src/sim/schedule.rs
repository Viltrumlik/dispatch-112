//! Seeded per-shift call scheduler.

use super::catalog::{self, FixedPlace, Place, TKind, TEMPLATES};
use super::config::{Difficulty, SHIFT_TICKS};
use super::map::{CityMap, District, Pos};
use super::rng::{mix, Rng};

pub const STORM_NIGHT: u8 = 2;
pub const STADIUM_NIGHT: u8 = 3;
pub const FINALE_NIGHT: u8 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scheduled {
    pub tick: u32,
    pub template: usize,
    pub variant: u8,
    pub pos: Pos,
    pub severity: u8,
    /// Scripted story calls are deferred (not skipped) during the coffee break.
    pub story: bool,
}

/// Number of arsonist calls planned for a night (Mon..Thu); Friday is the finale.
pub fn arson_count(night: u8, seed: u64) -> u32 {
    let mut rng = Rng::new(mix(seed, 7_000 + night as u64));
    match night {
        0 => 1,
        1 => 2,
        2 | 3 => {
            if rng.chance(400) {
                3
            } else {
                2
            }
        }
        _ => 0,
    }
}

fn district_weights(night: u8, include_industrial: bool, exclude: Option<District>) -> [u32; 6] {
    // Order matches District::ALL: OldTown, Bazaar, NewBlocks, Railway, Industrial, Stadium.
    let mut w = [3, 3, 3, 2, 0, 1];
    if include_industrial {
        w[4] = 1;
    }
    if night == STADIUM_NIGHT {
        w[5] = 3;
    }
    if let Some(d) = exclude {
        w[d.index()] = 0;
    }
    w
}

fn pick_pos(rng: &mut Rng, map: &CityMap, place: Place, night: u8) -> Pos {
    let district = match place {
        Place::Fixed(FixedPlace::KarimHome) => return map.karim_home,
        Place::Fixed(FixedPlace::BotirBakery) => return map.botir_bakery,
        Place::In(d) => d,
        Place::Residential => {
            let w = district_weights(night, false, None);
            District::ALL[rng.weighted(&w).unwrap_or(0)]
        }
        Place::Any => {
            let w = district_weights(night, true, None);
            District::ALL[rng.weighted(&w).unwrap_or(0)]
        }
        Place::NotIn(d) => {
            let w = district_weights(night, d != District::Industrial, Some(d));
            District::ALL[rng.weighted(&w).unwrap_or(0)]
        }
    };
    let cells = map.street_cells(district);
    *rng.pick(&cells).unwrap_or(&map.hospital)
}

/// Time-of-night load factor in percent (higher = calls further apart).
fn spacing_factor(night: u8, tick: u32) -> u32 {
    if night == 0 && tick < 45 {
        return 170; // Monday: a gentle start while Zuhra-opa explains things
    }
    if night == 0 && tick < 120 {
        return 125;
    }
    if night == STORM_NIGHT {
        // Fewer people out in the storm, but what comes in is nasty.
        return 115;
    }
    if night == STADIUM_NIGHT && tick < 100 {
        return 80; // the match has just ended
    }
    match tick {
        0..=59 => 100,
        60..=179 => 85,
        180..=299 => 100,
        300..=419 => 125,
        _ => 110,
    }
}

fn template_weight(i: usize, night: u8) -> u32 {
    let t = &TEMPLATES[i];
    if t.weight == 0 || t.prank {
        return 0;
    }
    let mut w = t.weight;
    match t.kind {
        TKind::Storm if night != STORM_NIGHT => return 0,
        TKind::Storm => w = w * 3 / 2,
        TKind::Stadium if night != STADIUM_NIGHT => return 0,
        TKind::Stadium => w *= 2,
        TKind::Normal | TKind::Decoy => {}
        _ => return 0,
    }
    if t.is_multi() || t.is_misleading() {
        w = w * (100 + 20 * night as u32) / 100;
    }
    w
}

fn make(rng: &mut Rng, map: &CityMap, night: u8, tick: u32, ti: usize, story: bool) -> Scheduled {
    let t = &TEMPLATES[ti];
    let variant = rng.below(t.variants as u32) as u8;
    let severity = rng.range(t.sev_min as i32, t.sev_max as i32) as u8;
    let pos = pick_pos(rng, map, t.place, night);
    Scheduled { tick, template: ti, variant, pos, severity, story }
}

/// Build the full call list for one night, sorted by arrival tick.
pub fn build(night: u8, difficulty: Difficulty, seed: u64, trap_set: bool) -> Vec<Scheduled> {
    let map = CityMap::get();
    let p = difficulty.params();
    let mut rng = Rng::new(mix(seed, 1_000 + night as u64));
    let mut out: Vec<Scheduled> = Vec::new();

    // Ordinary calls.
    let normal_weights: Vec<u32> = (0..TEMPLATES.len()).map(|i| template_weight(i, night)).collect();
    let prank_weights: Vec<u32> =
        TEMPLATES.iter().map(|t| if t.prank && t.kind == TKind::Normal { t.weight } else { 0 }).collect();
    let simple_weights: Vec<u32> = TEMPLATES
        .iter()
        .enumerate()
        .map(|(i, t)| {
            if t.kind == TKind::Normal && !t.prank && !t.is_multi() && !t.is_misleading() {
                normal_weights[i]
            } else {
                0
            }
        })
        .collect();
    // Mean spacing in hundredths of a tick.
    let mean = p.interval * 100 * 100 / (100 + p.night_growth * night as u32);
    let mut t: u32 = 3;
    let mut n = 0u32;
    while t < SHIFT_TICKS - 8 {
        let tutorial = night == 0 && n < 4;
        let ti = if tutorial {
            rng.weighted(&simple_weights)
        } else if rng.chance(p.prank_share) {
            rng.weighted(&prank_weights)
        } else {
            rng.weighted(&normal_weights)
        };
        if let Some(ti) = ti {
            let mut s = make(&mut rng, map, night, t, ti, false);
            if tutorial {
                s.severity = s.severity.min(2);
            }
            out.push(s);
            n += 1;
        }
        let m = mean * spacing_factor(night, t) / 100;
        let step = rng.range((m / 2) as i32, (m * 3 / 2) as i32) as u32;
        t += (step / 100).max(2);
    }

    // Monday tutorial beats: an obvious prank and a two-service crash.
    if night == 0 {
        let prank = catalog::find("prank_pizza");
        out.push(make(&mut rng, map, night, 52, prank, false));
        let crash = catalog::find("car_crash");
        let mut c = make(&mut rng, map, night, 95, crash, false);
        c.severity = 2;
        out.push(c);
    }

    // Recurring callers.
    let story = |rng: &mut Rng, out: &mut Vec<Scheduled>, id: &str, lo: i32, hi: i32, variant: Option<u8>| {
        let ti = catalog::find(id);
        let tick = rng.range(lo, hi) as u32;
        let mut s = make(rng, map, night, tick, ti, true);
        if let Some(v) = variant {
            s.variant = v % TEMPLATES[ti].variants;
        }
        out.push(s);
    };
    if night == STADIUM_NIGHT {
        story(&mut rng, &mut out, "karim_heart", 190, 230, None);
    } else {
        story(&mut rng, &mut out, "karim_lonely", 50, 110, Some(night % 3));
    }
    if night == FINALE_NIGHT {
        story(&mut rng, &mut out, "jasur_real", 200, 260, None);
    } else {
        story(&mut rng, &mut out, "jasur_prank", 120, 380, Some(night % 3));
    }
    match night {
        0 => story(&mut rng, &mut out, "botir_gas", 280, 330, None),
        1 => story(&mut rng, &mut out, "botir_breakin", 130, 170, None),
        2 => story(&mut rng, &mut out, "botir_van", 300, 350, None),
        3 => story(&mut rng, &mut out, "botir_petrol", 350, 400, None),
        _ => {}
    }

    // The arsonist.
    let arson_ids = [catalog::find("arson_warehouse"), catalog::find("arson_kiosk"), catalog::find("arson_garage")];
    if night < FINALE_NIGHT {
        let count = arson_count(night, seed);
        let span = (440 - 40) / count;
        let mut last = usize::MAX;
        for k in 0..count {
            let lo = 40 + k * span;
            let tick = lo + rng.below(span.saturating_sub(30).max(1));
            let mut choice = rng.below(3) as usize;
            if choice == last {
                choice = (choice + 1) % 3;
            }
            last = choice;
            out.push(make(&mut rng, map, night, tick, arson_ids[choice], true));
        }
    } else {
        let tick = rng.range(280, 320) as u32;
        if trap_set {
            out.push(make(&mut rng, map, night, tick, catalog::find("finale_decoy"), true));
            out.push(make(&mut rng, map, night, tick + 4, catalog::find("arson_finale"), true));
        } else {
            out.push(make(&mut rng, map, night, tick, arson_ids[0], true));
        }
    }

    out.sort_by_key(|s| (s.tick, s.story));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_is_deterministic_and_sorted() {
        let a = build(1, Difficulty::Dispatcher, 99, false);
        let b = build(1, Difficulty::Dispatcher, 99, false);
        assert_eq!(a, b);
        assert!(a.windows(2).all(|w| w[0].tick <= w[1].tick));
        assert!(a.iter().all(|s| s.tick < SHIFT_TICKS));
    }

    #[test]
    fn harder_difficulty_has_more_calls() {
        for night in 0..5 {
            let r = build(night, Difficulty::Rookie, 5, false).len();
            let v = build(night, Difficulty::Veteran, 5, false).len();
            assert!(v > r, "night {night}: rookie {r} veteran {v}");
        }
    }

    #[test]
    fn finale_only_with_trap() {
        let fin = catalog::find("arson_finale");
        assert!(build(4, Difficulty::Rookie, 3, true).iter().any(|s| s.template == fin));
        assert!(!build(4, Difficulty::Rookie, 3, false).iter().any(|s| s.template == fin));
    }

    #[test]
    fn storm_and_stadium_calls_on_their_nights_only() {
        for night in 0..5 {
            let s = build(night, Difficulty::Dispatcher, 11, false);
            let storm = s.iter().filter(|c| TEMPLATES[c.template].kind == TKind::Storm).count();
            let stadium = s.iter().filter(|c| TEMPLATES[c.template].kind == TKind::Stadium).count();
            assert_eq!(storm > 0, night == STORM_NIGHT, "night {night}");
            assert_eq!(stadium > 0, night == STADIUM_NIGHT, "night {night}");
        }
    }
}
