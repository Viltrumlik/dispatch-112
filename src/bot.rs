//! A scripted dispatcher used for `--autoplay`, `--headless` and balance tests.
//!
//! The bot plays "honestly": it reads what a careful human could infer. Each
//! call has a deterministic per-call chance that the bot takes a misleading
//! text at face value or fails to recognise a prank. It only learns the truth
//! the way a player does — from crew reports once a unit is on scene. It also
//! has a human-like reaction delay and handles one decision every few minutes.

use crate::sim::path::{cell_index, distances_from};
use crate::sim::rng::permille;
use crate::sim::*;
use std::cmp::Reverse;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BotProfile {
    pub active: bool,
    /// Ticks after a call arrives before the bot has read it.
    pub reaction: u32,
    /// Ticks spent per decision.
    pub think: u32,
    /// Chance (permille) to take a misleading call at face value.
    pub misread: u32,
    /// Chance (permille) to recognise a prank for what it is.
    pub prank_spot: u32,
    pub flag_arson: bool,
    pub use_coffee: bool,
}

impl BotProfile {
    /// A skilled, attentive dispatcher.
    pub fn good() -> BotProfile {
        BotProfile {
            active: true,
            reaction: 3,
            think: 4,
            misread: 250,
            prank_spot: 800,
            flag_arson: true,
            use_coffee: true,
        }
    }

    /// Same skill, but never notices the arsonist pattern.
    pub fn no_flags() -> BotProfile {
        BotProfile { flag_arson: false, ..BotProfile::good() }
    }

    /// Slow and careless.
    pub fn sloppy() -> BotProfile {
        BotProfile {
            active: true,
            reaction: 4,
            think: 4,
            misread: 600,
            prank_spot: 450,
            flag_arson: false,
            use_coffee: false,
        }
    }

    /// Never touches the console.
    pub fn idle() -> BotProfile {
        BotProfile { active: false, ..BotProfile::good() }
    }

    pub fn from_name(name: &str) -> Option<BotProfile> {
        match name {
            "good" => Some(BotProfile::good()),
            "noflags" | "no-flags" => Some(BotProfile::no_flags()),
            "sloppy" => Some(BotProfile::sloppy()),
            "idle" | "nothing" | "none" => Some(BotProfile::idle()),
            _ => None,
        }
    }
}

pub struct Bot {
    pub profile: BotProfile,
    seed: u64,
    busy_until: u32,
    /// Call the bot acted on last (the UI highlights it in autoplay).
    pub focus: Option<u32>,
}

impl Bot {
    pub fn new(profile: BotProfile, seed: u64) -> Bot {
        Bot { profile, seed, busy_until: 0, focus: None }
    }

    fn screening(shift: &Shift) -> bool {
        shift.params.upgrades.has(UpgradeKind::Screening)
    }

    fn spots_prank(&self, shift: &Shift, c: &Call) -> bool {
        let mut p = self.profile.prank_spot;
        if Bot::screening(shift) {
            p += (1000 - p.min(1000)) * 2 / 3;
        }
        permille(self.seed, c.id as u64, 1) < p
    }

    /// What the bot believes the call needs.
    pub fn believed_needs(&self, shift: &Shift, c: &Call) -> Needs {
        let t = c.template();
        if c.revealed {
            return c.needs;
        }
        if c.finale && self.profile.flag_arson {
            // Inspector Dilshod's radio message: police and fire together.
            return c.needs;
        }
        if c.prank {
            return t.claim;
        }
        if t.is_misleading() {
            let mut p = self.profile.misread;
            if Bot::screening(shift) {
                p /= 3;
            }
            if permille(self.seed, c.id as u64, 2) < p {
                return t.claim;
            }
        }
        c.needs
    }

