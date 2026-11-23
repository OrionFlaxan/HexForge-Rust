use crate::ruleset::Ruleset;
use crate::types::{CityId, CivId, UnitFormation, UnitId, INVALID_CITY, INVALID_TILE, INVALID_UNIT};
use crate::world::World;

#[derive(Debug, Clone, Copy)]
pub struct CombatPreview {
    pub attacker_dmg: i16,
    pub defender_dmg: i16,
    pub attacker_strength: i32,
    pub defender_strength: i32,
}

/// Effective combat strength of a unit including Corps/Army bonus, XP promotions, and HP scaling.
#[inline]
pub fn unit_effective_strength(
    world: &World,
    rules: &Ruleset,
    unit: UnitId,
    ranged: bool,
    defending: bool,
    vs_city: bool,
) -> i32 {
    let i = unit as usize;
    let def = &rules.units[world.units.kind[i] as usize];
    let base = if ranged && def.ranged_strength > 0 {
        def.ranged_strength as i32
    } else {
        def.strength as i32
    };
    let form_bonus = world.units.formation[i].strength_bonus();
    let mut strength = base + form_bonus;

    let mut pct_mod = 100i32;
    // XP promotions: +5% per 30 XP (up to +25%)
    let level = (world.units.xp[i] / 30).min(5) as i32;
    pct_mod += level * 5;

    if defending {
        pct_mod += world.map.defense_bonus_pct(world.units.pos[i]);
        if world.units.fortified[i] {
            pct_mod += 20;
        }
    }
    if vs_city {
        pct_mod += def.city_attack_bonus as i32;
    }

    // Wounded units lose up to 35% strength linearly with missing HP
    let hp = world.units.hp[i].clamp(1, 100) as i32;
    let hp_scale = 65 + (hp * 35) / 100;
    strength = (strength * pct_mod / 100) * hp_scale / 100;
    strength.max(1)
}

/// Effective defensive strength of a city (base + pop + buildings + garrison bonus).
#[inline]
pub fn city_defense_strength(world: &World, rules: &Ruleset, city: CityId) -> i32 {
    let ci = city as usize;
    let mut s = 10 + (world.cities.pop[ci] as i32) * 2;
    if world.cities.is_capital[ci] {
        s += 4;
    }
    let bbits = world.cities.buildings[ci];
    for b in &rules.buildings {
        if (bbits & (1u64 << b.id)) != 0 {
            s += b.defense as i32;
        }
    }
    if world.map.elevation[world.cities.pos[ci] as usize] == crate::types::Elevation::Hill {
        s += 4;
    }
    let garrison = world.units.tile_mil_unit[world.cities.pos[ci] as usize];
    if garrison != INVALID_UNIT && world.units.civ[garrison as usize] == world.cities.civ[ci] {
        let gdef = &rules.units[world.units.kind[garrison as usize] as usize];
        s += (gdef.strength as i32 + world.units.formation[garrison as usize].strength_bonus()) / 3;
    }
    s
}

#[inline]
fn civ_damage_formula(atk_str: i32, def_str: i32) -> i16 {
    let diff = (atk_str - def_str).clamp(-60, 60) as f32;
    let dmg = 30.0 * (diff / 25.0).exp();
    dmg.round().clamp(4.0, 100.0) as i16
}

/// Preview unit-vs-unit combat without mutating state.
pub fn preview_unit_combat(world: &World, rules: &Ruleset, attacker: UnitId, defender: UnitId) -> CombatPreview {
    let ai = attacker as usize;
    let di = defender as usize;
    let adef = &rules.units[world.units.kind[ai] as usize];
    let dist = world.map.distance(world.units.pos[ai], world.units.pos[di]);
    let is_ranged = adef.ranged_strength > 0 && (dist > 1 || adef.role.is_ranged_attacker());

    let mut atk_s = unit_effective_strength(world, rules, attacker, is_ranged, false, false);
    let def_s = unit_effective_strength(world, rules, defender, false, true, false);

    // Flanking bonus (+8% per other friendly adjacent combat unit)
    if !is_ranged {
        let aciv = world.units.civ[ai];
        let mut flank = 0;
        for &nb in world.map.neighbors_of(world.units.pos[di]) {
            if nb != INVALID_TILE {
                let u = world.units.tile_mil_unit[nb as usize];
                if u != INVALID_UNIT && u != attacker && world.units.civ[u as usize] == aciv {
                    flank += 8;
                }
            }
        }
        atk_s = atk_s * (100 + flank.min(32)) / 100;
    }

    let raw_def_dmg = civ_damage_formula(atk_s, def_s) as i32;
    let def_dmg = (raw_def_dmg * world.units.formation[di].incoming_damage_pct() / 100).max(4) as i16;

    let atk_dmg = if is_ranged {
        0
    } else {
        let raw_atk_dmg = civ_damage_formula(def_s, atk_s) as i32;
        (raw_atk_dmg * world.units.formation[ai].incoming_damage_pct() / 100).max(2) as i16
    };

    CombatPreview {
        attacker_dmg: atk_dmg,
        defender_dmg: def_dmg,
        attacker_strength: atk_s,
        defender_strength: def_s,
    }
}

