use crate::hex::HexMapSoA;
use crate::improvements::{advance_worker_orders_for_civ, improvement_yields};
use crate::policies::{adopt_policy, aggregate_policy_bonuses, pick_ai_policy, policy_cost};
use crate::ruleset::Ruleset;
use crate::types::{
    CityId, CivId, Elevation, Feature, Improvement, ProductionItem, Resource, Road, TechBits,
    Terrain, TileId, VictoryKind, VictoryState, Yields, INVALID_TILE, NO_CIV, VICTORY_DOMINATION,
    VICTORY_SCIENCE, VICTORY_SCORE,
};
use crate::world::World;

/// Zero-allocation stack computation of a single tile's yields (including tech-scaled improvements & pillaging).
#[inline]
pub fn tile_yields(map: &HexMapSoA, rules: &Ruleset, tile: TileId, owner_techs: TechBits) -> Yields {
    let i = tile as usize;
    let mut y = match map.terrain[i] {
        Terrain::Ocean => Yields::new(1, 0, 1, 0, 0),
        Terrain::Coast => Yields::new(1, 0, 2, 0, 0),
        Terrain::Grassland => Yields::new(2, 0, 0, 0, 0),
        Terrain::Plains => Yields::new(1, 1, 0, 0, 0),
        Terrain::Desert => Yields::new(0, 0, 0, 0, 0),
        Terrain::Tundra => Yields::new(1, 0, 0, 0, 0),
        Terrain::Snow => Yields::new(0, 0, 0, 0, 0),
    };
    if map.elevation[i] == Elevation::Hill {
        y.food = y.food.min(1);
        y.prod += 2;
    } else if map.elevation[i] == Elevation::Mountain {
        return Yields::default();
    }
    match map.feature[i] {
        Feature::Forest => y.prod += 1,
        Feature::Jungle => {
            y.food = y.food.max(2);
            y.science += 1;
        }
        Feature::Marsh => y.food = (y.food - 1).max(0),
        Feature::None => {}
    }
    if map.has_river(tile) {
        y.gold += 1;
        if matches!(map.terrain[i], Terrain::Desert | Terrain::Plains) {
            y.food += 1;
        }
    }
    match map.resource[i] {
        Resource::Wheat => y.food += 2,
        Resource::Cattle => {
            y.food += 1;
            y.prod += 1;
        }
        Resource::Iron => y.prod += 2,
        Resource::Horses => {
            y.prod += 1;
            y.gold += 1;
        }
        Resource::Gold => y.gold += 3,
        Resource::Gems => {
            y.gold += 3;
            y.culture += 1;
        }
        Resource::Oil => {
            y.prod += 2;
            y.gold += 2;
        }
        Resource::Uranium => {
            y.prod += 2;
            y.science += 2;
        }
        Resource::None => {}
    }
    let imp = map.improvement[i];
    if imp != Improvement::None && !map.pillaged[i] {
        y.add_assign(improvement_yields(map, rules, tile, imp, owner_techs));
    }
    y
}