    pub fn decide(&mut self, shift: &Shift) -> Vec<Command> {
        let mut cmds = Vec::new();
        if !self.profile.active || shift.is_over() {
            return cmds;
        }
        let tick = shift.tick;
        let read = |c: &Call| tick >= c.arrived + self.profile.reaction;

        if self.profile.flag_arson {
            for c in shift.queue() {
                if c.arson && !c.flagged && read(c) {
                    cmds.push(Command::Flag { call: c.id });
                }
            }
        }
        if tick < self.busy_until {
            return cmds;
        }

        if self.profile.use_coffee && !shift.coffee_used && shift.stress >= 5500 && shift.unanswered() <= 1 {
            cmds.push(Command::Coffee);
            self.busy_until = tick + self.profile.think;
            return cmds;
        }

        for c in shift.open_calls() {
            if c.prank && read(c) && self.spots_prank(shift, c) {
                cmds.push(Command::Dismiss { call: c.id });
                self.focus = Some(c.id);
                self.busy_until = tick + self.profile.think;
                return cmds;
            }
        }

        let mut cands: Vec<&Call> = shift.open_calls().filter(|c| read(c)).collect();
        cands.sort_by_key(|c| (Reverse(c.severity), c.deadline.unwrap_or(u32::MAX), c.arrived));
        let map = CityMap::get();
        for c in cands {
            let believed = self.believed_needs(shift, c);
            let mut wanted: Vec<(UnitKind, u8)> = Vec::new();
            for k in UnitKind::ALL {
                let have = c.served.get(k) + shift.assigned(c.id, k);
                let missing = believed.get(k).saturating_sub(have);
                if missing > 0 {
                    wanted.push((k, missing));
                }
            }
            if wanted.is_empty() {
                continue;
            }
            let dist = distances_from(map, c.pos, &shift.blocked);
            let mut chosen: Vec<usize> = Vec::new();
            for (k, n) in wanted {
                for _ in 0..n {
                    let best = shift
                        .units
                        .iter()
                        .filter(|u| u.kind == k && u.is_available() && !chosen.contains(&u.id))
                        .filter_map(|u| {
                            let d = dist[cell_index(u.pos)];
                            (d != u32::MAX).then(|| (d * 100 / shift.speed(u, true), u.id))
                        })
                        .min();
                    if let Some((_, id)) = best {
                        chosen.push(id);
                    }
                }
            }
            if !chosen.is_empty() {
                cmds.push(Command::Dispatch { call: c.id, units: chosen });
                self.focus = Some(c.id);
                self.busy_until = tick + self.profile.think;
                return cmds;
            }
        }
        cmds
    }

    /// The bot's upgrade preference.
    pub fn pick_upgrade(options: &[UpgradeKind]) -> Option<UpgradeKind> {
        const ORDER: [UpgradeKind; 9] = [
            UpgradeKind::ExtraAmbulance,
            UpgradeKind::FieldPost,
            UpgradeKind::ExtraFire,
            UpgradeKind::GreenWave,
            UpgradeKind::Screening,
            UpgradeKind::FastFire,
            UpgradeKind::ExtraPolice,
            UpgradeKind::RapidTriage,
            UpgradeKind::CalmTraining,
        ];
        ORDER.iter().copied().find(|u| options.contains(u)).or_else(|| options.first().copied())
    }
}

/// Play a whole week with a bot profile; returns the final week state.
pub fn play_week(difficulty: Difficulty, seed: u64, profile: BotProfile) -> Week {
    let mut week = Week::new(difficulty, seed);
    while !week.is_over() {
        let mut shift = week.start_shift();
        let mut bot = Bot::new(profile, crate::sim::rng::mix(seed, 77 + week.night as u64));
        play_shift(&mut shift, &mut bot);
        week.finish_shift(&shift);
        if week.is_over() {
            break;
        }
        if let Some(u) = Bot::pick_upgrade(&week.upgrade_options()) {
            week.apply_upgrade(u);
        }
    }
    week
}

/// Run a shift to the end with the bot at the console.
pub fn play_shift(shift: &mut Shift, bot: &mut Bot) {
    while !shift.is_over() {
        for cmd in bot.decide(shift) {
            let _ = shift.apply(&cmd);
        }
        shift.tick();
        shift.pending_tips.clear();
    }
}
