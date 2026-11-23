use crate::hex::HexMapSoA;
use crate::ruleset::Ruleset;
use crate::types::{
    CivId, Elevation, Feature, GameConfig, Resource, Terrain, TileId, INVALID_CITY, INVALID_TILE,
};
use crate::world::{CitySoA, CivState, DiploMatrix, UnitSoA, VisionState, World};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use std::f64::consts::TAU;

/// Deterministic multi-octave 3D Perlin noise for seamless cylindrical world generation.
struct PerlinNoise {
    perm: [u8; 512],
}

impl PerlinNoise {
    fn new(rng: &mut Xoshiro256PlusPlus) -> Self {
        let mut p = [0u8; 256];
        for i in 0..256 {
            p[i] = i as u8;
        }
        for i in (1..256).rev() {
            let j = rng.gen_range(0..=i);
            p.swap(i, j);
        }
        let mut perm = [0u8; 512];
        perm[..256].copy_from_slice(&p);
        perm[256..].copy_from_slice(&p);
        Self { perm }
    }

    #[inline]
    fn fade(t: f64) -> f64 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    #[inline]
    fn lerp(a: f64, b: f64, t: f64) -> f64 {
        a + t * (b - a)
    }

    #[inline]
    fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
        let h = hash & 15;
        let u = if h < 8 { x } else { y };
        let v = if h < 4 {
            y
        } else if h == 12 || h == 14 {
            x
        } else {
            z
        };
        (if (h & 1) == 0 { u } else { -u }) + (if (h & 2) == 0 { v } else { -v })
    }

    fn noise3(&self, x: f64, y: f64, z: f64) -> f64 {
        let xi = (x.floor() as i32 & 255) as usize;
        let yi = (y.floor() as i32 & 255) as usize;
        let zi = (z.floor() as i32 & 255) as usize;

        let xf = x - x.floor();
        let yf = y - y.floor();
        let zf = z - z.floor();

        let u = Self::fade(xf);
        let v = Self::fade(yf);
        let w = Self::fade(zf);

        let p = &self.perm;
        let a = p[xi] as usize + yi;
        let aa = p[a] as usize + zi;
        let ab = p[a + 1] as usize + zi;
        let b = p[xi + 1] as usize + yi;
        let ba = p[b] as usize + zi;
        let bb = p[b + 1] as usize + zi;

        Self::lerp(
            Self::lerp(
                Self::lerp(Self::grad(p[aa], xf, yf, zf), Self::grad(p[ba], xf - 1.0, yf, zf), u),
                Self::lerp(Self::grad(p[ab], xf, yf - 1.0, zf), Self::grad(p[bb], xf - 1.0, yf - 1.0, zf), u),
                v,
            ),
            Self::lerp(
                Self::lerp(Self::grad(p[aa + 1], xf, yf, zf - 1.0), Self::grad(p[ba + 1], xf - 1.0, yf, zf - 1.0), u),
                Self::lerp(
                    Self::grad(p[ab + 1], xf, yf - 1.0, zf - 1.0),
                    Self::grad(p[bb + 1], xf - 1.0, yf - 1.0, zf - 1.0),
                    u,
                ),
                v,
            ),
            w,
        )
    }

    fn fbm_cylinder(&self, u: f64, v: f64, octaves: u32, freq_x: f64, freq_y: f64) -> f64 {
        let angle = u * TAU;
        let cx = angle.cos();
        let cz = angle.sin();
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut total = 0.0;
        let mut max = 0.0;
        for _ in 0..octaves {
            total += amp * self.noise3(cx * freq_x * freq, v * freq_y * freq, cz * freq_x * freq);
            max += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        total / max
    }
}