/// Execute unit attacking another unit. Returns `true` if attack occurred.
pub fn attack_unit(world: &mut World, rules: &Ruleset, attacker: UnitId, defender: UnitId) -> bool {
    if !world.units.is_alive(attacker) || !world.units.is_alive(defender) {
        return false;
    }
    let ai = attacker as usize;
    let di = defender as usize;
    if world.units.has_attacked[ai] || world.units.moves_halves[ai] == 0 {
        return false;
    }
    let aciv = world.units.civ[ai];
    let dciv = world.units.civ[di];
    if !world.diplo.is_at_war(aciv, dciv) {
        return false;
    }

    let adef = &rules.units[world.units.kind[ai] as usize];
    let dist = world.map.distance(world.units.pos[ai], world.units.pos[di]);
    let max_range = adef.range.max(1) as u16;
    if dist > max_range {
        return false;
    }
    if dist > 1 && !world.map.has_los(world.units.pos[ai], world.units.pos[di]) {
        return false;
    }

    let preview = preview_unit_combat(world, rules, attacker, defender);
    let def_pos = world.units.pos[di];
    let is_ranged = adef.ranged_strength > 0 && (dist > 1 || adef.role.is_ranged_attacker());

    world.units.hp[di] -= preview.defender_dmg;
    world.units.hp[ai] -= preview.attacker_dmg;
    world.units.xp[ai] = world.units.xp[ai].saturating_add(5);
    world.units.xp[di] = world.units.xp[di].saturating_add(3);
    world.units.has_attacked[ai] = true;
    world.units.fortified[ai] = false;
    world.units.moves_halves[ai] = 0;

    let def_died = world.units.hp[di] <= 0;
    let atk_died = world.units.hp[ai] <= 0;

    if def_died {
        world.despawn_unit(rules, defender);
    }
    if atk_died {
        world.despawn_unit(rules, attacker);
    } else if def_died && !is_ranged {
        // Melee/Cavalry advances into the cleared tile if passable and no enemy city stands there
        let can_advance = world.cities.tile_city[def_pos as usize] == INVALID_CITY
            || world.cities.civ[world.cities.tile_city[def_pos as usize] as usize] == aciv;
        if can_advance {
            world.move_unit_step(rules, attacker, def_pos, 0);
        }
    }

    if world.civs[aciv as usize].is_human || world.civs[dciv as usize].is_human {
        world.mark_nearby_civs_dirty(def_pos, 7);
    }
    true
}

