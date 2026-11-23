pub mod scratch;
pub mod speculative;
pub mod theater;

use crate::ai::scratch::with_tls_scratch;
use crate::ai::theater::{compute_land_flow_field, compute_threat_field, TheaterField};
use crate::combat::{attack_city, attack_unit, can_merge_units, city_bombard, find_merge_partner, merge_units, preview_unit_combat};
use crate::economy::{best_improvement_for_tile, pick_next_available_tech, tile_yields};
use crate::ruleset::Ruleset;
use crate::types::{
    CityId, CivId, DiploStatus, Elevation, Improvement, ProductionItem, TechId, TileId,
    UnitFormation, UnitId, UnitRole, INVALID_CITY, INVALID_TILE, INVALID_UNIT, NO_CIV,
};
use crate::world::World;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum DiploOrder {
    DeclareWar(CivId),
    MakePeace(CivId),
}

/// Complete read-only plan produced for a single AI civilization during Phase 1 (Parallel Planning).
/// Executed deterministically in `CivId` order during Phase 2.
#[derive(Debug, Clone)]
pub struct CivPlan {
    pub civ: CivId,
    pub diplo_orders: Vec<DiploOrder>,
    pub tech_choice: Option<TechId>,
    pub city_orders: Vec<(CityId, ProductionItem)>,
    pub attack_flow: Option<TheaterField>,
    pub defense_flow: Option<TheaterField>,
    pub explore_flow: Option<TheaterField>,
    pub worker_flow: Option<TheaterField>,
    pub settle_flow: Option<TheaterField>,
    pub settle_target: Option<TileId>,
    pub threat_dist: Vec<u8>,
}

