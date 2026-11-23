use crate::types::{
    CivId, Elevation, Feature, Improvement, Resource, Road, Terrain, TileId, WorkerOrder,
    INVALID_TILE, NO_CIV,
};
use serde::{Deserialize, Serialize};

/// Flat Structure-of-Arrays (SoA) Hex Map using pointy-top odd-r offset layout
/// with cylindrical horizontal wrapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexMapSoA {
    pub width: u16,
    pub height: u16,
    pub num_tiles: usize,
    /// Precomputed 6 neighbors per tile `[tile * 6 + dir]`, `INVALID_TILE` at map top/bottom edges.
    /// Directions: 0=E, 1=NE, 2=NW, 3=W, 4=SW, 5=SE.
    pub neighbors: Vec<TileId>,
    pub terrain: Vec<Terrain>,
    pub elevation: Vec<Elevation>,
    pub feature: Vec<Feature>,
    pub resource: Vec<Resource>,
    pub improvement: Vec<Improvement>,
    pub pillaged: Vec<bool>,
    pub work_order: Vec<Option<WorkerOrder>>,
    pub work_turns_left: Vec<u8>,
    pub road: Vec<Road>,
    /// Bit `d` set if river runs along edge `d` (0..6) of this tile.
    pub river_mask: Vec<u8>,
    /// Owning civilization (`NO_CIV` if unclaimed).
    pub owner: Vec<CivId>,
    /// Owning city (`INVALID_CITY` if not inside a city's borders).
    pub owner_city: Vec<u32>,
    /// Precomputed continent ID for land tiles (`0` = water).
    pub continent: Vec<u16>,
}

/// Odd-r neighbor deltas for even rows (`row % 2 == 0`): E, NE, NW, W, SW, SE
const EVEN_ROW_DELTAS: [(i32, i32); 6] = [(1, 0), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1)];
/// Odd-r neighbor deltas for odd rows (`row % 2 == 1`): E, NE, NW, W, SW, SE
const ODD_ROW_DELTAS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (0, 1), (1, 1)];

impl HexMapSoA {
    pub fn new(width: u16, height: u16) -> Self {
        let num_tiles = (width as usize) * (height as usize);
        let mut neighbors = vec![INVALID_TILE; num_tiles * 6];
        let w = width as i32;
        let h = height as i32;

        for r in 0..h {
            let deltas = if (r & 1) == 0 {
                &EVEN_ROW_DELTAS
            } else {
                &ODD_ROW_DELTAS
            };
            for c in 0..w {
                let idx = (r * w + c) as usize;
                for (d, &(dc, dr)) in deltas.iter().enumerate() {
                    let nr = r + dr;
                    if nr < 0 || nr >= h {
                        neighbors[idx * 6 + d] = INVALID_TILE;
                    } else {
                        let nc = (c + dc).rem_euclid(w);
                        neighbors[idx * 6 + d] = (nr * w + nc) as TileId;
                    }
                }
            }
        }

        Self {
            width,
            height,
            num_tiles,
            neighbors,
            terrain: vec![Terrain::Ocean; num_tiles],
            elevation: vec![Elevation::Flat; num_tiles],
            feature: vec![Feature::None; num_tiles],
            resource: vec![Resource::None; num_tiles],
            improvement: vec![Improvement::None; num_tiles],
            pillaged: vec![false; num_tiles],
            work_order: vec![None; num_tiles],
            work_turns_left: vec![0; num_tiles],
            road: vec![Road::None; num_tiles],
            river_mask: vec![0; num_tiles],
            owner: vec![NO_CIV; num_tiles],
            owner_city: vec![u32::MAX; num_tiles],
            continent: vec![0; num_tiles],
        }
    }

    #[inline]
    pub fn tile_id(&self, col: u16, row: u16) -> TileId {
        (row as u32) * (self.width as u32) + (col as u32)
    }

    #[inline]
    pub fn col_row(&self, tile: TileId) -> (u16, u16) {
        let w = self.width as u32;
        ((tile % w) as u16, (tile / w) as u16)
    }

    #[inline]
    pub fn neighbors_of(&self, tile: TileId) -> &[TileId] {
        let base = (tile as usize) * 6;
        &self.neighbors[base..base + 6]
    }

