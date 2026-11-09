//! The city of Yangiobod: a 58x18 character grid with roads, a river with four
//! bridges, six districts and three stations.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const MAP_W: i32 = 58;
pub const MAP_H: i32 = 18;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32) -> Self {
        Pos { x, y }
    }
    pub fn manhattan(self, o: Pos) -> i32 {
        (self.x - o.x).abs() + (self.y - o.y).abs()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Road,
    Bridge,
    Water,
    Block,
    Park,
    Warehouse,
    Stadium,
    Rail,
    Hospital,
    FireStation,
    PoliceStation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum District {
    OldTown,
    Bazaar,
    NewBlocks,
    Railway,
    Industrial,
    Stadium,
}

impl District {
    pub const ALL: [District; 6] = [
        District::OldTown,
        District::Bazaar,
        District::NewBlocks,
        District::Railway,
        District::Industrial,
        District::Stadium,
    ];

    pub fn key(self) -> &'static str {
        match self {
            District::OldTown => "district.old_town",
            District::Bazaar => "district.bazaar",
            District::NewBlocks => "district.new_blocks",
            District::Railway => "district.railway",
            District::Industrial => "district.industrial",
            District::Stadium => "district.stadium",
        }
    }

    pub fn index(self) -> usize {
        District::ALL.iter().position(|d| *d == self).unwrap_or(0)
    }
}

pub struct CityMap {
    tiles: Vec<Tile>,
    pub hospital: Pos,
    pub fire_station: Pos,
    pub police_station: Pos,
    /// Optional ambulance post near the railway station (unlocked by an upgrade).
    pub field_post: Pos,
    /// The bridge that floods during Wednesday's storm.
    pub flood_bridge: Pos,
    /// Fixed addresses of the recurring callers.
    pub karim_home: Pos,
    pub botir_bakery: Pos,
}

// Horizontal road segments: (y, x0, x1) inclusive.
const H_ROADS: &[(i32, i32, i32)] =
    &[(0, 0, 57), (4, 0, 57), (7, 0, 57), (9, 0, 57), (13, 0, 49), (17, 0, 57), (2, 0, 7), (11, 21, 35), (15, 42, 49)];

// Vertical road segments: (x, y0, y1) inclusive. Row 8 is the river; only
// the four bridge columns cross it.
const V_ROADS: &[(i32, i32, i32)] = &[
    (0, 0, 7),
    (0, 9, 17),
    (3, 0, 4),
    (7, 0, 17),
    (14, 4, 7),
    (14, 9, 17),
    (21, 0, 17),
    (28, 0, 7),
    (28, 9, 13),
    (31, 13, 17),
    (35, 0, 17),
    (42, 0, 7),
    (42, 9, 17),
    (49, 0, 17),
    (57, 0, 7),
    (57, 9, 17),
];

const RIVER_Y: i32 = 8;
const BRIDGES: &[i32] = &[7, 21, 35, 49];

// Special block rectangles (x0, y0, x1, y1) inclusive; they only replace plain blocks.
const PARKS: &[(i32, i32, i32, i32)] = &[(15, 1, 20, 3), (43, 10, 48, 12)];
const WAREHOUSES: &[(i32, i32, i32, i32)] = &[(22, 14, 34, 16), (36, 10, 41, 12), (36, 14, 41, 16), (29, 10, 34, 10)];
const STADIUM: &[(i32, i32, i32, i32)] = &[(50, 10, 56, 16)];
const RAILS: &[(i32, i32, i32, i32)] = &[(1, 15, 20, 15)];