/// Execute unit attacking an enemy city (ranged or melee capture).
pub fn attack_city(world: &mut World, rules: &Ruleset, attacker: UnitId, city: CityId) -> bool {
    if !world.units.is_alive(attacker) || !world.cities.is_alive(city) {
        return false;
    }
    let ai = attacker as usize;
    let ci = city as usize;
    if world.units.has_attacked[ai] || world.units.moves_halves[ai] == 0 {
        return false;
    }
    let aciv = world.units.civ[ai];
    let dciv = world.cities.civ[ci];
    if !world.diplo.is_at_war(aciv, dciv) {
        return false;
    }

    let adef = &rules.units[world.units.kind[ai] as usize];
    let cpos = world.cities.pos[ci];
    let dist = world.map.distance(world.units.pos[ai], cpos);
    let max_range = adef.range.max(1) as u16;
    if dist > max_range {
        return false;
    }
    if dist > 1 && !world.map.has_los(world.units.pos[ai], cpos) {
        return false;
    }

    let is_ranged = adef.ranged_strength > 0 && (dist > 1 || adef.role.is_ranged_attacker());
    let atk_s = unit_effective_strength(world, rules, attacker, is_ranged, false, true);
    let def_s = city_defense_strength(world, rules, city);

    let city_dmg = civ_damage_formula(atk_s, def_s);
    let atk_dmg = if is_ranged {
        0
    } else {
        let raw = civ_damage_formula(def_s, atk_s) as i32;
        (raw * world.units.formation[ai].incoming_damage_pct() / 100).max(4) as i16
    };

    world.cities.hp[ci] -= city_dmg;
    world.units.hp[ai] -= atk_dmg;
    world.units.xp[ai] = world.units.xp[ai].saturating_add(6);
    world.units.has_attacked[ai] = true;
    world.units.moves_halves[ai] = 0;

    if world.units.hp[ai] <= 0 {
        world.despawn_unit(rules, attacker);
    } else if world.cities.hp[ci] <= 0 {
        if adef.role.can_capture_city() && !is_ranged {
            capture_city(world, rules, city, aciv);
            // Move conqueror into captured city if empty
            if world.units.tile_mil_unit[cpos as usize] == INVALID_UNIT {
                world.move_unit_step(rules, attacker, cpos, 0);
            }
        } else {
            // Ranged/siege reduces city to 1 HP; needs melee/cavalry to occupy
            world.cities.hp[ci] = 1;
        }
    }

    if world.civs[aciv as usize].is_human || world.civs[dciv as usize].is_human {
        world.mark_nearby_civs_dirty(cpos, 8);
    }
    true
}

/// City ranged bombardment against an enemy unit within 2 hexes.
pub fn city_bombard(world: &mut World, rules: &Ruleset, city: CityId, target: UnitId) -> bool {
    if !world.cities.is_alive(city) || !world.units.is_alive(target) {
        return false;
    }
    let ci = city as usize;
    let ti = target as usize;
    if world.cities.has_bombarded[ci] {
        return false;
    }
    let cciv = world.cities.civ[ci];
    let tciv = world.units.civ[ti];
    if !world.diplo.is_at_war(cciv, tciv) {
        return false;
    }
    let cpos = world.cities.pos[ci];
    let tpos = world.units.pos[ti];
    if world.map.distance(cpos, tpos) > 2 {
        return false;
    }

    let atk_s = city_defense_strength(world, rules, city);
    let def_s = unit_effective_strength(world, rules, target, false, true, false);
    let raw = civ_damage_formula(atk_s, def_s) as i32;
    let dmg = (raw * world.units.formation[ti].incoming_damage_pct() / 100).max(4) as i16;

    world.units.hp[ti] -= dmg;
    world.cities.has_bombarded[ci] = true;
    if world.units.hp[ti] <= 0 {
        world.despawn_unit(rules, target);
    }
    true
}

