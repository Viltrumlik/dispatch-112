//! Rendering of every screen. Pure function of `App` state.

use super::map_view::{kind_color, map_lines, severity_color};
use super::{App, Focus, Overlay, Page, Screen, TitleItem, MIN_H, MIN_W};
use crate::args;
use crate::sim::*;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Wrap};
use ratatui::Frame;

const ACCENT: Color = Color::LightYellow;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    if area.width < MIN_W || area.height < MIN_H {
        draw_too_small(f, app, area);
        return;
    }
    match app.screen {
        Screen::Title => draw_title(f, app, area),
        Screen::Briefing => draw_briefing(f, app, area),
        Screen::Shift => draw_shift(f, app, area),
        Screen::Report(p) => draw_report(f, app, area, p),
        Screen::Ending => draw_ending(f, app, area),
    }
    if let Some(o) = app.overlay {
        draw_overlay(f, app, area, o);
    }
}

// ------------------------------------------------------------------ helpers

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h)
}

fn panel(title: String, color: Color) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color))
        .title(Span::styled(format!(" {title} "), Style::new().fg(color).add_modifier(Modifier::BOLD)))
}

fn bold(s: impl Into<String>, c: Color) -> Span<'static> {
    Span::styled(s.into(), Style::new().fg(c).add_modifier(Modifier::BOLD))
}

fn plain(s: impl Into<String>) -> Span<'static> {
    Span::raw(s.into())
}

fn dim(s: impl Into<String>) -> Span<'static> {
    Span::styled(s.into(), Style::new().fg(Color::DarkGray))
}

fn colored(s: impl Into<String>, c: Color) -> Span<'static> {
    Span::styled(s.into(), Style::new().fg(c))
}

fn meter_bar(pct: i32, width: usize, color: Color) -> Vec<Span<'static>> {
    let filled = ((pct.clamp(0, 100) as usize) * width).div_ceil(100).min(width);
    vec![colored("█".repeat(filled), color), dim("░".repeat(width - filled))]
}

fn night_name(app: &App, n: u8) -> &'static str {
    app.t(&format!("night.{}", n.min(4)))
}

fn sev_marks(sev: u8) -> String {
    format!("{:<3}", "!".repeat(sev as usize))
}

fn minutes(app: &App, n: u32) -> String {
    app.tf("ui.min", args!(n = n))
}

fn needs_text(app: &App, needs: Needs) -> String {
    let mut parts = Vec::new();
    for k in UnitKind::ALL {
        let n = needs.get(k);
        if n == 1 {
            parts.push(app.t(k.key()).to_string());
        } else if n > 1 {
            parts.push(format!("{} ×{}", app.t(k.key()), n));
        }
    }
    parts.join(", ")
}

/// Missing services according to the true needs (used for crew reports).
fn missing_needs(shift: &Shift, c: &Call) -> Needs {
    let mut m = Needs::NONE;
    for k in UnitKind::ALL {
        *m.get_mut(k) = c.needs.get(k).saturating_sub(c.served.get(k) + shift.assigned(c.id, k));
    }
    m
}

fn status_word(app: &App, s: CallStatus) -> (&'static str, Color) {
    match s {
        CallStatus::Resolved => (app.t("ui.closed.resolved"), Color::Green),
        CallStatus::Failed => (app.t("ui.closed.failed"), Color::Red),
        CallStatus::Dismissed => (app.t("ui.closed.dismissed"), Color::Gray),
        CallStatus::Expired => (app.t("ui.closed.expired"), Color::Gray),
        CallStatus::PrankExposed => (app.t("ui.closed.prank"), Color::Gray),
        CallStatus::HandedOver => (app.t("ui.closed.handed"), Color::Gray),
        CallStatus::Waiting => ("", Color::White),
    }
}

fn status_glyph(s: CallStatus) -> &'static str {
    match s {
        CallStatus::Resolved => "✓",
        CallStatus::Failed => "✗",
        CallStatus::PrankExposed => "?",
        _ => "–",
    }
}

fn unit_state(app: &App, u: &Unit) -> String {
    match u.state {
        UnitState::AtBase => app.t("ustate.base").to_string(),
        UnitState::Returning => app.t("ustate.returning").to_string(),
        UnitState::EnRoute { call } => format!("→#{call}"),
        UnitState::OnScene { call, .. } => app.tf("ustate.onscene", args!(call = call)),
    }
}

// ------------------------------------------------------------------ guard