/// Compute yields of a city with zero-allocation citizen ranking, buildings/wonders, Railroads, Civ Trait, Policies & Difficulty.
pub fn compute_city_yields(world: &World, rules: &Ruleset, city: CityId) -> Yields {
    let ci = city as usize;
    let cpos = world.cities.pos[ci];
    let pop = world.cities.pop[ci] as usize;
    let civ = world.cities.civ[ci];
    let cs = &world.civs[civ as usize];
    let tpl = &rules.civs[cs.template_idx as usize];
    let pbonus = aggregate_policy_bonuses(rules, cs.policy_bits);
    let techs = cs.tech_bits;

    let center = tile_yields(&world.map, rules, cpos, techs);
    let mut total = Yields::new(
        center.food.max(2) + tpl.city_food + pbonus.city_food,
        center.prod.max(1) + if world.cities.is_capital[ci] { 2 } else { 0 } + pbonus.city_prod,
        center.gold + if world.cities.is_capital[ci] { 3 } else { 0 } + pbonus.city_gold,
        center.science + (pop as i16) + if world.cities.is_capital[ci] { 3 } else { 0 } + pbonus.city_science,
        center.culture + if world.cities.is_capital[ci] { 2 } else { 1 } + tpl.city_culture + pbonus.city_culture,
    );

    let mut tile_scores: [(i32, Yields); 18] = [(i32::MIN, Yields::default()); 18];
    let mut count = 0usize;
    let mut visited = [INVALID_TILE; 19];
    let mut vlen = 1usize;
    visited[0] = cpos;

    for &n1 in world.map.neighbors_of(cpos) {
        if n1 == INVALID_TILE {
            continue;
        }
        if !visited[..vlen].contains(&n1) {
            visited[vlen] = n1;
            vlen += 1;
            if world.map.owner[n1 as usize] == civ && count < 18 {
                let y = tile_yields(&world.map, rules, n1, techs);
                let rank = (y.food as i32) * 32 + (y.prod as i32) * 28 + (y.science as i32) * 25 + (y.gold as i32) * 14;
                tile_scores[count] = (rank, y);
                count += 1;
            }
        }
    }
    let ring1_end = vlen;
    for idx in 1..ring1_end {
        for &n2 in world.map.neighbors_of(visited[idx]) {
            if n2 == INVALID_TILE {
                continue;
            }
            if !visited[..vlen].contains(&n2) && vlen < 19 {
                visited[vlen] = n2;
                vlen += 1;
                if world.map.owner[n2 as usize] == civ && count < 18 {
                    let y = tile_yields(&world.map, rules, n2, techs);
                    let rank =
                        (y.food as i32) * 32 + (y.prod as i32) * 28 + (y.science as i32) * 25 + (y.gold as i32) * 14;
                    tile_scores[count] = (rank, y);
                    count += 1;
                }
            }
        }
    }

    let worked = pop.min(count);
    for k in 0..worked {
        let mut best_j = k;
        for j in (k + 1)..count {
            if tile_scores[j].0 > tile_scores[best_j].0 {
                best_j = j;
            }
        }
        tile_scores.swap(k, best_j);
        total.add_assign(tile_scores[k].1);
    }

    let bbits = world.cities.buildings[ci];
    let mut prod_pct = (tpl.prod_pct as i32) + pbonus.prod_pct;
    if world.map.road[cpos as usize] == Road::Railroad {
        prod_pct += 15; // Realistic Railroad industrial throughput bonus!
    }
    let mut gold_pct = (tpl.gold_pct as i32) + pbonus.gold_pct;
    let mut sci_pct = (tpl.sci_pct as i32) + pbonus.sci_pct;
    let mut b_upkeep = 0i16;
    for b in &rules.buildings {
        if (bbits & (1u64 << b.id)) != 0 {
            total.food += b.food;
            total.prod += b.production;
            total.gold += b.gold;
            total.science += b.science;
            total.culture += b.culture;
            if b.science_per_pop_half {
                total.science += (pop as i16) / 2;
            }
            prod_pct += b.production_pct as i32;
            gold_pct += b.gold_pct as i32;
            sci_pct += b.science_pct as i32;
            b_upkeep += b.upkeep;
        }
    }

    // Difficulty yield bonus for AI when playing against a human
    let diff_pct = if !cs.is_human && world.config.human_civ.is_some() {
        world.config.rules.difficulty.ai_yield_pct()
    } else {
        100
    };

    total.prod = (((total.prod as i32) * (100 + prod_pct) / 100) * diff_pct / 100) as i16;
    total.gold = (((total.gold as i32) * (100 + gold_pct) / 100) * diff_pct / 100) as i16 - b_upkeep;
    total.science = (((total.science as i32) * (100 + sci_pct) / 100) * diff_pct / 100) as i16;

    // Supply cap production penalty (Pillar 4)
    if cs.supply_used > cs.supply_cap {
        let excess = (cs.supply_used - cs.supply_cap) as i32;
        let penalty = (excess * 10).min(70);
        total.prod = ((total.prod as i32) * (100 - penalty) / 100).max(1) as i16;
    }

    total.food -= (pop as i16) * 2;
    total
}

/// Compute total Rush-Buy discount (%) for `civ` from Wonders (Big Ben) and Policies (Mercantilism).
pub fn civ_rush_discount_pct(world: &World, rules: &Ruleset, civ: CivId) -> i32 {
    let pbonus = aggregate_policy_bonuses(rules, world.civs[civ as usize].policy_bits);
    let mut disc = pbonus.rush_discount_pct;
    for ci in 0..world.cities.len() {
        if world.cities.alive[ci] && world.cities.civ[ci] == civ {
            let bbits = world.cities.buildings[ci];
            for b in &rules.buildings {
                if b.rush_buy_discount_pct > 0 && (bbits & (1u64 << b.id)) != 0 {
                    disc += b.rush_buy_discount_pct as i32;
                }
            }
        }
    }
    disc.min(50)
}

