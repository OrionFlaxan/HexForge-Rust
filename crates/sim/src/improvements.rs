use crate::hex::HexMapSoA;
use crate::policies::has_policy;
use crate::ruleset::Ruleset;
use crate::types::{
    CityId, CivId, Elevation, Feature, GameSpeed, Improvement, Resource, Road, TechBits, TechId,
    Terrain, TileId, UnitId, UnitRole, WorkerOrder, Yields, INVALID_CITY, NO_CIV,
};
use crate::world::World;

#[derive(Debug, Clone, Copy)]
pub struct WorkerOrderInfo {
    pub order: WorkerOrder,
    pub turns: u8,
    pub yield_bonus: Yields,
    pub note: &'static str,
    pub is_active: bool,
}

/// Tech required to construct `imp` (`None` means available from Turn 1 / Agriculture).
#[inline]
pub fn improvement_required_tech(rules: &Ruleset, imp: Improvement) -> Option<TechId> {
    match imp {
        Improvement::None | Improvement::Farm => None,
        Improvement::Mine => rules.mining_tech,
        Improvement::Pasture => rules.animal_husbandry_tech,
        Improvement::Quarry => rules.masonry_tech,
        Improvement::Plantation => rules.calendar_tech,
        Improvement::Camp => rules.trapping_tech,
        Improvement::LumberMill => rules.construction_tech,
        Improvement::TerraceFarm => rules.engineering_tech,
        Improvement::TradingPost => rules.guilds_tech,
        Improvement::Fort => rules.engineering_tech,
        Improvement::OilWell => rules.biology_tech,
        Improvement::PowerPlant => rules.electricity_tech,
        Improvement::SolarFarm => rules.ecology_tech,
        Improvement::ResearchOutpost => rules.computers_tech,
        Improvement::Airbase => rules.radar_tech,
        Improvement::Arcology => rules.nanotech_tech,
    }
}

/// Human-readable short tactical note for an improvement (shown in Worker Sheet & Codex).
#[inline]
pub const fn improvement_note(imp: Improvement) -> &'static str {
    match imp {
        Improvement::None => "",
        Improvement::Farm => "+1F (+2F River; +1F w/ Fertilizer)",
        Improvement::Mine => "+2P (+1P w/ Chemistry)",
        Improvement::Pasture => "+1F +1P (+1P w/ Fertilizer)",
        Improvement::Quarry => "+2P +1G (+1P w/ Chemistry)",
        Improvement::Plantation => "+1F +2G (+1G w/ Economics)",
        Improvement::Camp => "+1F +1G (+1G w/ Economics)",
        Improvement::LumberMill => "+2P, keeps Forest Def (+1P Sci Theory)",
        Improvement::TerraceFarm => "+2F +1P on Hills (+1F adj. Mountain)",
        Improvement::TradingPost => "+2G (+1S w/ Economics)",
        Improvement::Fort => "+50% Defense (valid on neutral border!)",
        Improvement::OilWell => "+2P +2G (+1P w/ Plastics)",
        Improvement::PowerPlant => "+2P +2G +1S on River or Hill",
        Improvement::SolarFarm => "+2P +2S +1G on Desert or Plains",
        Improvement::ResearchOutpost => "+3S (+1S w/ Nanotechnology)",
        Improvement::Airbase => "+1P, +50% Defense & Air Unit Rebase!",
        Improvement::Arcology => "+3F +3P +3S Future Self-Contained Dome",
    }
}