/// Phase 1 (Read-Only, Pure, Parallelizable): compute `CivPlan` for one AI civilization.
pub fn plan_civ(world: &World, rules: &Ruleset, civ: CivId) -> CivPlan {
    let cs = &world.civs[civ as usize];
    let mut diplo_orders = Vec::new();
    let mut city_orders = Vec::new();

    if !cs.alive {
        return CivPlan {
            civ,
            diplo_orders,
            tech_choice: None,
            city_orders,
            attack_flow: None,
            defense_flow: None,
            explore_flow: None,
            worker_flow: None,
            settle_flow: None,
            settle_target: None,
            threat_dist: Vec::new(),
        };
    }

    // 1. Count civ cities, units, roles, and evaluate war status
    let mut own_cities: Vec<CityId> = Vec::new();
    let mut total_combat_units = 0u32;
    let mut num_settlers = 0u32;
    let mut num_workers = 0u32;
    let mut num_ranged = 0u32;
    let mut num_siege = 0u32;
    let mut num_melee_cav = 0u32;
    let mut own_strength = 0i32;

    for c in 0..world.cities.len() {
        if world.cities.alive[c] && world.cities.civ[c] == civ {
            own_cities.push(c as CityId);
        }
    }
    for u in 0..world.units.len() {
        if !world.units.alive[u] || world.units.civ[u] != civ {
            continue;
        }
        let def = &rules.units[world.units.kind[u] as usize];
        match def.role {
            UnitRole::Settler => num_settlers += 1,
            UnitRole::Worker => num_workers += 1,
            UnitRole::Ranged => {
                num_ranged += 1;
                total_combat_units += 1;
            }
            UnitRole::Siege => {
                num_siege += 1;
                total_combat_units += 1;
            }
            UnitRole::Melee | UnitRole::Cavalry | UnitRole::Recon | UnitRole::Naval | UnitRole::Air => {
                num_melee_cav += 1;
                total_combat_units += 1;
            }
        }
        if def.role.is_combat() {
            own_strength += def.strength.max(def.ranged_strength) as i32
                + world.units.formation[u].strength_bonus();
        }
    }

    // 2. Diplomacy Planning (war declarations & peace treaties)
    let mut at_war_count = 0usize;
    for other in 0..world.civs.len() {
        let oid = other as CivId;
        if oid == civ || !world.civs[other].alive {
            continue;
        }
        if world.diplo.is_at_war(civ, oid) {
            at_war_count += 1;
            let wt = world.diplo.war_turns(civ, oid);
            if wt >= 24 && (own_strength < world.civs[other].score / 2 || wt >= 45) {
                diplo_orders.push(DiploOrder::MakePeace(oid));
            }
        }
    }

    if at_war_count == 0 && world.turn >= 18 && total_combat_units >= 4 && !own_cities.is_empty() {
        let mut best_target: Option<(CivId, i32)> = None;
        for other in 0..world.civs.len() {
            let oid = other as CivId;
            if oid == civ || !world.civs[other].alive || !world.diplo.has_met(civ, oid) {
                continue;
            }
            if world.diplo.peace_cooldown(civ, oid) > 0 {
                continue;
            }
            let mut min_d = u16::MAX;
            for &oc in &own_cities {
                for ec in 0..world.cities.len() {
                    if world.cities.alive[ec] && world.cities.civ[ec] == oid {
                        let d = world.map.distance(world.cities.pos[oc as usize], world.cities.pos[ec]);
                        if d < min_d {
                            min_d = d;
                        }
                    }
                }
            }
            if min_d <= 16 {
                let desire = (cs.aggression as i32) * 2
                    + (18 - min_d as i32) * 8
                    + (cs.score - world.civs[other].score) / 3
                    - 115;
                if desire > 0 && best_target.map_or(true, |(_, bs)| desire > bs) {
                    best_target = Some((oid, desire));
                }
            }
        }
        if let Some((target_civ, _)) = best_target {
            diplo_orders.push(DiploOrder::DeclareWar(target_civ));
            at_war_count += 1;
        }
    }

    // 3. Tech Selection
    let tech_choice = if cs.current_tech.is_some() {
        cs.current_tech
    } else {
        pick_ai_tech(cs.tech_bits, cs.science_bias, cs.aggression, rules)
    };

    // 4. City Site Scoring for Settlers
    let desired_cities = ((cs.expansion as usize) / 13).clamp(3, 6);
    let need_settler = own_cities.len() + (num_settlers as usize) < desired_cities
        && world.turn < 240
        && own_cities.len() < 6;

    let settle_target = if num_settlers > 0 {
        find_best_settle_tile(world, civ, &own_cities)
    } else {
        None
    };

    // 5. City Production Planning (respects Supply Cap, Unique Wonders, Air Power & Science Victory)
    let at_supply_cap = cs.supply_used >= cs.supply_cap;
    let near_supply_cap = cs.supply_used + 1 >= cs.supply_cap;
    let mut planned_settlers = num_settlers;
    let mut planned_workers = num_workers;

    for &cid in &own_cities {
        let ci = cid as usize;
        if world.cities.producing[ci].is_some() {
            continue;
        }
        let bbits = world.cities.buildings[ci];
        let mut chosen: Option<ProductionItem> = None;

        if let Some(sc_b) = rules.space_center_building {
            if rules.can_build_building(cs.tech_bits, bbits, world.wonders_built, sc_b) {
                chosen = Some(ProductionItem::Building(sc_b));
            }
        }

        if chosen.is_none()
            && need_settler
            && planned_settlers == 0
            && world.cities.pop[ci] >= 2
        {
            chosen = Some(ProductionItem::Unit(rules.settler_kind));
            planned_settlers += 1;
        }

        if chosen.is_none() && (planned_workers as usize) < own_cities.len().min(4) {
            chosen = Some(ProductionItem::Unit(rules.worker_kind));
            planned_workers += 1;
        }

        let want_military = !at_supply_cap
            && (total_combat_units < cs.supply_cap as u32
                && (total_combat_units < (own_cities.len() as u32 * 3 + 3)
                    || (at_war_count > 0 && !near_supply_cap)));

        if chosen.is_none() && want_military {
            let role = if rules.best_unit_for_role(cs.tech_bits, UnitRole::Air).is_some()
                && (cid as usize + world.turn as usize) % 7 == 0
            {
                UnitRole::Air
            } else if num_melee_cav <= num_ranged + num_siege {
                if rules.best_unit_for_role(cs.tech_bits, UnitRole::Cavalry).is_some()
                    && (cid as usize + world.turn as usize) % 3 == 0
                {
                    UnitRole::Cavalry
                } else {
                    UnitRole::Melee
                }
            } else if num_siege * 3 < num_ranged + 1
                && rules.best_unit_for_role(cs.tech_bits, UnitRole::Siege).is_some()
            {
                UnitRole::Siege
            } else if rules.best_unit_for_role(cs.tech_bits, UnitRole::Ranged).is_some() {
                UnitRole::Ranged
            } else {
                UnitRole::Melee
            };
            if let Some(uk) = rules.best_unit_for_role(cs.tech_bits, role) {
                chosen = Some(ProductionItem::Unit(uk));
            }
        }

        if chosen.is_none() {
            let mut best_b: Option<(u8, i32)> = None;
            for b in &rules.buildings {
                if rules.can_build_building(cs.tech_bits, bbits, world.wonders_built, b.id) {
                    // Only capitals / strong cities attempt expensive World Wonders
                    if b.is_wonder && !world.cities.is_capital[ci] && world.cities.pop[ci] < 5 {
                        continue;
                    }
                    let mut val = (b.science as i32) * 18
                        + (b.science_pct as i32) * 3
                        + (b.production as i32) * 16
                        + (b.production_pct as i32) * 3
                        + (b.food as i32) * 14
                        + (b.gold as i32) * 10
                        + (b.supply as i32) * if near_supply_cap { 35 } else { 10 }
                        + (b.defense as i32) * if at_war_count > 0 { 12 } else { 4 }
                        + (b.culture as i32) * 10;
                    if b.science_per_pop_half {
                        val += (world.cities.pop[ci] as i32) * 8;
                    }
                    if b.is_wonder && at_war_count == 0 {
                        val += 60;
                    }
                    if b.wins_science {
                        val += 5000;
                    }
                    if best_b.map_or(true, |(_, bv)| val > bv) {
                        best_b = Some((b.id, val));
                    }
                }
            }
            if let Some((bid, _)) = best_b {
                chosen = Some(ProductionItem::Building(bid));
            } else if !at_supply_cap {
                if let Some(uk) = rules.best_unit_for_role(cs.tech_bits, UnitRole::Melee) {
                    chosen = Some(ProductionItem::Unit(uk));
                }
            }
        }

        if let Some(item) = chosen {
            city_orders.push((cid, item));
        }
    }

    // 6. Compute O(V) Threat Field & Theater Flow-Fields in thread-local zero-alloc scratch arena!
    let (threat_dist, attack_flow, defense_flow, explore_flow, worker_flow, settle_flow) =
        with_tls_scratch(|scratch| {
            let threat = compute_threat_field(world, civ, scratch);

            // Attack theater sources: enemy cities (and enemy units if no city known)
            let mut attack_sources: Vec<TileId> = Vec::with_capacity(32);
            let mut war_civs = 0u64;
            for o in 0..world.civs.len() {
                if world.diplo.is_at_war(civ, o as CivId) {
                    war_civs |= 1u64 << o;
                }
            }
            for order in &diplo_orders {
                if let DiploOrder::DeclareWar(target) = *order {
                    war_civs |= 1u64 << target;
                }
            }
            if war_civs != 0 {
                for c in 0..world.cities.len() {
                    if world.cities.alive[c] && (war_civs & (1u64 << world.cities.civ[c])) != 0 {
                        attack_sources.push(world.cities.pos[c]);
                    }
                }
                if attack_sources.is_empty() {
                    for u in 0..world.units.len() {
                        if world.units.alive[u] && (war_civs & (1u64 << world.units.civ[u])) != 0 {
                            attack_sources.push(world.units.pos[u]);
                        }
                    }
                }
            }
            let attack_flow = compute_land_flow_field(world, civ, &attack_sources, 44, scratch);

            // Defense theater sources: threatened friendly cities (enemy within 5 tiles)
            let mut defense_sources: Vec<TileId> = Vec::with_capacity(8);
            for &cid in &own_cities {
                let pos = world.cities.pos[cid as usize];
                if threat[pos as usize] <= 5 {
                    defense_sources.push(pos);
                }
            }
            let defense_flow = compute_land_flow_field(world, civ, &defense_sources, 26, scratch);

            // Explore theater sources: passable tiles adjacent to unexplored fog (only if civ has idle/recon units)
            let explore_flow = if war_civs == 0 && total_combat_units > 0 {
                let mut frontier = Vec::with_capacity(64);
                for t in 0..world.map.num_tiles {
                    if !world.map.terrain[t].is_land() || world.map.elevation[t] == Elevation::Mountain {
                        continue;
                    }
                    let tid = t as TileId;
                    if !world.vision.is_explored(civ, tid) {
                        // Seed frontier tiles bordering explored tiles
                        for &nb in world.map.neighbors_of(tid) {
                            if nb != INVALID_TILE && world.vision.is_explored(civ, nb) {
                                frontier.push(tid);
                                break;
                            }
                        }
                    }
                }
                compute_land_flow_field(world, civ, &frontier, 30, scratch)
            } else {
                None
            };

            // Worker flow-field: unimproved owned tiles near own cities
            let worker_flow = if num_workers > 0 {
                let mut work_tiles = Vec::with_capacity(32);
                for t in 0..world.map.num_tiles {
                    if world.map.owner[t] == civ
                        && world.map.improvement[t] == Improvement::None
                        && world.cities.tile_city[t] == INVALID_CITY
                        && best_improvement_for_tile(&world.map, t as TileId) != Improvement::None
                        && threat[t] > 3
                    {
                        work_tiles.push(t as TileId);
                    }
                }
                compute_land_flow_field(world, civ, &work_tiles, 24, scratch)
            } else {
                None
            };

            let settle_flow = settle_target
                .and_then(|st| compute_land_flow_field(world, civ, &[st], 36, scratch));

            (threat, attack_flow, defense_flow, explore_flow, worker_flow, settle_flow)
        });

    CivPlan {
        civ,
        diplo_orders,
        tech_choice,
        city_orders,
        attack_flow,
        defense_flow,
        explore_flow,
        worker_flow,
        settle_flow,
        settle_target,
        threat_dist,
    }
}