/// Gold cost to rush-buy `item` in `civ` (honoring GameSpeed and rush-buy discounts).
pub fn rush_buy_cost(world: &World, rules: &Ruleset, civ: CivId, item: ProductionItem) -> i32 {
    let base = match item {
        ProductionItem::Unit(uk) => Ruleset::scaled_cost(rules.units[uk as usize].cost, world.config.rules.speed),
        ProductionItem::Building(bid) => {
            Ruleset::scaled_cost(rules.buildings[bid as usize].cost, world.config.rules.speed)
        }
    };
    let disc = civ_rush_discount_pct(world, rules, civ);
    ((base * 2) * (100 - disc) / 100).max(20)
}

/// Recompute Supply Cap, Supply Used, per-turn yields, and scores for every civilization.
pub fn recompute_all_civs(world: &mut World, rules: &Ruleset) {
    let n_civs = world.civs.len();
    let mut city_counts = vec![0u16; n_civs];
    let mut pop_totals = vec![0u16; n_civs];
    let mut supply_bldg = vec![0u16; n_civs];
    let mut supply_used = vec![0u16; n_civs];
    let mut unit_upkeep = vec![0i32; n_civs];
    let mut unit_score = vec![0i32; n_civs];
    let mut wonder_counts = vec![0i32; n_civs];

    for c in 0..world.cities.len() {
        if !world.cities.alive[c] {
            continue;
        }
        let cid = world.cities.civ[c] as usize;
        city_counts[cid] += 1;
        pop_totals[cid] += world.cities.pop[c];
        let bbits = world.cities.buildings[c];
        for b in &rules.buildings {
            if (bbits & (1u64 << b.id)) != 0 {
                if b.supply > 0 {
                    supply_bldg[cid] += b.supply as u16;
                }
                if b.is_wonder {
                    wonder_counts[cid] += 1;
                }
            }
        }
    }

    for u in 0..world.units.len() {
        if !world.units.alive[u] {
            continue;
        }
        let cid = world.units.civ[u] as usize;
        let def = &rules.units[world.units.kind[u] as usize];
        if def.role.is_combat() {
            supply_used[cid] += 1;
        }
        unit_upkeep[cid] += def.upkeep as i32;
        unit_score[cid] += (def.strength.max(def.ranged_strength) as i32
            + world.units.formation[u].strength_bonus())
            / 2;
    }

    for cid in 0..n_civs {
        let tpl = &rules.civs[world.civs[cid].template_idx as usize];
        let pbonus = aggregate_policy_bonuses(rules, world.civs[cid].policy_bits);
        let cap = 5
            + 2 * city_counts[cid]
            + pop_totals[cid] / 2
            + supply_bldg[cid]
            + tpl.supply_bonus.max(0) as u16
            + pbonus.supply_bonus;
        world.civs[cid].supply_cap = cap;
        world.civs[cid].supply_used = supply_used[cid];
        world.civs[cid].gold_per_turn = 0;
        world.civs[cid].science_per_turn = 0;
        world.civs[cid].culture_per_turn = 0;
    }

    for c in 0..world.cities.len() {
        if !world.cities.alive[c] {
            continue;
        }
        let y = compute_city_yields(world, rules, c as CityId);
        world.cities.last_yields[c] = y;
        let cid = world.cities.civ[c] as usize;
        world.civs[cid].gold_per_turn += y.gold as i32;
        world.civs[cid].science_per_turn += y.science as i32;
        world.civs[cid].culture_per_turn += y.culture as i32;
    }

    for cid in 0..n_civs {
        if !world.civs[cid].alive {
            world.civs[cid].score = 0;
            continue;
        }
        let excess_upkeep = if supply_used[cid] > world.civs[cid].supply_cap {
            let ex = (supply_used[cid] - world.civs[cid].supply_cap) as i32;
            ex * ex * 2
        } else {
            0
        };
        world.civs[cid].gold_per_turn -= unit_upkeep[cid] + excess_upkeep;
        let tech_count = world.civs[cid].tech_bits.count_ones() as i32;
        let pol_count = world.civs[cid].policy_bits.count_ones() as i32;
        world.civs[cid].score = (city_counts[cid] as i32) * 25
            + (pop_totals[cid] as i32) * 8
            + tech_count * 12
            + pol_count * 16
            + wonder_counts[cid] * 40
            + unit_score[cid];
    }
}