fn draw_too_small(f: &mut Frame, app: &App, area: Rect) {
    let text = app.tf("ui.too_small", args!(w = MIN_W, h = MIN_H, cw = area.width, ch = area.height));
    let p =
        Paragraph::new(vec![Line::from(bold(app.t("game.title"), Color::LightRed)), Line::from(""), Line::from(text)])
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
    let h = 5.min(area.height);
    let r = Rect::new(area.x, area.y + area.height.saturating_sub(h) / 2, area.width, h);
    f.render_widget(p, r);
}

// ------------------------------------------------------------------ title

const LOGO: [&str; 5] =
    [" ██    ██   █████ ", "███   ███       ██", " ██    ██    ████ ", " ██    ██   ██    ", "████  ████  ██████"];

fn draw_title(f: &mut Frame, app: &App, area: Rect) {
    let siren = if (app.clock_ms / 600).is_multiple_of(2) { Color::LightRed } else { Color::LightBlue };
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for row in LOGO {
        lines.push(Line::from(bold(row, siren)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(bold(app.t("game.title"), Color::White)));
    lines.push(Line::from(colored(app.t("game.subtitle"), Color::Gray)));
    lines.push(Line::from(""));
    let items = app.title_items();
    for (i, item) in items.iter().enumerate() {
        let sel = i == app.title_cursor.min(items.len() - 1);
        let text = match item {
            TitleItem::Continue => {
                let s = app.saved.as_ref();
                let night = s.map(|s| night_name(app, s.week.night)).unwrap_or("");
                let diff = s.map(|s| app.t(s.week.difficulty.key())).unwrap_or("");
                app.tf("title.continue", args!(night = night, difficulty = diff))
            }
            TitleItem::NewGame => app.t("title.new").to_string(),
            TitleItem::Language => format!("{}: ◀ {} ▶", app.t("title.language"), app.lang.native_name()),
            TitleItem::Difficulty => {
                format!("{}: ◀ {} ▶", app.t("title.difficulty"), app.t(app.difficulty.key()))
            }
            TitleItem::Quit => app.t("title.quit").to_string(),
        };
        let line = if sel {
            Line::from(vec![bold("▶ ", ACCENT), bold(text, ACCENT), bold(" ◀", ACCENT)])
        } else {
            Line::from(plain(text))
        };
        lines.push(line);
    }
    lines.push(Line::from(""));
    lines.push(Line::from(colored(app.t(&format!("title.desc.{}", app.difficulty.code())), Color::Gray)));
    if let Some(seed) = app.seed {
        lines.push(Line::from(dim(app.tf("title.seed", args!(seed = seed)))));
    }
    if app.autoplay {
        lines.push(Line::from(bold(app.t("ui.autoplay"), Color::LightMagenta)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(dim(app.t("title.hint"))));
    let box_area = centered(area, 86, 28);
    let p = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(Block::bordered().border_type(BorderType::Double).border_style(Style::new().fg(Color::DarkGray)));
    f.render_widget(p, box_area);
}

// ------------------------------------------------------------------ briefing

fn draw_briefing(f: &mut Frame, app: &App, area: Rect) {
    let Some(week) = &app.week else { return };
    let n = week.night;
    let mut lines = vec![
        Line::from(bold(app.t(&format!("night.event.{n}")), ACCENT)),
        Line::from(""),
        Line::from(vec![bold(format!("{}: ", app.t("zuhra.title")), Color::LightMagenta)]),
        Line::from(plain(app.t(&format!("briefing.{n}")))),
        Line::from(""),
    ];
    if n == 4 {
        let key = if week.trap_set { "briefing.trap" } else { "briefing.notrap" };
        lines.push(Line::from(colored(app.t(key), Color::LightCyan)));
        lines.push(Line::from(""));
    }
    let units = crate::sim::shift::build_fleet(week.difficulty, &week.upgrades);
    let count = |k: UnitKind| units.iter().filter(|u| u.kind == k).count();
    lines.push(Line::from(dim(app.tf(
        "briefing.info",
        args!(
            difficulty = app.t(week.difficulty.key()),
            amb = count(UnitKind::Ambulance),
            fire = count(UnitKind::Fire),
            police = count(UnitKind::Police),
            trust = week.trust / 100,
            seed = week.seed
        ),
    ))));
    lines.push(Line::from(""));
    lines.push(Line::from(bold(app.t("briefing.start"), Color::LightGreen)));
    let title = app.tf("briefing.title", args!(night = night_name(app, n)));
    let p = Paragraph::new(lines).wrap(Wrap { trim: true }).block(panel(title, Color::LightMagenta));
    f.render_widget(p, centered(area, 88, 22));
}

// ------------------------------------------------------------------ shift

fn draw_shift(f: &mut Frame, app: &App, area: Rect) {
    let Some(shift) = &app.shift else { return };
    let [status, middle, bottom, footer] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(20), Constraint::Min(7), Constraint::Length(1)])
            .areas(area);
    let extra = area.width.saturating_sub(MIN_W);
    let qw = 21 + extra / 2;
    let uw = 19 + extra - extra / 2;
    let [qa, ma, ua] =
        Layout::horizontal([Constraint::Length(qw), Constraint::Length(60), Constraint::Length(uw)]).areas(middle);
    let [da, la] = Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)]).areas(bottom);

    draw_status(f, app, shift, status);
    draw_queue(f, app, shift, qa);
    draw_map(f, app, shift, ma);
    draw_units(f, app, shift, ua);
    draw_details(f, app, shift, da);
    draw_log(f, app, shift, la);
    draw_footer(f, app, footer);
}

