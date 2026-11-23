use crate::ruleset::Ruleset;
use crate::types::{Elevation, TileId, UnitId, INVALID_CITY, INVALID_TILE, INVALID_UNIT};
use crate::world::World;
use std::collections::BinaryHeap;
use std::cmp::Reverse;

#[derive(Debug, Clone)]
pub struct ReachableSet {
    pub tiles: Vec<(TileId, u8)>, // (tile, cost_halves)
    pub parent: Vec<(TileId, TileId)>,
}

impl ReachableSet {
    pub fn contains(&self, tile: TileId) -> Option<u8> {
        self.tiles.iter().find(|&&(t, _)| t == tile).map(|&(_, c)| c)
    }

    pub fn reconstruct_path(&self, start: TileId, goal: TileId) -> Vec<TileId> {
        let mut path = Vec::new();
        let mut cur = goal;
        for _ in 0..32 {
            if cur == start {
                break;
            }
            path.push(cur);
            if let Some(&(_, p)) = self.parent.iter().find(|&&(t, _)| t == cur) {
                cur = p;
            } else {
                return Vec::new();
            }
        }
        path.reverse();
        path
    }
}

/// Compute all tiles reachable by `unit` within its remaining movement points this turn.
pub fn compute_reachable_tiles(world: &World, rules: &Ruleset, unit: UnitId) -> ReachableSet {
    if !world.units.is_alive(unit) {
        return ReachableSet {
            tiles: Vec::new(),
            parent: Vec::new(),
        };
    }
    let ui = unit as usize;
    let budget = world.units.moves_halves[ui];
    if budget == 0 {
        return ReachableSet {
            tiles: Vec::new(),
            parent: Vec::new(),
        };
    }
    let start = world.units.pos[ui];
    let civ = world.units.civ[ui];
    let def = &rules.units[world.units.kind[ui] as usize];
    let is_naval = def.role.is_naval();
    let is_civ = def.role.is_civilian();
    let occ = if is_civ {
        &world.units.tile_civ_unit
    } else {
        &world.units.tile_mil_unit
    };

    let mut best_cost: Vec<(TileId, u8)> = vec![(start, 0)];
    let mut parent: Vec<(TileId, TileId)> = Vec::new();
    let mut heap: BinaryHeap<Reverse<(u8, TileId)>> = BinaryHeap::new();
    heap.push(Reverse((0, start)));

    while let Some(Reverse((cost, u))) = heap.pop() {
        let cur_best = best_cost
            .iter()
            .find(|&&(t, _)| t == u)
            .map_or(u8::MAX, |&(_, c)| c);
        if cost > cur_best {
            continue;
        }
        for &nb in world.map.neighbors_of(u) {
            if nb == INVALID_TILE {
                continue;
            }
            let ni = nb as usize;
            let step_cost = if is_naval {
                if world.map.terrain[ni].is_water() {
                    2u8
                } else {
                    continue;
                }
            } else {
                let c = world.map.land_enter_cost_halves(nb, world.map.road[u as usize]);
                if c == u8::MAX {
                    continue;
                }
                c
            };
            // Cannot enter enemy city via plain movement (use attack instead)
            let cid = world.cities.tile_city[ni];
            if cid != INVALID_CITY && world.cities.civ[cid as usize] != civ {
                continue;
            }
            // Cannot pass through enemy combat units or end on friendly same-layer units
            let mil = world.units.tile_mil_unit[ni];
            if mil != INVALID_UNIT && world.units.civ[mil as usize] != civ {
                continue;
            }
            if occ[ni] != INVALID_UNIT {
                continue;
            }
            // Standard Civ rule: from start with >0 moves, entering any adjacent passable tile is allowed
            let new_cost = if u == start && cost == 0 {
                step_cost.min(budget)
            } else {
                cost.saturating_add(step_cost)
            };
            if new_cost > budget {
                continue;
            }
            let prev = best_cost.iter().position(|&(t, _)| t == nb);
            let improved = match prev {
                Some(idx) => {
                    if new_cost < best_cost[idx].1 {
                        best_cost[idx].1 = new_cost;
                        true
                    } else {
                        false
                    }
                }
                None => {
                    best_cost.push((nb, new_cost));
                    true
                }
            };
            if improved {
                if let Some(p) = parent.iter_mut().find(|(t, _)| *t == nb) {
                    p.1 = u;
                } else {
                    parent.push((nb, u));
                }
                if new_cost < budget {
                    heap.push(Reverse((new_cost, nb)));
                }
            }
        }
    }

    best_cost.retain(|&(t, _)| t != start);
    ReachableSet {
        tiles: best_cost,
        parent,
    }
}