impl CityMap {
    /// Shared immutable instance of the city.
    pub fn get() -> &'static CityMap {
        static MAP: OnceLock<CityMap> = OnceLock::new();
        MAP.get_or_init(CityMap::yangiobod)
    }

    pub fn yangiobod() -> CityMap {
        let mut tiles = vec![Tile::Block; (MAP_W * MAP_H) as usize];
        let idx = |x: i32, y: i32| (y * MAP_W + x) as usize;
        let rect = |tiles: &mut Vec<Tile>, r: &(i32, i32, i32, i32), t: Tile| {
            for y in r.1..=r.3 {
                for x in r.0..=r.2 {
                    let i = idx(x, y);
                    if tiles[i] == Tile::Block {
                        tiles[i] = t;
                    }
                }
            }
        };
        for x in 0..MAP_W {
            tiles[idx(x, RIVER_Y)] = Tile::Water;
        }
        for &(y, x0, x1) in H_ROADS {
            for x in x0..=x1 {
                if y != RIVER_Y {
                    tiles[idx(x, y)] = Tile::Road;
                }
            }
        }
        for &(x, y0, y1) in V_ROADS {
            for y in y0..=y1 {
                if y != RIVER_Y {
                    tiles[idx(x, y)] = Tile::Road;
                }
            }
        }
        for &x in BRIDGES {
            tiles[idx(x, RIVER_Y)] = Tile::Bridge;
        }
        for r in PARKS {
            rect(&mut tiles, r, Tile::Park);
        }
        for r in WAREHOUSES {
            rect(&mut tiles, r, Tile::Warehouse);
        }
        for r in STADIUM {
            rect(&mut tiles, r, Tile::Stadium);
        }
        for r in RAILS {
            rect(&mut tiles, r, Tile::Rail);
        }
        let hospital = Pos::new(14, 4);
        let fire_station = Pos::new(35, 9);
        let police_station = Pos::new(42, 4);
        tiles[idx(hospital.x, hospital.y)] = Tile::Hospital;
        tiles[idx(fire_station.x, fire_station.y)] = Tile::FireStation;
        tiles[idx(police_station.x, police_station.y)] = Tile::PoliceStation;
        CityMap {
            tiles,
            hospital,
            fire_station,
            police_station,
            field_post: Pos::new(7, 13),
            flood_bridge: Pos::new(21, RIVER_Y),
            karim_home: Pos::new(3, 2),
            botir_bakery: Pos::new(31, 4),
        }
    }

    pub fn in_bounds(&self, p: Pos) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < MAP_W && p.y < MAP_H
    }

    pub fn tile(&self, p: Pos) -> Tile {
        if !self.in_bounds(p) {
            return Tile::Water;
        }
        self.tiles[(p.y * MAP_W + p.x) as usize]
    }

    /// Cells vehicles can drive on.
    pub fn is_passable(&self, p: Pos) -> bool {
        matches!(self.tile(p), Tile::Road | Tile::Bridge | Tile::Hospital | Tile::FireStation | Tile::PoliceStation)
    }

    pub fn district(&self, p: Pos) -> District {
        let north = p.y <= RIVER_Y;
        match (p.x, north) {
            (x, true) if x <= 20 => District::OldTown,
            (x, true) if x <= 41 => District::Bazaar,
            (_, true) => District::NewBlocks,
            (x, false) if x <= 20 => District::Railway,
            (x, false) if x <= 41 => District::Industrial,
            _ => District::Stadium,
        }
    }

    /// Plain road cells (no bridges or stations) inside a district — valid call addresses.
    pub fn street_cells(&self, d: District) -> Vec<Pos> {
        let mut out = Vec::new();
        for y in 0..MAP_H {
            for x in 0..MAP_W {
                let p = Pos::new(x, y);
                if self.tile(p) == Tile::Road && self.district(p) == d {
                    out.push(p);
                }
            }
        }
        out
    }

    /// Four-neighbourhood in a fixed order (N, E, S, W) for deterministic search.
    pub fn neighbors(p: Pos) -> [Pos; 4] {
        [Pos::new(p.x, p.y - 1), Pos::new(p.x + 1, p.y), Pos::new(p.x, p.y + 1), Pos::new(p.x - 1, p.y)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stations_are_passable_and_in_expected_districts() {
        let m = CityMap::get();
        assert!(m.is_passable(m.hospital));
        assert!(m.is_passable(m.fire_station));
        assert!(m.is_passable(m.police_station));
        assert!(m.is_passable(m.field_post));
        assert_eq!(m.tile(m.flood_bridge), Tile::Bridge);
        assert_eq!(m.tile(m.karim_home), Tile::Road);
        assert_eq!(m.tile(m.botir_bakery), Tile::Road);
        assert_eq!(m.district(m.karim_home), District::OldTown);
        assert_eq!(m.district(m.botir_bakery), District::Bazaar);
    }

    #[test]
    fn every_district_has_streets() {
        let m = CityMap::get();
        for d in District::ALL {
            assert!(m.street_cells(d).len() >= 20, "{d:?} too small");
        }
    }
}