fn draw_status(f: &mut Frame, app: &App, s: &Shift, r: Rect) {
    let (h, m) = s.clock();
    let trust = s.trust_pct();
    let stress = s.stress_pct();
    let tc = if trust > 50 {
        Color::LightGreen
    } else if trust > 25 {
        Color::LightYellow
    } else {
        Color::LightRed
    };
    let sc = if stress < 40 {
        Color::LightGreen
    } else if stress < 70 {
        Color::LightYellow
    } else {
        Color::LightRed
    };
    let sep = || dim(" │ ");
    let mut spans = vec![bold(format!(" {} {:02}:{:02}", night_name(app, s.night()), h, m), Color::White), sep()];
    spans.push(plain(format!("{} ", app.t("ui.trust"))));
    spans.extend(meter_bar(trust, 8, tc));
    spans.push(bold(format!(" {trust:>3}"), tc));
    spans.push(sep());
    spans.push(plain(format!("{} ", app.t("ui.stress"))));
    spans.extend(meter_bar(stress, 8, sc));
    spans.push(bold(format!(" {stress:>3}"), sc));
    spans.push(sep());
    spans.push(bold(format!("×{}", app.speed), Color::White));
    spans.push(sep());
    if s.coffee_active() {
        spans.push(bold(app.t("ui.coffee_now"), Color::LightMagenta));
    } else if s.coffee_used {
        spans.push(dim(app.t("ui.coffee_used")));
    } else {
        spans.push(plain(app.t("ui.coffee_ready")));
    }
    let flags = s.calls.iter().filter(|c| c.flagged).count();
    if flags > 0 {
        spans.push(sep());
        spans.push(colored(app.tf("ui.flags", args!(n = flags)), Color::LightCyan));
    }
    if s.is_storm() {
        spans.push(sep());
        spans.push(colored(app.t("ui.storm"), Color::LightBlue));
    }
    if app.paused {
        spans.push(sep());
        spans.push(Span::styled(
            app.t("ui.paused").to_string(),
            Style::new().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }
    if app.autoplay {
        spans.push(sep());
        spans.push(bold(app.t("ui.autoplay"), Color::LightMagenta));
    }
    f.render_widget(Paragraph::new(Line::from(spans)).style(Style::new().bg(Color::Rgb(25, 25, 35))), r);
}

fn draw_queue(f: &mut Frame, app: &App, s: &Shift, r: Rect) {
    let ids = app.queue_ids();
    let open = s.open_calls().count();
    let focus = app.focus == Focus::Queue;
    let border = if focus { ACCENT } else { Color::Gray };
    let block = panel(format!("{} ({open})", app.t("ui.queue")), border);
    let inner_h = r.height.saturating_sub(2) as usize;
    let sel_pos = app.selected_call.and_then(|id| ids.iter().position(|&x| x == id)).unwrap_or(0);
    let start = if sel_pos >= inner_h { sel_pos + 1 - inner_h } else { 0 };
    let mut lines = Vec::new();
    if ids.is_empty() {
        lines.push(Line::from(dim(app.t("ui.queue_empty"))));
    }
    for &id in ids.iter().skip(start).take(inner_h) {
        let Some(c) = s.call(id) else { continue };
        let sel = app.selected_call == Some(id);
        let marker = if sel { bold("▶", if focus { ACCENT } else { Color::White }) } else { plain(" ") };
        let flag = if c.flagged { colored("⚑", Color::LightCyan) } else { plain(" ") };
        let district = app.t(c.district.key());
        if c.is_open() {
            let wait = s.tick.saturating_sub(c.arrived);
            let assigned = s.units_on(id);
            let glyph = if assigned.iter().any(|&u| s.units[u].is_working_on(id)) {
                colored("●", Color::LightGreen)
            } else if !assigned.is_empty() {
                colored("→", Color::LightGreen)
            } else {
                plain(" ")
            };
            let mut sev_style = Style::new().fg(severity_color(c.severity)).add_modifier(Modifier::BOLD);
            if c.severity == 3 && (app.clock_ms / 400).is_multiple_of(2) {
                sev_style = sev_style.add_modifier(Modifier::REVERSED);
            }
            let id_style = if sel { Style::new().add_modifier(Modifier::BOLD) } else { Style::new() };
            lines.push(Line::from(vec![
                marker,
                Span::styled(format!("#{id:<2}"), id_style),
                flag,
                Span::styled(sev_marks(c.severity), sev_style),
                plain(format!("{:>4}", minutes(app, wait))),
                glyph,
                plain(district.to_string()),
            ]));
        } else {
            let (_, color) = status_word(app, c.status);
            lines.push(Line::from(vec![
                marker,
                dim(format!("#{id:<2}")),
                flag,
                colored(format!("{} ", status_glyph(c.status)), color),
                dim(district.to_string()),
            ]));
        }
    }
    f.render_widget(Paragraph::new(lines).block(block), r);
}

fn draw_map(f: &mut Frame, app: &App, s: &Shift, r: Rect) {
    let blink = (app.clock_ms / 500).is_multiple_of(2);
    let block = panel(app.t("ui.map").to_string(), Color::Gray).title_bottom(Line::from(vec![
        dim(" "),
        Span::styled("H", Style::new().fg(Color::White).bg(Color::Green)),
        dim(format!(" {} ", app.t("ui.legend.h"))),
        Span::styled("F", Style::new().fg(Color::White).bg(Color::Red)),
        dim(format!(" {} ", app.t("ui.legend.f"))),
        Span::styled("P", Style::new().fg(Color::White).bg(Color::Blue)),
        dim(format!(" {} ", app.t("ui.legend.p"))),
    ]));
    f.render_widget(Paragraph::new(map_lines(s, app.selected_call, blink)).block(block), r);
}

fn draw_units(f: &mut Frame, app: &App, s: &Shift, r: Rect) {
    let focus = app.focus == Focus::Units;
    let block = panel(app.t("ui.units").to_string(), if focus { ACCENT } else { Color::Gray });
    let target = app.selected_call.and_then(|id| s.call(id)).filter(|c| c.is_open()).map(|c| c.pos);
    let mut lines = Vec::new();
    for u in &s.units {
        let chosen = app.chosen.contains(&u.id);
        let cursor = focus && app.unit_cursor == u.id;
        let kc = kind_color(u.kind);
        let mut row = vec![
            if cursor {
                bold("▶", ACCENT)
            } else if chosen {
                bold("+", ACCENT)
            } else {
                plain(" ")
            },
            bold(format!("{}", u.id + 1), Color::White),
            plain(" "),
            bold(u.code(), kc),
            plain(" "),
        ];
        let state = unit_state(app, u);
        if u.is_available() {
            row.push(colored(state, if u.state == UnitState::AtBase { Color::Gray } else { Color::DarkGray }));
            if let Some(eta) = target.and_then(|p| s.eta(u.id, p)) {
                row.push(colored(format!(" ~{}", minutes(app, eta)), Color::LightGreen));
            }
        } else {
            row.push(colored(state, Color::Yellow));
        }
        let mut line = Line::from(row);
        if chosen {
            line = line.style(Style::new().bg(Color::Rgb(60, 60, 20)));
        }
        lines.push(line);
    }
    lines.push(Line::from(""));
    for k in UnitKind::ALL {
        lines.push(Line::from(vec![bold(k.letter().to_string(), kind_color(k)), dim(format!(" {}", app.t(k.key())))]));
    }
    f.render_widget(Paragraph::new(lines).block(block), r);
}

fn draw_details(f: &mut Frame, app: &App, s: &Shift, r: Rect) {
    let Some(c) = app.selected_call.and_then(|id| s.call(id)) else {
        let p = Paragraph::new(dim(app.t("ui.details_none")))
            .wrap(Wrap { trim: true })
            .block(panel(app.t("ui.details_title").to_string(), Color::Gray));
        f.render_widget(p, r);
        return;
    };
    let title = app.tf("ui.details", args!(call = c.id, district = app.t(c.district.key())));
    let mut lines: Vec<Line> = Vec::new();
    let mut head = vec![
        bold(sev_marks(c.severity), severity_color(c.severity)),
        bold(app.t(&format!("ui.sev.{}", c.severity)), severity_color(c.severity)),
    ];
    if c.is_open() {
        let wait = s.tick.saturating_sub(c.arrived);
        head.push(dim(" · "));
        head.push(plain(app.tf("ui.waiting", args!(n = wait))));
        if let Some(d) = c.deadline.filter(|_| !c.prank) {
            head.push(dim(" · "));
            head.push(bold(app.tf("ui.deadline", args!(n = d.saturating_sub(s.tick))), Color::Red));
        }
    } else {
        let (w, col) = status_word(app, c.status);
        head.push(dim(" · "));
        head.push(bold(w, col));
    }
    if c.callback {
        head.push(dim(" · "));
        head.push(bold(app.t("ui.callback"), Color::LightRed));
    }
    if c.flagged {
        head.push(dim(" · "));
        head.push(bold(format!("⚑ {}", app.t("ui.flagged")), Color::LightCyan));
    }
    lines.push(Line::from(head));
    lines.push(Line::from(Span::styled(
        format!("“{}”", app.t(&c.text_key())),
        Style::new().fg(Color::White).add_modifier(Modifier::ITALIC),
    )));
    // Crew report.
    if c.is_open() {
        match c.report {
            Some(Report::WrongUnit) => {
                let need = needs_text(app, missing_needs(s, c));
                lines.push(Line::from(bold(app.tf("ui.report.wrong", args!(needs = need)), Color::LightRed)));
            }
            Some(Report::NeedMore) => {
                let m = missing_needs(s, c);
                if !m.is_empty() {
                    lines.push(Line::from(bold(app.tf("ui.report.more", args!(needs = needs_text(app, m))), ACCENT)));
                }
            }
            _ => {}
        }
    } else if c.report == Some(Report::Nothing) {
        lines.push(Line::from(colored(app.t("ui.report.nothing"), Color::Gray)));
    }
    if s.params.upgrades.has(UpgradeKind::Screening) {
        if let Some(h) = c.template().hint {
            lines.push(Line::from(colored(
                app.tf("ui.screening", args!(hint = app.t(&format!("hint.{h}")))),
                Color::LightCyan,
            )));
        }
    }
    // Units on this call.
    let on = s.units_on(c.id);
    if on.is_empty() {
        if c.is_open() {
            lines.push(Line::from(dim(app.t("ui.assigned_none"))));
        }
    } else {
        let list: Vec<String> = on
            .iter()
            .map(|&u| {
                let unit = &s.units[u];
                match unit.state {
                    UnitState::EnRoute { .. } => {
                        let eta = (unit.path.len() as u32 * 100).div_ceil(s.speed(unit, true));
                        format!("{} {}", unit.code(), app.tf("ui.unit_enroute", args!(n = minutes(app, eta))))
                    }
                    _ => format!("{} {}", unit.code(), app.t("ui.unit_onscene")),
                }
            })
            .collect();
        lines.push(Line::from(colored(app.tf("ui.assigned", args!(list = list.join(", "))), Color::LightGreen)));
    }
    if c.is_open() {
        if app.chosen.is_empty() {
            lines.push(Line::from(dim(app.t("ui.pick_hint"))));
        } else {
            let list: Vec<String> = app.chosen.iter().filter_map(|&u| s.units.get(u)).map(|u| u.code()).collect();
            lines.push(Line::from(bold(app.tf("ui.chosen", args!(list = list.join(", "))), ACCENT)));
        }
    }
    let p = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(panel(title, if c.severity == 3 && c.is_open() { Color::LightRed } else { Color::Gray }));
    f.render_widget(p, r);
}

fn log_text(app: &App, s: &Shift, e: &Event) -> (String, Color) {
    let code = |u: usize| s.units.get(u).map(|u| u.code()).unwrap_or_default();
    let district = |c: u32| s.call(c).map(|c| app.t(c.district.key())).unwrap_or("");
    match e {
        Event::ShiftStart => (app.t("log.shift_start").into(), Color::Gray),
        Event::NewCall { call } => {
            (app.tf("log.new_call", args!(call = call, district = district(*call))), Color::White)
        }
        Event::Dispatched { call, unit } => {
            (app.tf("log.dispatched", args!(unit = code(*unit), call = call)), Color::Gray)
        }
        Event::Arrived { call, unit } => (app.tf("log.arrived", args!(unit = code(*unit), call = call)), Color::Gray),
        Event::WrongUnit { call, unit } => {
            (app.tf("log.wrong_unit", args!(unit = code(*unit), call = call)), Color::LightRed)
        }
        Event::NeedMore { call, unit } => (app.tf("log.need_more", args!(unit = code(*unit), call = call)), ACCENT),
        Event::PrankFound { call, unit } => {
            (app.tf("log.prank_found", args!(unit = code(*unit), call = call)), Color::Yellow)
        }
        Event::NotNeeded { call, unit } => {
            (app.tf("log.not_needed", args!(unit = code(*unit), call = call)), Color::DarkGray)
        }
        Event::Resolved { call } => (app.tf("log.resolved", args!(call = call)), Color::LightGreen),
        Event::Escalated { call, severity } => {
            (app.tf("log.escalated", args!(call = call, sev = severity)), Color::Yellow)
        }
        Event::Failed { call } => (app.tf("log.failed", args!(call = call)), Color::Red),
        Event::PrankDismissed { call } | Event::Dismissed { call } => {
            (app.tf("log.dismissed", args!(call = call)), Color::Gray)
        }
        Event::Callback { call } => (app.tf("log.callback", args!(call = call)), Color::LightRed),
        Event::Expired { call } => (app.tf("log.expired", args!(call = call)), Color::DarkGray),
        Event::Flagged { call, on } => {
            let key = if *on { "log.flag_on" } else { "log.flag_off" };
            (app.tf(key, args!(call = call)), Color::LightCyan)
        }
        Event::CoffeeStart => (app.t("log.coffee_start").into(), Color::LightMagenta),
        Event::CoffeeEnd => (app.t("log.coffee_end").into(), Color::LightMagenta),
        Event::ZuhraTookCall => (app.t("log.zuhra_took").into(), Color::Magenta),
        Event::Recalled { unit } => (app.tf("log.recalled", args!(unit = code(*unit))), Color::Gray),
        Event::AkmalGrumble => (app.t("log.akmal").into(), Color::LightRed),
        Event::DilshodRadio => (app.t("log.dilshod").into(), Color::LightCyan),
        Event::ArsonistCaught { call } => (app.tf("log.caught", args!(call = call)), Color::LightGreen),
        Event::ArsonistEscaped { call } => (app.tf("log.escaped", args!(call = call)), Color::LightRed),
        Event::HandedOver { count } => (app.tf("log.handed_over", args!(count = count)), Color::Gray),
    }
}

fn draw_log(f: &mut Frame, app: &App, s: &Shift, r: Rect) {
    let inner_h = r.height.saturating_sub(2) as usize;
    let inner_w = r.width.saturating_sub(2) as usize;
    // Wrap long lines ourselves so the newest entry is always visible at the bottom.
    let mut rows: Vec<Line> = Vec::new();
    for entry in s.log.iter().rev() {
        if rows.len() >= inner_h {
            break;
        }
        if matches!(entry.event, Event::Dispatched { .. } | Event::Arrived { .. }) && rows.len() > inner_h / 2 {
            continue;
        }
        let m = 22 * 60 + entry.tick;
        let stamp = format!("{:02}:{:02} ", (m / 60) % 24, m % 60);
        let (text, color) = log_text(app, s, &entry.event);
        let width = inner_w.saturating_sub(6).max(10);
        let chunks = wrap_chars(&text, width);
        if rows.len() + chunks.len() > inner_h {
            break;
        }
        for (i, chunk) in chunks.iter().enumerate().rev() {
            let prefix = if i == 0 { dim(stamp.clone()) } else { plain("      ") };
            rows.push(Line::from(vec![prefix, colored(chunk.clone(), color)]));
        }
    }
    rows.truncate(inner_h);
    rows.reverse();
    f.render_widget(Paragraph::new(rows).block(panel(app.t("ui.log").to_string(), Color::Gray)), r);
}

/// Greedy word wrap by character count.
fn wrap_chars(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        let need = cur.chars().count() + word.chars().count() + usize::from(!cur.is_empty());
        if need > width && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() || out.is_empty() {
        out.push(cur);
    }
    out
}

fn draw_footer(f: &mut Frame, app: &App, r: Rect) {
    let line = match &app.message {
        Some((m, _)) => Line::from(bold(format!(" {m}"), Color::Black)).style(Style::new().bg(Color::Yellow)),
        None => Line::from(dim(format!(" {}", app.t("ui.footer")))),
    };
    f.render_widget(Paragraph::new(line), r);
}

// ------------------------------------------------------------------ reports

fn draw_report(f: &mut Frame, app: &App, area: Rect, page: Page) {
    let (Some(week), Some(rec)) = (&app.week, &app.record) else { return };
    let box_area = centered(area, 88, 24);
    let night = night_name(app, rec.night);
    let cont = Line::from(bold(app.t("report.continue"), Color::LightGreen));
    match page {
        Page::Grade => {
            let st = &rec.stats;
            let sc = &rec.score;
            let resp = st.response_ticks.checked_div(st.response_count).unwrap_or(0);
            let row = |label: &str, value: String| {
                Line::from(vec![plain(format!("  {label:<34}")), bold(value, Color::White)])
            };
            let gc = grade_color(rec.grade);
            let lines = vec![
                Line::from(plain(
                    app.tf("report.calls", args!(calls = st.calls, real = st.real_calls, pranks = st.pranks)),
                )),
                Line::from(colored(app.tf("report.resolved", args!(n = st.resolved)), Color::LightGreen)),
                Line::from(colored(
                    app.tf("report.failed", args!(n = st.failed, lives = st.lives_lost)),
                    if st.failed > 0 { Color::LightRed } else { Color::Gray },
                )),
                Line::from(plain(app.tf("report.handed", args!(n = st.handed_over)))),
                Line::from(plain(app.tf("report.pranks", args!(ok = st.prank_dismissed, total = st.pranks)))),
                Line::from(plain(app.tf("report.wrong", args!(n = st.wrong_units + st.prank_dispatched)))),
                Line::from(plain(app.tf("report.response", args!(n = resp)))),
                Line::from(""),
                row(app.t("report.score_outcome"), format!("{:>3} / 50", sc.outcome)),
                row(app.t("report.score_response"), format!("{:>3} / 30", sc.response)),
                row(app.t("report.score_pranks"), format!("{:>3} / 20", sc.pranks)),
                row(app.t("report.score_penalty"), format!("{:>3}", -(sc.penalty as i32))),
                row(app.t("report.score_total"), format!("{:>3} / 100", sc.total)),
                Line::from(""),
                Line::from(vec![
                    plain(format!("  {}: ", app.t("report.grade"))),
                    Span::styled(
                        format!(" {} ", rec.grade.letter()),
                        Style::new().fg(Color::Black).bg(gc).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(dim(
                    app.tf("report.meters", args!(trust = rec.trust_end / 100, stress = rec.stress_end / 100))
                )),
                Line::from(""),
                cont,
            ];
            let title = app.tf("report.title", args!(night = night));
            f.render_widget(Clear, box_area);
            f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }).block(panel(title, gc)), box_area);
        }
        Page::Zuhra => {
            let mut lines = vec![Line::from(plain(app.t(&format!("zuhra.{}", rec.grade.letter())))), Line::from("")];
            if rec.stats.wrong_units >= 2 {
                lines.push(Line::from(colored(app.t("zuhra.akmal"), Color::LightRed)));
                lines.push(Line::from(""));
            }
            if rec.night == 4 {
                lines.push(Line::from(colored(app.t("zuhra.farewell"), Color::LightMagenta)));
                lines.push(Line::from(""));
            }
            lines.push(cont);
            let p = Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .block(panel(app.t("zuhra.title").to_string(), Color::LightMagenta));
            f.render_widget(p, centered(area, 80, 14));
        }
        Page::Inspector => {
            let mut lines = Vec::new();
            let st = &rec.stats;
            if rec.night < 4 {
                let flagged = st.flags_correct + st.flags_false;
                if flagged == 0 {
                    lines.push(Line::from(plain(app.t("insp.none"))));
                } else {
                    lines.push(Line::from(plain(app.tf(
                        "insp.review",
                        args!(flagged = flagged, correct = st.flags_correct, wrong = st.flags_false),
                    ))));
                }
                let missed = st.arson_calls.saturating_sub(st.flags_correct);
                if missed > 0 {
                    lines.push(Line::from(colored(app.tf("insp.missed", args!(missed = missed)), Color::Yellow)));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(plain(app.tf(
                    "insp.total",
                    args!(correct = week.flags_correct, wrong = week.flags_false, need = crate::sim::week::TRAP_FLAGS),
                ))));
                if week.flags_false > week.flags_correct {
                    lines.push(Line::from(colored(app.t("insp.too_many_false"), Color::LightRed)));
                }
                if week.trap_set {
                    lines.push(Line::from(bold(app.t("insp.ready"), Color::LightGreen)));
                } else if rec.night == 3 {
                    lines.push(Line::from(bold(app.t("insp.not_ready_final"), Color::LightRed)));
                }
            } else {
                let key = if week.arsonist_caught {
                    "insp.caught"
                } else if week.trap_set {
                    "insp.escaped"
                } else {
                    "insp.notrap_end"
                };
                let c = if week.arsonist_caught { Color::LightGreen } else { Color::LightRed };
                lines.push(Line::from(bold(app.t(key), c)));
            }
            lines.push(Line::from(""));
            lines.push(cont);
            let p = Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .block(panel(app.t("insp.title").to_string(), Color::LightCyan));
            f.render_widget(p, centered(area, 84, 16));
        }
        Page::Upgrade => {
            let mut lines = vec![Line::from("")];
            for (i, u) in app.upgrade_opts.iter().enumerate() {
                lines
                    .push(Line::from(vec![bold(format!("  {}) ", i + 1), ACCENT), bold(app.t(u.key()), Color::White)]));
                lines.push(Line::from(colored(format!("     {}", app.t(&format!("{}.desc", u.key()))), Color::Gray)));
                lines.push(Line::from(""));
            }
            lines.push(Line::from(bold(app.t("upgrade.choose"), Color::LightGreen)));
            let p = Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .block(panel(app.t("upgrade.title").to_string(), ACCENT));
            f.render_widget(p, centered(area, 84, 16));
        }
    }
}

fn grade_color(g: Grade) -> Color {
    match g {
        Grade::S => Color::LightMagenta,
        Grade::A => Color::LightGreen,
        Grade::B => Color::Green,
        Grade::C => Color::Yellow,
        Grade::D => Color::LightRed,
    }
}

fn draw_ending(f: &mut Frame, app: &App, area: Rect) {
    let Some(week) = &app.week else { return };
    let Some(e) = week.ending else { return };
    let color = if e.is_loss() { Color::LightRed } else { Color::LightYellow };
    let mut lines = vec![
        Line::from(bold(app.t(e.key()), color)).alignment(Alignment::Center),
        Line::from(""),
        Line::from(plain(app.t(&format!("{}.text", e.key())))),
        Line::from(""),
        Line::from(bold(app.t("ending.week"), Color::White)),
    ];
    let mut grades = Vec::new();
    for r in &week.records {
        grades.push(plain(format!("{}: ", night_name(app, r.night))));
        grades.push(bold(r.grade.letter(), grade_color(r.grade)));
        grades.push(plain("   "));
    }
    lines.push(Line::from(grades));
    let caught = if week.arsonist_caught { "ending.caught_yes" } else { "ending.caught_no" };
    lines.push(Line::from(colored(app.t(caught), Color::LightCyan)));
    lines.push(Line::from(dim(app.tf("ending.trust", args!(n = week.trust / 100)))));
    lines.push(Line::from(""));
    lines.push(Line::from(bold(app.t("ending.back"), Color::LightGreen)));
    let title = app.tf("ending.title", args!(code = e.code()));
    let p = Paragraph::new(lines).wrap(Wrap { trim: true }).block(panel(title, color));
    f.render_widget(p, centered(area, 84, 20));
}

// ------------------------------------------------------------------ overlays

fn draw_overlay(f: &mut Frame, app: &App, area: Rect, o: Overlay) {
    match o {
        Overlay::Help => {
            let mut lines: Vec<Line> = app.t("help.keys").lines().map(|l| Line::from(plain(l.to_string()))).collect();
            lines.push(Line::from(""));
            lines.extend(app.t("help.legend").lines().map(|l| Line::from(colored(l.to_string(), Color::Gray))));
            lines.push(Line::from(""));
            lines.push(Line::from(dim(app.t("help.close"))));
            let r = centered(area, 90, 26);
            f.render_widget(Clear, r);
            f.render_widget(
                Paragraph::new(lines).wrap(Wrap { trim: false }).block(panel(app.t("help.title").to_string(), ACCENT)),
                r,
            );
        }
        Overlay::QuitConfirm => {
            let r = centered(area, 64, 7);
            f.render_widget(Clear, r);
            let p = Paragraph::new(vec![
                Line::from(plain(app.t("quit.text"))),
                Line::from(""),
                Line::from(bold(app.t("quit.keys"), ACCENT)),
            ])
            .wrap(Wrap { trim: true })
            .block(panel(app.t("quit.title").to_string(), Color::LightRed));
            f.render_widget(p, r);
        }
        Overlay::Tip(tip) => {
            let r = centered(area, 76, 11);
            f.render_widget(Clear, r);
            let p = Paragraph::new(vec![
                Line::from(plain(app.t(tip.key()))),
                Line::from(""),
                Line::from(bold(app.t("tip.continue"), Color::LightGreen)),
            ])
            .wrap(Wrap { trim: true })
            .block(panel(app.t("tip.title").to_string(), Color::LightMagenta));
            f.render_widget(p, r);
        }
    }
}
