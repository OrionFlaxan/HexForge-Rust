use hexforge_sim::ai::speculative::SpeculativePlanner;
use hexforge_sim::mapgen::generate_world;
use hexforge_sim::ruleset::Ruleset;
use hexforge_sim::save::{load_from_bytes, save_to_bytes};
use hexforge_sim::types::{GameConfig, MapSize};
use hexforge_sim::{end_human_turn, step_full_turn};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn parse_args() -> (MapSize, u16, u32, u64, Option<usize>, bool) {
    let args: Vec<String> = std::env::args().collect();
    let mut size = MapSize::Huge;
    let mut players = 22u16;
    let mut turns = 350u32;
    let mut seed = 42u64;
    let mut threads: Option<usize> = None;
    let mut speculative = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--size" if i + 1 < args.len() => {
                size = match args[i + 1].to_lowercase().as_str() {
                    "small" => MapSize::Small,
                    "standard" => MapSize::Standard,
                    "large" => MapSize::Large,
                    _ => MapSize::Huge,
                };
                i += 2;
            }
            "--players" if i + 1 < args.len() => {
                players = args[i + 1].parse().unwrap_or(22);
                i += 2;
            }
            "--turns" if i + 1 < args.len() => {
                turns = args[i + 1].parse().unwrap_or(350);
                i += 2;
            }
            "--seed" if i + 1 < args.len() => {
                seed = args[i + 1].parse().unwrap_or(42);
                i += 2;
            }
            "--threads" if i + 1 < args.len() => {
                threads = args[i + 1].parse().ok();
                i += 2;
            }
            "--speculative" => {
                speculative = true;
                i += 1;
            }
            _ => i += 1,
        }
    }
    (size, players, turns, seed, threads, speculative)
}

fn main() {
    let (size, players, turns, seed, threads, speculative) = parse_args();
    if let Some(n) = threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .expect("failed to configure rayon pool");
    }

    let num_threads = rayon::current_num_threads();
    let rules = Arc::new(Ruleset::load_default());

    let t_gen = Instant::now();
    let cfg = GameConfig {
        size,
        num_civs: players,
        seed,
        human_civ: if speculative { Some(0) } else { None },
        turn_limit: turns + 50,
    };
    let mut world = generate_world(cfg, &rules);
    let mapgen_ms = t_gen.elapsed().as_secs_f64() * 1000.0;

    println!(
        "=== HexForge-Rust Benchmark | {} ({} tiles) | {} civs | {} turns | seed {} | {} threads | speculative={} ===",
        size.name(),
        world.map.num_tiles,
        world.civs.len(),
        turns,
        seed,
        num_threads,
        speculative
    );
    println!("Mapgen + Setup: {:.2} ms", mapgen_ms);

    let mut planner = if speculative {
        Some(SpeculativePlanner::new())
    } else {
        None
    };

    let mut turn_ms = Vec::with_capacity(turns as usize);
    let mut plan_ms = Vec::with_capacity(turns as usize);
    let mut exec_ms = Vec::with_capacity(turns as usize);
    let mut econ_ms = Vec::with_capacity(turns as usize);
    let mut total_hits = 0usize;
    let mut total_ai = 0usize;

    let t_all = Instant::now();
    for _t in 1..=turns {
        let m = if let Some(ref mut sp) = planner {
            // Simulate human taking a brief pause while background speculative AI finishes
            sp.start_precompute(world.clone(), Arc::clone(&rules));
            while !sp.is_ready() {
                std::thread::sleep(Duration::from_micros(200));
            }
            // Simulate human interacting near 1 bordering AI civ
            world.speculative_dirty_civs = 1u64 << 1;
            let res = end_human_turn(&mut world, &rules, Some(sp));
            total_hits += res.speculative.cache_hits;
            total_ai += res.speculative.total_ai_civs;
            res
        } else {
            step_full_turn(&mut world, &rules)
        };
        turn_ms.push(m.total_ms);
        plan_ms.push(m.plan_ms);
        exec_ms.push(m.exec_ms);
        econ_ms.push(m.econ_ms);
    }
    let total_wall_ms = t_all.elapsed().as_secs_f64() * 1000.0;

    let (singles, corps, armies) = world.units.formation_counts();
    let alive_cities = world.cities.alive.iter().filter(|&&a| a).count();
    let alive_civs = world.civs.iter().filter(|c| c.alive).count();

    println!(
        "\nBucket    |    Mean   |  Median   |     P95   |     Max   |  Plan/Exec/Econ"
    );
    println!("{}", "-".repeat(78));
    let mut lo = 0usize;
    while lo < turn_ms.len() {
        let hi = (lo + 50).min(turn_ms.len());
        let slice = &turn_ms[lo..hi];
        let mut sorted = slice.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let mean: f64 = slice.iter().sum::<f64>() / (slice.len() as f64);
        let median = sorted[sorted.len() / 2];
        let p95 = sorted[((sorted.len() - 1) as f64 * 0.95) as usize];
        let max = *sorted.last().unwrap();
        let pm: f64 = plan_ms[lo..hi].iter().sum::<f64>() / (slice.len() as f64);
        let em: f64 = exec_ms[lo..hi].iter().sum::<f64>() / (slice.len() as f64);
        let cm: f64 = econ_ms[lo..hi].iter().sum::<f64>() / (slice.len() as f64);
        println!(
            "{:>3}-{:<3}   | {:>6.2} ms | {:>6.2} ms | {:>6.2} ms | {:>6.2} ms | {:.2} / {:.2} / {:.2} ms",
            lo + 1,
            hi,
            mean,
            median,
            p95,
            max,
            pm,
            em,
            cm
        );
        lo = hi;
    }

    let overall_mean: f64 = turn_ms.iter().sum::<f64>() / (turn_ms.len() as f64);
    let overall_max: f64 = turn_ms.iter().cloned().fold(0.0, f64::max);

    // Save / Load microbenchmark on the actual Turn-350 Huge World!
    let ts = Instant::now();
    let saved = save_to_bytes(&world).expect("save failed");
    let save_ms = ts.elapsed().as_secs_f64() * 1000.0;
    let tl = Instant::now();
    let loaded = load_from_bytes(&saved).expect("load failed");
    let load_ms = tl.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(loaded.state_hash(), world.state_hash());

    println!("\n=== Summary ===");
    println!(
        "Total {} turns wall-clock: {:.2} ms ({:.2} s) | Mean: {:.2} ms/turn | Max: {:.2} ms",
        turns,
        total_wall_ms,
        total_wall_ms / 1000.0,
        overall_mean,
        overall_max
    );
    println!(
        "End State (Turn {}): {} alive civs, {} cities, {} active units ({} Singles, {} Corps, {} Armies)",
        world.turn, alive_civs, alive_cities, world.units.alive_count, singles, corps, armies
    );
    if speculative && total_ai > 0 {
        println!(
            "Speculative AI Hit Rate: {}/{} ({:.1}%)",
            total_hits,
            total_ai,
            (total_hits as f64 / total_ai as f64) * 100.0
        );
    }
    println!(
        "Save/Load Turn-{}: {:.2} ms save ({:.1} KB compressed) | {:.2} ms load (hash verified)",
        world.turn,
        save_ms,
        saved.len() as f64 / 1024.0,
        load_ms
    );
    println!("State Hash: 0x{:016x}", world.state_hash());
}
