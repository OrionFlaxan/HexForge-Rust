use crate::ai::{plan_all_ai_parallel, plan_civ, CivPlan};
use crate::ruleset::Ruleset;
use crate::types::CivId;
use crate::world::World;
use rayon::prelude::*;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, Copy, Default)]
pub struct SpeculativeStats {
    pub total_ai_civs: usize,
    pub cache_hits: usize,
    pub cache_recomputed: usize,
    pub background_ready: bool,
}

struct PrecomputeJob {
    generation: u64,
    snapshot: World,
    rules: Arc<Ruleset>,
}

struct PrecomputeOutput {
    generation: u64,
    plans: Vec<CivPlan>,
}

/// Pillar 3: Background Speculative AI Pre-Computation.
/// While the human player reads the map and moves troops, a background thread computes
/// Phase 1 Parallel AI Plans (Threat Fields, Theater Flow-Fields, City/Tech/Diplo plans)
/// on a compact snapshot. On End Turn, untouched AI civs reuse their plans in 0 ms!
pub struct SpeculativePlanner {
    tx: Option<Sender<PrecomputeJob>>,
    latest: Arc<Mutex<Option<PrecomputeOutput>>>,
    generation: u64,
    _worker: Option<JoinHandle<()>>,
}

impl Default for SpeculativePlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeculativePlanner {
    pub fn new() -> Self {
        let (tx, rx): (Sender<PrecomputeJob>, Receiver<PrecomputeJob>) = mpsc::channel();
        let latest: Arc<Mutex<Option<PrecomputeOutput>>> = Arc::new(Mutex::new(None));
        let latest_worker = Arc::clone(&latest);

        let handle = thread::Builder::new()
            .name("hexforge-speculative-ai".into())
            .spawn(move || {
                while let Ok(mut job) = rx.recv() {
                    // Drain any queued superseded jobs so we always work on the newest snapshot
                    while let Ok(newer) = rx.try_recv() {
                        job = newer;
                    }
                    let plans = plan_all_ai_parallel(&job.snapshot, &job.rules);
                    if let Ok(mut guard) = latest_worker.lock() {
                        *guard = Some(PrecomputeOutput {
                            generation: job.generation,
                            plans,
                        });
                    }
                }
            })
            .ok();

        Self {
            tx: Some(tx),
            latest,
            generation: 0,
            _worker: handle,
        }
    }

    /// Launch background pre-computation of all AI Phase-1 plans for the upcoming turn.
    pub fn start_precompute(&mut self, snapshot: World, rules: Arc<Ruleset>) {
        self.generation = self.generation.wrapping_add(1);
        if let Ok(mut guard) = self.latest.lock() {
            *guard = None;
        }
        if let Some(ref tx) = self.tx {
            let _ = tx.send(PrecomputeJob {
                generation: self.generation,
                snapshot,
                rules,
            });
        }
    }

    /// Returns `true` if the background worker has already finished precomputing AI plans for this turn.
    pub fn is_ready(&self) -> bool {
        if let Ok(guard) = self.latest.lock() {
            if let Some(ref out) = *guard {
                return out.generation == self.generation;
            }
        }
        false
    }

    /// Consume precomputed plans for untouched AI civs and replan only dirty bordering civs in parallel!
    pub fn resolve_plans(&self, world: &World, rules: &Ruleset) -> (Vec<CivPlan>, SpeculativeStats) {
        let dirty_mask = world.speculative_dirty_civs;
        let cached = self
            .latest
            .lock()
            .ok()
            .and_then(|mut g| g.take())
            .filter(|o| o.generation == self.generation);

        let ai_civs: Vec<CivId> = world
            .civs
            .iter()
            .filter(|c| c.alive && !c.is_human)
            .map(|c| c.id)
            .collect();

        if let Some(out) = cached {
            // Map cached plans by CivId (small array indexed by civ id)
            let mut cached_by_civ: Vec<Option<CivPlan>> = vec![None; world.civs.len()];
            for p in out.plans {
                let idx = p.civ as usize;
                if idx < cached_by_civ.len() {
                    cached_by_civ[idx] = Some(p);
                }
            }

            let mut dirty_civs = Vec::new();
            let mut clean_count = 0usize;
            for &cid in &ai_civs {
                let is_dirty = (cid as usize) >= 64 || (dirty_mask & (1u64 << cid)) != 0;
                if is_dirty || cached_by_civ[cid as usize].is_none() {
                    dirty_civs.push(cid);
                } else {
                    clean_count += 1;
                }
            }

            // Only replan dirty bordering civs in parallel!
            let replanned: Vec<CivPlan> = dirty_civs
                .par_iter()
                .map(|&cid| plan_civ(world, rules, cid))
                .collect();
            for p in replanned {
                let cid = p.civ as usize;
                cached_by_civ[cid] = Some(p);
            }

            let mut final_plans = Vec::with_capacity(ai_civs.len());
            for &cid in &ai_civs {
                if let Some(p) = cached_by_civ[cid as usize].take() {
                    final_plans.push(p);
                }
            }
            let stats = SpeculativeStats {
                total_ai_civs: ai_civs.len(),
                cache_hits: clean_count,
                cache_recomputed: dirty_civs.len(),
                background_ready: true,
            };
            (final_plans, stats)
        } else {
            let plans = plan_all_ai_parallel(world, rules);
            let stats = SpeculativeStats {
                total_ai_civs: ai_civs.len(),
                cache_hits: 0,
                cache_recomputed: ai_civs.len(),
                background_ready: false,
            };
            (plans, stats)
        }
    }
}

impl Drop for SpeculativePlanner {
    fn drop(&mut self) {
        self.tx.take();
    }
}
