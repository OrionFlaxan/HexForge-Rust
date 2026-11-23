#![forbid(unsafe_code)]

pub mod ai;
pub mod combat;
pub mod economy;
pub mod hex;
pub mod mapgen;
pub mod pathing;
pub mod ruleset;
pub mod save;
pub mod types;
pub mod world;

use crate::ai::speculative::{SpeculativePlanner, SpeculativeStats};
use crate::ai::{execute_civ_plan, plan_all_ai_parallel};
use crate::economy::{advance_global_turn, process_civ_turn_end};
use crate::ruleset::Ruleset;
use crate::types::CivId;
use crate::world::World;
use std::time::Instant;

#[derive(Debug, Clone, Copy, Default)]
pub struct TurnMetrics {
    pub total_ms: f64,
    pub plan_ms: f64,
    pub exec_ms: f64,
    pub econ_ms: f64,
    pub speculative: SpeculativeStats,
}

/// Advance a complete turn in pure AI / Spectator mode using 2-Phase Parallel Rayon AI.
/// Guaranteed 100% deterministic across 1 thread and N threads!
pub fn step_full_turn(world: &mut World, rules: &Ruleset) -> TurnMetrics {
    let t0 = Instant::now();
    // Phase 1: Parallel Read-Only AI Planning across all CPU cores
    let plans = plan_all_ai_parallel(world, rules);
    let t1 = Instant::now();

    // Phase 2: Deterministic execution in CivId order
    for plan in &plans {
        execute_civ_plan(world, rules, plan);
    }
    let t2 = Instant::now();

    // Phase 3: End-of-turn city/tech/economy/diplomacy processing
    let n_civs = world.civs.len();
    for cid in 0..n_civs {
        process_civ_turn_end(world, rules, cid as CivId);
    }
    advance_global_turn(world, rules);
    let t3 = Instant::now();

    TurnMetrics {
        total_ms: (t3 - t0).as_secs_f64() * 1000.0,
        plan_ms: (t1 - t0).as_secs_f64() * 1000.0,
        exec_ms: (t2 - t1).as_secs_f64() * 1000.0,
        econ_ms: (t3 - t2).as_secs_f64() * 1000.0,
        speculative: SpeculativeStats::default(),
    }
}

