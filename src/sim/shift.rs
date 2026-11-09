//! One night shift: the tick-based simulation of calls, units and meters.
//! Pure and deterministic: the same parameters and the same commands at the
//! same ticks always produce the same result.

use super::call::{Call, CallStatus, Report};
use super::catalog::{self, Needs, TKind};
use super::config::*;
use super::map::{CityMap, Pos};
use super::path::find_path;
use super::schedule::{self, Scheduled, FINALE_NIGHT, STORM_NIGHT};
use super::score::{response_quality, ShiftStats};
use super::units::{Unit, UnitKind, UnitState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShiftParams {
    pub night: u8,
    pub difficulty: Difficulty,
    pub seed: u64,
    pub upgrades: Upgrades,
    /// Starting trust, hundredths.
    pub trust: i32,
    /// Starting stress, hundredths.
    pub stress: i32,
    pub trap_set: bool,
    /// Show Zuhra-opa's tutorial tips.
    pub tutorial: bool,
}

impl ShiftParams {
    pub fn new(night: u8, difficulty: Difficulty, seed: u64) -> ShiftParams {
        ShiftParams {
            night,
            difficulty,
            seed,
            upgrades: Upgrades::default(),
            trust: TRUST_START,
            stress: 0,
            trap_set: false,
            tutorial: night == 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShiftEnd {
    Completed,
    /// City trust fell to zero.
    Fired,
    /// Stress reached 100.
    Burnout,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    Dispatch { call: u32, units: Vec<usize> },
    Dismiss { call: u32 },
    Flag { call: u32 },
    Coffee,
    Recall { unit: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmdError {
    ShiftOver,
    NoSuchCall,
    CallClosed,
    NoUnits,
    NoSuchUnit(usize),
    UnitBusy(usize),
    Unreachable,
    CoffeeUsed,
    NotRecallable,
}

impl CmdError {
    pub fn key(self) -> &'static str {
        match self {
            CmdError::ShiftOver => "err.shift_over",
            CmdError::NoSuchCall => "err.no_call",
            CmdError::CallClosed => "err.call_closed",
            CmdError::NoUnits => "err.no_units",
            CmdError::NoSuchUnit(_) => "err.no_such_unit",
            CmdError::UnitBusy(_) => "err.unit_busy",
            CmdError::Unreachable => "err.unreachable",
            CmdError::CoffeeUsed => "err.coffee_used",
            CmdError::NotRecallable => "err.not_recallable",
        }
    }
}

/// Structured log events; the UI renders them in the current language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    ShiftStart,
    NewCall { call: u32 },
    Dispatched { call: u32, unit: usize },
    Arrived { call: u32, unit: usize },
    WrongUnit { call: u32, unit: usize },
    NeedMore { call: u32, unit: usize },
    PrankFound { call: u32, unit: usize },
    NotNeeded { call: u32, unit: usize },
    Resolved { call: u32 },
    Escalated { call: u32, severity: u8 },
    Failed { call: u32 },
    PrankDismissed { call: u32 },
    Dismissed { call: u32 },
    Callback { call: u32 },
    Expired { call: u32 },
    Flagged { call: u32, on: bool },
    CoffeeStart,
    CoffeeEnd,
    ZuhraTookCall,
    Recalled { unit: usize },
    AkmalGrumble,
    DilshodRadio,
    ArsonistCaught { call: u32 },
    ArsonistEscaped { call: u32 },
    HandedOver { count: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub tick: u32,
    pub event: Event,
}

/// Zuhra-opa's tutorial tips (shown as overlays by the UI).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tip {
    Welcome,
    FirstCall,
    Prank,
    Multi,
    Escalation,
    Report,
    Arson,
    Stress,
}

impl Tip {
    pub fn key(self) -> &'static str {
        match self {
            Tip::Welcome => "tip.welcome",
            Tip::FirstCall => "tip.first_call",
            Tip::Prank => "tip.prank",
            Tip::Multi => "tip.multi",
            Tip::Escalation => "tip.escalation",
            Tip::Report => "tip.report",
            Tip::Arson => "tip.arson",
            Tip::Stress => "tip.stress",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Shift {
    pub params: ShiftParams,
    pub tick: u32,
    pub calls: Vec<Call>,
    pub units: Vec<Unit>,
    /// Hundredths, 0..=10_000.
    pub trust: i32,
    /// Hundredths, 0..=10_000.
    pub stress: i32,
    pub stats: ShiftStats,
    pub log: Vec<LogEntry>,
    pub end: Option<ShiftEnd>,
    pub coffee_used: bool,
    pub coffee_until: Option<u32>,
    pub blocked: Vec<Pos>,
    pub arsonist_caught: bool,
    pub pending_tips: Vec<Tip>,
    tips_seen: Vec<Tip>,
    schedule: Vec<Scheduled>,
    next_sched: usize,
    deferred: Vec<Scheduled>,
    next_id: u32,
    wrong_fire_count: u32,
}

/// Build the fleet for a difficulty plus upgrades (at most nine vehicles).
pub fn build_fleet(difficulty: Difficulty, upgrades: &Upgrades) -> Vec<Unit> {
    let map = CityMap::get();
    let p = difficulty.params();
    let amb = p.units[0] as usize + upgrades.count(UpgradeKind::ExtraAmbulance);
    let fire = p.units[1] as usize + upgrades.count(UpgradeKind::ExtraFire);
    let police = p.units[2] as usize + upgrades.count(UpgradeKind::ExtraPolice);
    let mut spec: Vec<(UnitKind, Pos)> = Vec::new();
    for _ in 0..amb {
        spec.push((UnitKind::Ambulance, map.hospital));
    }
    if upgrades.has(UpgradeKind::FieldPost) {
        spec.push((UnitKind::Ambulance, map.field_post));
    }
    for _ in 0..fire {
        spec.push((UnitKind::Fire, map.fire_station));
    }
    for _ in 0..police {
        spec.push((UnitKind::Police, map.police_station));
    }
    spec.truncate(MAX_UNITS);
    let mut ordinals = [0u8; 3];
    spec.into_iter()
        .enumerate()
        .map(|(i, (k, base))| {
            ordinals[k.idx()] += 1;
            Unit::new(i, k, ordinals[k.idx()], base)
        })
        .collect()
}

impl Shift {
    pub fn new(params: ShiftParams) -> Shift {
        let schedule = schedule::build(params.night, params.difficulty, params.seed, params.trap_set);
        Shift::with_schedule(params, schedule)
    }

    /// A shift with a hand-written call list (used by tests).
    pub fn with_schedule(params: ShiftParams, mut schedule: Vec<Scheduled>) -> Shift {
        schedule.sort_by_key(|s| (s.tick, s.story));
        let units = build_fleet(params.difficulty, &params.upgrades);
        let blocked = if params.night == STORM_NIGHT { vec![CityMap::get().flood_bridge] } else { Vec::new() };
        Shift {
            trust: params.trust.clamp(1, METER_MAX),
            stress: params.stress.clamp(0, METER_MAX - 1),
            params,
            tick: 0,
            calls: Vec::new(),
            units,
            stats: ShiftStats::default(),
            log: Vec::new(),
            end: None,
            coffee_used: false,
            coffee_until: None,
            blocked,
            arsonist_caught: false,
            pending_tips: Vec::new(),
            tips_seen: Vec::new(),
            schedule,
            next_sched: 0,
            deferred: Vec::new(),
            next_id: 1,
            wrong_fire_count: 0,
        }
    }

    // ------------------------------------------------------------------ queries

    pub fn night(&self) -> u8 {
        self.params.night
    }

    pub fn is_over(&self) -> bool {
        self.end.is_some()
    }

    pub fn is_storm(&self) -> bool {
        self.params.night == STORM_NIGHT
    }

    pub fn coffee_active(&self) -> bool {
        self.coffee_until.is_some_and(|t| t > self.tick)
    }

    pub fn trust_pct(&self) -> i32 {
        (self.trust + 50) / 100
    }

    pub fn stress_pct(&self) -> i32 {
        self.stress / 100
    }

    /// Clock time (hours, minutes) for the current tick; 22:00 at tick 0.
    pub fn clock(&self) -> (u32, u32) {
        let m = 22 * 60 + self.tick.min(SHIFT_TICKS);
        ((m / 60) % 24, m % 60)
    }

    pub fn call(&self, id: u32) -> Option<&Call> {
        self.calls.iter().find(|c| c.id == id)
    }

    fn call_idx(&self, id: u32) -> Option<usize> {
        self.calls.iter().position(|c| c.id == id)
    }

    pub fn open_calls(&self) -> impl Iterator<Item = &Call> {
        self.calls.iter().filter(|c| c.is_open())
    }

    /// Calls shown in the queue: open ones plus recently closed ones (still flaggable).
    pub fn queue(&self) -> Vec<&Call> {
        self.calls
            .iter()
            .filter(|c| c.is_open() || c.closed_at.is_some_and(|t| self.tick < t + CLOSED_LINGER))
            .collect()
    }

    /// Units committed to a call (on the way or on scene).
    pub fn units_on(&self, call: u32) -> Vec<usize> {
        self.units.iter().filter(|u| u.call() == Some(call)).map(|u| u.id).collect()
    }

    /// Units of `kind` on the way or working at `call`.
    pub fn assigned(&self, call: u32, kind: UnitKind) -> u8 {
        self.units
            .iter()
            .filter(|u| u.kind == kind)
            .filter(|u| match u.state {
                UnitState::EnRoute { call: c } => c == call,
                UnitState::OnScene { call: c, working, .. } => c == call && working,
                _ => false,
            })
            .count() as u8
    }

    pub fn working(&self, call: u32, kind: UnitKind) -> u8 {
        self.units.iter().filter(|u| u.kind == kind && u.is_working_on(call)).count() as u8
    }

    /// Every service the call needs is on scene or done: escalation is frozen.
    pub fn is_covered(&self, c: &Call) -> bool {
        UnitKind::ALL.iter().all(|&k| {
            let need = c.needs.get(k);
            need == 0 || c.served.get(k) + self.working(c.id, k) >= need
        })
    }

    /// Open calls with nobody assigned.
    pub fn unanswered(&self) -> usize {
        self.open_calls().filter(|c| self.units_on(c.id).is_empty()).count()
    }

    pub fn speed(&self, unit: &Unit, siren: bool) -> u32 {
        let up = &self.params.upgrades;
        let mut s = if siren {
            let mut s = siren_speed(unit.kind);
            if unit.kind == UnitKind::Fire && up.has(UpgradeKind::FastFire) {
                s += 50;
            }
            s
        } else {
            RETURN_SPEED
        };
        if up.has(UpgradeKind::GreenWave) {
            s = s * 115 / 100;
        }
        if self.is_storm() {
            s = s * 85 / 100;
        }
        s.max(20)
    }

    /// Estimated minutes for `unit` to reach `pos` with sirens on.
    pub fn eta(&self, unit: usize, pos: Pos) -> Option<u32> {
        let u = self.units.get(unit)?;
        let len = find_path(CityMap::get(), u.pos, pos, &self.blocked)?.len() as u32;
        let sp = self.speed(u, true);
        Some((len * 100).div_ceil(sp))
    }

    pub fn take_tips(&mut self) -> Vec<Tip> {
        std::mem::take(&mut self.pending_tips)
    }

    /// Calls still to come (used by tests and the finale check).
    pub fn scheduled(&self) -> &[Scheduled] {
        &self.schedule
    }

    // --------------------------------------------------------------- commands

    pub fn apply(&mut self, cmd: &Command) -> Result<(), CmdError> {
        if self.end.is_some() {
            return Err(CmdError::ShiftOver);
        }
        match cmd {
            Command::Dispatch { call, units } => self.dispatch(*call, units),
            Command::Dismiss { call } => self.dismiss(*call),
            Command::Flag { call } => self.flag(*call),
            Command::Coffee => self.coffee(),
            Command::Recall { unit } => self.recall(*unit),
        }
    }

    fn dispatch(&mut self, call: u32, unit_ids: &[usize]) -> Result<(), CmdError> {
        if unit_ids.is_empty() {
            return Err(CmdError::NoUnits);
        }
        let ci = self.call_idx(call).ok_or(CmdError::NoSuchCall)?;
        if !self.calls[ci].is_open() {
            return Err(CmdError::CallClosed);
        }
        let target = self.calls[ci].pos;
        let mut ids: Vec<usize> = unit_ids.to_vec();
        ids.sort_unstable();
        ids.dedup();
        let mut paths = Vec::new();
        for &u in &ids {
            let unit = self.units.get(u).ok_or(CmdError::NoSuchUnit(u))?;
            if !unit.is_available() {
                return Err(CmdError::UnitBusy(u));
            }
            let path = find_path(CityMap::get(), unit.pos, target, &self.blocked).ok_or(CmdError::Unreachable)?;
            paths.push((u, path));
        }
        for (u, path) in paths {
            let unit = &mut self.units[u];
            unit.state = UnitState::EnRoute { call };
            unit.path = path.into();
            self.push(Event::Dispatched { call, unit: u });
        }
        Ok(())
    }

    fn dismiss(&mut self, call: u32) -> Result<(), CmdError> {
        let ci = self.call_idx(call).ok_or(CmdError::NoSuchCall)?;
        if !self.calls[ci].is_open() {
            return Err(CmdError::CallClosed);
        }
        for u in self.units_on(call) {
            self.send_home(u);
        }
        let tick = self.tick;
        let c = &mut self.calls[ci];
        c.status = CallStatus::Dismissed;
        c.closed_at = Some(tick);
        if c.prank {
            self.stats.prank_dismissed += 1;
            self.add_trust(50);
            self.push(Event::PrankDismissed { call });
        } else {
            c.callback_at = Some(tick + 10);
            self.stats.real_dismissed += 1;
            self.push(Event::Dismissed { call });
        }
        Ok(())
    }

    fn flag(&mut self, call: u32) -> Result<(), CmdError> {
        let ci = self.call_idx(call).ok_or(CmdError::NoSuchCall)?;
        let on = !self.calls[ci].flagged;
        self.calls[ci].flagged = on;
        self.push(Event::Flagged { call, on });
        Ok(())
    }

    fn coffee(&mut self) -> Result<(), CmdError> {
        if self.coffee_used {
            return Err(CmdError::CoffeeUsed);
        }
        self.coffee_used = true;
        self.coffee_until = Some(self.tick + COFFEE_TICKS);
        self.stats.coffee = true;
        self.add_trust(-400);
        self.stress = (self.stress - 3000).max(0);
        self.push(Event::CoffeeStart);
        Ok(())
    }

    fn recall(&mut self, unit: usize) -> Result<(), CmdError> {
        let u = self.units.get(unit).ok_or(CmdError::NoSuchUnit(unit))?;
        match u.state {
            UnitState::EnRoute { .. } | UnitState::OnScene { .. } => {
                self.send_home(unit);
                self.push(Event::Recalled { unit });
                Ok(())
            }
            _ => Err(CmdError::NotRecallable),
        }
    }

    // ------------------------------------------------------------- simulation

    /// Advance the simulation by one game minute.
    pub fn tick(&mut self) {
        if self.end.is_some() {
            return;
        }
        if self.tick == 0 {
            self.push(Event::ShiftStart);
            if self.params.tutorial {
                self.tip(Tip::Welcome);
            }
            if self.params.night == FINALE_NIGHT && self.params.trap_set {
                self.push(Event::DilshodRadio);
            }
        }
        self.spawn_calls();
        self.process_callbacks();
        self.move_units();
        self.finish_scene_work();
        self.escalate();
        self.update_stress();
        self.check_meters();
        if self.end.is_none() {
            self.tick += 1;
            if self.tick >= SHIFT_TICKS {
                self.close_shift();
            }
        }
    }

    fn push(&mut self, event: Event) {
        self.log.push(LogEntry { tick: self.tick, event });
    }

    fn tip(&mut self, t: Tip) {
        if !self.tips_seen.contains(&t) {
            self.tips_seen.push(t);
            self.pending_tips.push(t);
        }
    }

    fn add_trust(&mut self, d: i32) {
        self.trust = (self.trust + d).clamp(0, METER_MAX);
    }

    fn stress_mult(&self) -> i32 {
        let mut m = self.params.difficulty.params().stress_mult;
        if self.params.upgrades.has(UpgradeKind::CalmTraining) {
            m = m * 75 / 100;
        }
        m
    }

    fn add_stress(&mut self, d: i32) {
        let d = if d > 0 { d * self.stress_mult() / 100 } else { d };
        self.stress = (self.stress + d).clamp(0, METER_MAX);
    }

    fn spawn_calls(&mut self) {
        if self.coffee_until == Some(self.tick) {
            self.coffee_until = None;
            self.push(Event::CoffeeEnd);
            let deferred = std::mem::take(&mut self.deferred);
            for s in deferred {
                self.open_call(&s);
            }
        }
        while self.next_sched < self.schedule.len() && self.schedule[self.next_sched].tick <= self.tick {
            let s = self.schedule[self.next_sched].clone();
            self.next_sched += 1;
            if self.coffee_active() {
                if s.story {
                    self.deferred.push(s);
                } else {
                    self.push(Event::ZuhraTookCall);
                }
                continue;
            }
            self.open_call(&s);
        }
    }

    fn open_call(&mut self, s: &Scheduled) {
        let t = catalog::get(s.template);
        let p = self.params.difficulty.params();
        let id = self.next_id;
        self.next_id += 1;
        let sev = s.severity.clamp(1, 3);
        let call = Call {
            id,
            template: s.template,
            variant: s.variant,
            pos: s.pos,
            district: CityMap::get().district(s.pos),
            arrived: self.tick,
            severity: sev,
            base_severity: sev,
            needs: t.needs,
            served: Needs::NONE,
            prank: t.prank,
            arson: t.arson,
            finale: t.kind == TKind::Finale,
            status: CallStatus::Waiting,
            next_escalation: self.tick + p.esc_interval,
            deadline: if sev >= 3 { Some(self.tick + p.crit_window) } else { None },
            first_response: None,
            flagged: false,
            revealed: false,
            report: None,
            callback: false,
            callback_at: None,
            expire_at: if t.prank { Some(self.tick + p.prank_patience) } else { None },
            closed_at: None,
            prank_penalised: false,
        };
        self.stats.calls += 1;
        if t.prank {
            self.stats.pranks += 1;
        } else {
            self.stats.real_calls += 1;
        }
        if t.arson {
            self.stats.arson_calls += 1;
        }
        self.calls.push(call);
        self.push(Event::NewCall { call: id });
        if self.params.tutorial {
            self.tip(Tip::FirstCall);
            if t.prank && t.weight > 0 {
                self.tip(Tip::Prank);
            }
            if t.is_multi() {
                self.tip(Tip::Multi);
            }
        }
        if t.arson && self.params.night == 0 {
            self.tip(Tip::Arson);
        }
    }

    fn process_callbacks(&mut self) {
        let p = self.params.difficulty.params();
        let tick = self.tick;
        let mut called_back = Vec::new();
        for c in self.calls.iter_mut() {
            if c.status == CallStatus::Dismissed && c.callback_at == Some(tick) {
                c.status = CallStatus::Waiting;
                c.callback = true;
                c.callback_at = None;
                c.closed_at = None;
                c.severity = (c.severity + 1).min(3);
                c.next_escalation = tick + p.esc_interval;
                if c.severity == 3 {
                    c.deadline = Some(tick + p.crit_window);
                }
                called_back.push(c.id);
            }
        }
        for id in called_back {
            self.add_trust(-1200);
            self.add_stress(500);
            self.push(Event::Callback { call: id });
        }
    }

    fn move_units(&mut self) {
        let mut arrivals = Vec::new();
        for i in 0..self.units.len() {
            let siren = match self.units[i].state {
                UnitState::EnRoute { .. } => true,
                UnitState::Returning => false,
                _ => continue,
            };
            let sp = self.speed(&self.units[i], siren);
            let u = &mut self.units[i];
            if !u.path.is_empty() {
                u.budget += sp;
                while u.budget >= 100 {
                    match u.path.pop_front() {
                        Some(p) => {
                            u.pos = p;
                            u.budget -= 100;
                        }
                        None => break,
                    }
                }
            }
            if u.path.is_empty() {
                u.budget = 0;
                arrivals.push(i);
            }
        }
        for i in arrivals {
            match self.units[i].state {
                UnitState::Returning => self.units[i].state = UnitState::AtBase,
                UnitState::EnRoute { call } => self.arrive(i, call),
                _ => {}
            }
        }
    }

    fn work_time(&self, c: &Call, kind: UnitKind) -> u32 {
        let base = c.template().work;
        let mut w = base * (100 + 20 * (c.severity as u32 - 1)) / 100;
        if kind == UnitKind::Ambulance && self.params.upgrades.has(UpgradeKind::RapidTriage) {
            w = w * 80 / 100;
        }
        w.max(3)
    }

    fn arrive(&mut self, ui: usize, call: u32) {
        let Some(ci) = self.call_idx(call) else {
            self.send_home(ui);
            return;
        };
        if !self.calls[ci].is_open() {
            self.send_home(ui);
            return;
        }
        let kind = self.units[ui].kind;
        let tick = self.tick;
        if self.calls[ci].prank {
            let c = &mut self.calls[ci];
            c.status = CallStatus::PrankExposed;
            c.closed_at = Some(tick);
            c.revealed = true;
            c.report = Some(Report::Nothing);
            let first = !c.prank_penalised;
            c.prank_penalised = true;
            self.units[ui].state = UnitState::OnScene { call, until: tick + 2, working: false };
            if first {
                self.stats.prank_dispatched += 1;
                self.add_trust(-150);
                self.add_stress(100);
            }
            self.push(Event::PrankFound { call, unit: ui });
            return;
        }
        let need = self.calls[ci].needs.get(kind);
        let busy = self.calls[ci].served.get(kind) + self.working(call, kind);
        if need > busy {
            let work = self.work_time(&self.calls[ci], kind);
            self.units[ui].state = UnitState::OnScene { call, until: tick + work, working: true };
            let c = &mut self.calls[ci];
            c.revealed = true;
            if c.first_response.is_none() {
                c.first_response = Some(tick);
                let rt = tick - c.arrived;
                let q = response_quality(c.base_severity, rt);
                self.stats.response_count += 1;
                self.stats.response_ticks += rt;
                self.stats.response_quality += q;
            }
            // Does the crew need other services that nobody has been sent for?
            let missing = UnitKind::ALL.iter().any(|&k| {
                k != kind && {
                    let c = &self.calls[ci];
                    c.needs.get(k) > c.served.get(k) + self.assigned(call, k)
                }
            });
            if missing {
                self.calls[ci].report = Some(Report::NeedMore);
                self.push(Event::NeedMore { call, unit: ui });
                if self.params.tutorial {
                    self.tip(Tip::Report);
                }
            } else {
                self.push(Event::Arrived { call, unit: ui });
            }
        } else if need == 0 {
            self.units[ui].state = UnitState::OnScene { call, until: tick + 2, working: false };
            let c = &mut self.calls[ci];
            c.revealed = true;
            c.report = Some(Report::WrongUnit);
            self.stats.wrong_units += 1;
            self.add_trust(-100);
            self.add_stress(200);
            self.push(Event::WrongUnit { call, unit: ui });
            if kind == UnitKind::Fire {
                self.wrong_fire_count += 1;
                if self.wrong_fire_count % 2 == 1 {
                    self.push(Event::AkmalGrumble);
                }
            }
            if self.params.tutorial {
                self.tip(Tip::Report);
            }
        } else {
            self.send_home(ui);
            self.push(Event::NotNeeded { call, unit: ui });
        }
    }

    fn send_home(&mut self, ui: usize) {
        let map = CityMap::get();
        let (pos, base) = (self.units[ui].pos, self.units[ui].base);
        let path = find_path(map, pos, base, &self.blocked).unwrap_or_default();
        let u = &mut self.units[ui];
        u.path = path.into();
        u.state = if u.path.is_empty() && u.pos == u.base { UnitState::AtBase } else { UnitState::Returning };
        if u.path.is_empty() && u.pos != u.base {
            // Should not happen on a connected map; snap home to stay robust.
            u.pos = u.base;
            u.state = UnitState::AtBase;
        }
    }

    fn finish_scene_work(&mut self) {
        let tick = self.tick;
        for i in 0..self.units.len() {
            if let UnitState::OnScene { call, until, working } = self.units[i].state {
                if until > tick {
                    continue;
                }
                if working {
                    self.units[i].jobs += 1;
                    let kind = self.units[i].kind;
                    if let Some(ci) = self.call_idx(call) {
                        if self.calls[ci].is_open() {
                            *self.calls[ci].served.get_mut(kind) += 1;
                            if self.calls[ci].fully_served() {
                                self.resolve(ci);
                            }
                        }
                    }
                }
                self.send_home(i);
            }
        }
    }

    fn resolve(&mut self, ci: usize) {
        let tick = self.tick;
        let c = &mut self.calls[ci];
        c.status = CallStatus::Resolved;
        c.closed_at = Some(tick);
        let (id, sev, finale) = (c.id, c.severity, c.finale);
        self.stats.resolved += 1;
        self.add_trust(50 * sev as i32);
        if sev == 3 {
            self.add_stress(-150);
        }
        self.push(Event::Resolved { call: id });
        if finale {
            self.arsonist_caught = true;
            self.push(Event::ArsonistCaught { call: id });
        }
        for u in self.units_on(id) {
            if !self.units[u].is_working_on(id) {
                self.send_home(u);
            }
        }
    }

    fn fail(&mut self, ci: usize) {
        let tick = self.tick;
        let c = &mut self.calls[ci];
        c.status = CallStatus::Failed;
        c.closed_at = Some(tick);
        let (id, amb, finale) = (c.id, c.needs.amb > 0, c.finale);
        self.stats.failed += 1;
        if amb {
            self.stats.lives_lost += 1;
        }
        let penalty = self.params.difficulty.params().fail_trust;
        self.add_trust(-penalty);
        self.add_stress(700);
        self.push(Event::Failed { call: id });
        if finale {
            self.push(Event::ArsonistEscaped { call: id });
        }
        for u in self.units_on(id) {
            self.send_home(u);
        }
    }

    fn escalate(&mut self) {
        let p = self.params.difficulty.params();
        let tick = self.tick;
        for ci in 0..self.calls.len() {
            if !self.calls[ci].is_open() {
                continue;
            }
            if self.calls[ci].prank {
                if self.calls[ci].expire_at.is_some_and(|t| tick >= t) {
                    let c = &mut self.calls[ci];
                    c.status = CallStatus::Expired;
                    c.closed_at = Some(tick);
                    let id = c.id;
                    self.stats.prank_expired += 1;
                    self.push(Event::Expired { call: id });
                }
                continue;
            }
            if self.is_covered(&self.calls[ci]) {
                let c = &mut self.calls[ci];
                c.next_escalation += 1;
                if let Some(d) = c.deadline.as_mut() {
                    *d += 1;
                }
                continue;
            }
            let c = &mut self.calls[ci];
            if c.severity < 3 && tick >= c.next_escalation {
                c.severity += 1;
                c.next_escalation = tick + p.esc_interval;
                if c.severity == 3 {
                    c.deadline = Some(tick + p.crit_window);
                }
                let (id, severity) = (c.id, c.severity);
                self.push(Event::Escalated { call: id, severity });
                if self.params.tutorial {
                    self.tip(Tip::Escalation);
                }
            } else if c.severity >= 3 && c.deadline.is_some_and(|d| tick >= d) {
                self.fail(ci);
            }
        }
    }

    fn update_stress(&mut self) {
        let q = self.unanswered() as i32;
        let open = self.open_calls().count() as i32;
        let critical = self.open_calls().any(|c| !c.prank && c.severity == 3 && self.units_on(c.id).is_empty());
        let mut d = 2 * open.min(8);
        if q >= 2 {
            d += (q - 1).min(4) * 10;
        }
        if open == 0 {
            d -= 30;
        } else if open <= 2 && !critical {
            d -= 8;
        }
        if self.coffee_active() {
            d -= 10;
        }
        self.add_stress(d);
        // Callers left without any help for a while erode the city's trust.
        let tick = self.tick;
        let ignored = self
            .open_calls()
            .filter(|c| !c.prank && tick >= c.arrived + IGNORED_AFTER && self.units_on(c.id).is_empty())
            .count() as i32;
        self.add_trust(-IGNORED_DRAIN * ignored);
        if self.stress >= 5000 && self.params.night <= 1 {
            self.tip(Tip::Stress);
        }
    }

    fn check_meters(&mut self) {
        if self.trust <= 0 {
            self.trust = 0;
            self.finish(ShiftEnd::Fired);
        } else if self.stress >= METER_MAX {
            self.stress = METER_MAX;
            self.finish(ShiftEnd::Burnout);
        }
    }

    fn close_shift(&mut self) {
        let mut handed = 0;
        let mut complaints = 0;
        for c in self.calls.iter_mut() {
            match c.status {
                CallStatus::Waiting if c.prank => {
                    c.status = CallStatus::Expired;
                    self.stats.prank_expired += 1;
                }
                CallStatus::Waiting => {
                    c.status = CallStatus::HandedOver;
                    handed += 1;
                }
                CallStatus::Dismissed if c.callback_at.is_some() => {
                    c.callback_at = None;
                    complaints += 1;
                }
                _ => {}
            }
            c.closed_at.get_or_insert(SHIFT_TICKS);
        }
        self.stats.handed_over += handed;
        self.add_trust(-1200 * complaints);
        self.push(Event::HandedOver { count: handed });
        if self.trust <= 0 {
            self.finish(ShiftEnd::Fired);
        } else {
            self.finish(ShiftEnd::Completed);
        }
    }

    fn finish(&mut self, end: ShiftEnd) {
        self.stats.flags_correct = self.calls.iter().filter(|c| c.flagged && c.arson).count() as u32;
        self.stats.flags_false = self.calls.iter().filter(|c| c.flagged && !c.arson).count() as u32;
        self.end = Some(end);
    }
}
