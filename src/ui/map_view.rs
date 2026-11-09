//! ASCII/box-drawing rendering of the city map with calls and units.

use crate::sim::map::{MAP_H, MAP_W};
use crate::sim::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn kind_color(k: UnitKind) -> Color {
    match k {
        UnitKind::Ambulance => Color::LightGreen,
        UnitKind::Fire => Color::LightRed,
        UnitKind::Police => Color::LightCyan,
    }
}

pub fn severity_color(sev: u8) -> Color {
    match sev {
        3 => Color::Red,
        2 => Color::LightYellow,
        _ => Color::Yellow,
    }
}

fn district_tint(d: District) -> Color {
    match d {
        District::OldTown => Color::Rgb(120, 100, 70),
        District::Bazaar => Color::Rgb(120, 80, 110),
        District::NewBlocks => Color::Rgb(70, 100, 120),
        District::Railway => Color::Rgb(95, 95, 95),
        District::Industrial => Color::Rgb(125, 110, 50),
        District::Stadium => Color::Rgb(70, 115, 80),
    }
}

fn road_glyph(map: &CityMap, p: Pos) -> char {
    let conn = |q: Pos| map.is_passable(q);
    let n = conn(Pos::new(p.x, p.y - 1));
    let e = conn(Pos::new(p.x + 1, p.y));
    let s = conn(Pos::new(p.x, p.y + 1));
    let w = conn(Pos::new(p.x - 1, p.y));
    match (n, e, s, w) {
        (true, true, true, true) => '┼',
        (true, true, true, false) => '├',
        (true, false, true, true) => '┤',
        (false, true, true, true) => '┬',
        (true, true, false, true) => '┴',
        (true, true, false, false) => '└',
        (false, true, true, false) => '┌',
        (false, false, true, true) => '┐',
        (true, false, false, true) => '┘',
        (true, false, true, false) | (true, false, false, false) | (false, false, true, false) => '│',
        _ => '─',
    }
}

fn base_cell(map: &CityMap, p: Pos, shift: &Shift) -> (char, Style) {
    let dim = Style::new().fg(Color::DarkGray);
    match map.tile(p) {
        Tile::Road => (road_glyph(map, p), Style::new().fg(Color::Gray)),
        Tile::Bridge if shift.blocked.contains(&p) => ('×', Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Tile::Bridge => ('║', Style::new().fg(Color::Rgb(180, 140, 90))),
        Tile::Water => ('≈', Style::new().fg(if shift.is_storm() { Color::LightBlue } else { Color::Blue })),
        Tile::Block => ('·', Style::new().fg(district_tint(map.district(p)))),
        Tile::Park => ('♣', Style::new().fg(Color::Green)),
        Tile::Warehouse => ('▪', Style::new().fg(Color::Rgb(150, 125, 60))),
        Tile::Stadium => ('░', Style::new().fg(Color::Rgb(90, 160, 100))),
        Tile::Rail => ('═', dim),
        Tile::Hospital => ('H', Style::new().fg(Color::White).bg(Color::Green).add_modifier(Modifier::BOLD)),
        Tile::FireStation => ('F', Style::new().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD)),
        Tile::PoliceStation => ('P', Style::new().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)),
    }
}

/// Map lines (MAP_H rows of MAP_W cells).
pub fn map_lines(shift: &Shift, selected: Option<u32>, blink: bool) -> Vec<Line<'static>> {
    let map = CityMap::get();
    let w = MAP_W as usize;
    let mut cells: Vec<(char, Style)> = Vec::with_capacity(w * MAP_H as usize);
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            cells.push(base_cell(map, Pos::new(x, y), shift));
        }
    }
    let idx = |p: Pos| p.y as usize * w + p.x as usize;
    if shift.params.upgrades.has(UpgradeKind::FieldPost) {
        cells[idx(map.field_post)] = ('+', Style::new().fg(Color::White).bg(Color::Green).add_modifier(Modifier::BOLD));
    }
    // Calls: open ones as severity-coloured '!', the selected one highlighted.
    for c in shift.open_calls() {
        let mut st = Style::new().fg(severity_color(c.severity)).add_modifier(Modifier::BOLD);
        if c.severity == 3 && blink {
            st = st.add_modifier(Modifier::REVERSED);
        }
        let ch = if c.flagged { '⚑' } else { '!' };
        if Some(c.id) == selected {
            st = Style::new().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD);
        }
        cells[idx(c.pos)] = (ch, st);
    }
    // Units: their key number in the kind's colour.
    for u in &shift.units {
        let ch = char::from_digit((u.id + 1) as u32, 10).unwrap_or('#');
        let mut st = Style::new().fg(kind_color(u.kind)).add_modifier(Modifier::BOLD);
        if matches!(u.state, UnitState::OnScene { .. }) {
            st = st.add_modifier(Modifier::UNDERLINED);
        }
        if u.state == UnitState::AtBase {
            continue; // parked units are summarised by the station glyph
        }
        cells[idx(u.pos)] = (ch, st);
    }
    cells
        .chunks(w)
        .map(|row| Line::from(row.iter().map(|(c, s)| Span::styled(c.to_string(), *s)).collect::<Vec<_>>()))
        .collect()
}