/// Phase 1 across ALL alive AI civilizations in parallel using Rayon!
/// Because `plan_civ` only reads `&World`, results are 100% independent of thread scheduling.
pub fn plan_all_ai_parallel(world: &World, rules: &Ruleset) -> Vec<CivPlan> {
    let civ_ids: Vec<CivId> = world
        .civs
        .iter()
        .filter(|c| c.alive && !c.is_human)
        .map(|c| c.id)
        .collect();
    civ_ids
        .par_iter()
        .map(|&cid| plan_civ(world, rules, cid))
        .collect()
}

/// Phase 2 (Deterministic Execution): execute a single civ's precomputed `CivPlan`.
pub fn execute_civ_plan(world: &mut World, rules: &Ruleset, plan: &CivPlan) {
    let civ = plan.civ;
    if !world.civs[civ as usize].alive {
        return;
    }

    // 1. Diplomacy orders
    for &order in &plan.diplo_orders {
        match order {
            DiploOrder::DeclareWar(target) => {
                if world.civs[target as usize].alive
                    && world.diplo.has_met(civ, target)
                    && world.diplo.peace_cooldown(civ, target) == 0
                    && !world.diplo.is_at_war(civ, target)
                {
                    world.diplo.set_status(civ, target, DiploStatus::War);
                }
            }
            DiploOrder::MakePeace(target) => {
                if world.civs[target as usize].alive && world.diplo.is_at_war(civ, target) {
                    world.diplo.set_status(civ, target, DiploStatus::Peace);
                }
            }
        }
    }

    // 2. Tech & City production orders
    if world.civs[civ as usize].current_tech.is_none() {
        world.civs[civ as usize].current_tech = plan.tech_choice;
    }
    for &(cid, item) in &plan.city_orders {
        let ci = cid as usize;
        if world.cities.is_alive(cid) && world.cities.civ[ci] == civ && world.cities.producing[ci].is_none() {
            world.cities.producing[ci] = Some(item);
        }
    }

    // 3. City ranged bombardments against enemies within 2 hexes
    let mut ring = Vec::with_capacity(19);
    for ci in 0..world.cities.len() {
        if !world.cities.alive[ci] || world.cities.civ[ci] != civ || world.cities.has_bombarded[ci] {
            continue;
        }
        let cpos = world.cities.pos[ci];
        world.map.collect_tiles_in_radius(cpos, 2, &mut ring);
        let mut best_target: Option<(UnitId, i16)> = None;
        for &t in &ring {
            let u = world.units.tile_mil_unit[t as usize];
            if u != INVALID_UNIT && world.diplo.is_at_war(civ, world.units.civ[u as usize]) {
                let hp = world.units.hp[u as usize];
                if best_target.map_or(true, |(_, bhp)| hp < bhp) {
                    best_target = Some((u, hp));
                }
            }
        }
        if let Some((target, _)) = best_target {
            city_bombard(world, rules, ci as CityId, target);
        }
    }

    // 4. Collect this civ's alive unit IDs deterministically
    let mut my_units: Vec<UnitId> = Vec::new();
    for u in 0..world.units.len() {
        if world.units.alive[u] && world.units.civ[u] == civ {
            my_units.push(u as UnitId);
        }
    }

    // 4a. Pillar 2: Automatic Corps & Army consolidation!
    let techs = world.civs[civ as usize].tech_bits;
    if Ruleset::has_tech(techs, rules.corps_tech) {
        for &uid in &my_units {
            if !world.units.is_alive(uid) {
                continue;
            }
            if world.units.formation[uid as usize] == UnitFormation::Army {
                continue;
            }
            if let Some(partner) = find_merge_partner(world, rules, uid) {
                // Keep the higher-formation or higher-XP unit as keeper
                let (keeper, donor) = if world.units.formation[uid as usize] >= world.units.formation[partner as usize] {
                    (uid, partner)
                } else {
                    (partner, uid)
                };
                if can_merge_units(world, rules, keeper, donor).is_some() {
                    merge_units(world, rules, keeper, donor);
                }
            }
        }
    }

    // 4b. Execute each unit using O(1) Theater Flow-Field lookups + local tactical checks
    for &uid in &my_units {
        if !world.units.is_alive(uid) {
            continue;
        }
        let role = rules.units[world.units.kind[uid as usize] as usize].role;
        match role {
            UnitRole::Settler => execute_settler(world, rules, uid, plan),
            UnitRole::Worker => execute_worker(world, rules, uid, plan),
            UnitRole::Naval => execute_naval(world, rules, uid),
            UnitRole::Air => {
                // Air units launch long-range strikes ignoring LOS (and stay based safely when healthy)
                if world.units.hp[uid as usize] >= 40 {
                    try_local_attack(world, rules, uid);
                }
            }
            _ => execute_land_combat(world, rules, uid, plan),
        }
    }
}

