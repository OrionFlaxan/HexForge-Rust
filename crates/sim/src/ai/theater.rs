use crate::ai::scratch::AiScratch;
use crate::types::{CivId, Elevation, TileId, INVALID_TILE, INVALID_UNIT};
use crate::world::World;

/// Precomputed O(V) Theater Flow-Field:
/// Any unit on tile `t` reads its optimal next step toward the theater objective in O(1) via `next_hop[t]`.
#[derive(Debug, Clone)]
pub struct TheaterField {
    pub next_hop: Vec<TileId>,
    pub dist: Vec<u16>,
}

impl TheaterField {
    #[inline]
    pub fn step_from(&self, tile: TileId) -> TileId {
        self.next_hop[tile as usize]
    }

    #[inline]
    pub fn dist_at(&self, tile: TileId) -> u16 {
        self.dist[tile as usize]
    }
}

/// Compute an O(V + E) multi-source Dial-bucket Dijkstra flow-field pulling land units toward `sources`.
/// `max_cost_halves` bounds the flood radius so huge-map searches stay sub-100us.
pub fn compute_land_flow_field(
    world: &World,
    civ: CivId,
    sources: &[TileId],
    max_cost_halves: u16,
    scratch: &mut AiScratch,
) -> Option<TheaterField> {
    if sources.is_empty() {
        return None;
    }
    let n = world.map.num_tiles;
    let gen = scratch.prepare(n);
    let mut next_hop = vec![INVALID_TILE; n];
    let mut out_dist = vec![u16::MAX; n];

    let mut remaining = 0usize;
    for &src in sources {
        let si = src as usize;
        if scratch.stamp[si] != gen {
            scratch.stamp[si] = gen;
            scratch.dist[si] = 0;
            out_dist[si] = 0;
            next_hop[si] = src;
            scratch.buckets[0].push(src);
            remaining += 1;
        }
    }

    let mut cur_cost: u16 = 0;
    while remaining > 0 && cur_cost <= max_cost_halves {
        let bidx = (cur_cost as usize) & 7;
        // Swap-pop or index-iterate current bucket; no elements with `cur_cost` are pushed into the same bucket
        let mut cursor = 0;
        while cursor < scratch.buckets[bidx].len() {
            let u = scratch.buckets[bidx][cursor];
            cursor += 1;
            remaining -= 1;

            let ui = u as usize;
            if scratch.dist[ui] < cur_cost {
                continue;
            }
            let road_u = world.map.road[ui];
            let base = ui * 6;
            for d in 0..6 {
                let nb = world.map.neighbors[base + d];
                if nb == INVALID_TILE {
                    continue;
                }
                let ni = nb as usize;
                // Only expand through passable land (or impassable target already seeded at cost 0)
                if !world.map.terrain[ni].is_land() || world.map.elevation[ni] == Elevation::Mountain {
                    continue;
                }
                // Cost for a unit standing on `nb` to enter `u`
                let step_cost = if cur_cost == 0 {
                    2u16
                } else {
                    let c = world.map.land_enter_cost_halves(u, world.map.road[ni]);
                    if c == u8::MAX {
                        continue;
                    }
                    c as u16
                };
                // Mild penalty for moving through non-war foreign territory (respects borders)
                let owner = world.map.owner[ni];
                let border_pen = if owner != civ && owner != crate::types::NO_CIV && !world.diplo.is_at_war(civ, owner) {
                    2u16
                } else {
                    0
                };
                let _ = road_u;
                let new_cost = cur_cost + step_cost + border_pen;
                if new_cost > max_cost_halves {
                    continue;
                }
                let old_cost = if scratch.stamp[ni] == gen {
                    scratch.dist[ni]
                } else {
                    u16::MAX
                };
                if new_cost < old_cost {
                    scratch.stamp[ni] = gen;
                    scratch.dist[ni] = new_cost;
                    out_dist[ni] = new_cost;
                    next_hop[ni] = u;
                    scratch.buckets[(new_cost as usize) & 7].push(nb);
                    remaining += 1;
                }
            }
        }
        scratch.buckets[bidx].clear();
        cur_cost += 1;
    }

    Some(TheaterField {
        next_hop,
        dist: out_dist,
    })
}

/// Compute an O(V) multi-source unweighted BFS threat field (`0..=255` hex steps from closest enemy).
pub fn compute_threat_field(world: &World, civ: CivId, scratch: &mut AiScratch) -> Vec<u8> {
    let n = world.map.num_tiles;
    let gen = scratch.prepare(n);
    let mut threat = vec![255u8; n];

    // Seed with all visible enemy combat units and enemy cities
    for u in 0..world.units.len() {
        if !world.units.alive[u] {
            continue;
        }
        let uciv = world.units.civ[u];
        if !world.diplo.is_at_war(civ, uciv) {
            continue;
        }
        let pos = world.units.pos[u];
        if world.vision.is_visible(civ, pos) || world.vision.is_explored(civ, pos) {
            let pi = pos as usize;
            if scratch.stamp[pi] != gen {
                scratch.stamp[pi] = gen;
                threat[pi] = 0;
                scratch.bfs_queue.push(pos);
            }
        }
    }
    for c in 0..world.cities.len() {
        if !world.cities.alive[c] {
            continue;
        }
        if world.diplo.is_at_war(civ, world.cities.civ[c]) {
            let pos = world.cities.pos[c];
            let pi = pos as usize;
            if scratch.stamp[pi] != gen {
                scratch.stamp[pi] = gen;
                threat[pi] = 0;
                scratch.bfs_queue.push(pos);
            }
        }
    }

    let mut qh = 0;
    while qh < scratch.bfs_queue.len() {
        let u = scratch.bfs_queue[qh];
        qh += 1;
        let d = threat[u as usize];
        if d >= 14 {
            continue;
        }
        let base = (u as usize) * 6;
        for dir in 0..6 {
            let nb = world.map.neighbors[base + dir];
            if nb == INVALID_TILE {
                continue;
            }
            let ni = nb as usize;
            if scratch.stamp[ni] != gen {
                scratch.stamp[ni] = gen;
                threat[ni] = d + 1;
                scratch.bfs_queue.push(nb);
            }
        }
    }
    threat
}

/// Fast local check for whether tile `t` has an enemy combat unit visible to `civ`.
#[inline]
pub fn enemy_unit_at(world: &World, civ: CivId, tile: TileId) -> Option<u32> {
    let u = world.units.tile_mil_unit[tile as usize];
    if u != INVALID_UNIT && world.diplo.is_at_war(civ, world.units.civ[u as usize]) {
        Some(u)
    } else {
        None
    }
}