    /// Shortest wrapped hex distance between two tiles in O(1).
    #[inline]
    pub fn distance(&self, a: TileId, b: TileId) -> u16 {
        if a == b {
            return 0;
        }
        let (ac, ar) = self.col_row(a);
        let (bc, br) = self.col_row(b);
        let w = self.width as i32;
        let ar_i = ar as i32;
        let br_i = br as i32;

        // Evaluate direct, +width, and -width wrapped copies of b in cube coordinates
        let aq = (ac as i32) - (ar_i - (ar_i & 1)) / 2;
        let az = ar_i;
        let ay = -aq - az;

        let mut best = i32::MAX;
        for shift in [-w, 0, w] {
            let bcol = (bc as i32) + shift;
            let bq = bcol - (br_i - (br_i & 1)) / 2;
            let bz = br_i;
            let by = -bq - bz;
            let d = (aq - bq).abs().max((ay - by).abs()).max((az - bz).abs());
            if d < best {
                best = d;
            }
        }
        best as u16
    }

    /// Convert tile `(col, row)` to world-space pixel center (radius = 1.0, pointy-top odd-r).
    #[inline]
    pub fn tile_world_pos(&self, tile: TileId) -> (f32, f32) {
        let (c, r) = self.col_row(tile);
        let sqrt3 = 1.7320508_f32;
        let x = sqrt3 * (c as f32 + if (r & 1) == 1 { 0.5 } else { 0.0 });
        let y = 1.5 * (r as f32);
        (x, y)
    }

    /// Total horizontal world period for cylindrical wrapping.
    #[inline]
    pub fn world_width(&self) -> f32 {
        1.7320508_f32 * (self.width as f32)
    }

    /// Total vertical world height.
    #[inline]
    pub fn world_height(&self) -> f32 {
        1.5 * (self.height as f32) + 0.5
    }

    /// Convert world `(wx, wy)` coordinate (with horizontal wrapping) to nearest `TileId`.
    pub fn world_pos_to_tile(&self, wx: f32, wy: f32) -> Option<TileId> {
        let ww = self.world_width();
        let x = wx.rem_euclid(ww);
        // Fractional axial (q, r) for pointy-top radius 1.0
        let q = 1.7320508_f32 / 3.0 * x - wy / 3.0;
        let r = (2.0 / 3.0) * wy;
        let s = -q - r;

        let mut rq = q.round();
        let mut rr = r.round();
        let rs = s.round();

        let dq = (rq - q).abs();
        let dr = (rr - r).abs();
        let ds = (rs - s).abs();

        if dq > dr && dq > ds {
            rq = -rr - rs;
        } else if dr > ds {
            rr = -rq - rs;
        }

        let row = rr as i32;
        if row < 0 || row >= self.height as i32 {
            return None;
        }
        let col = (rq as i32 + (row - (row & 1)) / 2).rem_euclid(self.width as i32);
        Some(self.tile_id(col as u16, row as u16))
    }

    /// Append all tiles within `<= radius` hex steps of `center` (including `center`) into `out`.
    pub fn collect_tiles_in_radius(&self, center: TileId, radius: u8, out: &mut Vec<TileId>) {
        out.clear();
        out.push(center);
        if radius == 0 {
            return;
        }
        // Small fixed BFS ring expansion (radius is typically 1..4 -> <= 61 tiles)
        let mut frontier_start = 0;
        for _ in 0..radius {
            let frontier_end = out.len();
            for i in frontier_start..frontier_end {
                let t = out[i];
                let base = (t as usize) * 6;
                for d in 0..6 {
                    let nb = self.neighbors[base + d];
                    if nb != INVALID_TILE && !out.contains(&nb) {
                        out.push(nb);
                    }
                }
            }
            frontier_start = frontier_end;
        }
    }

    /// Movement cost (in fixed-point halves, so 2 = 1.0 move point) to enter `tile` for land units.
    #[inline]
    pub fn land_enter_cost_halves(&self, tile: TileId, road_from: Road) -> u8 {
        let i = tile as usize;
        if self.terrain[i].is_water() || self.elevation[i] == Elevation::Mountain {
            return u8::MAX;
        }
        let road_to = self.road[i];
        if road_from == Road::Railroad && road_to == Road::Railroad {
            return 1; // 0.5 move (2x road speed)
        }
        if road_from != Road::None && road_to != Road::None {
            return 1; // 0.5 move on connected roads
        }
        let mut cost = 2u8; // 1.0 base
        if self.elevation[i] == Elevation::Hill {
            cost += 2;
        }
        if matches!(self.feature[i], Feature::Forest | Feature::Jungle | Feature::Marsh) {
            cost += 2;
        }
        cost.min(4) // max 2.0 moves for rough terrain
    }