fn execute_settler(world: &mut World, rules: &Ruleset, uid: UnitId, plan: &CivPlan) {
    let civ = world.units.civ[uid as usize];
    let pos = world.units.pos[uid as usize];
    if plan.settle_target == Some(pos) {
        if world.found_city(rules, civ, pos).is_some() {
            world.despawn_unit(rules, uid);
            return;
        }
    }
    if let Some(ref flow) = plan.settle_flow {
        follow_flow_field(world, rules, uid, flow, true);
    }
    if world.units.is_alive(uid) {
        let new_pos = world.units.pos[uid as usize];
        if plan.settle_target == Some(new_pos) || can_found_here_good(world, new_pos) {
            if world.found_city(rules, civ, new_pos).is_some() {
                world.despawn_unit(rules, uid);
            }
        }
    }
}

fn execute_worker(world: &mut World, rules: &Ruleset, uid: UnitId, plan: &CivPlan) {
    let civ = world.units.civ[uid as usize];
    let pos = world.units.pos[uid as usize];
    let pi = pos as usize;

    // Build improvement immediately if standing on an unimproved owned tile
    if world.map.owner[pi] == civ
        && world.map.improvement[pi] == Improvement::None
        && world.cities.tile_city[pi] == INVALID_CITY
    {
        let imp = best_improvement_for_tile(&world.map, pos);
        if imp != Improvement::None {
            world.map.improvement[pi] = imp;
            world.units.moves_halves[uid as usize] = 0;
            return;
        }
    }
    if let Some(ref flow) = plan.worker_flow {
        follow_flow_field(world, rules, uid, flow, true);
        if world.units.is_alive(uid) && world.units.moves_halves[uid as usize] > 0 {
            let np = world.units.pos[uid as usize];
            let npi = np as usize;
            if world.map.owner[npi] == civ
                && world.map.improvement[npi] == Improvement::None
                && world.cities.tile_city[npi] == INVALID_CITY
            {
                let imp = best_improvement_for_tile(&world.map, np);
                if imp != Improvement::None {
                    world.map.improvement[npi] = imp;
                    world.units.moves_halves[uid as usize] = 0;
                }
            }
        }
    }
}