/// Advance workers, cities, tech research, culture/policies, treasury, and unit movement for `civ`.
pub fn process_civ_turn_end(world: &mut World, rules: &Ruleset, civ: CivId) {
    if !world.civs[civ as usize].alive {
        return;
    }
    // 0. Advance multi-turn worker orders first so completed improvements & Forest Chops (+20P) apply immediately!
    advance_worker_orders_for_civ(world, rules, civ);

    let speed = world.config.rules.speed;
    let pbonus = aggregate_policy_bonuses(rules, world.civs[civ as usize].policy_bits);

    // 1. Process all cities belonging to `civ`
    let num_cities = world.cities.len();
    for ci in 0..num_cities {
        if !world.cities.alive[ci] || world.cities.civ[ci] != civ {
            continue;
        }
        let y = compute_city_yields(world, rules, ci as CityId);
        world.cities.last_yields[ci] = y;
        world.cities.has_bombarded[ci] = false;
        world.cities.hp[ci] = (world.cities.hp[ci] + 15).min(world.cities.max_hp[ci]);

        // Population growth
        world.cities.food_stored[ci] += y.food as i32;
        let growth_needed = Ruleset::scaled_cost(15 + (world.cities.pop[ci] as i32) * 8, speed);
        if world.cities.food_stored[ci] >= growth_needed {
            world.cities.food_stored[ci] -= growth_needed;
            world.cities.pop[ci] = world.cities.pop[ci].saturating_add(1);
        } else if world.cities.food_stored[ci] < 0 {
            world.cities.food_stored[ci] = 0;
        }

        // Border expansion
        world.cities.culture_stored[ci] += y.culture as i32;
        let border_threshold = Ruleset::scaled_cost(20 * (world.cities.border_radius[ci] as i32 + 1), speed);
        if world.cities.culture_stored[ci] >= border_threshold && world.cities.border_radius[ci] < 3 {
            world.cities.culture_stored[ci] -= border_threshold;
            expand_city_borders(world, ci as CityId);
        }

        // Production completion
        if let Some(item) = world.cities.producing[ci] {
            // Check if another civ already completed this unique World Wonder
            if let ProductionItem::Building(bid) = item {
                let bdef = &rules.buildings[bid as usize];
                if bdef.is_wonder && (world.wonders_built & (1u64 << bid)) != 0 {
                    let refund = (world.cities.prod_stored[ci] / 2).max(10);
                    world.civs[civ as usize].gold += refund;
                    world.cities.prod_stored[ci] = 0;
                    world.cities.producing[ci] = None;
                    continue;
                }
            }

            world.cities.prod_stored[ci] += y.prod.max(1) as i32;
            let cost = match item {
                ProductionItem::Unit(uk) => Ruleset::scaled_cost(rules.units[uk as usize].cost, speed),
                ProductionItem::Building(bid) => Ruleset::scaled_cost(rules.buildings[bid as usize].cost, speed),
            };
            if world.cities.prod_stored[ci] >= cost {
                world.cities.prod_stored[ci] -= cost;
                world.cities.producing[ci] = None;
                match item {
                    ProductionItem::Unit(uk) => {
                        let def = &rules.units[uk as usize];
                        let xp = city_unit_xp(rules, world.cities.buildings[ci]) + pbonus.unit_xp;
                        if let Some(spawn_t) = world.find_spawn_tile(
                            world.cities.pos[ci],
                            def.role.is_civilian(),
                            def.role.is_naval(),
                        ) {
                            world.spawn_unit(rules, civ, uk, spawn_t, xp);
                        }
                    }
                    ProductionItem::Building(bid) => {
                        let bdef = &rules.buildings[bid as usize];
                        let mask = 1u64 << bid;
                        if bdef.is_wonder {
                            if (world.wonders_built & mask) != 0 {
                                world.civs[civ as usize].gold += cost / 2;
                                continue;
                            }
                            world.wonders_built |= mask;
                        }
                        world.cities.buildings[ci] |= mask;
                        if bdef.grants_free_tech {
                            if let Some(ft) = pick_next_available_tech(world.civs[civ as usize].tech_bits, rules) {
                                world.civs[civ as usize].tech_bits |= 1u128 << ft;
                                if world.civs[civ as usize].current_tech == Some(ft) {
                                    world.civs[civ as usize].current_tech =
                                        pick_next_available_tech(world.civs[civ as usize].tech_bits, rules);
                                }
                            }
                        }
                        if bdef.wins_science
                            && (world.config.rules.victory_mask & VICTORY_SCIENCE) != 0
                            && world.victory.is_none()
                        {
                            world.victory = Some(VictoryState {
                                winner: civ,
                                kind: VictoryKind::Science,
                                turn: world.turn,
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. Treasury, Research & Culture / Social Policies
    let gpt = world.civs[civ as usize].gold_per_turn;
    let spt = world.civs[civ as usize].science_per_turn.max(1);
    let cpt = world.civs[civ as usize].culture_per_turn.max(1);
    world.civs[civ as usize].gold = (world.civs[civ as usize].gold + gpt).max(-50);
    world.civs[civ as usize].culture_stored += cpt;

    // AI civs auto-adopt social policies when they can afford one
    if !world.civs[civ as usize].is_human {
        let adopted = world.civs[civ as usize].policy_bits.count_ones();
        if world.civs[civ as usize].culture_stored >= policy_cost(adopted, speed) {
            let cs = &world.civs[civ as usize];
            if let Some(pid) = pick_ai_policy(rules, cs.policy_bits, cs.aggression, cs.science_bias) {
                adopt_policy(world, rules, civ, pid);
            }
        }
    }

    if let Some(tech) = world.civs[civ as usize].current_tech {
        world.civs[civ as usize].tech_progress += spt;
        let cost = Ruleset::scaled_cost(rules.techs[tech as usize].cost, speed);
        if world.civs[civ as usize].tech_progress >= cost {
            world.civs[civ as usize].tech_progress -= cost;
            world.civs[civ as usize].tech_bits |= 1u128 << tech;
            world.civs[civ as usize].current_tech = pick_next_available_tech(world.civs[civ as usize].tech_bits, rules);
        }
    } else {
        world.civs[civ as usize].current_tech = pick_next_available_tech(world.civs[civ as usize].tech_bits, rules);
    }

    // 3. Refresh all units of `civ`
    for u in 0..world.units.len() {
        if !world.units.alive[u] || world.units.civ[u] != civ {
            continue;
        }
        let def = &rules.units[world.units.kind[u] as usize];
        let idle = !world.units.has_attacked[u] && world.units.moves_halves[u] == def.moves * 2;
        if world.units.hp[u] < 100 && (world.units.fortified[u] || idle || def.role.is_air()) {
            let in_friendly = world.map.owner[world.units.pos[u] as usize] == civ;
            let heal = if in_friendly { 18 } else { 10 };
            world.units.hp[u] = (world.units.hp[u] + heal).min(100);
        }
        world.units.moves_halves[u] = def.moves * 2;
        world.units.has_attacked[u] = false;
    }

    // 4. Auto-upgrade units in friendly territory if civ has plenty of gold
    if world.civs[civ as usize].gold >= 160 {
        let techs = world.civs[civ as usize].tech_bits;
        for u in 0..world.units.len() {
            if !world.units.alive[u] || world.units.civ[u] != civ {
                continue;
            }
            if world.map.owner[world.units.pos[u] as usize] != civ {
                continue;
            }
            let kind = world.units.kind[u];
            if let Some(upg) = rules.units[kind as usize].upgrades_to {
                let udef = &rules.units[upg as usize];
                if udef.tech.map_or(true, |t| Ruleset::has_tech(techs, t)) && world.civs[civ as usize].gold >= 90 {
                    world.civs[civ as usize].gold -= 75;
                    world.units.kind[u] = upg;
                    world.units.moves_halves[u] = udef.moves * 2;
                    if world.civs[civ as usize].gold < 120 {
                        break;
                    }
                }
            }
        }
    }
}

#[inline]
fn city_unit_xp(rules: &Ruleset, bbits: u64) -> u16 {
    let mut xp = 0u16;
    for b in &rules.buildings {
        if b.unit_xp > 0 && (bbits & (1u64 << b.id)) != 0 {
            xp += b.unit_xp as u16;
        }
    }
    xp
}

fn expand_city_borders(world: &mut World, city: CityId) {
    let ci = city as usize;
    let civ = world.cities.civ[ci];
    let cpos = world.cities.pos[ci];
    let next_r = (world.cities.border_radius[ci] + 1).min(3);
    world.cities.border_radius[ci] = next_r;

    let mut ring = Vec::with_capacity(37);
    world.map.collect_tiles_in_radius(cpos, next_r, &mut ring);
    let mut claimed = 0;
    for &t in &ring {
        let ti = t as usize;
        if world.map.owner[ti] == NO_CIV && world.map.elevation[ti] != Elevation::Mountain {
            world.map.owner[ti] = civ;
            world.map.owner_city[ti] = city;
            world.vision.add_tile_ref(civ, t);
            claimed += 1;
            if claimed >= 4 {
                break;
            }
        }
    }
}

pub fn pick_next_available_tech(tech_bits: TechBits, rules: &Ruleset) -> Option<u8> {
    let mut best: Option<(u8, i32)> = None;
    for t in &rules.techs {
        if rules.can_research(tech_bits, t.id) {
            if best.map_or(true, |(_, c)| t.cost < c) {
                best = Some((t.id, t.cost));
            }
        }
    }
    best.map(|(id, _)| id)
}

/// Update diplomacy counters once per global turn and check Domination / Score victory.
pub fn advance_global_turn(world: &mut World, rules: &Ruleset) {
    let nc = world.diplo.num_civs;
    for a in 0..nc {
        for b in (a + 1)..nc {
            let ab = a * nc + b;
            let ba = b * nc + a;
            if world.diplo.status[ab] == crate::types::DiploStatus::War {
                world.diplo.war_turns[ab] = world.diplo.war_turns[ab].saturating_add(1);
                world.diplo.war_turns[ba] = world.diplo.war_turns[ba].saturating_add(1);
            } else if world.diplo.peace_cooldown[ab] > 0 {
                world.diplo.peace_cooldown[ab] -= 1;
                world.diplo.peace_cooldown[ba] -= 1;
            }
        }
    }

    world.turn += 1;
    recompute_all_civs(world, rules);

    if world.victory.is_none() {
        let vmask = world.config.rules.victory_mask;
        let mut alive_civs = 0;
        let mut last_alive = 0 as CivId;
        for c in &world.civs {
            if c.alive {
                alive_civs += 1;
                last_alive = c.id;
            }
        }
        if alive_civs == 1 && (vmask & VICTORY_DOMINATION) != 0 {
            world.victory = Some(VictoryState {
                winner: last_alive,
                kind: VictoryKind::Domination,
                turn: world.turn,
            });
        } else if !world.config.rules.no_turn_limit
            && (vmask & VICTORY_SCORE) != 0
            && world.turn >= world.config.turn_limit
        {
            let winner = world
                .civs
                .iter()
                .filter(|c| c.alive)
                .max_by_key(|c| (c.score, std::cmp::Reverse(c.id)))
                .map(|c| c.id)
                .unwrap_or(0);
            world.victory = Some(VictoryState {
                winner,
                kind: VictoryKind::Score,
                turn: world.turn,
            });
        }
    }
}

/// Instant gold purchase of a unit or building/wonder in `city` (with Wonder/Policy discounts).
pub fn rush_buy(world: &mut World, rules: &Ruleset, city: CityId, item: ProductionItem) -> bool {
    if !world.cities.is_alive(city) {
        return false;
    }
    let ci = city as usize;
    let civ = world.cities.civ[ci];
    let cost = rush_buy_cost(world, rules, civ, item);
    if world.civs[civ as usize].gold < cost {
        return false;
    }
    let pbonus = aggregate_policy_bonuses(rules, world.civs[civ as usize].policy_bits);
    match item {
        ProductionItem::Unit(uk) => {
            let def = &rules.units[uk as usize];
            if !rules.can_train_unit(world.civs[civ as usize].tech_bits, uk) {
                return false;
            }
            let Some(spawn_t) =
                world.find_spawn_tile(world.cities.pos[ci], def.role.is_civilian(), def.role.is_naval())
            else {
                return false;
            };
            world.civs[civ as usize].gold -= cost;
            let xp = city_unit_xp(rules, world.cities.buildings[ci]) + pbonus.unit_xp;
            world.spawn_unit(rules, civ, uk, spawn_t, xp);
        }
        ProductionItem::Building(bid) => {
            let bdef = &rules.buildings[bid as usize];
            if bdef.is_wonder {
                return false; // World Wonders must be constructed, not rush-bought!
            }
            if !rules.can_build_building(
                world.civs[civ as usize].tech_bits,
                world.cities.buildings[ci],
                world.wonders_built,
                bid,
            ) {
                return false;
            }
            world.civs[civ as usize].gold -= cost;
            world.cities.buildings[ci] |= 1u64 << bid;
            if bdef.wins_science
                && (world.config.rules.victory_mask & VICTORY_SCIENCE) != 0
                && world.victory.is_none()
            {
                world.victory = Some(VictoryState {
                    winner: civ,
                    kind: VictoryKind::Science,
                    turn: world.turn,
                });
            }
        }
    }
    recompute_all_civs(world, rules);
    true
}
