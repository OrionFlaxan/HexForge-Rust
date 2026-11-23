use crate::hex::HexMapSoA;
use crate::ruleset::Ruleset;
use crate::types::{
    BuildingBits, CityId, CivId, DiploStatus, GameConfig, ProductionItem, TechBits, TechId, TileId,
    UnitFormation, UnitId, UnitKindId, VictoryState, Yields, INVALID_CITY, INVALID_TILE,
    INVALID_UNIT, NO_CIV,
};
use serde::{Deserialize, Serialize};

/// Dense Structure-of-Arrays storage for all units on the map.
/// Uses slot reuse (`free_slots`) + O(1) tile occupancy LUTs (`tile_mil_unit`, `tile_civ_unit`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitSoA {
    pub alive: Vec<bool>,
    pub pos: Vec<TileId>,
    pub civ: Vec<CivId>,
    pub kind: Vec<UnitKindId>,
    pub formation: Vec<UnitFormation>,
    pub hp: Vec<i16>,
    /// Remaining movement points in halves (e.g. 2.0 moves = 4 halves).
    pub moves_halves: Vec<u8>,
    pub xp: Vec<u16>,
    pub fortified: Vec<bool>,
    pub has_attacked: Vec<bool>,
    pub auto_explore: Vec<bool>,
    pub free_slots: Vec<UnitId>,
    pub alive_count: u32,
    /// O(1) spatial lookup: combat unit occupying each tile (`INVALID_UNIT` if empty).
    pub tile_mil_unit: Vec<UnitId>,
    /// O(1) spatial lookup: civilian unit (Settler/Worker) occupying each tile.
    pub tile_civ_unit: Vec<UnitId>,
}

