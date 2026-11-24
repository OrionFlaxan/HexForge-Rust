pub mod camera;
pub mod codex;
pub mod hud;
pub mod renderer;
pub mod settings;
pub mod sprites;
pub mod storage;

use crate::camera::MapCamera;
use crate::hud::{
    draw_game_hud, draw_title_screen, ActiveModal, CityTab, GameUiState, HudAction, TitleAction,
    TitleUiState,
};
use crate::renderer::{render_and_interact_minimap, render_world};
use crate::settings::{AppOptions, UiMetrics};
use crate::storage::{has_save, load_slot, save_slot};
use hexforge_sim::ai::speculative::SpeculativePlanner;
use hexforge_sim::combat::{attack_city, attack_unit, merge_units};
use hexforge_sim::economy::{next_mandatory_choice, recompute_all_civs, rush_buy};
use hexforge_sim::improvements::{pillage_tile, start_worker_order};
use hexforge_sim::mapgen::generate_world;
use hexforge_sim::pathing::{
    compute_reachable_tiles, list_attackable_tiles, move_unit_along_path, ReachableSet,
};
use hexforge_sim::policies::adopt_policy;
use hexforge_sim::ruleset::Ruleset;
use hexforge_sim::types::{
    DiploStatus, GameConfig, PendingChoice, ProductionItem, TileId, UnitId, UnitRole, INVALID_CITY,
    INVALID_UNIT,
};
use hexforge_sim::world::World;
use hexforge_sim::{end_human_turn, step_full_turn, TurnMetrics};
use macroquad::prelude::*;
use std::sync::Arc;

