//! Terminal UI: application state, input handling and the real-time loop.
//! Rendering lives in `draw.rs` and `map_view.rs`.

mod draw;
mod map_view;

use crate::bot::{Bot, BotProfile};
use crate::cli::Options;
use crate::i18n::{t, tf, Lang};
use crate::save::{self, SaveData};
use crate::sim::rng::mix;
use crate::sim::*;
use ratatui::crossterm::event::{self, Event as CEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Real-time length of one game minute at x1 (480 ticks ~ 6.8 minutes).
pub const TICK_MS: u64 = 850;
pub const MIN_W: u16 = 100;
pub const MIN_H: u16 = 30;
const AUTO_STEP_MS: u64 = 2500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    Briefing,
    Shift,
    Report(Page),
    Ending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Grade,
    Zuhra,
    Inspector,
    Upgrade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    Help,
    QuitConfirm,
    Tip(Tip),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Queue,
    Units,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleItem {
    Continue,
    NewGame,
    Language,
    Difficulty,
    Quit,
}

pub struct App {
    pub lang: Lang,
    pub screen: Screen,
    pub overlay: Option<Overlay>,
    pub title_cursor: usize,
    pub difficulty: Difficulty,
    pub seed: Option<u64>,
    pub week: Option<Week>,
    pub shift: Option<Shift>,
    /// The shift that just ended (for the report pages).
    pub finished: Option<Shift>,
    pub record: Option<NightRecord>,
    pub upgrade_opts: Vec<UpgradeKind>,
    pub tips: VecDeque<Tip>,
    pub selected_call: Option<u32>,
    pub unit_cursor: usize,
    pub chosen: Vec<usize>,
    pub focus: Focus,
    pub paused: bool,
    pub speed: u32,
    pub message: Option<(String, Duration)>,
    pub autoplay: bool,
    bot: Option<Bot>,
    pub save_dir: PathBuf,
    pub saved: Option<SaveData>,
    pub quit: bool,
    accum: Duration,
    auto_timer: Duration,
    /// Milliseconds since start, for blinking.
    pub clock_ms: u128,
}

impl App {
    pub fn new(opts: &Options, save_dir: PathBuf) -> App {
        let saved = save::load(&save_dir);
        let lang = opts.lang.or(saved.as_ref().map(|s| s.lang)).unwrap_or_else(Lang::detect);
        App {
            lang,
            screen: Screen::Title,
            overlay: None,
            title_cursor: 0,
            difficulty: opts.difficulty.unwrap_or(Difficulty::Dispatcher),
            seed: opts.seed,
            week: None,
            shift: None,
            finished: None,
            record: None,
            upgrade_opts: Vec::new(),
            tips: VecDeque::new(),
            selected_call: None,
            unit_cursor: 0,
            chosen: Vec::new(),
            focus: Focus::Queue,
            paused: false,
            speed: 1,
            message: None,
            autoplay: opts.autoplay,
            bot: None,
            save_dir,
            saved,
            quit: false,
            accum: Duration::ZERO,
            auto_timer: Duration::ZERO,
            clock_ms: 0,
        }
    }

    pub fn t(&self, key: &str) -> &'static str {
        t(self.lang, key)
    }

    pub fn tf(&self, key: &str, args: &[(&str, String)]) -> String {
        tf(self.lang, key, args)
    }

    pub fn title_items(&self) -> Vec<TitleItem> {
        let mut v = Vec::new();
        if self.saved.is_some() && !self.autoplay {
            v.push(TitleItem::Continue);
        }
        v.extend([TitleItem::NewGame, TitleItem::Language, TitleItem::Difficulty, TitleItem::Quit]);
        v
    }

    fn flash(&mut self, text: String) {
        self.message = Some((text, Duration::from_millis(3500)));
    }

    fn persist(&mut self) {
        if self.autoplay {
            return;
        }
        if let Some(w) = &self.week {
            if w.is_over() {
                let _ = save::clear(&self.save_dir);
                self.saved = None;
            } else {
                let data = SaveData::new(self.lang, w.clone());
                let _ = save::save(&self.save_dir, &data);
                self.saved = Some(data);
            }
        }
    }

    // ------------------------------------------------------------ flow

    fn new_game(&mut self) {
        let seed = self.seed.unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
                % 1_000_000
        });
        let mut w = Week::new(self.difficulty, seed);
        w.tutorial = !self.autoplay;
        self.week = Some(w);
        self.screen = Screen::Briefing;
    }

    fn continue_game(&mut self) {
        if let Some(s) = self.saved.clone() {
            self.difficulty = s.week.difficulty;
            self.week = Some(s.week);
            self.screen = Screen::Briefing;
        }
    }

    fn start_shift(&mut self) {
        let Some(week) = &self.week else { return };
        let shift = week.start_shift();
        self.bot = self.autoplay.then(|| Bot::new(BotProfile::good(), mix(week.seed, 77 + week.night as u64)));
        self.shift = Some(shift);
        self.selected_call = None;
        self.chosen.clear();
        self.unit_cursor = 0;
        self.focus = Focus::Queue;
        self.paused = false;
        self.speed = if self.autoplay { 4 } else { 1 };
        self.accum = Duration::ZERO;
        self.tips.clear();
        self.screen = Screen::Shift;
    }

    fn end_shift(&mut self) {
        let (Some(shift), Some(week)) = (self.shift.take(), self.week.as_mut()) else { return };
        let rec = week.finish_shift(&shift);
        let lost = week.ending.is_some_and(|e| e.is_loss());
        self.upgrade_opts = if week.is_over() { Vec::new() } else { week.upgrade_options() };
        self.record = Some(rec);
        self.finished = Some(shift);
        self.overlay = None;
        self.tips.clear();
        self.persist();
        self.screen = if lost { Screen::Ending } else { Screen::Report(Page::Grade) };
    }

    fn next_page(&mut self) {
        let over = self.week.as_ref().is_some_and(|w| w.is_over());
        self.screen = match self.screen {
            Screen::Report(Page::Grade) => Screen::Report(Page::Zuhra),
            Screen::Report(Page::Zuhra) => Screen::Report(Page::Inspector),
            Screen::Report(Page::Inspector) if over => Screen::Ending,
            Screen::Report(Page::Inspector) if self.upgrade_opts.is_empty() => Screen::Briefing,
            Screen::Report(Page::Inspector) => Screen::Report(Page::Upgrade),
            s => s,
        };
    }

    fn choose_upgrade(&mut self, i: usize) {
        if let Some(&u) = self.upgrade_opts.get(i) {
            if let Some(w) = self.week.as_mut() {
                w.apply_upgrade(u);
            }
            self.upgrade_opts.clear();
            self.persist();
            self.screen = Screen::Briefing;
        }
    }

    fn back_to_title(&mut self) {
        self.week = None;
        self.shift = None;
        self.finished = None;
        self.record = None;
        self.saved = save::load(&self.save_dir);
        self.title_cursor = 0;
        self.screen = Screen::Title;
    }

    // ------------------------------------------------------------ time

    /// Advance real time; runs simulation ticks while a shift is live.
    pub fn update(&mut self, dt: Duration) {
        self.clock_ms += dt.as_millis();
        if let Some((_, left)) = self.message.as_mut() {
            *left = left.saturating_sub(dt);
            if left.is_zero() {
                self.message = None;
            }
        }
        if self.screen == Screen::Shift && self.overlay.is_none() && !self.paused {
            self.accum += dt * self.speed;
            let tick = Duration::from_millis(TICK_MS);
            while self.accum >= tick {
                self.accum -= tick;
                self.step();
                if self.screen != Screen::Shift || self.overlay.is_some() {
                    self.accum = Duration::ZERO;
                    break;
                }
            }
        }
        if self.autoplay {
            self.auto_update(dt);
        }
    }

    /// One simulation tick (plus the bot in autoplay).
    pub fn step(&mut self) {
        let Some(shift) = self.shift.as_mut() else { return };
        if let Some(bot) = self.bot.as_mut() {
            for cmd in bot.decide(shift) {
                let _ = shift.apply(&cmd);
            }
            if let Some(f) = bot.focus {
                self.selected_call = Some(f);
            }
        }
        shift.tick();
        self.tips.extend(shift.take_tips());
        if self.overlay.is_none() {
            if let Some(tip) = self.tips.pop_front() {
                self.overlay = Some(Overlay::Tip(tip));
            }
        }
        self.fix_selection();
        if self.shift.as_ref().is_some_and(|s| s.is_over()) {
            self.end_shift();
        }
    }

    fn auto_update(&mut self, dt: Duration) {
        self.auto_timer += dt;
        let wait = Duration::from_millis(AUTO_STEP_MS);
        if self.auto_timer < wait {
            return;
        }
        if let Some(o) = self.overlay {
            if matches!(o, Overlay::Tip(_)) {
                self.auto_timer = Duration::ZERO;
                self.close_tip();
            }
            return;
        }
        match self.screen {
            Screen::Title => self.new_game(),
            Screen::Briefing => self.start_shift(),
            Screen::Report(Page::Upgrade) => {
                let pick = Bot::pick_upgrade(&self.upgrade_opts)
                    .and_then(|u| self.upgrade_opts.iter().position(|&o| o == u))
                    .unwrap_or(0);
                self.choose_upgrade(pick);
            }
            Screen::Report(_) => self.next_page(),
            Screen::Ending => {
                if self.auto_timer >= wait * 3 {
                    self.back_to_title();
                } else {
                    return;
                }
            }
            Screen::Shift => return,
        }
        self.auto_timer = Duration::ZERO;
    }

    /// Calls in display order: open ones by arrival, then recently closed ones.
    pub fn queue_ids(&self) -> Vec<u32> {
        let Some(s) = &self.shift else { return Vec::new() };
        let q = s.queue();
        let mut ids: Vec<u32> = q.iter().filter(|c| c.is_open()).map(|c| c.id).collect();
        ids.extend(q.iter().filter(|c| !c.is_open()).map(|c| c.id));
        ids
    }

    fn fix_selection(&mut self) {
        let ids = self.queue_ids();
        let open: Vec<u32> = match &self.shift {
            Some(s) => ids.iter().copied().filter(|id| s.call(*id).is_some_and(|c| c.is_open())).collect(),
            None => Vec::new(),
        };
        let keep = self.selected_call.is_some_and(|id| ids.contains(&id));
        if !keep {
            self.selected_call = open.first().copied().or_else(|| ids.first().copied());
        }
        if let Some(s) = &self.shift {
            self.chosen.retain(|&u| s.units.get(u).is_some_and(|u| u.is_available()));
            if self.unit_cursor >= s.units.len() {
                self.unit_cursor = s.units.len().saturating_sub(1);
            }
        }
    }

    fn close_tip(&mut self) {
        self.overlay = self.tips.pop_front().map(Overlay::Tip);
    }

    // ------------------------------------------------------------ input

    pub fn on_key(&mut self, k: KeyEvent) {
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c') | KeyCode::Char('C')) {
            self.quit = true;
            return;
        }
        if let Some(o) = self.overlay {
            match o {
                Overlay::Help => {
                    if matches!(k.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') | KeyCode::Char('q')) {
                        self.overlay = None;
                    }
                }
                Overlay::QuitConfirm => match k.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => self.quit = true,
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => self.overlay = None,
                    _ => {}
                },
                Overlay::Tip(_) => {
                    if matches!(k.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ')) {
                        self.close_tip();
                    }
                }
            }
            return;
        }
        if matches!(k.code, KeyCode::Char('l') | KeyCode::Char('L')) {
            self.lang = self.lang.next();
            return;
        }
        match self.screen {
            Screen::Title => self.key_title(k),
            Screen::Briefing => match k.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.start_shift(),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => self.overlay = Some(Overlay::QuitConfirm),
                _ => {}
            },
            Screen::Shift => self.key_shift(k),
            Screen::Report(page) => match k.code {
                KeyCode::Char(c @ '1'..='3') if page == Page::Upgrade => self.choose_upgrade(c as usize - '1' as usize),
                KeyCode::Enter | KeyCode::Char(' ') if page != Page::Upgrade => self.next_page(),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => self.overlay = Some(Overlay::QuitConfirm),
                _ => {}
            },
            Screen::Ending => {
                if matches!(k.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ')) {
                    self.back_to_title();
                } else if matches!(k.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
                    self.quit = true;
                }
            }
        }
    }

    fn key_title(&mut self, k: KeyEvent) {
        let items = self.title_items();
        self.title_cursor = self.title_cursor.min(items.len() - 1);
        let item = items[self.title_cursor];
        match k.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.title_cursor = (self.title_cursor + items.len() - 1) % items.len();
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.title_cursor = (self.title_cursor + 1) % items.len();
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
                let back = k.code == KeyCode::Left;
                match item {
                    TitleItem::Language => {
                        self.lang = if back { self.lang.next().next() } else { self.lang.next() };
                    }
                    TitleItem::Difficulty => {
                        self.difficulty = if back { self.difficulty.prev() } else { self.difficulty.next() };
                    }
                    TitleItem::Continue if k.code == KeyCode::Enter => self.continue_game(),
                    TitleItem::NewGame if k.code == KeyCode::Enter => self.new_game(),
                    TitleItem::Quit if k.code == KeyCode::Enter => self.quit = true,
                    _ => {}
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => self.quit = true,
            _ => {}
        }
    }

    fn key_shift(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::F(1) => self.overlay = Some(Overlay::Help),
            KeyCode::Char('q') | KeyCode::Char('Q') => self.overlay = Some(Overlay::QuitConfirm),
            KeyCode::Char(' ') | KeyCode::Char('p') => self.paused = !self.paused,
            KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') => self.speed = (self.speed * 2).min(4),
            KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Char('[') => self.speed = (self.speed / 2).max(1),
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = if self.focus == Focus::Queue { Focus::Units } else { Focus::Queue };
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Char(c @ '1'..='9') => self.toggle_unit(c as usize - '1' as usize),
            KeyCode::Enter => self.dispatch(),
            KeyCode::Esc | KeyCode::Backspace => self.chosen.clear(),
            KeyCode::Char('d') | KeyCode::Char('D') => self.command_on_call(|id| Command::Dismiss { call: id }),
            KeyCode::Char('f') | KeyCode::Char('F') => self.command_on_call(|id| Command::Flag { call: id }),
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.run(Command::Coffee);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => self.recall(),
            _ => {}
        }
    }

    fn move_cursor(&mut self, d: i32) {
        match self.focus {
            Focus::Queue => {
                let ids = self.queue_ids();
                if ids.is_empty() {
                    return;
                }
                let cur = self.selected_call.and_then(|id| ids.iter().position(|&x| x == id)).unwrap_or(0) as i32;
                let next = (cur + d).clamp(0, ids.len() as i32 - 1) as usize;
                self.selected_call = Some(ids[next]);
            }
            Focus::Units => {
                let n = self.shift.as_ref().map_or(0, |s| s.units.len()) as i32;
                if n > 0 {
                    self.unit_cursor = (self.unit_cursor as i32 + d).clamp(0, n - 1) as usize;
                }
            }
        }
    }

    fn toggle_unit(&mut self, i: usize) {
        let Some(s) = &self.shift else { return };
        let Some(u) = s.units.get(i) else { return };
        self.unit_cursor = i;
        if let Some(pos) = self.chosen.iter().position(|&x| x == i) {
            self.chosen.remove(pos);
        } else if u.is_available() {
            self.chosen.push(i);
        } else {
            let msg = self.tf(CmdError::UnitBusy(i).key(), crate::args!(unit = u.code()));
            self.flash(msg);
        }
    }

    fn run(&mut self, cmd: Command) -> bool {
        let Some(s) = self.shift.as_mut() else { return false };
        match s.apply(&cmd) {
            Ok(()) => true,
            Err(e) => {
                let unit = match e {
                    CmdError::UnitBusy(u) | CmdError::NoSuchUnit(u) => {
                        s.units.get(u).map(|u| u.code()).unwrap_or_default()
                    }
                    _ => String::new(),
                };
                let msg = self.tf(e.key(), crate::args!(unit = unit));
                self.flash(msg);
                false
            }
        }
    }

    fn command_on_call(&mut self, make: impl Fn(u32) -> Command) {
        match self.selected_call {
            Some(id) => {
                self.run(make(id));
            }
            None => {
                let m = self.t(CmdError::NoSuchCall.key()).to_string();
                self.flash(m);
            }
        }
    }

    fn dispatch(&mut self) {
        let Some(call) = self.selected_call else {
            let m = self.t(CmdError::NoSuchCall.key()).to_string();
            self.flash(m);
            return;
        };
        let units = if !self.chosen.is_empty() {
            self.chosen.clone()
        } else if self.focus == Focus::Units {
            vec![self.unit_cursor]
        } else {
            let m = self.t("ui.pick_hint").to_string();
            self.flash(m);
            return;
        };
        if self.run(Command::Dispatch { call, units }) {
            self.chosen.clear();
        }
    }

    /// R: in the units pane recall the unit under the cursor; in the queue,
    /// recall every unit committed to the selected call.
    fn recall(&mut self) {
        let Some(s) = &self.shift else { return };
        let targets: Vec<usize> = match self.focus {
            Focus::Units => vec![self.unit_cursor],
            Focus::Queue => self.selected_call.map(|c| s.units_on(c)).unwrap_or_default(),
        };
        if targets.is_empty() {
            let m = self.t(CmdError::NotRecallable.key()).to_string();
            self.flash(m);
        }
        for u in targets {
            self.run(Command::Recall { unit: u });
        }
    }
}

/// Render the current state into a frame (also used by the UI smoke tests).
pub fn render(f: &mut ratatui::Frame, app: &App) {
    draw::draw(f, app);
}

/// Run the interactive TUI until the player quits.
pub fn run(opts: &Options) -> io::Result<()> {
    let mut app = App::new(opts, save::default_dir());
    let mut terminal = ratatui::try_init()?;
    let result = (|| -> io::Result<()> {
        let mut last = Instant::now();
        while !app.quit {
            terminal.draw(|f| draw::draw(f, &app))?;
            if event::poll(Duration::from_millis(40))? {
                if let CEvent::Key(k) = event::read()? {
                    app.on_key(k);
                }
            }
            let now = Instant::now();
            app.update(now - last);
            last = now;
        }
        Ok(())
    })();
    ratatui::restore();
    result
}
