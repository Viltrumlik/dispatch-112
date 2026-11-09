//! Breadth-first pathfinding on the road grid (all road cells cost 1).

use super::map::{CityMap, Pos, MAP_H, MAP_W};
use std::collections::VecDeque;

/// Shortest road path from `from` to `to`, excluding `from` and including `to`.
/// `blocked` cells are impassable (e.g. a flooded bridge). Returns `None` if unreachable.
pub fn find_path(map: &CityMap, from: Pos, to: Pos, blocked: &[Pos]) -> Option<Vec<Pos>> {
    if from == to {
        return Some(Vec::new());
    }
    if !map.in_bounds(to) || !map.in_bounds(from) {
        return None;
    }
    let w = MAP_W as usize;
    let idx = |p: Pos| p.y as usize * w + p.x as usize;
    let mut prev: Vec<u32> = vec![u32::MAX; w * MAP_H as usize];
    let start = idx(from);
    prev[start] = start as u32;
    let mut q = VecDeque::new();
    q.push_back(from);
    let passable = |p: Pos| map.in_bounds(p) && map.is_passable(p) && !blocked.contains(&p);
    if !passable(to) {
        return None;
    }
    while let Some(cur) = q.pop_front() {
        if cur == to {
            break;
        }
        for n in CityMap::neighbors(cur) {
            if !passable(n) {
                continue;
            }
            let ni = idx(n);
            if prev[ni] != u32::MAX {
                continue;
            }
            prev[ni] = idx(cur) as u32;
            q.push_back(n);
        }
    }
    let goal = idx(to);
    if prev[goal] == u32::MAX {
        return None;
    }
    let mut path = Vec::new();
    let mut cur = goal;
    while cur != start {
        path.push(Pos::new((cur % w) as i32, (cur / w) as i32));
        cur = prev[cur] as usize;
    }
    path.reverse();
    Some(path)
}

/// Single-source road distances from `src` to every cell (`u32::MAX` = unreachable).
/// Roads are undirected, so this also gives the distance from any cell to `src`.
pub fn distances_from(map: &CityMap, src: Pos, blocked: &[Pos]) -> Vec<u32> {
    let w = MAP_W as usize;
    let mut dist = vec![u32::MAX; w * MAP_H as usize];
    if !map.in_bounds(src) {
        return dist;
    }
    let idx = |p: Pos| p.y as usize * w + p.x as usize;
    dist[idx(src)] = 0;
    let mut q = VecDeque::new();
    q.push_back(src);
    while let Some(cur) = q.pop_front() {
        let d = dist[idx(cur)];
        for n in CityMap::neighbors(cur) {
            if !map.in_bounds(n) || !map.is_passable(n) || blocked.contains(&n) {
                continue;
            }
            let ni = idx(n);
            if dist[ni] == u32::MAX {
                dist[ni] = d + 1;
                q.push_back(n);
            }
        }
    }
    dist
}

/// Index of a position in the vectors returned by [`distances_from`].
pub fn cell_index(p: Pos) -> usize {
    p.y as usize * MAP_W as usize + p.x as usize
}

/// Road distance in cells, or `None` if unreachable.
pub fn distance(map: &CityMap, from: Pos, to: Pos, blocked: &[Pos]) -> Option<usize> {
    find_path(map, from, to, blocked).map(|p| p.len())
}