/// Move `unit` step-by-step along `path` computed by `ReachableSet::reconstruct_path`.
pub fn move_unit_along_path(world: &mut World, rules: &Ruleset, unit: UnitId, path: &[TileId]) -> bool {
    if !world.units.is_alive(unit) || path.is_empty() {
        return false;
    }
    let ui = unit as usize;
    let is_naval = rules.units[world.units.kind[ui] as usize].role.is_naval();
    let mut moved = false;
    for &step in path {
        if !world.units.is_alive(unit) || world.units.moves_halves[unit as usize] == 0 {
            break;
        }
        let pos = world.units.pos[unit as usize];
        let cost = if is_naval {
            2u8
        } else {
            let c = world.map.land_enter_cost_halves(step, world.map.road[pos as usize]);
            if c == u8::MAX {
                break;
            }
            c
        };
        let spend = cost.min(world.units.moves_halves[unit as usize]);
        if world.move_unit_step(rules, unit, step, spend) {
            moved = true;
        } else {
            break;
        }
    }
    moved
}

/// List all enemy unit tiles and enemy city tiles currently attackable by `unit`.
pub fn list_attackable_tiles(world: &World, rules: &Ruleset, unit: UnitId) -> Vec<TileId> {
    let mut out = Vec::new();
    if !world.units.is_alive(unit) {
        return out;
    }
    let ui = unit as usize;
    if world.units.has_attacked[ui] || world.units.moves_halves[ui] == 0 {
        return out;
    }
    let def = &rules.units[world.units.kind[ui] as usize];
    if def.role.is_civilian() {
        return out;
    }
    let civ = world.units.civ[ui];
    let pos = world.units.pos[ui];
    let range = def.range.max(1);
    let mut ring = Vec::with_capacity(37);
    world.map.collect_tiles_in_radius(pos, range, &mut ring);

    for &t in &ring {
        if t == pos {
            continue;
        }
        let dist = world.map.distance(pos, t);
        if dist > 1 && !world.map.has_los(pos, t) {
            continue;
        }
        let eu = world.units.tile_mil_unit[t as usize];
        let has_enemy_unit = eu != INVALID_UNIT && world.diplo.is_at_war(civ, world.units.civ[eu as usize]);
        let ec = world.cities.tile_city[t as usize];
        let has_enemy_city = ec != INVALID_CITY && world.diplo.is_at_war(civ, world.cities.civ[ec as usize]);
        if has_enemy_unit || has_enemy_city {
            out.push(t);
        }
    }
    out
}

/// Compute human worker build-improvement action on its current tile.
pub fn human_worker_improve(world: &mut World, unit: UnitId) -> bool {
    if !world.units.is_alive(unit) {
        return false;
    }
    let ui = unit as usize;
    if world.units.moves_halves[ui] == 0 {
        return false;
    }
    let pos = world.units.pos[ui];
    let pi = pos as usize;
    let civ = world.units.civ[ui];
    if world.map.owner[pi] != civ || world.cities.tile_city[pi] != INVALID_CITY {
        return false;
    }
    if world.map.elevation[pi] == Elevation::Mountain || !world.map.terrain[pi].is_land() {
        return false;
    }
    let imp = crate::economy::best_improvement_for_tile(&world.map, pos);
    if imp == crate::types::Improvement::None || world.map.improvement[pi] == imp {
        return false;
    }
    world.map.improvement[pi] = imp;
    world.units.moves_halves[ui] = 0;
    true
}