/// Exact tech-scaled yield bonus contributed by `imp` on `tile` for an owner with `owner_techs`.
#[inline]
pub fn improvement_yields(
    map: &HexMapSoA,
    rules: &Ruleset,
    tile: TileId,
    imp: Improvement,
    owner_techs: TechBits,
) -> Yields {
    match imp {
        Improvement::None => Yields::default(),
        Improvement::Farm => {
            let mut f = if map.has_river(tile) { 2 } else { 1 };
            if rules.fertilizer_tech.map_or(false, |t| Ruleset::has_tech(owner_techs, t)) {
                f += 1;
            }
            Yields::new(f, 0, 0, 0, 0)
        }
        Improvement::Mine => {
            let mut p = 2;
            if rules.chemistry_tech.map_or(false, |t| Ruleset::has_tech(owner_techs, t)) {
                p += 1;
            }
            Yields::new(0, p, 0, 0, 0)
        }
        Improvement::Pasture => {
            let mut p = 1;
            if rules.fertilizer_tech.map_or(false, |t| Ruleset::has_tech(owner_techs, t)) {
                p += 1;
            }
            Yields::new(1, p, 0, 0, 0)
        }
        Improvement::Quarry => {
            let mut p = 2;
            if rules.chemistry_tech.map_or(false, |t| Ruleset::has_tech(owner_techs, t)) {
                p += 1;
            }
            Yields::new(0, p, 1, 0, 0)
        }
        Improvement::Plantation => {
            let mut g = 2;
            if rules.economics_tech.map_or(false, |t| Ruleset::has_tech(owner_techs, t)) {
                g += 1;
            }
            Yields::new(1, 0, g, 0, 0)
        }
        Improvement::Camp => {
            let mut g = 1;
            if rules.economics_tech.map_or(false, |t| Ruleset::has_tech(owner_techs, t)) {
                g += 1;
            }
            Yields::new(1, 0, g, 0, 0)
        }
        Improvement::LumberMill => {
            let mut p = 2;
            if rules
                .scientific_theory_tech
                .map_or(false, |t| Ruleset::has_tech(owner_techs, t))
            {
                p += 1;
            }
            Yields::new(0, p, 0, 0, 0)
        }
        Improvement::TerraceFarm => {
            let mut f = 2;
            if map.is_adjacent_to_mountain(tile) {
                f += 1;
            }
            Yields::new(f, 1, 0, 0, 0)
        }
        Improvement::TradingPost => {
            let s = if rules.economics_tech.map_or(false, |t| Ruleset::has_tech(owner_techs, t)) {
                1
            } else {
                0
            };
            Yields::new(0, 0, 2, s, 0)
        }
        Improvement::Fort => Yields::default(),
        Improvement::OilWell => {
            let mut p = 2;
            if rules.plastics_tech.map_or(false, |t| Ruleset::has_tech(owner_techs, t)) {
                p += 1;
            }
            Yields::new(0, p, 2, 0, 0)
        }
        Improvement::PowerPlant => Yields::new(0, 2, 2, 1, 0),
        Improvement::SolarFarm => Yields::new(0, 2, 1, 2, 0),
        Improvement::ResearchOutpost => {
            let mut s = 3;
            if rules.nanotech_tech.map_or(false, |t| Ruleset::has_tech(owner_techs, t)) {
                s += 1;
            }
            Yields::new(0, 0, 0, s, 0)
        }
        Improvement::Airbase => Yields::new(0, 1, 0, 0, 0),
        Improvement::Arcology => Yields::new(3, 3, 0, 3, 0),
    }
}