    /// Defensive combat bonus (%) granted to a unit defending on `tile`.
    #[inline]
    pub fn defense_bonus_pct(&self, tile: TileId) -> i32 {
        let i = tile as usize;
        let mut bonus = 0;
        if self.elevation[i] == Elevation::Hill {
            bonus += 25;
        }
        match self.feature[i] {
            Feature::Forest | Feature::Jungle => bonus += 25,
            Feature::Marsh => bonus -= 15,
            Feature::None => {}
        }
        if self.improvement[i].is_fortification() && !self.pillaged[i] {
            bonus += 50;
        }
        bonus
    }

    #[inline]
    pub fn is_adjacent_to_water(&self, tile: TileId) -> bool {
        for &nb in self.neighbors_of(tile) {
            if nb != INVALID_TILE && self.terrain[nb as usize].is_water() {
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn is_adjacent_to_mountain(&self, tile: TileId) -> bool {
        for &nb in self.neighbors_of(tile) {
            if nb != INVALID_TILE && self.elevation[nb as usize] == Elevation::Mountain {
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn has_river(&self, tile: TileId) -> bool {
        self.river_mask[tile as usize] != 0
    }

    /// Exact line-of-sight check between `from` and `to` accounting for hills, forests, jungles, and mountains.
    pub fn has_los(&self, from: TileId, to: TileId) -> bool {
        let dist = self.distance(from, to);
        if dist <= 1 {
            return true;
        }
        let (fc, fr) = self.col_row(from);
        let (tc, tr) = self.col_row(to);
        let w = self.width as i32;
        let fr_i = fr as i32;
        let tr_i = tr as i32;

        // Pick closest wrapped column for `to`
        let fq = (fc as i32) - (fr_i - (fr_i & 1)) / 2;
        let fz = fr_i;
        let fy = -fq - fz;

        let mut best_shift = 0;
        let mut best_d = i32::MAX;
        for shift in [-w, 0, w] {
            let bq = (tc as i32 + shift) - (tr_i - (tr_i & 1)) / 2;
            let bz = tr_i;
            let by = -bq - bz;
            let d = (fq - bq).abs().max((fy - by).abs()).max((fz - bz).abs());
            if d < best_d {
                best_d = d;
                best_shift = shift;
            }
        }
        let tq = (tc as i32 + best_shift) - (tr_i - (tr_i & 1)) / 2;
        let tz = tr_i;
        let ty = -tq - tz;

        let observer_elev: i8 = match self.elevation[from as usize] {
            Elevation::Flat => 0,
            Elevation::Hill => 1,
            Elevation::Mountain => 2,
        };

        let steps = dist as i32;
        for s in 1..steps {
            let frac = (s as f32) / (steps as f32);
            // Tiny epsilon nudge avoids ambiguous hex-edge ties deterministically
            let mq = fq as f32 + (tq - fq) as f32 * frac + 1e-4;
            let my = fy as f32 + (ty - fy) as f32 * frac + 2e-4;
            let mz = fz as f32 + (tz - fz) as f32 * frac - 3e-4;

            let mut rq = mq.round();
            let mut ry = my.round();
            let mut rz = mz.round();
            let dq = (rq - mq).abs();
            let dy = (ry - my).abs();
            let dz = (rz - mz).abs();
            if dq > dy && dq > dz {
                rq = -ry - rz;
            } else if dy > dz {
                ry = -rq - rz;
            } else {
                rz = -rq - ry;
            }
            let row = rz as i32;
            if row < 0 || row >= self.height as i32 {
                continue;
            }
            let col = (rq as i32 + (row - (row & 1)) / 2).rem_euclid(w);
            let mid = self.tile_id(col as u16, row as u16);
            if mid == from || mid == to {
                continue;
            }
            let m = mid as usize;
            let blocker: i8 = match self.elevation[m] {
                Elevation::Mountain => 3,
                Elevation::Hill => {
                    if matches!(self.feature[m], Feature::Forest | Feature::Jungle) {
                        2
                    } else {
                        1
                    }
                }
                Elevation::Flat => {
                    if matches!(self.feature[m], Feature::Forest | Feature::Jungle) {
                        1
                    } else {
                        0
                    }
                }
            };
            if blocker > observer_elev {
                return false;
            }
        }
        true
    }
}