impl UnitSoA {
    pub fn new(num_tiles: usize) -> Self {
        Self {
            alive: Vec::with_capacity(256),
            pos: Vec::with_capacity(256),
            civ: Vec::with_capacity(256),
            kind: Vec::with_capacity(256),
            formation: Vec::with_capacity(256),
            hp: Vec::with_capacity(256),
            moves_halves: Vec::with_capacity(256),
            xp: Vec::with_capacity(256),
            fortified: Vec::with_capacity(256),
            has_attacked: Vec::with_capacity(256),
            auto_explore: Vec::with_capacity(256),
            free_slots: Vec::new(),
            alive_count: 0,
            tile_mil_unit: vec![INVALID_UNIT; num_tiles],
            tile_civ_unit: vec![INVALID_UNIT; num_tiles],
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.alive.len()
    }

    #[inline]
    pub fn is_alive(&self, id: UnitId) -> bool {
        (id as usize) < self.alive.len() && self.alive[id as usize]
    }

    pub fn spawn(
        &mut self,
        civ: CivId,
        kind: UnitKindId,
        tile: TileId,
        moves: u8,
        is_civilian: bool,
        xp: u16,
    ) -> UnitId {
        let id = if let Some(slot) = self.free_slots.pop() {
            let i = slot as usize;
            self.alive[i] = true;
            self.pos[i] = tile;
            self.civ[i] = civ;
            self.kind[i] = kind;
            self.formation[i] = UnitFormation::Single;
            self.hp[i] = 100;
            self.moves_halves[i] = moves * 2;
            self.xp[i] = xp;
            self.fortified[i] = false;
            self.has_attacked[i] = false;
            self.auto_explore[i] = false;
            slot
        } else {
            let slot = self.alive.len() as UnitId;
            self.alive.push(true);
            self.pos.push(tile);
            self.civ.push(civ);
            self.kind.push(kind);
            self.formation.push(UnitFormation::Single);
            self.hp.push(100);
            self.moves_halves.push(moves * 2);
            self.xp.push(xp);
            self.fortified.push(false);
            self.has_attacked.push(false);
            self.auto_explore.push(false);
            slot
        };
        if is_civilian {
            self.tile_civ_unit[tile as usize] = id;
        } else {
            self.tile_mil_unit[tile as usize] = id;
        }
        self.alive_count += 1;
        id
    }

    pub fn despawn(&mut self, id: UnitId, is_civilian: bool) {
        let i = id as usize;
        if i >= self.alive.len() || !self.alive[i] {
            return;
        }
        let tile = self.pos[i] as usize;
        if is_civilian {
            if self.tile_civ_unit[tile] == id {
                self.tile_civ_unit[tile] = INVALID_UNIT;
            }
        } else if self.tile_mil_unit[tile] == id {
            self.tile_mil_unit[tile] = INVALID_UNIT;
        }
        self.alive[i] = false;
        self.alive_count = self.alive_count.saturating_sub(1);
        self.free_slots.push(id);
    }

    #[inline]
    pub fn relocate(&mut self, id: UnitId, new_tile: TileId, is_civilian: bool) {
        let i = id as usize;
        let old_tile = self.pos[i] as usize;
        if is_civilian {
            if self.tile_civ_unit[old_tile] == id {
                self.tile_civ_unit[old_tile] = INVALID_UNIT;
            }
            self.tile_civ_unit[new_tile as usize] = id;
        } else {
            if self.tile_mil_unit[old_tile] == id {
                self.tile_mil_unit[old_tile] = INVALID_UNIT;
            }
            self.tile_mil_unit[new_tile as usize] = id;
        }
        self.pos[i] = new_tile;
    }

    /// Count formations across all alive units `(singles, corps, armies)`.
    pub fn formation_counts(&self) -> (u32, u32, u32) {
        let mut s = 0;
        let mut c = 0;
        let mut a = 0;
        for i in 0..self.alive.len() {
            if self.alive[i] {
                match self.formation[i] {
                    UnitFormation::Single => s += 1,
                    UnitFormation::Corps => c += 1,
                    UnitFormation::Army => a += 1,
                }
            }
        }
        (s, c, a)
    }
}

/// Dense Structure-of-Arrays storage for all cities on the map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitySoA {
    pub alive: Vec<bool>,
    pub pos: Vec<TileId>,
    pub civ: Vec<CivId>,
    pub name: Vec<String>,
    pub pop: Vec<u16>,
    pub food_stored: Vec<i32>,
    pub prod_stored: Vec<i32>,
    pub culture_stored: Vec<i32>,
    pub hp: Vec<i16>,
    pub max_hp: Vec<i16>,
    pub buildings: Vec<BuildingBits>,
    pub producing: Vec<Option<ProductionItem>>,
    pub has_bombarded: Vec<bool>,
    pub is_capital: Vec<bool>,
    pub border_radius: Vec<u8>,
    pub last_yields: Vec<Yields>,
    /// O(1) spatial lookup: city at tile (`INVALID_CITY` if none).
    pub tile_city: Vec<CityId>,
}