/// Generate a complete, playable world with continents, biomes, rivers, resources, and starting civs.
pub fn generate_world(config: GameConfig, rules: &Ruleset) -> World {
    let (width, height) = config.size.dimensions();
    let num_civs = (config.num_civs as usize).clamp(2, rules.civs.len());
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(config.seed ^ 0x9E3779B97F4A7C15);

    let elev_noise = PerlinNoise::new(&mut rng);
    let moist_noise = PerlinNoise::new(&mut rng);
    let ridge_noise = PerlinNoise::new(&mut rng);

    let mut map = HexMapSoA::new(width, height);
    let n = map.num_tiles;
    let mut height_vals = vec![0.0f64; n];

    // 1. Seamless cylindrical elevation + continental shaping
    for r in 0..height {
        let v = (r as f64 + 0.5) / (height as f64);
        let lat = (v - 0.5).abs() * 2.0; // 0 = equator, 1 = pole
        let polar_falloff = if lat > 0.84 { (lat - 0.84) * 2.5 } else { 0.0 };

        for c in 0..width {
            let u = (c as f64) / (width as f64);
            let idx = map.tile_id(c, r) as usize;
            let e = elev_noise.fbm_cylinder(u, v, 5, 1.35, 3.6) - polar_falloff + 0.10;
            height_vals[idx] = e;
        }
    }

    // Determine sea level so ~62% of tiles are land (rich Pangaea/Continents gameplay for 22 civs!)
    let mut sorted = height_vals.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let sea_level = sorted[(n as f64 * 0.36) as usize];

    for r in 0..height {
        let v = (r as f64 + 0.5) / (height as f64);
        let lat = (v - 0.5).abs() * 2.0;

        for c in 0..width {
            let u = (c as f64) / (width as f64);
            let idx = map.tile_id(c, r) as usize;
            let h = height_vals[idx] - sea_level;
            if h < 0.0 {
                map.terrain[idx] = Terrain::Ocean;
                continue;
            }

            let moisture = moist_noise.fbm_cylinder(u, v, 4, 2.0, 4.5);
            let ridge = 1.0 - ridge_noise.fbm_cylinder(u, v, 3, 2.8, 6.0).abs();

            // Elevation
            if h > 0.34 || (h > 0.20 && ridge > 0.86) {
                map.elevation[idx] = Elevation::Mountain;
            } else if h > 0.16 || ridge > 0.78 {
                map.elevation[idx] = Elevation::Hill;
            } else {
                map.elevation[idx] = Elevation::Flat;
            }

            // Climate biome
            let temp = 1.0 - lat * 1.15 - if map.elevation[idx] == Elevation::Hill { 0.08 } else { 0.0 };
            map.terrain[idx] = if temp < 0.12 {
                Terrain::Snow
            } else if temp < 0.28 {
                Terrain::Tundra
            } else if temp > 0.72 && moisture < -0.06 {
                Terrain::Desert
            } else if moisture > 0.04 {
                Terrain::Grassland
            } else {
                Terrain::Plains
            };

            // Features (not on mountains/snow/desert)
            if map.elevation[idx] != Elevation::Mountain {
                match map.terrain[idx] {
                    Terrain::Grassland | Terrain::Plains => {
                        if temp > 0.65 && moisture > 0.18 {
                            map.feature[idx] = Feature::Jungle;
                        } else if moisture > 0.14 {
                            map.feature[idx] = Feature::Forest;
                        } else if map.elevation[idx] == Elevation::Flat && moisture > 0.26 && rng.gen_bool(0.25) {
                            map.feature[idx] = Feature::Marsh;
                        }
                    }
                    Terrain::Tundra => {
                        if moisture > 0.08 && rng.gen_bool(0.45) {
                            map.feature[idx] = Feature::Forest;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // 2. Mark coastal waters adjacent to land
    for t in 0..n {
        if map.terrain[t] == Terrain::Ocean {
            for &nb in map.neighbors_of(t as TileId) {
                if nb != INVALID_TILE && map.terrain[nb as usize].is_land() {
                    map.terrain[t] = Terrain::Coast;
                    break;
                }
            }
        }
    }

    // 3. Carve rivers from highlands downhill toward coasts
    let num_rivers = (n / 180).clamp(6, 36);
    for _ in 0..num_rivers {
        let mut cur = rng.gen_range(0..n) as TileId;
        if map.elevation[cur as usize] != Elevation::Hill {
            continue;
        }
        for _step in 0..16 {
            let mut best_nb = INVALID_TILE;
            let mut best_h = height_vals[cur as usize];
            let mut best_dir = 0u8;
            for (d, &nb) in map.neighbors_of(cur).iter().enumerate() {
                if nb != INVALID_TILE && height_vals[nb as usize] < best_h {
                    best_h = height_vals[nb as usize];
                    best_nb = nb;
                    best_dir = d as u8;
                }
            }
            if best_nb == INVALID_TILE {
                break;
            }
            let opp_dir = (best_dir + 3) % 6;
            map.river_mask[cur as usize] |= 1u8 << best_dir;
            map.river_mask[best_nb as usize] |= 1u8 << opp_dir;
            if map.terrain[best_nb as usize].is_water() {
                break;
            }
            cur = best_nb;
        }
    }

    // 4. Assign continents via iterative flat BFS
    let mut continent_id = 0u16;
    let mut queue = Vec::with_capacity(512);
    for t in 0..n {
        if map.terrain[t].is_land() && map.elevation[t] != Elevation::Mountain && map.continent[t] == 0 {
            continent_id += 1;
            map.continent[t] = continent_id;
            queue.clear();
            queue.push(t as TileId);
            let mut qh = 0;
            while qh < queue.len() {
                let cur = queue[qh];
                qh += 1;
                let base = (cur as usize) * 6;
                for d in 0..6 {
                    let nb = map.neighbors[base + d];
                    if nb != INVALID_TILE {
                        let ni = nb as usize;
                        if map.terrain[ni].is_land()
                            && map.elevation[ni] != Elevation::Mountain
                            && map.continent[ni] == 0
                        {
                            map.continent[ni] = continent_id;
                            queue.push(nb);
                        }
                    }
                }
            }
        }
    }

    // 5. Place balanced resources on passable land tiles
    for t in 0..n {
        if !map.terrain[t].is_land() || map.elevation[t] == Elevation::Mountain {
            continue;
        }
        let roll: f64 = rng.gen();
        if roll > 0.145 {
            continue;
        }
        let res = match (map.terrain[t], map.elevation[t]) {
            (Terrain::Grassland, Elevation::Flat) => {
                if rng.gen_bool(0.55) { Resource::Cattle } else { Resource::Horses }
            }
            (Terrain::Plains, Elevation::Flat) => {
                if rng.gen_bool(0.55) { Resource::Wheat } else { Resource::Horses }
            }
            (Terrain::Desert, _) => {
                if rng.gen_bool(0.55) { Resource::Oil } else { Resource::Gold }
            }
            (Terrain::Tundra | Terrain::Snow, _) => {
                if rng.gen_bool(0.50) { Resource::Oil } else { Resource::Uranium }
            }
            (_, Elevation::Hill) => {
                let r2 = rng.gen_range(0..10);
                if r2 < 4 {
                    Resource::Iron
                } else if r2 < 7 {
                    Resource::Gold
                } else if r2 < 9 {
                    Resource::Gems
                } else {
                    Resource::Uranium
                }
            }
            _ => Resource::None,
        };
        map.resource[t] = res;
    }

    // 6. Pick spread-out fair starting positions for all civilizations
    let mut candidate_starts: Vec<(TileId, i32)> = Vec::new();
    for t in 0..n {
        let tid = t as TileId;
        if !map.terrain[t].is_land() || map.elevation[t] == Elevation::Mountain {
            continue;
        }
        if matches!(map.terrain[t], Terrain::Snow | Terrain::Desert) {
            continue;
        }
        let (_, r) = map.col_row(tid);
        if r < 2 || r >= height - 2 {
            continue;
        }
        let mut score = 0i32;
        if map.has_river(tid) {
            score += 4;
        }
        let mut passable_neighbors = 0;
        for &nb in map.neighbors_of(tid) {
            if nb != INVALID_TILE {
                let ni = nb as usize;
                if map.terrain[ni].is_land() && map.elevation[ni] != Elevation::Mountain {
                    passable_neighbors += 1;
                    if matches!(map.terrain[ni], Terrain::Grassland | Terrain::Plains) {
                        score += 2;
                    }
                    if map.elevation[ni] == Elevation::Hill {
                        score += 2;
                    }
                    if map.resource[ni] != Resource::None {
                        score += 3;
                    }
                }
            }
        }
        if passable_neighbors >= 4 {
            candidate_starts.push((tid, score));
        }
    }
    candidate_starts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut chosen_starts: Vec<TileId> = Vec::with_capacity(num_civs);
    let mut min_dist: u16 = ((width as f32 / (num_civs as f32).sqrt()) * 0.95).clamp(5.0, 14.0) as u16;
    while chosen_starts.len() < num_civs && min_dist >= 3 {
        for &(tid, _) in &candidate_starts {
            if chosen_starts.len() >= num_civs {
                break;
            }
            if chosen_starts.iter().all(|&s| map.distance(s, tid) >= min_dist) {
                chosen_starts.push(tid);
            }
        }
        min_dist = min_dist.saturating_sub(1);
    }

    // 7. Create CivStates, World, and spawn initial Settler + Warrior + Scout
    let mut civs = Vec::with_capacity(num_civs);
    for c in 0..num_civs {
        let tpl = &rules.civs[c];
        civs.push(CivState {
            id: c as CivId,
            name: tpl.name.clone(),
            color: tpl.color,
            accent: tpl.accent,
            alive: true,
            is_human: config.human_civ == Some(c as CivId),
            aggression: tpl.aggression,
            science_bias: tpl.science_bias,
            expansion: tpl.expansion,
            gold: 40,
            gold_per_turn: 0,
            science_per_turn: 0,
            culture_per_turn: 0,
            tech_bits: 1u64 << 0, // Start with Agriculture
            current_tech: Some(1),
            tech_progress: 0,
            supply_used: 0,
            supply_cap: 7,
            cities_founded: 0,
            capital: INVALID_CITY,
            score: 0,
            template_idx: c as u8,
        });
    }

    let mut world = World {
        config: GameConfig {
            num_civs: num_civs as u16,
            ..config
        },
        turn: 1,
        map,
        units: UnitSoA::new(n),
        cities: CitySoA::new(n),
        civs,
        diplo: DiploMatrix::new(num_civs),
        vision: VisionState::new(num_civs, n),
        victory: None,
        speculative_dirty_civs: u64::MAX,
    };

    for (c, &start_tile) in chosen_starts.iter().enumerate() {
        let civ_id = c as CivId;
        // Found initial capital immediately so Turn 1 starts briskly on mobile & AI
        world.found_city(rules, civ_id, start_tile);
        if let Some(t) = world.find_spawn_tile(start_tile, false, false) {
            world.spawn_unit(rules, civ_id, rules.warrior_kind, t, 0);
        }
        if let Some(t) = world.find_spawn_tile(start_tile, false, false) {
            world.spawn_unit(rules, civ_id, rules.scout_kind, t, 0);
        }
        if let Some(t) = world.find_spawn_tile(start_tile, true, false) {
            world.spawn_unit(rules, civ_id, rules.worker_kind, t, 0);
        }
        world.vision.rebuild_civ(&world.map, &world.units, &world.cities, rules, civ_id);
    }

    // Initial yield & supply pass
    crate::economy::recompute_all_civs(&mut world, rules);
    world.speculative_dirty_civs = 0;
    world
}