/// Check if `civ` can construct `imp` on `tile` given terrain, features, resources, borders, and techs.
pub fn can_build_improvement(
    world: &World,
    rules: &Ruleset,
    civ: CivId,
    tile: TileId,
    imp: Improvement,
) -> bool {
    if imp == Improvement::None {
        return false;
    }
    let ti = tile as usize;
    if !world.map.terrain[ti].is_land() || world.map.elevation[ti] == Elevation::Mountain {
        return false;
    }
    if world.cities.tile_city[ti] != INVALID_CITY {
        return false;
    }
    let owner = world.map.owner[ti];
    if imp.allows_neutral_tile() {
        if owner != civ && owner != NO_CIV {
            return false;
        }
    } else if owner != civ {
        return false;
    }
    if world.map.improvement[ti] == imp && !world.map.pillaged[ti] {
        return false;
    }
    let techs = world.civs[civ as usize].tech_bits;
    if !Ruleset::has_opt_tech(techs, improvement_required_tech(rules, imp)) {
        return false;
    }

    let terr = world.map.terrain[ti];
    let elev = world.map.elevation[ti];
    let feat = world.map.feature[ti];
    let res = world.map.resource[ti];
    let river = world.map.has_river(tile);

    match imp {
        Improvement::None => false,
        Improvement::Farm => {
            elev == Elevation::Flat
                && feat == Feature::None
                && (matches!(terr, Terrain::Grassland | Terrain::Plains)
                    || (terr == Terrain::Desert && river)
                    || res == Resource::Wheat)
        }
        Improvement::Mine => {
            elev == Elevation::Hill
                || matches!(res, Resource::Iron | Resource::Gold | Resource::Gems | Resource::Uranium)
        }
        Improvement::Pasture => {
            matches!(res, Resource::Cattle | Resource::Horses)
                || (elev == Elevation::Flat
                    && feat == Feature::None
                    && matches!(terr, Terrain::Grassland | Terrain::Plains))
        }
        Improvement::Quarry => {
            elev == Elevation::Hill || matches!(res, Resource::Gold | Resource::Gems)
        }
        Improvement::Plantation => {
            feat == Feature::Jungle
                || (elev == Elevation::Flat
                    && feat == Feature::None
                    && matches!(terr, Terrain::Grassland | Terrain::Plains))
        }
        Improvement::Camp => feat == Feature::Forest || terr == Terrain::Tundra,
        Improvement::LumberMill => feat == Feature::Forest,
        Improvement::TerraceFarm => elev == Elevation::Hill && feat != Feature::Marsh,
        Improvement::TradingPost => feat != Feature::Marsh && terr != Terrain::Snow,
        Improvement::Fort => true,
        Improvement::OilWell => res == Resource::Oil || matches!(terr, Terrain::Desert | Terrain::Tundra),
        Improvement::PowerPlant => river || elev == Elevation::Hill,
        Improvement::SolarFarm => {
            matches!(terr, Terrain::Desert | Terrain::Plains) && feat == Feature::None
        }
        Improvement::ResearchOutpost => feat != Feature::Marsh,
        Improvement::Airbase => feat != Feature::Marsh,
        Improvement::Arcology => true,
    }
}

/// Check if `civ` can execute `order` on `tile`.
pub fn can_execute_worker_order(
    world: &World,
    rules: &Ruleset,
    civ: CivId,
    tile: TileId,
    order: WorkerOrder,
) -> bool {
    let ti = tile as usize;
    if !world.map.terrain[ti].is_land() || world.map.elevation[ti] == Elevation::Mountain {
        return false;
    }
    let techs = world.civs[civ as usize].tech_bits;
    let owner = world.map.owner[ti];
    match order {
        WorkerOrder::BuildImprovement(imp) => can_build_improvement(world, rules, civ, tile, imp),
        WorkerOrder::BuildRoad => {
            (owner == civ || owner == NO_CIV)
                && world.map.road[ti] == Road::None
                && Ruleset::has_tech(techs, rules.road_tech)
        }
        WorkerOrder::BuildRailroad => {
            (owner == civ || owner == NO_CIV)
                && world.map.road[ti] != Road::Railroad
                && Ruleset::has_tech(techs, rules.railroad_tech)
        }
        WorkerOrder::ChopForest => {
            (owner == civ || owner == NO_CIV)
                && world.cities.tile_city[ti] == INVALID_CITY
                && world.map.feature[ti] == Feature::Forest
                && Ruleset::has_opt_tech(techs, rules.mining_tech)
        }
        WorkerOrder::ClearJungle => {
            (owner == civ || owner == NO_CIV)
                && world.cities.tile_city[ti] == INVALID_CITY
                && world.map.feature[ti] == Feature::Jungle
                && Ruleset::has_opt_tech(techs, rules.bronze_working_tech)
        }
        WorkerOrder::DrainMarsh => {
            (owner == civ || owner == NO_CIV)
                && world.cities.tile_city[ti] == INVALID_CITY
                && world.map.feature[ti] == Feature::Marsh
                && Ruleset::has_opt_tech(techs, rules.masonry_tech)
        }
        WorkerOrder::Repair => {
            (owner == civ || owner == NO_CIV)
                && world.map.improvement[ti] != Improvement::None
                && world.map.pillaged[ti]
        }
    }
}