fn execute_land_combat(world: &mut World, rules: &Ruleset, uid: UnitId, plan: &CivPlan) {
    let civ = world.units.civ[uid as usize];
    let hp = world.units.hp[uid as usize];

    // 1. Try attacking immediately if an enemy unit or city is already in range
    if try_local_attack(world, rules, uid) {
        return;
    }

    // 2. Wounded units fortify/heal when safe or near friendly cities
    if hp < 45 {
        world.units.fortified[uid as usize] = true;
        return;
    }

    // 3. Pick active Theater Flow-Field in O(1): Defense > Attack > Explore
    let pos = world.units.pos[uid as usize];
    let chosen_flow = if let Some(ref df) = plan.defense_flow {
        if df.dist_at(pos) <= 18 {
            Some(df)
        } else {
            plan.attack_flow.as_ref().or(plan.explore_flow.as_ref())
        }
    } else {
        plan.attack_flow.as_ref().or(plan.explore_flow.as_ref())
    };

    if let Some(flow) = chosen_flow {
        follow_flow_field(world, rules, uid, flow, false);
    }

    // 4. After moving, check if an attack or Corps/Army merge is now available!
    if world.units.is_alive(uid) && !world.units.has_attacked[uid as usize] {
        if !try_local_attack(world, rules, uid) {
            if Ruleset::has_tech(world.civs[civ as usize].tech_bits, rules.corps_tech) {
                if let Some(partner) = find_merge_partner(world, rules, uid) {
                    if can_merge_units(world, rules, uid, partner).is_some() {
                        merge_units(world, rules, uid, partner);
                        return;
                    }
                }
            }
            if world.units.is_alive(uid) && world.units.moves_halves[uid as usize] > 0 {
                world.units.fortified[uid as usize] = true;
            }
        }
    }
}