/// Transfer city ownership, kill enemy garrison if any, transfer territory, and check civ elimination.
pub fn capture_city(world: &mut World, rules: &Ruleset, city: CityId, new_owner: CivId) {
    let ci = city as usize;
    let old_owner = world.cities.civ[ci];
    let cpos = world.cities.pos[ci];

    // Destroy any garrison belonging to old_owner
    let gar = world.units.tile_mil_unit[cpos as usize];
    if gar != INVALID_UNIT && world.units.civ[gar as usize] == old_owner {
        world.despawn_unit(rules, gar);
    }

    world.cities.civ[ci] = new_owner;
    world.cities.hp[ci] = world.cities.max_hp[ci] / 2;
    world.cities.pop[ci] = (world.cities.pop[ci] / 2).max(1);
    world.cities.is_capital[ci] = false;
    world.cities.producing[ci] = None;
    world.cities.prod_stored[ci] = 0;

    // Transfer owned tiles belonging to this city
    for t in 0..world.map.num_tiles {
        if world.map.owner_city[t] == city {
            world.map.owner[t] = new_owner;
        }
    }

    // Reassign capital or eliminate old_owner if 0 cities remain
    let mut remaining_city = INVALID_CITY;
    for c in 0..world.cities.len() {
        if world.cities.alive[c] && world.cities.civ[c] == old_owner {
            remaining_city = c as CityId;
            break;
        }
    }
    if remaining_city == INVALID_CITY {
        world.civs[old_owner as usize].alive = false;
        world.civs[old_owner as usize].capital = INVALID_CITY;
        // Disband remaining units of defeated civ
        let mut to_kill = Vec::new();
        for u in 0..world.units.len() {
            if world.units.alive[u] && world.units.civ[u] == old_owner {
                to_kill.push(u as UnitId);
            }
        }
        for u in to_kill {
            world.despawn_unit(rules, u);
        }
    } else if world.civs[old_owner as usize].capital == city {
        world.civs[old_owner as usize].capital = remaining_city;
        world.cities.is_capital[remaining_city as usize] = true;
    }

    // Rebuild vision for both civs cleanly
    world.vision.rebuild_civ(&world.map, &world.units, &world.cities, rules, new_owner);
    if world.civs[old_owner as usize].alive {
        world.vision.rebuild_civ(&world.map, &world.units, &world.cities, rules, old_owner);
    }
}

/// Check if `unit_a` and `unit_b` can merge into a Corps or Army (Pillar 2).
pub fn can_merge_units(world: &World, rules: &Ruleset, unit_a: UnitId, unit_b: UnitId) -> Option<UnitFormation> {
    if unit_a == unit_b || !world.units.is_alive(unit_a) || !world.units.is_alive(unit_b) {
        return None;
    }
    let ai = unit_a as usize;
    let bi = unit_b as usize;
    let civ = world.units.civ[ai];
    if world.units.civ[bi] != civ || world.units.kind[ai] != world.units.kind[bi] {
        return None;
    }
    if rules.units[world.units.kind[ai] as usize].role.is_civilian() {
        return None;
    }
    if world.map.distance(world.units.pos[ai], world.units.pos[bi]) > 1 {
        return None;
    }
    let techs = world.civs[civ as usize].tech_bits;
    match (world.units.formation[ai], world.units.formation[bi]) {
        (UnitFormation::Single, UnitFormation::Single) => {
            if Ruleset::has_tech(techs, rules.corps_tech) {
                Some(UnitFormation::Corps)
            } else {
                None
            }
        }
        (UnitFormation::Corps, UnitFormation::Single) | (UnitFormation::Single, UnitFormation::Corps) => {
            if Ruleset::has_tech(techs, rules.army_tech) {
                Some(UnitFormation::Army)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Merge `donor` into `keeper`, upgrading `keeper` to Corps or Army and freeing 1 supply slot!
pub fn merge_units(world: &mut World, rules: &Ruleset, keeper: UnitId, donor: UnitId) -> bool {
    let Some(next_form) = can_merge_units(world, rules, keeper, donor) else {
        return false;
    };
    let ki = keeper as usize;
    let di = donor as usize;
    let merged_hp = ((world.units.hp[ki] as i32 + world.units.hp[di] as i32) / 2 + 25).min(100) as i16;
    let merged_xp = world.units.xp[ki].max(world.units.xp[di]).saturating_add(10);

    world.units.formation[ki] = next_form;
    world.units.hp[ki] = merged_hp;
    world.units.xp[ki] = merged_xp;
    world.units.moves_halves[ki] = 0;

    let pos = world.units.pos[ki];
    let civ = world.units.civ[ki];
    world.despawn_unit(rules, donor);

    if world.civs[civ as usize].is_human {
        world.mark_nearby_civs_dirty(pos, 6);
    }
    true
}

/// Find the first adjacent merge partner for `unit`, if any.
pub fn find_merge_partner(world: &World, rules: &Ruleset, unit: UnitId) -> Option<UnitId> {
    if !world.units.is_alive(unit) {
        return None;
    }
    let pos = world.units.pos[unit as usize];
    for &nb in world.map.neighbors_of(pos) {
        if nb == INVALID_TILE {
            continue;
        }
        let other = world.units.tile_mil_unit[nb as usize];
        if other != INVALID_UNIT && can_merge_units(world, rules, unit, other).is_some() {
            return Some(other);
        }
    }
    None
}