/// Compute realistic build duration in turns, scaled by GameSpeed and accelerated by Pyramids + Citizenship!
pub fn worker_order_turns(world: &World, rules: &Ruleset, civ: CivId, order: WorkerOrder) -> u8 {
    let base: u8 = match order {
        WorkerOrder::Repair | WorkerOrder::BuildRoad => 1,
        WorkerOrder::BuildRailroad
        | WorkerOrder::ChopForest
        | WorkerOrder::ClearJungle
        | WorkerOrder::DrainMarsh => 2,
        WorkerOrder::BuildImprovement(imp) => match imp {
            Improvement::Farm | Improvement::Pasture | Improvement::Camp | Improvement::TradingPost => 2,
            Improvement::Mine
            | Improvement::Quarry
            | Improvement::Plantation
            | Improvement::LumberMill
            | Improvement::TerraceFarm
            | Improvement::Fort
            | Improvement::OilWell => 3,
            Improvement::PowerPlant
            | Improvement::SolarFarm
            | Improvement::ResearchOutpost
            | Improvement::Airbase
            | Improvement::Arcology => 4,
            Improvement::None => 1,
        },
    };

    let mut scaled: i32 = match world.config.rules.speed {
        GameSpeed::Quick => ((base as i32) * 2 + 1) / 3,
        GameSpeed::Standard => base as i32,
        GameSpeed::Epic => ((base as i32) * 4 + 1) / 3,
        GameSpeed::Marathon => ((base as i32) * 3 + 1) / 2,
    };

    // Great Pyramids wonder speedup (-1 turn!)
    if let Some(pyr) = rules.pyramids_building {
        let mask = 1u64 << pyr;
        for ci in 0..world.cities.len() {
            if world.cities.alive[ci] && world.cities.civ[ci] == civ && (world.cities.buildings[ci] & mask) != 0 {
                scaled -= 1;
                break;
            }
        }
    }

    // Liberty: Citizenship policy speedup (-1 turn!)
    if let Some(cit) = rules.citizenship_policy {
        if has_policy(world.civs[civ as usize].policy_bits, cit) {
            scaled -= 1;
        }
    }

    scaled.clamp(1, 8) as u8
}

/// List every valid WorkerOrder for `unit` at its current tile (for the interactive Mobile Worker Sheet).
pub fn available_worker_orders(world: &World, rules: &Ruleset, unit: UnitId) -> Vec<WorkerOrderInfo> {
    let mut out = Vec::new();
    if !world.units.is_alive(unit) {
        return out;
    }
    let ui = unit as usize;
    if rules.units[world.units.kind[ui] as usize].role != UnitRole::Worker {
        return out;
    }
    let civ = world.units.civ[ui];
    let tile = world.units.pos[ui];
    let ti = tile as usize;
    let techs = world.civs[civ as usize].tech_bits;
    let cur_order = world.map.work_order[ti];
    let cur_rem = world.map.work_turns_left[ti];

    let mut push_order = |order: WorkerOrder, yield_bonus: Yields, note: &'static str| {
        if can_execute_worker_order(world, rules, civ, tile, order) {
            let is_active = world.units.working[ui] && cur_order == Some(order);
            let turns = if cur_order == Some(order) && cur_rem > 0 {
                cur_rem
            } else {
                worker_order_turns(world, rules, civ, order)
            };
            out.push(WorkerOrderInfo {
                order,
                turns,
                yield_bonus,
                note,
                is_active,
            });
        }
    };

    // 1. Repair always shown first if tile is pillaged
    push_order(
        WorkerOrder::Repair,
        Yields::default(),
        "Restores pillaged improvement yields & defense",
    );

    // 2. All 16 realistic tile improvements eligible on this hex
    for &imp in &Improvement::ALL {
        let y = improvement_yields(&world.map, rules, tile, imp, techs);
        push_order(WorkerOrder::BuildImprovement(imp), y, improvement_note(imp));
    }

    // 3. Infrastructure & Terrain Engineering
    push_order(
        WorkerOrder::BuildRoad,
        Yields::default(),
        "2x land movement speed between connected hexes",
    );
    push_order(
        WorkerOrder::BuildRailroad,
        Yields::default(),
        "High-speed rail +15% Production when on city hex!",
    );
    push_order(
        WorkerOrder::ChopForest,
        Yields::default(),
        "Clears Forest & delivers +20 Prod to nearest city!",
    );
    push_order(
        WorkerOrder::ClearJungle,
        Yields::default(),
        "Clears dense Jungle to unlock Farms & Solar Farms",
    );
    push_order(
        WorkerOrder::DrainMarsh,
        Yields::default(),
        "Converts Marsh (-15% Def) into fertile Grassland!",
    );

    out
}