fn execute_naval(world: &mut World, rules: &Ruleset, uid: UnitId) {
    if try_local_attack(world, rules, uid) {
        return;
    }
    let civ = world.units.civ[uid as usize];
    for _ in 0..4 {
        if !world.units.is_alive(uid) || world.units.moves_halves[uid as usize] < 2 {
            break;
        }
        let pos = world.units.pos[uid as usize];
        let mut best_nb = INVALID_TILE;
        let mut best_score = i32::MIN;
        for &nb in world.map.neighbors_of(pos) {
            if nb == INVALID_TILE {
                continue;
            }
            let ni = nb as usize;
            if !world.map.terrain[ni].is_water() || world.units.tile_mil_unit[ni] != INVALID_UNIT {
                continue;
            }
            let mut s = if !world.vision.is_explored(civ, nb) { 20 } else { 0 };
            for &nb2 in world.map.neighbors_of(nb) {
                if nb2 != INVALID_TILE {
                    let u2 = world.units.tile_mil_unit[nb2 as usize];
                    if u2 != INVALID_UNIT && world.diplo.is_at_war(civ, world.units.civ[u2 as usize]) {
                        s += 40;
                    }
                }
            }
            if s > best_score {
                best_score = s;
                best_nb = nb;
            }
        }
        if best_nb != INVALID_TILE {
            world.move_unit_step(rules, uid, best_nb, 2);
            if try_local_attack(world, rules, uid) {
                break;
            }
        } else {
            break;
        }
    }
}

