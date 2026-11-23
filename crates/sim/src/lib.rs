#![forbid(unsafe_code)]

pub mod ai;
pub mod combat;
pub mod economy;
pub mod hex;
pub mod mapgen;
pub mod pathing;
pub mod policies;
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
    let plans = plan_all_ai_parallel(world, rules);
    let t1 = Instant::now();

    for plan in &plans {
        execute_civ_plan(world, rules, plan);
    }
    let t2 = Instant::now();

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
    if let Some(hciv) = world.config.human_civ {
        process_civ_turn_end(world, rules, hciv);
    }

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
    use crate::policies::{adopt_policy, aggregate_policy_bonuses, policy_cost, CULTURE_VICTORY_POLICIES};
    use crate::save::{load_from_bytes, save_to_bytes};
    use crate::types::{
        Difficulty, GameConfig, GameRules, GameSpeed, MapSize, MapType, ProductionItem, Road,
        UnitFormation, UnitRole, VictoryKind,
    };
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn ruleset_v3_loads_6_eras_50_techs_32_units_10_wonders_20_policies() {
        let rules = Ruleset::load_default();
        assert_eq!(rules.civs.len(), 22);
        assert_eq!(rules.techs.len(), 50);
        assert!(rules.units.len() >= 32);
        let wonders = rules.buildings.iter().filter(|b| b.is_wonder).count();
        assert_eq!(wonders, 10);
        assert_eq!(rules.policies.len(), 20);
        assert!(rules.techs[rules.corps_tech as usize].unlocks_corps);
        assert!(rules.techs[rules.army_tech as usize].unlocks_army);
        assert!(rules.techs[rules.flight_tech as usize].unlocks_flight);
        assert!(rules.techs[rules.railroad_tech as usize].unlocks_railroads);
        assert!(rules.units.iter().any(|u| u.role == UnitRole::Air));
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
    fn all_map_types_and_start_eras_generate_cleanly() {
        let rules = Ruleset::load_default();
        for mt in [MapType::Continents, MapType::Pangaea, MapType::Archipelago, MapType::Fractal] {
            let w = generate_world(
                GameConfig {
                    size: MapSize::Small,
                    num_civs: 6,
                    seed: 101 + mt as u64,
                    human_civ: Some(0),
                    turn_limit: 350,
                    rules: GameRules {
                        map_type: mt,
                        start_era: 2,
                        human_civ_template: 9, // Yamato
                        ..GameRules::default()
                    },
                },
                &rules,
            );
            assert_eq!(w.civs[0].name, "Yamato");
            assert!(rules.era_for_techs(w.civs[0].tech_bits) >= 1);
            assert!(w.cities.alive.iter().filter(|&&a| a).count() == 6);
        }
    }

    #[test]
    fn corps_and_army_merging_frees_supply_and_respects_rule_toggle() {
        let rules = Ruleset::load_default();
        let mut world = generate_world(
            GameConfig {
                size: MapSize::Small,
                num_civs: 4,
                seed: 7,
                human_civ: Some(0),
                turn_limit: 350,
                rules: GameRules::default(),
            },
            &rules,
        );
        world.civs[0].tech_bits |= (1u64 << rules.corps_tech) | (1u64 << rules.army_tech);
        let cap_pos = world.cities.pos[world.civs[0].capital as usize];
        let t1 = world.find_spawn_tile(cap_pos, false, false).unwrap();
        let u1 = world.spawn_unit(&rules, 0, rules.warrior_kind, t1, 10);
        let t2 = world.find_spawn_tile(t1, false, false).unwrap();
        let u2 = world.spawn_unit(&rules, 0, rules.warrior_kind, t2, 20);
        let t3 = world.find_spawn_tile(t1, false, false).unwrap();
        let u3 = world.spawn_unit(&rules, 0, rules.warrior_kind, t3, 5);

        // Toggle off Corps/Armies rule -> merge must be blocked
        world.config.rules.allow_corps_armies = false;
        assert_eq!(can_merge_units(&world, &rules, u1, u2), None);
        world.config.rules.allow_corps_armies = true;

        economy::recompute_all_civs(&mut world, &rules);
        let supply_before = world.civs[0].supply_used;

        assert_eq!(can_merge_units(&world, &rules, u1, u2), Some(UnitFormation::Corps));
        assert!(merge_units(&mut world, &rules, u1, u2));
        assert_eq!(world.units.formation[u1 as usize], UnitFormation::Corps);
        assert!(!world.units.is_alive(u2));

        assert_eq!(can_merge_units(&world, &rules, u1, u3), Some(UnitFormation::Army));
        assert!(merge_units(&mut world, &rules, u1, u3));
        assert_eq!(world.units.formation[u1 as usize], UnitFormation::Army);

        economy::recompute_all_civs(&mut world, &rules);
        assert_eq!(world.civs[0].supply_used, supply_before - 2);
    }

    #[test]
    fn unique_world_wonder_exclusivity_and_free_tech() {
        let rules = Ruleset::load_default();
        let mut world = generate_world(
            GameConfig {
                size: MapSize::Small,
                num_civs: 4,
                seed: 21,
                human_civ: Some(0),
                turn_limit: 350,
                rules: GameRules::default(),
            },
            &rules,
        );
        let gl = rules.buildings.iter().find(|b| b.key == "wonder_great_library").unwrap().id;
        let writing = rules.techs.iter().find(|t| t.key == "writing").unwrap().id;
        world.civs[0].tech_bits |= 1u64 << writing;
        world.civs[1].tech_bits |= 1u64 << writing;
        let c0 = world.civs[0].capital as usize;
        let c1 = world.civs[1].capital as usize;

        assert!(rules.can_build_building(world.civs[0].tech_bits, 0, world.wonders_built, gl));
        world.cities.producing[c0] = Some(ProductionItem::Building(gl));
        world.cities.prod_stored[c0] = 500;
        world.cities.producing[c1] = Some(ProductionItem::Building(gl));
        world.cities.prod_stored[c1] = 100;

        let techs_before = world.civs[0].tech_bits.count_ones();
        economy::process_civ_turn_end(&mut world, &rules, 0);
        // Civ 0 built Great Library -> globally locked + granted 1 free tech!
        assert_ne!(world.wonders_built & (1u64 << gl), 0);
        assert!(world.civs[0].tech_bits.count_ones() > techs_before);
        assert!(!rules.can_build_building(world.civs[1].tech_bits, 0, world.wonders_built, gl));

        // Civ 1 gets refunded gold and stops building Great Library
        let g1_before = world.civs[1].gold;
        economy::process_civ_turn_end(&mut world, &rules, 1);
        assert!(world.cities.producing[c1].is_none());
        assert!(world.civs[1].gold >= g1_before);
    }

    #[test]
    fn social_policies_bonuses_and_culture_victory() {
        let rules = Ruleset::load_default();
        let mut world = generate_world(
            GameConfig {
                size: MapSize::Small,
                num_civs: 4,
                seed: 33,
                human_civ: Some(0),
                turn_limit: 350,
                rules: GameRules {
                    speed: GameSpeed::Quick,
                    ..GameRules::default()
                },
            },
            &rules,
        );
        world.civs[0].culture_stored = 50_000;
        let mut adopted = 0u32;
        for tree in 0..3u8 {
            for tier in 0..4u8 {
                let p = rules.policies.iter().find(|p| p.tree == tree && p.tier == tier).unwrap();
                assert!(adopt_policy(&mut world, &rules, 0, p.id));
                adopted += 1;
            }
        }
        assert_eq!(adopted, CULTURE_VICTORY_POLICIES);
        let bonuses = aggregate_policy_bonuses(&rules, world.civs[0].policy_bits);
        assert!(bonuses.combat_pct > 0 && bonuses.prod_pct > 0);
        assert_eq!(world.victory.map(|v| v.kind), Some(VictoryKind::Culture));
        assert!(policy_cost(adopted, GameSpeed::Quick) < policy_cost(adopted, GameSpeed::Marathon));
    }

    #[test]
    fn worker_builds_road_and_railroad() {
        let rules = Ruleset::load_default();
        let mut world = generate_world(
            GameConfig {
                size: MapSize::Small,
                num_civs: 4,
                seed: 44,
                human_civ: Some(0),
                turn_limit: 350,
                rules: GameRules::default(),
            },
            &rules,
        );
        let cap_pos = world.cities.pos[world.civs[0].capital as usize];
        let nb = *world
            .map
            .neighbors_of(cap_pos)
            .iter()
            .find(|&&t| t != types::INVALID_TILE && world.map.terrain[t as usize].is_land())
            .unwrap();
        let w = world.spawn_unit(&rules, 0, rules.worker_kind, nb, 0);
        world.civs[0].tech_bits |= 1u64 << rules.road_tech;
        assert_eq!(pathing::human_worker_build_road(&mut world, &rules, w), Some(Road::Road));
        world.units.moves_halves[w as usize] = 4;
        world.civs[0].tech_bits |= 1u64 << rules.railroad_tech;
        assert_eq!(pathing::human_worker_build_road(&mut world, &rules, w), Some(Road::Railroad));
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
            rules: GameRules {
                difficulty: Difficulty::King,
                ..GameRules::default()
            },
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
                rules: GameRules::default(),
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
                rules: GameRules::default(),
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