/// Assign `order` to Worker `unit` (starts or resumes multi-turn construction on its tile).
pub fn start_worker_order(
    world: &mut World,
    rules: &Ruleset,
    unit: UnitId,
    order: WorkerOrder,
) -> bool {
    if !world.units.is_alive(unit) {
        return false;
    }
    let ui = unit as usize;
    if rules.units[world.units.kind[ui] as usize].role != UnitRole::Worker {
        return false;
    }
    if world.units.moves_halves[ui] == 0 {
        return false;
    }
    let civ = world.units.civ[ui];
    let tile = world.units.pos[ui];
    if !can_execute_worker_order(world, rules, civ, tile, order) {
        return false;
    }

    let ti = tile as usize;
    let turns = if world.map.work_order[ti] == Some(order) && world.map.work_turns_left[ti] > 0 {
        world.map.work_turns_left[ti]
    } else {
        worker_order_turns(world, rules, civ, order)
    };
    world.map.work_order[ti] = Some(order);
    world.map.work_turns_left[ti] = turns;
    world.units.working[ui] = true;
    world.units.fortified[ui] = false;
    world.units.moves_halves[ui] = 0;

    if world.civs[civ as usize].is_human {
        world.mark_nearby_civs_dirty(tile, 5);
    }
    true
}

/// Advance all active multi-turn worker orders for `civ` at turn end, completing any that reach 0 turns left.
pub fn advance_worker_orders_for_civ(world: &mut World, rules: &Ruleset, civ: CivId) {
    let n_units = world.units.len();
    for u in 0..n_units {
        if !world.units.alive[u] || world.units.civ[u] != civ || !world.units.working[u] {
            continue;
        }
        let tile = world.units.pos[u];
        let ti = tile as usize;
        let Some(order) = world.map.work_order[ti] else {
            world.units.working[u] = false;
            continue;
        };
        if !can_execute_worker_order(world, rules, civ, tile, order) {
            world.map.work_order[ti] = None;
            world.map.work_turns_left[ti] = 0;
            world.units.working[u] = false;
            continue;
        }
        let rem = world.map.work_turns_left[ti].saturating_sub(1);
        world.map.work_turns_left[ti] = rem;
        if rem == 0 {
            complete_worker_order(world, civ, tile, order);
            world.map.work_order[ti] = None;
            world.units.working[u] = false;
        }
    }
}

fn complete_worker_order(world: &mut World, civ: CivId, tile: TileId, order: WorkerOrder) {
    let ti = tile as usize;
    match order {
        WorkerOrder::BuildImprovement(imp) => {
            world.map.improvement[ti] = imp;
            world.map.pillaged[ti] = false;
        }
        WorkerOrder::BuildRoad => {
            if world.map.road[ti] == Road::None {
                world.map.road[ti] = Road::Road;
            }
        }
        WorkerOrder::BuildRailroad => {
            world.map.road[ti] = Road::Railroad;
        }
        WorkerOrder::ChopForest => {
            world.map.feature[ti] = Feature::None;
            // Deliver +20 Production (scaled by GameSpeed) to nearest friendly city within 5 hexes
            let bonus = Ruleset::scaled_cost(20, world.config.rules.speed);
            let mut best_city: Option<(CityId, u16)> = None;
            for c in 0..world.cities.len() {
                if world.cities.alive[c] && world.cities.civ[c] == civ {
                    let d = world.map.distance(tile, world.cities.pos[c]);
                    if d <= 5 && best_city.map_or(true, |(_, bd)| d < bd) {
                        best_city = Some((c as CityId, d));
                    }
                }
            }
            if let Some((cid, _)) = best_city {
                world.cities.prod_stored[cid as usize] += bonus;
            }
        }
        WorkerOrder::ClearJungle => {
            world.map.feature[ti] = Feature::None;
        }
        WorkerOrder::DrainMarsh => {
            world.map.feature[ti] = Feature::None;
            world.map.terrain[ti] = Terrain::Grassland;
        }
        WorkerOrder::Repair => {
            world.map.pillaged[ti] = false;
        }
    }
}