/// Follow a precomputed O(V) TheaterFlowField in O(1) per step (with local sidestepping around friendly traffic).
fn follow_flow_field(world: &mut World, rules: &Ruleset, uid: UnitId, flow: &TheaterField, is_civ: bool) {
    let civ = world.units.civ[uid as usize];
    for _step in 0..6 {
        if !world.units.is_alive(uid) || world.units.moves_halves[uid as usize] == 0 {
            break;
        }
        let pos = world.units.pos[uid as usize];
        let cur_dist = flow.dist_at(pos);
        if cur_dist == 0 || cur_dist == u16::MAX {
            break;
        }
        let primary = flow.step_from(pos);
        let occ = if is_civ {
            &world.units.tile_civ_unit
        } else {
            &world.units.tile_mil_unit
        };

        // Pick primary flow hop if free, else best unoccupied neighbor reducing flow distance
        let mut chosen = INVALID_TILE;
        if primary != INVALID_TILE
            && primary != pos
            && occ[primary as usize] == INVALID_UNIT
            && can_step_land(world, civ, primary)
        {
            chosen = primary;
        } else {
            let mut best_d = cur_dist;
            for &nb in world.map.neighbors_of(pos) {
                if nb == INVALID_TILE || occ[nb as usize] != INVALID_UNIT || !can_step_land(world, civ, nb) {
                    continue;
                }
                let nd = flow.dist_at(nb);
                if nd < best_d {
                    best_d = nd;
                    chosen = nb;
                }
            }
        }
        if chosen == INVALID_TILE {
            break;
        }
        let cost = world.map.land_enter_cost_halves(chosen, world.map.road[pos as usize]);
        if cost == u8::MAX {
            break;
        }
        // Unciv/Civ rule: a unit with >=1 movement left can always enter an adjacent passable tile
        let spend = cost.min(world.units.moves_halves[uid as usize]);
        if !world.move_unit_step(rules, uid, chosen, spend) {
            break;
        }
    }
}

#[inline]
fn can_step_land(world: &World, civ: CivId, tile: TileId) -> bool {
    let ti = tile as usize;
    if !world.map.terrain[ti].is_land() || world.map.elevation[ti] == Elevation::Mountain {
        return false;
    }
    let cid = world.cities.tile_city[ti];
    if cid != INVALID_CITY && world.cities.civ[cid as usize] != civ {
        return false;
    }
    true
}

/// Check for the highest-value enemy unit or city currently in attack range and strike it.
fn try_local_attack(world: &mut World, rules: &Ruleset, uid: UnitId) -> bool {
    if !world.units.is_alive(uid) {
        return false;
    }
    let ui = uid as usize;
    if world.units.has_attacked[ui] || world.units.moves_halves[ui] == 0 {
        return false;
    }
    let civ = world.units.civ[ui];
    let pos = world.units.pos[ui];
    let def = &rules.units[world.units.kind[ui] as usize];
    if def.role.is_civilian() {
        return false;
    }
    let range = def.range.max(1);
    let is_air = def.role.is_air();

    let mut ring = Vec::with_capacity(128);
    world.map.collect_tiles_in_radius(pos, range, &mut ring);

    let mut best_city: Option<(CityId, i32)> = None;
    let mut best_unit: Option<(UnitId, i32)> = None;

    for &t in &ring {
        if t == pos {
            continue;
        }
        let dist = world.map.distance(pos, t);
        if !is_air && dist > 1 && !world.map.has_los(pos, t) {
            continue;
        }
        let cid = world.cities.tile_city[t as usize];
        if cid != INVALID_CITY && world.diplo.is_at_war(civ, world.cities.civ[cid as usize]) {
            let hp = world.cities.hp[cid as usize] as i32;
            let mut score = 120 - hp / 2 + (def.city_attack_bonus as i32);
            if hp <= 35 && def.role.can_capture_city() && dist == 1 {
                score += 200; // Prioritize capturing low-HP cities!
            }
            if best_city.map_or(true, |(_, bs)| score > bs) {
                best_city = Some((cid, score));
            }
        }
        let enemy_u = world.units.tile_mil_unit[t as usize];
        if enemy_u != INVALID_UNIT && world.diplo.is_at_war(civ, world.units.civ[enemy_u as usize]) {
            let prev = preview_unit_combat(world, rules, uid, enemy_u);
            let kills = prev.defender_dmg >= world.units.hp[enemy_u as usize];
            let suicide = prev.attacker_dmg >= world.units.hp[ui];
            let mut score = (prev.defender_dmg as i32) * 3 - (prev.attacker_dmg as i32) * 2;
            if kills {
                score += 85;
            }
            if suicide && !kills {
                score -= 100;
            }
            if score > 5 && best_unit.map_or(true, |(_, bs)| score > bs) {
                best_unit = Some((enemy_u, score));
            }
        }
    }

    match (best_city, best_unit) {
        (Some((cid, cs)), Some((_, us))) if cs >= us => attack_city(world, rules, uid, cid),
        (Some((cid, _)), None) => attack_city(world, rules, uid, cid),
        (_, Some((eid, _))) => attack_unit(world, rules, uid, eid),
        (None, None) => false,
    }
}