impl CitySoA {
    pub fn new(num_tiles: usize) -> Self {
        Self {
            alive: Vec::with_capacity(64),
            pos: Vec::with_capacity(64),
            civ: Vec::with_capacity(64),
            name: Vec::with_capacity(64),
            pop: Vec::with_capacity(64),
            food_stored: Vec::with_capacity(64),
            prod_stored: Vec::with_capacity(64),
            culture_stored: Vec::with_capacity(64),
            hp: Vec::with_capacity(64),
            max_hp: Vec::with_capacity(64),
            buildings: Vec::with_capacity(64),
            producing: Vec::with_capacity(64),
            has_bombarded: Vec::with_capacity(64),
            is_capital: Vec::with_capacity(64),
            border_radius: Vec::with_capacity(64),
            last_yields: Vec::with_capacity(64),
            tile_city: vec![INVALID_CITY; num_tiles],
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.alive.len()
    }

    #[inline]
    pub fn is_alive(&self, id: CityId) -> bool {
        (id as usize) < self.alive.len() && self.alive[id as usize]
    }

    pub fn add_city(&mut self, civ: CivId, pos: TileId, name: String, is_capital: bool) -> CityId {
        let id = self.alive.len() as CityId;
        self.alive.push(true);
        self.pos.push(pos);
        self.civ.push(civ);
        self.name.push(name);
        self.pop.push(1);
        self.food_stored.push(0);
        self.prod_stored.push(0);
        self.culture_stored.push(0);
        self.hp.push(200);
        self.max_hp.push(200);
        self.buildings.push(0);
        self.producing.push(None);
        self.has_bombarded.push(false);
        self.is_capital.push(is_capital);
        self.border_radius.push(1);
        self.last_yields.push(Yields::default());
        self.tile_city[pos as usize] = id;
        id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivState {
    pub id: CivId,
    pub name: String,
    pub color: [u8; 3],
    pub accent: [u8; 3],
    pub alive: bool,
    pub is_human: bool,
    pub aggression: u8,
    pub science_bias: u8,
    pub expansion: u8,
    pub gold: i32,
    pub gold_per_turn: i32,
    pub science_per_turn: i32,
    pub culture_per_turn: i32,
    pub tech_bits: TechBits,
    pub current_tech: Option<TechId>,
    pub tech_progress: i32,
    pub supply_used: u16,
    pub supply_cap: u16,
    pub cities_founded: u16,
    pub capital: CityId,
    pub score: i32,
    /// Index into Ruleset's civ template for city naming.
    pub template_idx: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiploMatrix {
    pub num_civs: usize,
    pub status: Vec<DiploStatus>,
    pub opinion: Vec<i16>,
    pub war_turns: Vec<u16>,
    pub peace_cooldown: Vec<u16>,
}

impl DiploMatrix {
    pub fn new(num_civs: usize) -> Self {
        let n2 = num_civs * num_civs;
        Self {
            num_civs,
            status: vec![DiploStatus::Unmet; n2],
            opinion: vec![0; n2],
            war_turns: vec![0; n2],
            peace_cooldown: vec![0; n2],
        }
    }

    #[inline]
    fn idx(&self, a: CivId, b: CivId) -> usize {
        (a as usize) * self.num_civs + (b as usize)
    }

    #[inline]
    pub fn status(&self, a: CivId, b: CivId) -> DiploStatus {
        if a == b || a == NO_CIV || b == NO_CIV {
            return DiploStatus::Peace;
        }
        self.status[self.idx(a, b)]
    }

    #[inline]
    pub fn is_at_war(&self, a: CivId, b: CivId) -> bool {
        a != b && a != NO_CIV && b != NO_CIV && self.status[self.idx(a, b)] == DiploStatus::War
    }

    #[inline]
    pub fn has_met(&self, a: CivId, b: CivId) -> bool {
        a == b || (a != NO_CIV && b != NO_CIV && self.status[self.idx(a, b)] != DiploStatus::Unmet)
    }

    pub fn set_status(&mut self, a: CivId, b: CivId, st: DiploStatus) {
        if a == b || a == NO_CIV || b == NO_CIV {
            return;
        }
        let ab = self.idx(a, b);
        let ba = self.idx(b, a);
        self.status[ab] = st;
        self.status[ba] = st;
        if st == DiploStatus::War {
            self.war_turns[ab] = 0;
            self.war_turns[ba] = 0;
            self.opinion[ab] = (self.opinion[ab] - 40).max(-100);
            self.opinion[ba] = (self.opinion[ba] - 40).max(-100);
        } else if st == DiploStatus::Peace {
            self.peace_cooldown[ab] = 10;
            self.peace_cooldown[ba] = 10;
            self.war_turns[ab] = 0;
            self.war_turns[ba] = 0;
        }
    }

    pub fn meet(&mut self, a: CivId, b: CivId) {
        if a != b && a != NO_CIV && b != NO_CIV && self.status(a, b) == DiploStatus::Unmet {
            let ab = self.idx(a, b);
            let ba = self.idx(b, a);
            self.status[ab] = DiploStatus::Peace;
            self.status[ba] = DiploStatus::Peace;
        }
    }

    #[inline]
    pub fn peace_cooldown(&self, a: CivId, b: CivId) -> u16 {
        self.peace_cooldown[self.idx(a, b)]
    }

    #[inline]
    pub fn war_turns(&self, a: CivId, b: CivId) -> u16 {
        self.war_turns[self.idx(a, b)]
    }
}

/// Bitset + refcount Vision & Fog-of-War state for all civilizations.
/// O(1) visibility/exploration checks via 64-bit word ops, O(sight^2) incremental unit moves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionState {
    pub num_civs: usize,
    pub num_tiles: usize,
    pub words_per_civ: usize,
    pub visible_bits: Vec<u64>,
    pub explored_bits: Vec<u64>,
    pub refcounts: Vec<u16>,
}

impl VisionState {
    pub fn new(num_civs: usize, num_tiles: usize) -> Self {
        let words_per_civ = (num_tiles + 63) / 64;
        Self {
            num_civs,
            num_tiles,
            words_per_civ,
            visible_bits: vec![0; num_civs * words_per_civ],
            explored_bits: vec![0; num_civs * words_per_civ],
            refcounts: vec![0; num_civs * num_tiles],
        }
    }

    #[inline]
    pub fn is_visible(&self, civ: CivId, tile: TileId) -> bool {
        let base = (civ as usize) * self.words_per_civ;
        let w = (tile as usize) >> 6;
        let b = (tile as usize) & 63;
        (self.visible_bits[base + w] & (1u64 << b)) != 0
    }

    #[inline]
    pub fn is_explored(&self, civ: CivId, tile: TileId) -> bool {
        let base = (civ as usize) * self.words_per_civ;
        let w = (tile as usize) >> 6;
        let b = (tile as usize) & 63;
        (self.explored_bits[base + w] & (1u64 << b)) != 0
    }

    #[inline]
    pub fn visible_words(&self, civ: CivId) -> &[u64] {
        let base = (civ as usize) * self.words_per_civ;
        &self.visible_bits[base..base + self.words_per_civ]
    }

    #[inline]
    pub fn explored_words(&self, civ: CivId) -> &[u64] {
        let base = (civ as usize) * self.words_per_civ;
        &self.explored_bits[base..base + self.words_per_civ]
    }

    #[inline]
    pub fn add_tile_ref(&mut self, civ: CivId, tile: TileId) {
        let rc_idx = (civ as usize) * self.num_tiles + (tile as usize);
        let rc = self.refcounts[rc_idx];
        self.refcounts[rc_idx] = rc.saturating_add(1);
        if rc == 0 {
            let base = (civ as usize) * self.words_per_civ;
            let w = (tile as usize) >> 6;
            let mask = 1u64 << ((tile as usize) & 63);
            self.visible_bits[base + w] |= mask;
            self.explored_bits[base + w] |= mask;
        }
    }

    #[inline]
    pub fn remove_tile_ref(&mut self, civ: CivId, tile: TileId) {
        let rc_idx = (civ as usize) * self.num_tiles + (tile as usize);
        let rc = self.refcounts[rc_idx];
        if rc > 0 {
            let next = rc - 1;
            self.refcounts[rc_idx] = next;
            if next == 0 {
                let base = (civ as usize) * self.words_per_civ;
                let w = (tile as usize) >> 6;
                let mask = 1u64 << ((tile as usize) & 63);
                self.visible_bits[base + w] &= !mask;
            }
        }
    }

    pub fn add_source(&mut self, map: &HexMapSoA, civ: CivId, center: TileId, sight: u8, buf: &mut Vec<TileId>) {
        map.collect_tiles_in_radius(center, sight, buf);
        for &t in buf.iter() {
            if t == center || map.distance(center, t) <= 1 || map.has_los(center, t) {
                self.add_tile_ref(civ, t);
            }
        }
    }

    pub fn remove_source(&mut self, map: &HexMapSoA, civ: CivId, center: TileId, sight: u8, buf: &mut Vec<TileId>) {
        map.collect_tiles_in_radius(center, sight, buf);
        for &t in buf.iter() {
            if t == center || map.distance(center, t) <= 1 || map.has_los(center, t) {
                self.remove_tile_ref(civ, t);
            }
        }
    }

    pub fn rebuild_civ(&mut self, map: &HexMapSoA, units: &UnitSoA, cities: &CitySoA, rules: &Ruleset, civ: CivId) {
        let wbase = (civ as usize) * self.words_per_civ;
        self.visible_bits[wbase..wbase + self.words_per_civ].fill(0);
        let rbase = (civ as usize) * self.num_tiles;
        self.refcounts[rbase..rbase + self.num_tiles].fill(0);

        let mut buf = Vec::with_capacity(64);
        // Owned territory + 1 ring around cities
        for t in 0..self.num_tiles {
            if map.owner[t] == civ {
                self.add_tile_ref(civ, t as TileId);
                for &nb in map.neighbors_of(t as TileId) {
                    if nb != INVALID_TILE {
                        self.add_tile_ref(civ, nb);
                    }
                }
            }
        }
        for c in 0..cities.len() {
            if cities.alive[c] && cities.civ[c] == civ {
                self.add_source(map, civ, cities.pos[c], 2, &mut buf);
            }
        }
        for u in 0..units.len() {
            if units.alive[u] && units.civ[u] == civ {
                let sight = rules.units[units.kind[u] as usize].sight;
                self.add_source(map, civ, units.pos[u], sight, &mut buf);
            }
        }
    }
}

/// Complete game state — serializable cleanly via `bincode` + `flate2`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub config: GameConfig,
    pub turn: u32,
    pub map: HexMapSoA,
    pub units: UnitSoA,
    pub cities: CitySoA,
    pub civs: Vec<CivState>,
    pub diplo: DiploMatrix,
    pub vision: VisionState,
    pub victory: Option<VictoryState>,
    /// Per-civ dirty bitmask for Background Speculative AI invalidation:
    /// bit `c` is set if human actions during the current turn affected civ `c`.
    #[serde(skip)]
    pub speculative_dirty_civs: u64,
}

impl World {
    /// Spawn a unit and immediately update its civ's incremental vision + diplomacy contacts.
    pub fn spawn_unit(
        &mut self,
        rules: &Ruleset,
        civ: CivId,
        kind: UnitKindId,
        tile: TileId,
        xp: u16,
    ) -> UnitId {
        let def = &rules.units[kind as usize];
        let id = self
            .units
            .spawn(civ, kind, tile, def.moves, def.role.is_civilian(), xp);
        let mut buf = Vec::with_capacity(37);
        self.vision.add_source(&self.map, civ, tile, def.sight, &mut buf);
        self.check_contacts_around(civ, &buf);
        id
    }

    pub fn despawn_unit(&mut self, rules: &Ruleset, id: UnitId) {
        if !self.units.is_alive(id) {
            return;
        }
        let i = id as usize;
        let civ = self.units.civ[i];
        let tile = self.units.pos[i];
        let def = &rules.units[self.units.kind[i] as usize];
        self.units.despawn(id, def.role.is_civilian());
        let mut buf = Vec::with_capacity(37);
        self.vision.remove_source(&self.map, civ, tile, def.sight, &mut buf);
    }

    /// Move a unit to `new_tile`, deducting `cost_halves`, updating vision incrementally and checking civ contacts.
    pub fn move_unit_step(
        &mut self,
        rules: &Ruleset,
        id: UnitId,
        new_tile: TileId,
        cost_halves: u8,
    ) -> bool {
        if !self.units.is_alive(id) {
            return false;
        }
        let i = id as usize;
        let civ = self.units.civ[i];
        let old_tile = self.units.pos[i];
        let def = &rules.units[self.units.kind[i] as usize];
        let is_civ = def.role.is_civilian();

        // Occupancy check
        if is_civ {
            if self.units.tile_civ_unit[new_tile as usize] != INVALID_UNIT {
                return false;
            }
        } else if self.units.tile_mil_unit[new_tile as usize] != INVALID_UNIT {
            return false;
        }

        // Capture enemy civilian if combat unit steps onto it
        if !is_civ {
            let other_civ_u = self.units.tile_civ_unit[new_tile as usize];
            if other_civ_u != INVALID_UNIT {
                let ociv = self.units.civ[other_civ_u as usize];
                if self.diplo.is_at_war(civ, ociv) {
                    self.despawn_unit(rules, other_civ_u);
                }
            }
        }

        self.units.relocate(id, new_tile, is_civ);
        self.units.moves_halves[i] = self.units.moves_halves[i].saturating_sub(cost_halves);
        self.units.fortified[i] = false;

        let mut buf = Vec::with_capacity(37);
        self.vision.remove_source(&self.map, civ, old_tile, def.sight, &mut buf);
        self.vision.add_source(&self.map, civ, new_tile, def.sight, &mut buf);
        self.check_contacts_around(civ, &buf);

        // Mark nearby AI civs dirty for speculative invalidation when human moves
        if self.civs[civ as usize].is_human {
            self.mark_nearby_civs_dirty(new_tile, 6);
            self.mark_nearby_civs_dirty(old_tile, 6);
        }
        true
    }

    pub fn check_contacts_around(&mut self, civ: CivId, tiles: &[TileId]) {
        for &t in tiles {
            let ti = t as usize;
            let owner = self.map.owner[ti];
            if owner != NO_CIV && owner != civ {
                self.diplo.meet(civ, owner);
            }
            let mu = self.units.tile_mil_unit[ti];
            if mu != INVALID_UNIT {
                let uciv = self.units.civ[mu as usize];
                if uciv != civ {
                    self.diplo.meet(civ, uciv);
                }
            }
            let cu = self.units.tile_civ_unit[ti];
            if cu != INVALID_UNIT {
                let uciv = self.units.civ[cu as usize];
                if uciv != civ {
                    self.diplo.meet(civ, uciv);
                }
            }
        }
    }

    /// Mark AI civs within `radius` hexes of `tile` as dirty for speculative plan invalidation.
    pub fn mark_nearby_civs_dirty(&mut self, tile: TileId, radius: u16) {
        for c in 0..self.cities.len() {
            if self.cities.alive[c] && self.map.distance(tile, self.cities.pos[c]) <= radius {
                let cid = self.cities.civ[c];
                if (cid as usize) < 64 {
                    self.speculative_dirty_civs |= 1u64 << cid;
                }
            }
        }
        for u in 0..self.units.len() {
            if self.units.alive[u] && self.map.distance(tile, self.units.pos[u]) <= radius {
                let cid = self.units.civ[u];
                if (cid as usize) < 64 {
                    self.speculative_dirty_civs |= 1u64 << cid;
                }
            }
        }
    }

    /// Found a new city at `tile` for `civ`, claiming inner ring tiles and updating vision.
    pub fn found_city(&mut self, rules: &Ruleset, civ: CivId, tile: TileId) -> Option<CityId> {
        let ti = tile as usize;
        if !self.map.terrain[ti].is_land() || self.map.elevation[ti] == crate::types::Elevation::Mountain {
            return None;
        }
        if self.cities.tile_city[ti] != INVALID_CITY {
            return None;
        }
        // Minimum 3-hex spacing between cities
        for c in 0..self.cities.len() {
            if self.cities.alive[c] && self.map.distance(tile, self.cities.pos[c]) < 3 {
                return None;
            }
        }

        let cs = &mut self.civs[civ as usize];
        let is_capital = cs.capital == INVALID_CITY || !self.cities.is_alive(cs.capital);
        let tpl = &rules.civs[cs.template_idx as usize];
        let city_name = if (cs.cities_founded as usize) < tpl.cities.len() {
            tpl.cities[cs.cities_founded as usize].clone()
        } else {
            format!("{} {}", tpl.name, cs.cities_founded + 1)
        };
        cs.cities_founded += 1;

        let cid = self.cities.add_city(civ, tile, city_name, is_capital);
        if is_capital {
            self.civs[civ as usize].capital = cid;
        }

        // Claim center + immediate neighbors if unclaimed
        let mut ring = Vec::with_capacity(19);
        self.map.collect_tiles_in_radius(tile, 1, &mut ring);
        for &rt in &ring {
            let rti = rt as usize;
            if self.map.owner[rti] == NO_CIV || rt == tile {
                self.map.owner[rti] = civ;
                self.map.owner_city[rti] = cid;
            }
        }
        // Road on city tile
        self.map.road[ti] = crate::types::Road::Road;
        self.map.improvement[ti] = crate::types::Improvement::None;

        let mut buf = Vec::with_capacity(37);
        self.vision.add_source(&self.map, civ, tile, 2, &mut buf);
        self.check_contacts_around(civ, &buf);

        if self.civs[civ as usize].is_human {
            self.mark_nearby_civs_dirty(tile, 8);
        }
        Some(cid)
    }

    /// Find an unoccupied adjacent tile for spawning a unit near `center`.
    pub fn find_spawn_tile(&self, center: TileId, is_civilian: bool, is_naval: bool) -> Option<TileId> {
        let occ = if is_civilian {
            &self.units.tile_civ_unit
        } else {
            &self.units.tile_mil_unit
        };
        if !is_naval && occ[center as usize] == INVALID_UNIT {
            return Some(center);
        }
        for &nb in self.map.neighbors_of(center) {
            if nb == INVALID_TILE {
                continue;
            }
            let ni = nb as usize;
            let terrain_ok = if is_naval {
                self.map.terrain[ni].is_water()
            } else {
                self.map.terrain[ni].is_land()
                    && self.map.elevation[ni] != crate::types::Elevation::Mountain
            };
            if terrain_ok && occ[ni] == INVALID_UNIT {
                return Some(nb);
            }
        }
        None
    }

    /// Deterministic FNV-1a 64-bit state hash across all simulation arrays.
    /// Used by automated tests & benchmarks to prove 1-thread == N-thread bit-exact determinism.
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let mix = |h: &mut u64, v: u64| {
            *h ^= v;
            *h = h.wrapping_mul(0x100000001b3);
        };
        mix(&mut h, self.turn as u64);
        mix(&mut h, self.units.alive_count as u64);
        for i in 0..self.units.len() {
            if self.units.alive[i] {
                mix(&mut h, i as u64);
                mix(&mut h, self.units.pos[i] as u64);
                mix(&mut h, self.units.civ[i] as u64);
                mix(&mut h, self.units.kind[i] as u64);
                mix(&mut h, self.units.formation[i] as u64);
                mix(&mut h, self.units.hp[i] as u64);
                mix(&mut h, self.units.xp[i] as u64);
            }
        }
        for i in 0..self.cities.len() {
            if self.cities.alive[i] {
                mix(&mut h, i as u64);
                mix(&mut h, self.cities.pos[i] as u64);
                mix(&mut h, self.cities.civ[i] as u64);
                mix(&mut h, self.cities.pop[i] as u64);
                mix(&mut h, self.cities.hp[i] as u64);
                mix(&mut h, self.cities.buildings[i]);
                mix(&mut h, self.cities.prod_stored[i] as u64);
            }
        }
        for c in &self.civs {
            mix(&mut h, c.alive as u64);
            mix(&mut h, c.gold as u64);
            mix(&mut h, c.tech_bits);
            mix(&mut h, c.tech_progress as u64);
            mix(&mut h, c.score as u64);
        }
        for &st in &self.diplo.status {
            mix(&mut h, st as u64);
        }
        h
    }
}