/// Check if combat unit `unit` can pillage an enemy/neutral unpillaged improvement on its current tile.
pub fn can_pillage_tile(world: &World, rules: &Ruleset, unit: UnitId) -> bool {
    if !world.units.is_alive(unit) {
        return false;
    }
    let ui = unit as usize;
    let role = rules.units[world.units.kind[ui] as usize].role;
    if role.is_civilian() || role.is_air() || role.is_naval() {
        return false;
    }
    if world.units.moves_halves[ui] < 2 {
        return false;
    }
    let civ = world.units.civ[ui];
    let ti = world.units.pos[ui] as usize;
    if world.map.improvement[ti] == Improvement::None || world.map.pillaged[ti] {
        return false;
    }
    let owner = world.map.owner[ti];
    if owner == civ {
        return false;
    }
    owner == NO_CIV || world.diplo.is_at_war(civ, owner)
}

/// Pillage the improvement on `unit`'s tile: disables improvement, grants +20 Gold & heals +20 HP (costs 1 move).
pub fn pillage_tile(world: &mut World, rules: &Ruleset, unit: UnitId) -> bool {
    if !can_pillage_tile(world, rules, unit) {
        return false;
    }
    let ui = unit as usize;
    let civ = world.units.civ[ui];
    let pos = world.units.pos[ui];
    let ti = pos as usize;

    world.map.pillaged[ti] = true;
    world.map.work_order[ti] = None;
    world.map.work_turns_left[ti] = 0;
    world.units.moves_halves[ui] = world.units.moves_halves[ui].saturating_sub(2);
    world.units.fortified[ui] = false;
    world.units.hp[ui] = (world.units.hp[ui] + 20).min(100);
    world.civs[civ as usize].gold += 20;

    if world.civs[civ as usize].is_human {
        world.mark_nearby_civs_dirty(pos, 6);
    }
    true
}

