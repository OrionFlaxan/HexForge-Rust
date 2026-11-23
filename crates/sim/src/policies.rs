use crate::ruleset::Ruleset;
use crate::types::{CivId, GameSpeed, PolicyBits, PolicyId, VictoryKind, VictoryState, VICTORY_CULTURE};
use crate::world::World;

/// Number of adopted policies required to achieve Culture Victory (3 full trees = 12 policies).
pub const CULTURE_VICTORY_POLICIES: u32 = 12;

/// Aggregated O(1) empire bonuses from adopted Social Policies.
#[derive(Debug, Clone, Copy, Default)]
pub struct PolicyBonuses {
    pub prod_pct: i32,
    pub sci_pct: i32,
    pub gold_pct: i32,
    pub combat_pct: i32,
    pub city_food: i16,
    pub city_prod: i16,
    pub city_gold: i16,
    pub city_science: i16,
    pub city_culture: i16,
    pub city_defense: i16,
    pub supply_bonus: u16,
    pub unit_xp: u16,
    pub rush_discount_pct: i32,
}

#[inline]
pub fn policy_cost(adopted_count: u32, speed: GameSpeed) -> i32 {
    let base = 25 + (adopted_count as i32) * 30 + ((adopted_count * adopted_count) as i32) * 7;
    Ruleset::scaled_cost(base, speed)
}

#[inline]
pub fn has_policy(bits: PolicyBits, pid: PolicyId) -> bool {
    (bits & (1u32 << pid)) != 0
}

#[inline]
pub fn can_adopt_policy(rules: &Ruleset, bits: PolicyBits, pid: PolicyId) -> bool {
    if has_policy(bits, pid) {
        return false;
    }
    let p = &rules.policies[pid as usize];
    if p.tier == 0 {
        return true;
    }
    // Require previous tier in the same tree
    for other in &rules.policies {
        if other.tree == p.tree && other.tier == p.tier - 1 {
            return has_policy(bits, other.id);
        }
    }
    true
}

#[inline]
pub fn can_afford_any_policy(world: &World, rules: &Ruleset, civ: CivId) -> bool {
    let cs = &world.civs[civ as usize];
    let adopted = cs.policy_bits.count_ones();
    if adopted as usize >= rules.policies.len() {
        return false;
    }
    cs.culture_stored >= policy_cost(adopted, world.config.rules.speed)
}

/// Aggregate all bonuses from `bits` in a tiny flat pass over 20 policies (zero allocations).
#[inline]
pub fn aggregate_policy_bonuses(rules: &Ruleset, bits: PolicyBits) -> PolicyBonuses {
    let mut b = PolicyBonuses::default();
    if bits == 0 {
        return b;
    }
    for p in &rules.policies {
        if (bits & (1u32 << p.id)) != 0 {
            b.prod_pct += p.prod_pct as i32;
            b.sci_pct += p.sci_pct as i32;
            b.gold_pct += p.gold_pct as i32;
            b.combat_pct += p.combat_pct as i32;
            b.city_food += p.city_food;
            b.city_prod += p.city_prod;
            b.city_gold += p.city_gold;
            b.city_science += p.city_science;
            b.city_culture += p.city_culture;
            b.city_defense += p.city_defense;
            b.supply_bonus += p.supply_bonus.max(0) as u16;
            b.unit_xp += p.unit_xp.max(0) as u16;
            b.rush_discount_pct += p.rush_discount_pct as i32;
        }
    }
    b
}

/// Spend accumulated Culture to adopt `pid` for `civ`, checking Culture Victory!
pub fn adopt_policy(world: &mut World, rules: &Ruleset, civ: CivId, pid: PolicyId) -> bool {
    let cs = &world.civs[civ as usize];
    if !cs.alive || !can_adopt_policy(rules, cs.policy_bits, pid) {
        return false;
    }
    let adopted = cs.policy_bits.count_ones();
    let cost = policy_cost(adopted, world.config.rules.speed);
    if cs.culture_stored < cost {
        return false;
    }
    let cs_mut = &mut world.civs[civ as usize];
    cs_mut.culture_stored -= cost;
    cs_mut.policy_bits |= 1u32 << pid;

    let new_count = cs_mut.policy_bits.count_ones();
    if new_count >= CULTURE_VICTORY_POLICIES
        && (world.config.rules.victory_mask & VICTORY_CULTURE) != 0
        && world.victory.is_none()
    {
        world.victory = Some(VictoryState {
            winner: civ,
            kind: VictoryKind::Culture,
            turn: world.turn,
        });
    }
    true
}

/// Deterministic AI policy selector aligned with civ personality.
pub fn pick_ai_policy(rules: &Ruleset, bits: PolicyBits, aggression: u8, science_bias: u8) -> Option<PolicyId> {
    let mut best: Option<(PolicyId, i32)> = None;
    for p in &rules.policies {
        if !can_adopt_policy(rules, bits, p.id) {
            continue;
        }
        let mut score = (p.prod_pct as i32) * 4
            + (p.sci_pct as i32) * (3 + science_bias as i32 / 25)
            + (p.combat_pct as i32) * (2 + aggression as i32 / 20)
            + (p.gold_pct as i32) * 3
            + (p.city_food as i32 + p.city_prod as i32 + p.city_science as i32 + p.city_culture as i32) * 18
            + (p.supply_bonus as i32) * 16
            + (p.tier as i32) * 10;
        // Prefer finishing started trees toward Culture Victory
        let tree_progress = rules
            .policies
            .iter()
            .filter(|o| o.tree == p.tree && has_policy(bits, o.id))
            .count() as i32;
        score += tree_progress * 14;
        if best.map_or(true, |(_, bs)| score > bs) {
            best = Some((p.id, score));
        }
    }
    best.map(|(id, _)| id)
}