fn can_found_here_good(world: &World, tile: TileId) -> bool {
    let ti = tile as usize;
    if !world.map.terrain[ti].is_land()
        || world.map.elevation[ti] == Elevation::Mountain
        || world.map.owner[ti] != NO_CIV
    {
        return false;
    }
    for c in 0..world.cities.len() {
        if world.cities.alive[c] && world.map.distance(tile, world.cities.pos[c]) < 4 {
            return false;
        }
    }
    true
}

fn find_best_settle_tile(world: &World, civ: CivId, own_cities: &[CityId]) -> Option<TileId> {
    let mut best: Option<(TileId, i32)> = None;
    for t in 0..world.map.num_tiles {
        let tid = t as TileId;
        if !world.vision.is_explored(civ, tid)
            || !world.map.terrain[t].is_land()
            || world.map.elevation[t] == Elevation::Mountain
            || world.map.owner[t] != NO_CIV
        {
            continue;
        }
        // Minimum 4-hex spacing from any existing city
        let mut too_close = false;
        let mut min_own_d = u16::MAX;
        for c in 0..world.cities.len() {
            if !world.cities.alive[c] {
                continue;
            }
            let d = world.map.distance(tid, world.cities.pos[c]);
            if d < 4 {
                too_close = true;
                break;
            }
            if world.cities.civ[c] == civ && d < min_own_d {
                min_own_d = d;
            }
        }
        if too_close || (!own_cities.is_empty() && min_own_d > 14) {
            continue;
        }
        let mut score = if world.map.has_river(tid) { 10 } else { 0 };
        let cy = tile_yields(&world.map, tid);
        score += cy.total() * 3;
        for &nb in world.map.neighbors_of(tid) {
            if nb != INVALID_TILE {
                let y = tile_yields(&world.map, nb);
                score += (y.food as i32) * 3 + (y.prod as i32) * 3 + (y.gold as i32);
            }
        }
        if best.map_or(true, |(_, bs)| score > bs) {
            best = Some((tid, score));
        }
    }
    best.map(|(t, _)| t)
}

fn pick_ai_tech(tech_bits: u64, science_bias: u8, aggression: u8, rules: &Ruleset) -> Option<TechId> {
    let mut best: Option<(TechId, i32)> = None;
    for t in &rules.techs {
        if !rules.can_research(tech_bits, t.id) {
            continue;
        }
        let mut score = 200 - t.cost / 4;
        if t.unlocks_corps || t.unlocks_army {
            score += 90 + (aggression as i32);
        }
        if t.unlocks_science_victory {
            score += 200 + (science_bias as i32);
        }
        if best.map_or(true, |(_, bs)| score > bs) {
            best = Some((t.id, score));
        }
    }
    best.map(|(id, _)| id).or_else(|| pick_next_available_tech(tech_bits, rules))
}