/// Zero-allocation heuristic picking the smartest realistic WorkerOrder for AI & Auto-Work workers!
pub fn best_ai_worker_order(
    world: &World,
    rules: &Ruleset,
    civ: CivId,
    tile: TileId,
) -> Option<WorkerOrder> {
    let ti = tile as usize;
    if !world.map.terrain[ti].is_land() || world.map.elevation[ti] == Elevation::Mountain {
        return None;
    }
    if world.map.owner[ti] != civ {
        return None;
    }

    // 1. Always repair pillaged friendly improvements immediately
    if world.map.improvement[ti] != Improvement::None && world.map.pillaged[ti] {
        return Some(WorkerOrder::Repair);
    }

    // 2. City tile: upgrade Road -> Railroad when available for +15% city Production!
    if world.cities.tile_city[ti] != INVALID_CITY {
        if can_execute_worker_order(world, rules, civ, tile, WorkerOrder::BuildRailroad) {
            return Some(WorkerOrder::BuildRailroad);
        }
        return None;
    }

    // 3. Unimproved tile: pick best realistic improvement for this terrain, resource, and era
    if world.map.improvement[ti] == Improvement::None {
        // Drain Marsh first so it becomes fertile Grassland
        if world.map.feature[ti] == Feature::Marsh
            && can_execute_worker_order(world, rules, civ, tile, WorkerOrder::DrainMarsh)
        {
            return Some(WorkerOrder::DrainMarsh);
        }

        // Priority candidate order for resources, era wonders, and terrain specialties
        let candidates: [Improvement; 14] = match world.map.resource[ti] {
            Resource::Wheat => [
                Improvement::Arcology,
                Improvement::Farm,
                Improvement::Pasture,
                Improvement::ResearchOutpost,
                Improvement::Plantation,
                Improvement::SolarFarm,
                Improvement::TradingPost,
                Improvement::Mine,
                Improvement::Quarry,
                Improvement::LumberMill,
                Improvement::TerraceFarm,
                Improvement::Camp,
                Improvement::OilWell,
                Improvement::PowerPlant,
            ],
            Resource::Cattle | Resource::Horses => [
                Improvement::Arcology,
                Improvement::Pasture,
                Improvement::Farm,
                Improvement::ResearchOutpost,
                Improvement::SolarFarm,
                Improvement::Plantation,
                Improvement::TradingPost,
                Improvement::Mine,
                Improvement::Quarry,
                Improvement::LumberMill,
                Improvement::TerraceFarm,
                Improvement::Camp,
                Improvement::OilWell,
                Improvement::PowerPlant,
            ],
            Resource::Oil => [
                Improvement::OilWell,
                Improvement::Arcology,
                Improvement::SolarFarm,
                Improvement::ResearchOutpost,
                Improvement::TradingPost,
                Improvement::PowerPlant,
                Improvement::Mine,
                Improvement::Quarry,
                Improvement::Farm,
                Improvement::Pasture,
                Improvement::Plantation,
                Improvement::Camp,
                Improvement::LumberMill,
                Improvement::TerraceFarm,
            ],
            Resource::Gold | Resource::Gems => [
                Improvement::Quarry,
                Improvement::Mine,
                Improvement::Arcology,
                Improvement::TerraceFarm,
                Improvement::PowerPlant,
                Improvement::ResearchOutpost,
                Improvement::TradingPost,
                Improvement::Farm,
                Improvement::Pasture,
                Improvement::Plantation,
                Improvement::Camp,
                Improvement::LumberMill,
                Improvement::OilWell,
                Improvement::SolarFarm,
            ],
            Resource::Iron | Resource::Uranium => [
                Improvement::Mine,
                Improvement::Arcology,
                Improvement::Quarry,
                Improvement::PowerPlant,
                Improvement::TerraceFarm,
                Improvement::ResearchOutpost,
                Improvement::TradingPost,
                Improvement::Farm,
                Improvement::Pasture,
                Improvement::Plantation,
                Improvement::Camp,
                Improvement::LumberMill,
                Improvement::OilWell,
                Improvement::SolarFarm,
            ],
            Resource::None => [
                Improvement::Arcology,
                Improvement::SolarFarm,
                Improvement::PowerPlant,
                Improvement::ResearchOutpost,
                Improvement::TerraceFarm,
                Improvement::LumberMill,
                Improvement::Plantation,
                Improvement::Mine,
                Improvement::Quarry,
                Improvement::Farm,
                Improvement::Pasture,
                Improvement::Camp,
                Improvement::OilWell,
                Improvement::TradingPost,
            ],
        };

        let techs = world.civs[civ as usize].tech_bits;
        let mut best: Option<(Improvement, i32)> = None;
        for &imp in &candidates {
            if can_build_improvement(world, rules, civ, tile, imp) {
                let y = improvement_yields(&world.map, rules, tile, imp, techs);
                let score = (y.food as i32) * 14
                    + (y.prod as i32) * 15
                    + (y.science as i32) * 16
                    + (y.gold as i32) * 10;
                if best.map_or(true, |(_, bs)| score > bs) {
                    best = Some((imp, score));
                }
            }
        }
        if let Some((imp, _)) = best {
            return Some(WorkerOrder::BuildImprovement(imp));
        }
    }

    // 4. Upgrade outdated improvements to Future Arcology Dome if unlocked
    if world.map.improvement[ti] != Improvement::Arcology
        && !world.map.improvement[ti].is_fortification()
        && can_build_improvement(world, rules, civ, tile, Improvement::Arcology)
    {
        return Some(WorkerOrder::BuildImprovement(Improvement::Arcology));
    }

    None
}