/// End the human player's turn, leveraging `SpeculativePlanner` for instant perceived Next-Turn latency!
pub fn end_human_turn(
    world: &mut World,
    rules: &Ruleset,
    planner: Option<&SpeculativePlanner>,
) -> TurnMetrics {
    let t0 = Instant::now();
    // Process human civ's end-of-turn growth/production/tech first
    if let Some(hciv) = world.config.human_civ {
        process_civ_turn_end(world, rules, hciv);
    }

    // Resolve AI plans (0 ms for clean civs whose plans were precomputed in background!)
    let (plans, spec_stats) = if let Some(sp) = planner {
        sp.resolve_plans(world, rules)
    } else {
        (plan_all_ai_parallel(world, rules), SpeculativeStats::default())
    };
    let t1 = Instant::now();

    for plan in &plans {
        execute_civ_plan(world, rules, plan);
    }
    let t2 = Instant::now();

    let n_civs = world.civs.len();
    for cid in 0..n_civs {
        if world.config.human_civ == Some(cid as CivId) {
            continue;
        }
        process_civ_turn_end(world, rules, cid as CivId);
    }
    advance_global_turn(world, rules);
    world.speculative_dirty_civs = 0;
    let t3 = Instant::now();

    TurnMetrics {
        total_ms: (t3 - t0).as_secs_f64() * 1000.0,
        plan_ms: (t1 - t0).as_secs_f64() * 1000.0,
        exec_ms: (t2 - t1).as_secs_f64() * 1000.0,
        econ_ms: (t3 - t2).as_secs_f64() * 1000.0,
        speculative: spec_stats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::speculative::SpeculativePlanner;
    use crate::combat::{can_merge_units, merge_units};
    use crate::mapgen::generate_world;
    use crate::save::{load_from_bytes, save_to_bytes};
    use crate::types::{GameConfig, MapSize, UnitFormation};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn ruleset_loads_cleanly() {
        let rules = Ruleset::load_default();
        assert_eq!(rules.civs.len(), 22);
        assert!(rules.techs.len() >= 24);
        assert!(rules.units.len() >= 18);
        assert!(rules.buildings.len() >= 12);
        assert!(rules.techs[rules.corps_tech as usize].unlocks_corps);
        assert!(rules.techs[rules.army_tech as usize].unlocks_army);
    }

    #[test]
    fn hex_wrap_and_distance_symmetry() {
        let map = hex::HexMapSoA::new(48, 26);
        let left = map.tile_id(0, 10);
        let right = map.tile_id(47, 10);
        assert_eq!(map.distance(left, right), 1);
        assert_eq!(map.distance(right, left), 1);
        assert!(map.neighbors_of(left).contains(&right));
        assert!(map.neighbors_of(right).contains(&left));
    }

    #[test]
    fn corps_and_army_merging_frees_supply() {
        let rules = Ruleset::load_default();
        let mut world = generate_world(
            GameConfig {
                size: MapSize::Small,
                num_civs: 4,
                seed: 7,
                human_civ: Some(0),
                turn_limit: 350,
            },
            &rules,
        );
        // Unlock Corps & Army techs for civ 0
        world.civs[0].tech_bits |= (1u64 << rules.corps_tech) | (1u64 << rules.army_tech);
        let cap_pos = world.cities.pos[world.civs[0].capital as usize];
        let t1 = world.find_spawn_tile(cap_pos, false, false).unwrap();
        let u1 = world.spawn_unit(&rules, 0, rules.warrior_kind, t1, 10);
        let t2 = world.find_spawn_tile(t1, false, false).unwrap();
        let u2 = world.spawn_unit(&rules, 0, rules.warrior_kind, t2, 20);
        let t3 = world.find_spawn_tile(t1, false, false).unwrap();
        let u3 = world.spawn_unit(&rules, 0, rules.warrior_kind, t3, 5);

        economy::recompute_all_civs(&mut world, &rules);
        let supply_before = world.civs[0].supply_used;

        assert_eq!(can_merge_units(&world, &rules, u1, u2), Some(UnitFormation::Corps));
        assert!(merge_units(&mut world, &rules, u1, u2));
        assert_eq!(world.units.formation[u1 as usize], UnitFormation::Corps);
        assert!(!world.units.is_alive(u2));

        assert_eq!(can_merge_units(&world, &rules, u1, u3), Some(UnitFormation::Army));
        assert!(merge_units(&mut world, &rules, u1, u3));
        assert_eq!(world.units.formation[u1 as usize], UnitFormation::Army);
        assert!(!world.units.is_alive(u3));

        economy::recompute_all_civs(&mut world, &rules);
        assert_eq!(world.civs[0].supply_used, supply_before - 2);
    }

    #[test]
    fn single_thread_and_multi_thread_strict_determinism() {
        let rules = Ruleset::load_default();
        let cfg = GameConfig {
            size: MapSize::Small,
            num_civs: 8,
            seed: 12345,
            human_civ: None,
            turn_limit: 350,
        };

        let mut w1 = generate_world(cfg, &rules);
        let pool1 = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        pool1.install(|| {
            for _ in 0..40 {
                step_full_turn(&mut w1, &rules);
            }
        });

        let mut w4 = generate_world(cfg, &rules);
        let pool4 = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
        pool4.install(|| {
            for _ in 0..40 {
                step_full_turn(&mut w4, &rules);
            }
        });

        assert_eq!(
            w1.state_hash(),
            w4.state_hash(),
            "1-thread and 4-thread simulations MUST produce bit-for-bit identical worlds!"
        );
    }

    #[test]
    fn speculative_planner_reuses_clean_civ_plans() {
        let rules = Arc::new(Ruleset::load_default());
        let mut world = generate_world(
            GameConfig {
                size: MapSize::Standard,
                num_civs: 10,
                seed: 99,
                human_civ: Some(0),
                turn_limit: 350,
            },
            &rules,
        );
        let mut planner = SpeculativePlanner::new();
        planner.start_precompute(world.clone(), Arc::clone(&rules));
        for _ in 0..200 {
            if planner.is_ready() {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(planner.is_ready(), "Background speculative AI should complete quickly");
        // Mark just 1 civ dirty (simulating human border skirmish)
        world.speculative_dirty_civs = 1u64 << 1;
        let metrics = end_human_turn(&mut world, &rules, Some(&planner));
        assert!(metrics.speculative.background_ready);
        assert_eq!(metrics.speculative.cache_recomputed, 1);
        assert_eq!(metrics.speculative.cache_hits, 8);
    }

    #[test]
    fn save_and_load_roundtrip_bit_exact() {
        let rules = Ruleset::load_default();
        let mut world = generate_world(
            GameConfig {
                size: MapSize::Standard,
                num_civs: 8,
                seed: 42,
                human_civ: Some(0),
                turn_limit: 350,
            },
            &rules,
        );
        for _ in 0..10 {
            step_full_turn(&mut world, &rules);
        }
        let h_before = world.state_hash();
        let bytes = save_to_bytes(&world).expect("save failed");
        let loaded = load_from_bytes(&bytes).expect("load failed");
        assert_eq!(loaded.state_hash(), h_before);
        assert_eq!(loaded.turn, world.turn);
    }
}