pub fn window_conf() -> Conf {
    Conf {
        window_title: "HexForge-Rust v0.5.1 — Pure-Rust 4X".to_owned(),
        window_width: 1280,
        window_height: 760,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

pub fn launch_desktop() {
    macroquad::Window::from_config(window_conf(), run_game());
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn quad_main() {
    macroquad::Window::from_config(window_conf(), run_game());
}

struct Session {
    world: World,
    cam: MapCamera,
    planner: SpeculativePlanner,
    selected_tile: Option<TileId>,
    selected_unit: Option<UnitId>,
    reachable: ReachableSet,
    attackable: Vec<TileId>,
    /// Previewed destination tile awaiting a 2nd tap (or CONFIRM) when `confirm_unit_moves` is enabled.
    pending_order_tile: Option<TileId>,
    pending_path: Vec<TileId>,
    pending_is_attack: bool,
    /// Tracks first tap on an already-selected unit so a 2nd tap (double-tap) cleanly deselects it.
    deselect_armed_unit: Option<UnitId>,
    gui: GameUiState,
    last_metrics: TurnMetrics,
    reveal_all: bool,
    auto_spectate: bool,
    auto_timer: f32,
    toast: String,
    toast_timer: f32,
}

impl Session {
    fn new(world: World, rules: &Arc<Ruleset>, ui: &UiMetrics) -> Self {
        let mut cam = MapCamera::new(&world.map, ui);
        let viewer = world.config.human_civ;
        if let Some(hciv) = viewer {
            let cap = world.civs[hciv as usize].capital;
            if world.cities.is_alive(cap) {
                cam.focus_tile(&world.map, world.cities.pos[cap as usize]);
            }
        }
        let mut planner = SpeculativePlanner::new();
        if viewer.is_some() {
            planner.start_precompute(world.clone(), Arc::clone(rules));
        }
        let reveal_all = viewer.is_none() || !world.config.rules.fog_of_war;
        let mut gui = GameUiState::default();
        gui.tech_era_tab = world.config.rules.start_era.min(8);
        Self {
            world,
            cam,
            planner,
            selected_tile: None,
            selected_unit: None,
            reachable: ReachableSet {
                tiles: Vec::new(),
                parent: Vec::new(),
            },
            attackable: Vec::new(),
            pending_order_tile: None,
            pending_path: Vec::new(),
            pending_is_attack: false,
            deselect_armed_unit: None,
            gui,
            last_metrics: TurnMetrics::default(),
            reveal_all,
            auto_spectate: false,
            auto_timer: 0.0,
            toast: String::new(),
            toast_timer: 0.0,
        }
    }

    fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = msg.into();
        self.toast_timer = 2.8;
    }

    fn clear_pending_order(&mut self) {
        self.pending_order_tile = None;
        self.pending_path.clear();
        self.pending_is_attack = false;
        self.deselect_armed_unit = None;
    }

    fn deselect_unit(&mut self, rules: &Ruleset) {
        self.selected_unit = None;
        self.selected_tile = None;
        self.clear_pending_order();
        self.refresh_selection_overlays(rules);
    }

    fn refresh_selection_overlays(&mut self, rules: &Ruleset) {
        self.pending_order_tile = None;
        self.pending_path.clear();
        self.pending_is_attack = false;
        if let Some(uid) = self.selected_unit {
            if self.world.units.is_alive(uid)
                && self.world.config.human_civ == Some(self.world.units.civ[uid as usize])
            {
                self.reachable = compute_reachable_tiles(&self.world, rules, uid);
                self.attackable = list_attackable_tiles(&self.world, rules, uid);
                self.selected_tile = Some(self.world.units.pos[uid as usize]);
                return;
            }
        }
        self.reachable.tiles.clear();
        self.reachable.parent.clear();
        self.attackable.clear();
    }

    /// Execute a confirmed move or attack for `uid` targeting `tid`.
    fn execute_unit_order(&mut self, rules: &Ruleset, uid: UnitId, tid: TileId, is_attack: bool) -> bool {
        let Some(hciv) = self.world.config.human_civ else {
            return false;
        };
        if !self.world.units.is_alive(uid) || self.world.units.civ[uid as usize] != hciv {
            self.clear_pending_order();
            return false;
        }
        self.clear_pending_order();

        if is_attack && self.attackable.contains(&tid) {
            let eu = self.world.units.tile_mil_unit[tid as usize];
            let ec = self.world.cities.tile_city[tid as usize];
            let attacked = if eu != INVALID_UNIT
                && self.world.diplo.is_at_war(hciv, self.world.units.civ[eu as usize])
            {
                attack_unit(&mut self.world, rules, uid, eu)
            } else if ec != INVALID_CITY
                && self.world.diplo.is_at_war(hciv, self.world.cities.civ[ec as usize])
            {
                attack_city(&mut self.world, rules, uid, ec)
            } else {
                false
            };
            if attacked {
                recompute_all_civs(&mut self.world, rules);
                if !self.world.units.is_alive(uid) {
                    self.selected_unit = None;
                }
                self.refresh_selection_overlays(rules);
                return true;
            }
        } else if !is_attack && self.reachable.contains(tid).is_some() {
            let start = self.world.units.pos[uid as usize];
            let path = self.reachable.reconstruct_path(start, tid);
            if move_unit_along_path(&mut self.world, rules, uid, &path) {
                let role = rules.units[self.world.units.kind[uid as usize] as usize].role;
                if role == UnitRole::Settler {
                    self.set_toast("Tap FOUND CITY below to settle here");
                } else if role == UnitRole::Air {
                    self.set_toast("Air unit rebased!");
                }
                self.refresh_selection_overlays(rules);
                return true;
            }
        }
        false
    }

    fn select_next_ready_unit(&mut self, rules: &Ruleset) {
        let Some(hciv) = self.world.config.human_civ else {
            return;
        };
        let n = self.world.units.len();
        if n == 0 {
            return;
        }
        let start = self.selected_unit.map_or(0, |u| (u as usize + 1) % n);
        for offset in 0..n {
            let idx = (start + offset) % n;
            if self.world.units.alive[idx]
                && self.world.units.civ[idx] == hciv
                && self.world.units.moves_halves[idx] > 0
                && !self.world.units.fortified[idx]
                && !self.world.units.working[idx]
                && !self.world.units.auto_work[idx]
            {
                let uid = idx as UnitId;
                self.clear_pending_order();
                self.selected_unit = Some(uid);
                self.selected_tile = Some(self.world.units.pos[idx]);
                self.cam.focus_tile(&self.world.map, self.world.units.pos[idx]);
                self.refresh_selection_overlays(rules);
                return;
            }
        }
        self.set_toast("All units moved!");
    }

    /// Jump directly to the missing Technology, idle City, or affordable Policy modal.
    fn resolve_pending_choice(&mut self, rules: &Ruleset, choice: PendingChoice) {
        self.clear_pending_order();
        match choice {
            PendingChoice::ChooseTech => {
                if let Some(hciv) = self.world.config.human_civ {
                    let bits = self.world.civs[hciv as usize].tech_bits;
                    // Auto-select the earliest era that has an available researchable tech
                    let best_era = rules
                        .techs
                        .iter()
                        .filter(|t| rules.can_research(bits, t.id))
                        .map(|t| t.era)
                        .min()
                        .unwrap_or_else(|| rules.era_for_techs(bits));
                    self.gui.tech_era_tab = best_era;
                }
                self.gui.modal.0 = ActiveModal::Tech;
                self.set_toast("Choose a Technology before ending turn!");
            }
            PendingChoice::ChooseProduction(cid) => {
                if self.world.cities.is_alive(cid) {
                    let ci = cid as usize;
                    let pos = self.world.cities.pos[ci];
                    let cname = self.world.cities.name[ci].clone();
                    let civ = self.world.cities.civ[ci];
                    let cs = &self.world.civs[civ as usize];
                    let bbits = self.world.cities.buildings[ci];
                    let coastal = self.world.map.is_adjacent_to_water(pos);
                    let has_units = rules.units.iter().any(|u| {
                        (!u.role.is_naval() || coastal)
                            && rules.can_train_unit_for_civ(cs.tech_bits, cs.template_idx, u.id)
                    });
                    let has_bldgs = rules.buildings.iter().any(|b| {
                        !b.is_wonder
                            && rules.can_build_building_for_civ(
                                cs.tech_bits,
                                bbits,
                                self.world.wonders_built,
                                cs.template_idx,
                                b.id,
                            )
                    });
                    let has_wonders = rules.buildings.iter().any(|b| {
                        b.is_wonder
                            && rules.can_build_building_for_civ(
                                cs.tech_bits,
                                bbits,
                                self.world.wonders_built,
                                cs.template_idx,
                                b.id,
                            )
                    });
                    // Ensure the player never lands on an empty tab!
                    let tab_empty = match self.gui.city_tab {
                        CityTab::Units => !has_units,
                        CityTab::Buildings => !has_bldgs,
                        CityTab::Wonders => !has_wonders,
                        CityTab::Focus => false,
                        CityTab::Built => true,
                    };
                    if tab_empty {
                        self.gui.city_tab = if has_bldgs {
                            CityTab::Buildings
                        } else if has_units {
                            CityTab::Units
                        } else {
                            CityTab::Focus
                        };
                    }
                    self.cam.focus_tile(&self.world.map, pos);
                    self.selected_tile = Some(pos);
                    self.selected_unit = None;
                    self.refresh_selection_overlays(rules);
                    self.gui.city_scroll.reset();
                    self.gui.modal.0 = ActiveModal::City(cid);
                    self.set_toast(format!("Choose production or Focus for {cname}!"));
                }
            }
            PendingChoice::AdoptPolicy => {
                self.gui.modal.0 = ActiveModal::Policies;
                self.set_toast("Adopt a Social Policy before ending turn!");
            }
        }
    }

    /// Enforces Mandatory Pre-Turn Choices (Tech + Idle City Production) before ending human turn!
    fn try_end_turn(&mut self, rules: &Arc<Ruleset>, options: &AppOptions) {
        if options.require_turn_choices {
            if let Some(hciv) = self.world.config.human_civ {
                if let Some(choice) =
                    next_mandatory_choice(&self.world, rules, hciv, options.require_policy_choice)
                {
                    self.resolve_pending_choice(rules, choice);
                    return;
                }
            }
        }
        self.do_end_turn(rules, options);
    }

    fn do_end_turn(&mut self, rules: &Arc<Ruleset>, options: &AppOptions) {
        self.clear_pending_order();
        if self.world.config.human_civ.is_some() {
            self.last_metrics = end_human_turn(&mut self.world, rules, Some(&self.planner));
            self.planner.start_precompute(self.world.clone(), Arc::clone(rules));
            if options.autosave_interval > 0 && self.world.turn % options.autosave_interval == 0 {
                let _ = save_slot(&self.world, "autosave");
            }
            if options.auto_next_unit {
                self.select_next_ready_unit(rules);
                return;
            }
        } else {
            self.last_metrics = step_full_turn(&mut self.world, rules);
        }
        self.refresh_selection_overlays(rules);
    }
}

pub async fn run_game() {
    let rules = Arc::new(Ruleset::load_default());
    let mut options = AppOptions::load();
    let mut setup = GameConfig::default();
    let mut title_ui = TitleUiState::default();
    let mut title_msg = String::new();
    let mut session: Option<Session> = None;

    loop {
        let dt = get_frame_time().min(0.1);
        let ui = UiMetrics::compute(&options);

        if let Some(ref mut sess) = session {
            clear_background(Color::from_rgba(12, 16, 24, 255));

            if sess.toast_timer > 0.0 {
                sess.toast_timer -= dt;
                if sess.toast_timer <= 0.0 {
                    sess.toast.clear();
                }
            }

            if sess.auto_spectate && sess.world.victory.is_none() {
                sess.auto_timer += dt;
                if sess.auto_timer >= 0.08 {
                    sess.auto_timer = 0.0;
                    sess.do_end_turn(&rules, &options);
                }
            }

            if sess.gui.modal.0 == ActiveModal::None && sess.world.victory.is_none() {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    sess.try_end_turn(&rules, &options);
                } else if is_key_pressed(KeyCode::Tab) {
                    sess.select_next_ready_unit(&rules);
                } else if is_key_pressed(KeyCode::Escape) {
                    if sess.pending_order_tile.is_some() {
                        sess.clear_pending_order();
                        sess.set_toast("Move cancelled");
                    } else if sess.selected_unit.is_some() {
                        sess.deselect_unit(&rules);
                    } else {
                        sess.gui.modal.0 = ActiveModal::Menu;
                    }
                }
            } else if is_key_pressed(KeyCode::Escape) {
                sess.gui.modal.0 = ActiveModal::None;
            }

            let viewer = sess.world.config.human_civ;
            let minimap_w = (ui.sw * if ui.is_portrait { 0.30 } else { 0.19 }).clamp(ui.dp(102.0), ui.dp(200.0));
            let minimap_h = (minimap_w * 0.56).clamp(ui.dp(58.0), ui.dp(116.0));
            let minimap_rect = Rect::new(
                ui.sw - minimap_w - ui.dp(8.0),
                ui.sh - ui.bottom_bar_h - minimap_h - ui.dp(8.0),
                minimap_w,
                minimap_h,
            );
            let (mx, my) = mouse_position();
            let pointer_in_minimap =
                sess.gui.modal.0 == ActiveModal::None && minimap_rect.contains(vec2(mx, my));

            // 1. Render Hex Map World
            render_world(
                &sess.world,
                &rules,
                &sess.cam,
                &ui,
                &options,
                viewer,
                sess.reveal_all,
                sess.selected_tile,
                sess.selected_unit,
                &sess.reachable,
                &sess.attackable,
                sess.pending_order_tile,
                &sess.pending_path,
                sess.pending_is_attack,
            );

            // 2. Render & Handle Minimap Tap
            if sess.gui.modal.0 == ActiveModal::None && sess.world.victory.is_none() {
                if let Some(mtile) = render_and_interact_minimap(
                    &sess.world,
                    &sess.cam,
                    &ui,
                    viewer,
                    sess.reveal_all,
                    minimap_rect,
                ) {
                    sess.cam.focus_tile(&sess.world.map, mtile);
                }
            }

            // 3. Render HUD & Modals
            let spec_ready = viewer.is_some() && sess.planner.is_ready();
            let (hud_action, ui_consumed) = draw_game_hud(
                &ui,
                &mut options,
                &sess.world,
                &rules,
                viewer,
                sess.selected_tile,
                sess.selected_unit,
                sess.pending_order_tile,
                sess.pending_is_attack,
                &mut sess.gui,
                sess.last_metrics,
                spec_ready,
                sess.auto_spectate,
                &sess.toast,
            );

            // 4. Update Camera & Handle Map Tap
            let input = sess
                .cam
                .update(&sess.world.map, &ui, ui_consumed || pointer_in_minimap, dt);

            if let Some(tap) = input.tapped_screen {
                if let Some(tid) = sess.cam.screen_to_tile(&sess.world.map, tap.x, tap.y, ui.sw, ui.sh) {
                    handle_map_tap(sess, &rules, &options, tid);
                }
            }

            // 5. Execute HUD Action
            let mut back_to_title = false;
            match hud_action {
                HudAction::None => {}
                HudAction::EndTurn => sess.try_end_turn(&rules, &options),
                HudAction::ResolveChoice(choice) => sess.resolve_pending_choice(&rules, choice),
                HudAction::ConfirmPendingOrder => {
                    if let (Some(uid), Some(tid)) = (sess.selected_unit, sess.pending_order_tile) {
                        let is_atk = sess.pending_is_attack;
                        sess.execute_unit_order(&rules, uid, tid, is_atk);
                    }
                }
                HudAction::CancelPendingOrder => {
                    sess.clear_pending_order();
                    sess.set_toast("Move cancelled");
                }
                HudAction::DeselectUnit => {
                    sess.deselect_unit(&rules);
                    sess.set_toast("Unit deselected");
                }
                HudAction::NextUnit => sess.select_next_ready_unit(&rules),
                HudAction::MergeUnit(u1, u2) => {
                    if merge_units(&mut sess.world, &rules, u1, u2) {
                        recompute_all_civs(&mut sess.world, &rules);
                        let lbl = sess.world.units.formation[u1 as usize].label();
                        sess.set_toast(format!("Formed elite {lbl}! (+Strength, -1 Supply)"));
                        sess.refresh_selection_overlays(&rules);
                    }
                }
                HudAction::FoundCity(uid) => {
                    if sess.world.units.is_alive(uid) {
                        let pos = sess.world.units.pos[uid as usize];
                        let civ = sess.world.units.civ[uid as usize];
                        if sess.world.found_city(&rules, civ, pos).is_some() {
                            sess.world.despawn_unit(&rules, uid);
                            recompute_all_civs(&mut sess.world, &rules);
                            sess.selected_unit = None;
                            sess.selected_tile = Some(pos);
                            sess.clear_pending_order();
                            sess.refresh_selection_overlays(&rules);
                            sess.set_toast("New city founded!");
                        } else {
                            sess.set_toast("Too close to another city (need 3+ hexes)");
                        }
                    }
                }
                HudAction::StartWorkerOrder(uid, order) => {
                    if start_worker_order(&mut sess.world, &rules, uid, order) {
                        let rem = sess.world.map.work_turns_left[sess.world.units.pos[uid as usize] as usize];
                        sess.gui.modal.0 = ActiveModal::None;
                        sess.clear_pending_order();
                        sess.refresh_selection_overlays(&rules);
                        sess.set_toast(format!("Worker started {} ({}t)!", order.name(), rem));
                    }
                }
                HudAction::ToggleAutoWork(uid) => {
                    if sess.world.units.is_alive(uid) {
                        let ui_idx = uid as usize;
                        sess.world.units.auto_work[ui_idx] = !sess.world.units.auto_work[ui_idx];
                        let on = sess.world.units.auto_work[ui_idx];
                        sess.set_toast(if on {
                            "Worker AUTO WORK enabled (runs on End Turn)"
                        } else {
                            "Worker AUTO WORK disabled"
                        });
                    }
                }
                HudAction::PillageTile(uid) => {
                    let bonus = if sess.world.units.is_alive(uid) {
                        let c = sess.world.units.civ[uid as usize] as usize;
                        rules.civs[sess.world.civs[c].template_idx as usize].pillage_gold_bonus
                    } else {
                        0
                    };
                    if pillage_tile(&mut sess.world, &rules, uid) {
                        recompute_all_civs(&mut sess.world, &rules);
                        sess.clear_pending_order();
                        sess.refresh_selection_overlays(&rules);
                        sess.set_toast(format!("Pillaged tile! (+{} Gold, +20 HP)", 20 + bonus));
                    }
                }
                HudAction::FortifyUnit(uid) => {
                    if sess.world.units.is_alive(uid) {
                        sess.world.units.fortified[uid as usize] = true;
                        sess.world.units.moves_halves[uid as usize] = 0;
                        sess.clear_pending_order();
                        sess.refresh_selection_overlays(&rules);
                        sess.set_toast("Unit fortified (+20% Defense & Heal)");
                    }
                }
                HudAction::OpenModal(m) => {
                    sess.clear_pending_order();
                    if m == ActiveModal::Codex {
                        sess.gui.codex.scroll.reset();
                    } else if m == ActiveModal::Diplo {
                        sess.gui.diplo_scroll.reset();
                    } else if m == ActiveModal::Tech {
                        if let Some(hciv) = viewer {
                            let bits = sess.world.civs[hciv as usize].tech_bits;
                            sess.gui.tech_era_tab = rules
                                .techs
                                .iter()
                                .filter(|t| rules.can_research(bits, t.id))
                                .map(|t| t.era)
                                .min()
                                .unwrap_or_else(|| rules.era_for_techs(bits));
                        }
                    }
                    sess.gui.modal.0 = m;
                }
                HudAction::CloseModal => sess.gui.modal.0 = ActiveModal::None,
                HudAction::SetProduction(cid, item) => {
                    if sess.world.cities.is_alive(cid) {
                        sess.world.cities.producing[cid as usize] = Some(item);
                        recompute_all_civs(&mut sess.world, &rules);
                        sess.gui.modal.0 = ActiveModal::None;
                        let msg = match item {
                            ProductionItem::Unit(u) => format!("Training {}", rules.units[u as usize].name),
                            ProductionItem::Building(b) => format!("Building {}", rules.buildings[b as usize].name),
                            ProductionItem::Process(p) => format!("City set to {}", p.name()),
                        };
                        sess.set_toast(msg);
                    }
                }
                HudAction::RushBuy(cid, item) => {
                    if rush_buy(&mut sess.world, &rules, cid, item) {
                        sess.gui.modal.0 = ActiveModal::None;
                        sess.set_toast("Purchased with Gold!");
                    }
                }
                HudAction::SelectTech(tid) => {
                    if let Some(hciv) = viewer {
                        sess.world.civs[hciv as usize].current_tech = Some(tid);
                        sess.gui.modal.0 = ActiveModal::None;
                        sess.set_toast(format!("Researching {}", rules.techs[tid as usize].name));
                    }
                }
                HudAction::AdoptPolicy(pid) => {
                    if let Some(hciv) = viewer {
                        if adopt_policy(&mut sess.world, &rules, hciv, pid) {
                            recompute_all_civs(&mut sess.world, &rules);
                            sess.set_toast(format!("Adopted policy: {}!", rules.policies[pid as usize].name));
                        }
                    }
                }
                HudAction::DeclareWar(other) => {
                    if let Some(hciv) = viewer {
                        sess.world.diplo.set_status(hciv, other, DiploStatus::War);
                        sess.world.speculative_dirty_civs |= 1u64 << other;
                        sess.refresh_selection_overlays(&rules);
                        sess.set_toast(format!("War declared on {}!", sess.world.civs[other as usize].name));
                    }
                }
                HudAction::MakePeace(other) => {
                    if let Some(hciv) = viewer {
                        sess.world.diplo.set_status(hciv, other, DiploStatus::Peace);
                        sess.world.speculative_dirty_civs |= 1u64 << other;
                        sess.refresh_selection_overlays(&rules);
                        sess.set_toast(format!("Peace signed with {}!", sess.world.civs[other as usize].name));
                    }
                }
                HudAction::SaveGame => {
                    match save_slot(&sess.world, "autosave") {
                        Ok(sz) => sess.set_toast(format!("Saved ({:.1} KB)", sz as f32 / 1024.0)),
                        Err(e) => sess.set_toast(format!("Save error: {e}")),
                    }
                    sess.gui.modal.0 = ActiveModal::None;
                }
                HudAction::LoadGame => match load_slot("autosave") {
                    Ok(w) => {
                        *sess = Session::new(w, &rules, &ui);
                        sess.set_toast("Loaded saved game!");
                    }
                    Err(e) => {
                        sess.set_toast(format!("Load failed: {e}"));
                        sess.gui.modal.0 = ActiveModal::None;
                    }
                },
                HudAction::FastForward(n) => {
                    let t0 = get_time();
                    for _ in 0..n {
                        if sess.world.victory.is_some() {
                            break;
                        }
                        sess.last_metrics = step_full_turn(&mut sess.world, &rules);
                    }
                    let elapsed_ms = (get_time() - t0) * 1000.0;
                    if viewer.is_some() {
                        sess.planner.start_precompute(sess.world.clone(), Arc::clone(&rules));
                    }
                    sess.clear_pending_order();
                    sess.refresh_selection_overlays(&rules);
                    sess.gui.modal.0 = ActiveModal::None;
                    sess.set_toast(format!(
                        "Simulated {} turns in {:.0}ms ({:.1}ms/turn)!",
                        n,
                        elapsed_ms,
                        elapsed_ms / (n.max(1) as f64)
                    ));
                }
                HudAction::ToggleRevealMap => {
                    sess.reveal_all = !sess.reveal_all;
                    sess.gui.modal.0 = ActiveModal::None;
                }
                HudAction::ToggleAutoSpectate => {
                    sess.auto_spectate = !sess.auto_spectate;
                }
                HudAction::OptionsChanged => {
                    if !options.confirm_unit_moves {
                        sess.clear_pending_order();
                    }
                    options.save();
                }
                HudAction::ReturnToTitle => {
                    let _ = save_slot(&sess.world, "autosave");
                    back_to_title = true;
                }
            }

            if back_to_title {
                session = None;
            }
        } else {
            let act = draw_title_screen(
                &ui,
                &rules,
                &mut setup,
                &mut options,
                &mut title_ui,
                has_save("autosave"),
                &title_msg,
            );
            match act {
                TitleAction::None => {}
                TitleAction::OptionsChanged => {
                    options.save();
                }
                TitleAction::StartGame(cfg) => {
                    let world = generate_world(cfg, &rules);
                    session = Some(Session::new(world, &rules, &ui));
                    title_msg.clear();
                }
                TitleAction::ContinueAutosave => match load_slot("autosave") {
                    Ok(world) => {
                        session = Some(Session::new(world, &rules, &ui));
                        title_msg.clear();
                    }
                    Err(e) => {
                        title_msg = format!("Could not load save: {e}");
                    }
                },
            }
        }

        next_frame().await;
    }
}

fn handle_map_tap(sess: &mut Session, rules: &Ruleset, options: &AppOptions, tid: TileId) {
    let viewer = sess.world.config.human_civ;
    let ti = tid as usize;

    // 1. Active friendly unit interactions (Double-Tap Move/Attack Confirm + Double-Tap Unit Deselect)
    if let (Some(hciv), Some(uid)) = (viewer, sess.selected_unit) {
        if sess.world.units.is_alive(uid) && sess.world.units.civ[uid as usize] == hciv {
            let unit_pos = sess.world.units.pos[uid as usize];

            // Tapped the selected unit's own tile: cancel pending preview, cycle stack, or double-tap to deselect!
            if tid == unit_pos {
                if sess.pending_order_tile.is_some() {
                    sess.clear_pending_order();
                    sess.set_toast("Move cancelled");
                    return;
                }
                let mil = sess.world.units.tile_mil_unit[ti];
                let civ_u = sess.world.units.tile_civ_unit[ti];
                if mil != INVALID_UNIT && civ_u != INVALID_UNIT {
                    if uid == mil {
                        sess.clear_pending_order();
                        sess.selected_unit = Some(civ_u);
                        sess.selected_tile = Some(tid);
                        sess.refresh_selection_overlays(rules);
                        return;
                    } else {
                        sess.deselect_unit(rules);
                        sess.set_toast("Unit deselected");
                        return;
                    }
                }
                if sess.deselect_armed_unit == Some(uid) {
                    sess.deselect_unit(rules);
                    sess.set_toast("Unit deselected");
                } else {
                    sess.deselect_armed_unit = Some(uid);
                    sess.set_toast("Tap unit again to deselect");
                }
                return;
            }

            // Attackable target tile (2-tap confirm when `confirm_unit_moves` is enabled)
            if sess.attackable.contains(&tid) {
                if !options.confirm_unit_moves
                    || (sess.pending_order_tile == Some(tid) && sess.pending_is_attack)
                {
                    if sess.execute_unit_order(rules, uid, tid, true) {
                        return;
                    }
                } else {
                    sess.pending_order_tile = Some(tid);
                    sess.pending_is_attack = true;
                    sess.pending_path = vec![unit_pos, tid];
                    sess.deselect_armed_unit = None;
                    let eu = sess.world.units.tile_mil_unit[ti];
                    let ec = sess.world.cities.tile_city[ti];
                    if eu != INVALID_UNIT && sess.world.units.is_alive(eu) {
                        let eciv = &sess.world.civs[sess.world.units.civ[eu as usize] as usize].name;
                        let ename = &rules.units[sess.world.units.kind[eu as usize] as usize].name;
                        sess.set_toast(format!("Tap again (or CONFIRM) to attack {ename} [{eciv}]"));
                    } else if ec != INVALID_CITY && sess.world.cities.is_alive(ec) {
                        let eciv = &sess.world.civs[sess.world.cities.civ[ec as usize] as usize].name;
                        let cname = &sess.world.cities.name[ec as usize];
                        sess.set_toast(format!("Tap again (or CONFIRM) to attack {cname} [{eciv}]"));
                    } else {
                        sess.set_toast("Tap enemy tile again (or CONFIRM) to attack");
                    }
                    return;
                }
            }

            // Reachable destination tile (2-tap confirm when `confirm_unit_moves` is enabled)
            if sess.reachable.contains(tid).is_some() {
                if !options.confirm_unit_moves
                    || (sess.pending_order_tile == Some(tid) && !sess.pending_is_attack)
                {
                    if sess.execute_unit_order(rules, uid, tid, false) {
                        return;
                    }
                } else {
                    sess.pending_order_tile = Some(tid);
                    sess.pending_is_attack = false;
                    sess.pending_path = sess.reachable.reconstruct_path(unit_pos, tid);
                    sess.deselect_armed_unit = None;
                    sess.set_toast("Tap tile again (or CONFIRM) to move");
                    return;
                }
            }
        }
    }

    // 2. Inspect / Select Unit, City, or Tile (honoring Fog of War!)
    sess.clear_pending_order();
    let effective_reveal = sess.reveal_all || !sess.world.config.rules.fog_of_war;
    let tile_visible = match (viewer, effective_reveal) {
        (Some(vc), false) => sess.world.vision.is_visible(vc, tid),
        _ => true,
    };

    let mil = if tile_visible {
        sess.world.units.tile_mil_unit[ti]
    } else {
        INVALID_UNIT
    };
    let civ_u = if tile_visible {
        sess.world.units.tile_civ_unit[ti]
    } else {
        INVALID_UNIT
    };
    let city = sess.world.cities.tile_city[ti];

    let picked_unit = match (mil != INVALID_UNIT, civ_u != INVALID_UNIT) {
        (true, true) => {
            if sess.selected_unit == Some(mil) {
                Some(civ_u)
            } else {
                Some(mil)
            }
        }
        (true, false) => Some(mil),
        (false, true) => Some(civ_u),
        (false, false) => None,
    };

    // Allow double-tapping a selected foreign unit to deselect it cleanly too
    if picked_unit.is_some() && sess.selected_unit == picked_unit {
        sess.deselect_unit(rules);
        return;
    }

    if picked_unit.is_none()
        && city != INVALID_CITY
        && sess.selected_tile == Some(tid)
        && viewer == Some(sess.world.cities.civ[city as usize])
    {
        sess.gui.city_scroll.reset();
        sess.gui.modal.0 = ActiveModal::City(city);
        return;
    }

    sess.selected_tile = Some(tid);
    sess.selected_unit = picked_unit;
    sess.refresh_selection_overlays(rules);
}
