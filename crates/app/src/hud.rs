use hexforge_sim::combat::{can_merge_units, city_defense_strength, find_merge_partner};
use hexforge_sim::economy::{best_improvement_for_tile, compute_city_yields, tile_yields};
use hexforge_sim::ruleset::Ruleset;
use hexforge_sim::types::{
    CityId, CivId, DiploStatus, GameConfig, Improvement, MapSize, ProductionItem, TechId, TileId,
    UnitId, UnitRole, VictoryKind, INVALID_CITY,
};
use hexforge_sim::world::World;
use hexforge_sim::TurnMetrics;
use macroquad::prelude::*;

pub const BG_PANEL: Color = Color::new(0.06, 0.08, 0.12, 0.94);
pub const BG_CARD: Color = Color::new(0.10, 0.13, 0.19, 0.96);
pub const BORDER_GOLD: Color = Color::new(0.76, 0.62, 0.30, 0.95);
pub const BORDER_DIM: Color = Color::new(0.28, 0.34, 0.44, 0.85);
pub const ACCENT_GOLD: Color = Color::new(0.96, 0.82, 0.36, 1.0);
pub const ACCENT_CYAN: Color = Color::new(0.32, 0.82, 0.96, 1.0);
pub const ACCENT_GREEN: Color = Color::new(0.24, 0.78, 0.42, 1.0);
pub const ACCENT_RED: Color = Color::new(0.86, 0.28, 0.28, 1.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModal {
    None,
    City(CityId),
    Tech,
    Diplo,
    Menu,
}

#[derive(Debug, Clone)]
pub enum HudAction {
    None,
    EndTurn,
    NextUnit,
    MergeUnit(UnitId, UnitId),
    FoundCity(UnitId),
    BuildImprovement(UnitId),
    FortifyUnit(UnitId),
    OpenModal(ActiveModal),
    CloseModal,
    SetProduction(CityId, ProductionItem),
    RushBuy(CityId, ProductionItem),
    SelectTech(TechId),
    DeclareWar(CivId),
    MakePeace(CivId),
    SaveGame,
    LoadGame,
    FastForward(u32),
    ToggleRevealMap,
    ToggleAutoSpectate,
    ReturnToTitle,
}

#[derive(Debug, Clone)]
pub enum TitleAction {
    None,
    StartGame(GameConfig),
    ContinueAutosave,
}

/// Immediate-mode touch-friendly button returning `true` when clicked/tapped.
pub fn ui_button(rect: Rect, label: &str, bg: Color, border: Color, font_size: f32) -> bool {
    let (mx, my) = mouse_position();
    let hovered = rect.contains(vec2(mx, my));
    let fill = if hovered {
        Color::new((bg.r + 0.08).min(1.0), (bg.g + 0.08).min(1.0), (bg.b + 0.08).min(1.0), bg.a)
    } else {
        bg
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.8, border);

    let dims = measure_text(label, None, font_size as u16, 1.0);
    let tx = rect.x + (rect.w - dims.width) * 0.5;
    let ty = rect.y + (rect.h + dims.height * 0.75) * 0.5;
    draw_text(label, tx.max(rect.x + 4.0), ty, font_size, WHITE);

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

/// Render the Title & New Game Setup screen.
pub fn draw_title_screen(setup: &mut GameConfig, has_autosave: bool, status_msg: &str) -> TitleAction {
    let sw = screen_width();
    let sh = screen_height();
    clear_background(Color::from_rgba(12, 16, 24, 255));

    // Decorative background hexes
    let hex_r = (sw.min(sh) * 0.08).clamp(28.0, 64.0);
    for row in 0..8 {
        for col in 0..12 {
            let cx = col as f32 * hex_r * 1.732 + if row % 2 == 1 { hex_r * 0.866 } else { 0.0 };
            let cy = row as f32 * hex_r * 1.5;
            draw_poly_lines(cx, cy, 6, hex_r, 30.0, 1.0, Color::new(0.22, 0.30, 0.42, 0.20));
        }
    }

    let card_w = (sw * 0.92).min(560.0);
    let card_h = (sh * 0.90).min(560.0);
    let cx = (sw - card_w) * 0.5;
    let cy = (sh - card_h) * 0.5;

    draw_rectangle(cx, cy, card_w, card_h, BG_PANEL);
    draw_rectangle_lines(cx, cy, card_w, card_h, 2.2, BORDER_GOLD);

    let title = "HEXFORGE-RUST";
    let title_fs = (card_w * 0.075).clamp(24.0, 40.0);
    let td = measure_text(title, None, title_fs as u16, 1.0);
    draw_text(title, cx + (card_w - td.width) * 0.5, cy + 48.0, title_fs, ACCENT_GOLD);

    let sub = "100% Rust 4X Engine  |  Parallel AI  |  Corps & Armies  |  <5ms Late-Game";
    let sub_fs = (card_w * 0.029).clamp(12.0, 16.0);
    let sd = measure_text(sub, None, sub_fs as u16, 1.0);
    draw_text(sub, cx + (card_w - sd.width) * 0.5, cy + 74.0, sub_fs, ACCENT_CYAN);

    let pad = 22.0;
    let mut y = cy + 102.0;
    let btn_h = 44.0;
    let inner_w = card_w - pad * 2.0;
    let lbl_fs = 17.0;

    // Map Size selector
    draw_text("Map Size:", cx + pad, y + 18.0, lbl_fs, LIGHTGRAY);
    y += 26.0;
    let sizes = [
        (MapSize::Small, "Small (1.2K)"),
        (MapSize::Standard, "Std (2.3K)"),
        (MapSize::Large, "Large (3.3K)"),
        (MapSize::Huge, "Huge (5.0K)"),
    ];
    let sw_btn = (inner_w - 18.0) / 4.0;
    for (idx, &(sz, lbl)) in sizes.iter().enumerate() {
        let r = Rect::new(cx + pad + idx as f32 * (sw_btn + 6.0), y, sw_btn, btn_h);
        let active = setup.size == sz;
        if ui_button(
            r,
            lbl,
            if active { Color::new(0.18, 0.34, 0.48, 1.0) } else { BG_CARD },
            if active { ACCENT_GOLD } else { BORDER_DIM },
            14.0,
        ) {
            setup.size = sz;
            setup.num_civs = sz.default_players();
        }
    }
    y += btn_h + 18.0;

    // Civilizations stepper
    draw_text(
        &format!("Civilizations: {}  (Max 22)", setup.num_civs),
        cx + pad,
        y + 28.0,
        lbl_fs,
        WHITE,
    );
    let minus_r = Rect::new(cx + card_w - pad - 116.0, y, 52.0, btn_h);
    let plus_r = Rect::new(cx + card_w - pad - 56.0, y, 52.0, btn_h);
    if ui_button(minus_r, "-", BG_CARD, BORDER_DIM, 22.0) && setup.num_civs > 2 {
        setup.num_civs -= 2;
        if setup.num_civs < 2 {
            setup.num_civs = 2;
        }
    }
    if ui_button(plus_r, "+", BG_CARD, BORDER_DIM, 22.0) && setup.num_civs < 22 {
        setup.num_civs = (setup.num_civs + 2).min(22);
    }
    y += btn_h + 14.0;

    // Mode toggle (Human Play vs AI Spectator Benchmark)
    draw_text("Game Mode:", cx + pad, y + 18.0, lbl_fs, LIGHTGRAY);
    y += 26.0;
    let half_w = (inner_w - 10.0) * 0.5;
    let is_human = setup.human_civ.is_some();
    if ui_button(
        Rect::new(cx + pad, y, half_w, btn_h),
        "Play as Aurelia",
        if is_human { Color::new(0.18, 0.36, 0.24, 1.0) } else { BG_CARD },
        if is_human { ACCENT_GREEN } else { BORDER_DIM },
        16.0,
    ) {
        setup.human_civ = Some(0);
    }
    if ui_button(
        Rect::new(cx + pad + half_w + 10.0, y, half_w, btn_h),
        "AI Spectator Mode",
        if !is_human { Color::new(0.32, 0.22, 0.44, 1.0) } else { BG_CARD },
        if !is_human { ACCENT_CYAN } else { BORDER_DIM },
        16.0,
    ) {
        setup.human_civ = None;
    }
    y += btn_h + 16.0;

    // Seed selector
    draw_text(&format!("Map Seed: {}", setup.seed), cx + pad, y + 28.0, lbl_fs, LIGHTGRAY);
    if ui_button(
        Rect::new(cx + card_w - pad - 126.0, y, 126.0, btn_h),
        "Random Seed",
        BG_CARD,
        BORDER_DIM,
        15.0,
    ) {
        setup.seed = (get_time() * 100_000.0) as u64 ^ 0x5A17;
    }
    y += btn_h + 22.0;

    // Primary launch buttons
    if ui_button(
        Rect::new(cx + pad, y, inner_w, 52.0),
        "LAUNCH NEW CAMPAIGN",
        Color::new(0.14, 0.46, 0.26, 1.0),
        ACCENT_GOLD,
        20.0,
    ) {
        return TitleAction::StartGame(*setup);
    }
    y += 62.0;

    if has_autosave {
        if ui_button(
            Rect::new(cx + pad, y, inner_w, 46.0),
            "CONTINUE SAVED GAME",
            Color::new(0.16, 0.26, 0.42, 1.0),
            ACCENT_CYAN,
            18.0,
        ) {
            return TitleAction::ContinueAutosave;
        }
        y += 54.0;
    }

    if !status_msg.is_empty() {
        draw_text(status_msg, cx + pad, y + 18.0, 15.0, ACCENT_GOLD);
    }

    TitleAction::None
}

/// Top HUD bar height in logical pixels.
pub const TOP_BAR_H: f32 = 54.0;
/// Bottom HUD bar height in logical pixels.
pub const BOTTOM_BAR_H: f32 = 88.0;

/// Draw Top + Bottom HUD bars and active Modal Overlay. Returns `(HudAction, ui_consumed_pointer)`.
#[allow(clippy::too_many_arguments)]
pub fn draw_game_hud(
    world: &World,
    rules: &Ruleset,
    viewer_civ: Option<CivId>,
    selected_tile: Option<TileId>,
    selected_unit: Option<UnitId>,
    modal: ActiveModal,
    last_metrics: TurnMetrics,
    speculative_ready: bool,
    auto_spectate: bool,
    toast: &str,
    city_scroll: &mut usize,
) -> (HudAction, bool) {
    let sw = screen_width();
    let sh = screen_height();
    let mut action = HudAction::None;

    let (_mx, my) = mouse_position();
    let in_top = my <= TOP_BAR_H;
    let in_bot = my >= sh - BOTTOM_BAR_H;
    let modal_open = modal != ActiveModal::None || world.victory.is_some();

    // === TOP BAR ===
    draw_rectangle(0.0, 0.0, sw, TOP_BAR_H, BG_PANEL);
    draw_line(0.0, TOP_BAR_H, sw, TOP_BAR_H, 1.8, BORDER_GOLD);

    let active_civ_id = viewer_civ.unwrap_or(0);
    let cs = &world.civs[active_civ_id as usize];
    let era_name = Ruleset::era_name(rules.era_for_techs(cs.tech_bits));

    // Left: Civ & Turn & Yields
    let top_fs = if sw < 720.0 { 14.0 } else { 17.0 };
    if viewer_civ.is_some() {
        let info_line1 = format!(
            "{}  |  Turn {} ({})  |  Gold {} ({:+})  |  Sci {:+}",
            cs.name, world.turn, era_name, cs.gold, cs.gold_per_turn, cs.science_per_turn
        );
        draw_text(&info_line1, 12.0, 22.0, top_fs, WHITE);

        let tech_str = cs
            .current_tech
            .map(|tid| {
                let t = &rules.techs[tid as usize];
                let rem = ((t.cost - cs.tech_progress).max(1) + cs.science_per_turn.max(1) - 1)
                    / cs.science_per_turn.max(1);
                format!("Tech: {} ({}t)", t.name, rem)
            })
            .unwrap_or_else(|| "Tech: Complete".to_string());

        let supply_col = if cs.supply_used > cs.supply_cap {
            ACCENT_RED
        } else if cs.supply_used + 1 >= cs.supply_cap {
            ACCENT_GOLD
        } else {
            ACCENT_GREEN
        };
        let supply_str = format!("Supply {}/{}", cs.supply_used, cs.supply_cap);
        draw_text(&supply_str, 12.0, 44.0, top_fs - 1.0, supply_col);
        draw_text(&tech_str, 138.0, 44.0, top_fs - 1.0, ACCENT_CYAN);
    } else {
        let (s, c, a) = world.units.formation_counts();
        let cities_alive = world.cities.alive.iter().filter(|&&x| x).count();
        let civs_alive = world.civs.iter().filter(|c| c.alive).count();
        draw_text(
            &format!(
                "AI SPECTATOR  |  Turn {}  |  {} Civs  {} Cities  {} Units ({}C/{}A)",
                world.turn, civs_alive, cities_alive, world.units.alive_count, c, a
            ),
            12.0,
            23.0,
            top_fs,
            ACCENT_GOLD,
        );
        draw_text(
            &format!("Singles: {}  Corps: {}  Armies: {}", s, c, a),
            12.0,
            44.0,
            top_fs - 1.0,
            LIGHTGRAY,
        );
    }

    // Right: Performance telemetry + TECH / DIPLO / MENU buttons
    let btn_w = if sw < 700.0 { 64.0 } else { 78.0 };
    let menu_r = Rect::new(sw - btn_w - 8.0, 8.0, btn_w, 38.0);
    let diplo_r = Rect::new(menu_r.x - btn_w - 6.0, 8.0, btn_w, 38.0);
    let tech_r = Rect::new(diplo_r.x - btn_w - 6.0, 8.0, btn_w, 38.0);

    if ui_button(menu_r, "MENU", BG_CARD, BORDER_GOLD, 15.0) {
        action = HudAction::OpenModal(ActiveModal::Menu);
    }
    if viewer_civ.is_some() {
        if ui_button(diplo_r, "DIPLO", BG_CARD, BORDER_DIM, 15.0) {
            action = HudAction::OpenModal(ActiveModal::Diplo);
        }
        if ui_button(tech_r, "TECH", BG_CARD, ACCENT_CYAN, 15.0) {
            action = HudAction::OpenModal(ActiveModal::Tech);
        }
    }

    // Performance pill (shows live Rust turn ms & FPS!)
    if sw >= 680.0 {
        let perf_str = format!(
            "Rust: {:.1}ms | {} FPS",
            last_metrics.total_ms,
            get_fps()
        );
        let pd = measure_text(&perf_str, None, 14, 1.0);
        let px = (if viewer_civ.is_some() { tech_r.x } else { menu_r.x }) - pd.width - 18.0;
        if px > 320.0 {
            draw_text(&perf_str, px, 32.0, 14.0, ACCENT_GREEN);
        }
    }

    // Toast notification banner under top bar
    if !toast.is_empty() {
        let td = measure_text(toast, None, 16, 1.0);
        let tw = td.width + 24.0;
        let tx = (sw - tw) * 0.5;
        draw_rectangle(tx, TOP_BAR_H + 8.0, tw, 30.0, Color::new(0.08, 0.12, 0.18, 0.92));
        draw_rectangle_lines(tx, TOP_BAR_H + 8.0, tw, 30.0, 1.4, ACCENT_GOLD);
        draw_text(toast, tx + 12.0, TOP_BAR_H + 28.0, 16.0, ACCENT_GOLD);
    }

    // === BOTTOM BAR ===
    let by = sh - BOTTOM_BAR_H;
    draw_rectangle(0.0, by, sw, BOTTOM_BAR_H, BG_PANEL);
    draw_line(0.0, by, sw, by, 1.8, BORDER_GOLD);

    // Primary right-side button: END TURN (or AUTO-PLAY / +50T in Spectator mode)
    let end_w = if sw < 640.0 { 124.0 } else { 156.0 };
    let end_r = Rect::new(sw - end_w - 10.0, by + 12.0, end_w, BOTTOM_BAR_H - 24.0);
    if viewer_civ.is_some() {
        let end_lbl = if speculative_ready { "END TURN *" } else { "END TURN" };
        if ui_button(
            end_r,
            end_lbl,
            Color::new(0.14, 0.48, 0.26, 1.0),
            if speculative_ready { ACCENT_GOLD } else { ACCENT_GREEN },
            18.0,
        ) {
            action = HudAction::EndTurn;
        }
    } else {
        let lbl = if auto_spectate { "PAUSE AI" } else { "RUN AI" };
        if ui_button(
            end_r,
            lbl,
            if auto_spectate { Color::new(0.56, 0.24, 0.20, 1.0) } else { Color::new(0.14, 0.46, 0.28, 1.0) },
            ACCENT_GOLD,
            18.0,
        ) {
            action = HudAction::ToggleAutoSpectate;
        }
        let ff_r = Rect::new(end_r.x - 108.0, by + 12.0, 100.0, BOTTOM_BAR_H - 24.0);
        if ui_button(ff_r, "+50 TURNS", Color::new(0.18, 0.28, 0.46, 1.0), ACCENT_CYAN, 15.0) {
            action = HudAction::FastForward(50);
        }
    }

    // Left / Center Context Panel: Selected Unit, City, or Tile info + contextual action buttons
    if let Some(uid) = selected_unit {
        if world.units.is_alive(uid) {
            let ui = uid as usize;
            let def = &rules.units[world.units.kind[ui] as usize];
            let form = world.units.formation[ui];
            let form_lbl = if form.label().is_empty() {
                String::new()
            } else {
                format!(" [{}]", form.label())
            };
            let title = format!("{}{}", def.name, form_lbl);
            draw_text(&title, 14.0, by + 24.0, 18.0, ACCENT_GOLD);

            let stats = format!(
                "HP {} | CS {} | Mov {:.1}/{} | XP {}",
                world.units.hp[ui],
                def.strength.max(def.ranged_strength) as i32 + form.strength_bonus(),
                world.units.moves_halves[ui] as f32 * 0.5,
                def.moves,
                world.units.xp[ui]
            );
            draw_text(&stats, 14.0, by + 45.0, 15.0, LIGHTGRAY);

            // Action buttons row (only if owned by human viewer)
            if viewer_civ == Some(world.units.civ[ui]) {
                let mut bx = 14.0;
                let ay = by + 52.0;
                let ah = 30.0;

                // MERGE CORPS / ARMY button (highlighted in gold when partner adjacent!)
                if let Some(partner) = find_merge_partner(world, rules, uid) {
                    if let Some(next_f) = can_merge_units(world, rules, uid, partner) {
                        let mlbl = format!("MERGE {}", next_f.label().to_uppercase());
                        let mr = Rect::new(bx, ay, 134.0, ah);
                        if ui_button(mr, &mlbl, Color::new(0.48, 0.36, 0.10, 1.0), ACCENT_GOLD, 14.0) {
                            action = HudAction::MergeUnit(uid, partner);
                        }
                        bx += 140.0;
                    }
                }

                if def.role == UnitRole::Settler {
                    let fr = Rect::new(bx, ay, 118.0, ah);
                    if ui_button(fr, "FOUND CITY", Color::new(0.16, 0.42, 0.24, 1.0), ACCENT_GREEN, 14.0) {
                        action = HudAction::FoundCity(uid);
                    }
                    bx += 124.0;
                }

                if def.role == UnitRole::Worker {
                    let pos = world.units.pos[ui];
                    let imp = best_improvement_for_tile(&world.map, pos);
                    if imp != Improvement::None && world.map.owner[pos as usize] == world.units.civ[ui] {
                        let ir = Rect::new(bx, ay, 132.0, ah);
                        if ui_button(
                            ir,
                            &format!("BUILD {}", imp.name().to_uppercase()),
                            Color::new(0.20, 0.34, 0.46, 1.0),
                            ACCENT_CYAN,
                            13.0,
                        ) {
                            action = HudAction::BuildImprovement(uid);
                        }
                        bx += 138.0;
                    }
                }

                if def.role.is_combat() {
                    let fr = Rect::new(bx, ay, 86.0, ah);
                    if ui_button(fr, "FORTIFY", BG_CARD, BORDER_DIM, 14.0) {
                        action = HudAction::FortifyUnit(uid);
                    }
                    bx += 92.0;
                }

                if bx + 98.0 < end_r.x {
                    let nr = Rect::new(bx, ay, 96.0, ah);
                    if ui_button(nr, "NEXT UNIT", BG_CARD, ACCENT_CYAN, 14.0) {
                        action = HudAction::NextUnit;
                    }
                }
            }
        }
    } else if let Some(tid) = selected_tile {
        let ti = tid as usize;
        let cid = world.cities.tile_city[ti];
        if cid != INVALID_CITY && world.cities.alive[cid as usize] {
            let ci = cid as usize;
            let y = compute_city_yields(world, rules, cid);
            let def_s = city_defense_strength(world, rules, cid);
            draw_text(
                &format!(
                    "{} (Pop {}, Def {}, HP {}/{})",
                    world.cities.name[ci], world.cities.pop[ci], def_s, world.cities.hp[ci], world.cities.max_hp[ci]
                ),
                14.0,
                by + 25.0,
                18.0,
                ACCENT_GOLD,
            );
            draw_text(
                &format!(
                    "Food {:+} | Prod {} | Gold {:+} | Sci {} | Cult {}",
                    y.food, y.prod, y.gold, y.science, y.culture
                ),
                14.0,
                by + 46.0,
                15.0,
                LIGHTGRAY,
            );
            if viewer_civ == Some(world.cities.civ[ci]) {
                let cr = Rect::new(14.0, by + 52.0, 142.0, 30.0);
                if ui_button(cr, "MANAGE CITY", Color::new(0.18, 0.34, 0.50, 1.0), ACCENT_GOLD, 15.0) {
                    *city_scroll = 0;
                    action = HudAction::OpenModal(ActiveModal::City(cid));
                }
            }
        } else {
            let y = tile_yields(&world.map, tid);
            let feat = world.map.feature[ti].name();
            let res = world.map.resource[ti].name();
            let imp = world.map.improvement[ti].name();
            let desc = format!(
                "{}{}{}{}",
                world.map.terrain[ti].name(),
                if feat.is_empty() { String::new() } else { format!(" / {feat}") },
                if res.is_empty() { String::new() } else { format!("  [{res}]") },
                if imp.is_empty() { String::new() } else { format!("  ({imp})") },
            );
            draw_text(&desc, 14.0, by + 26.0, 17.0, WHITE);
            draw_text(
                &format!("Yields: {}F  {}P  {}G  {}S", y.food, y.prod, y.gold, y.science),
                14.0,
                by + 48.0,
                15.0,
                LIGHTGRAY,
            );
            if viewer_civ.is_some() {
                let nr = Rect::new(14.0, by + 54.0, 108.0, 28.0);
                if ui_button(nr, "NEXT UNIT", BG_CARD, ACCENT_CYAN, 14.0) {
                    action = HudAction::NextUnit;
                }
            }
        }
    } else if viewer_civ.is_some() {
        draw_text("Tap a unit to command, or tap a city to manage.", 14.0, by + 30.0, 16.0, LIGHTGRAY);
        let nr = Rect::new(14.0, by + 48.0, 124.0, 32.0);
        if ui_button(nr, "NEXT UNIT", BG_CARD, ACCENT_CYAN, 15.0) {
            action = HudAction::NextUnit;
        }
    }

    // === MODAL OVERLAYS ===
    if let Some(vic) = world.victory {
        let wname = &world.civs[vic.winner as usize].name;
        let vkind = match vic.kind {
            VictoryKind::Domination => "DOMINATION VICTORY",
            VictoryKind::Science => "SCIENCE VICTORY (Orbital Launch!)",
            VictoryKind::Score => "SCORE VICTORY",
        };
        let mw = (sw * 0.85).min(460.0);
        let mh = 230.0;
        let mx0 = (sw - mw) * 0.5;
        let my0 = (sh - mh) * 0.5;
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.58));
        draw_rectangle(mx0, my0, mw, mh, BG_PANEL);
        draw_rectangle_lines(mx0, my0, mw, mh, 2.5, ACCENT_GOLD);
        draw_text(vkind, mx0 + 24.0, my0 + 48.0, 24.0, ACCENT_GOLD);
        draw_text(
            &format!("{} triumphs on Turn {}!", wname, vic.turn),
            mx0 + 24.0,
            my0 + 92.0,
            19.0,
            WHITE,
        );
        if ui_button(
            Rect::new(mx0 + 24.0, my0 + 148.0, mw - 48.0, 48.0),
            "RETURN TO TITLE",
            Color::new(0.18, 0.38, 0.24, 1.0),
            ACCENT_GOLD,
            18.0,
        ) {
            action = HudAction::ReturnToTitle;
        }
        return (action, true);
    }

    match modal {
        ActiveModal::None => {}
        ActiveModal::City(cid) => {
            if let Some(a) = draw_city_modal(world, rules, cid, city_scroll) {
                action = a;
            }
        }
        ActiveModal::Tech => {
            if let Some(hciv) = viewer_civ {
                if let Some(a) = draw_tech_modal(world, rules, hciv) {
                    action = a;
                }
            }
        }
        ActiveModal::Diplo => {
            if let Some(hciv) = viewer_civ {
                if let Some(a) = draw_diplo_modal(world, hciv) {
                    action = a;
                }
            }
        }
        ActiveModal::Menu => {
            if let Some(a) = draw_menu_modal() {
                action = a;
            }
        }
    }

    let ui_consumed = modal_open || in_top || in_bot;
    (action, ui_consumed)
}

fn draw_modal_frame(title: &str) -> (Rect, bool) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.56));
    let w = (sw * 0.92).min(680.0);
    let h = (sh * 0.84).min(560.0);
    let rect = Rect::new((sw - w) * 0.5, (sh - h) * 0.5, w, h);
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, BG_PANEL);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.2, BORDER_GOLD);
    draw_text(title, rect.x + 18.0, rect.y + 34.0, 22.0, ACCENT_GOLD);

    let close_r = Rect::new(rect.x + rect.w - 86.0, rect.y + 10.0, 74.0, 32.0);
    let closed = ui_button(close_r, "CLOSE", Color::new(0.42, 0.16, 0.16, 1.0), ACCENT_RED, 15.0);
    draw_line(rect.x, rect.y + 50.0, rect.x + rect.w, rect.y + 50.0, 1.2, BORDER_DIM);
    (rect, closed)
}

fn draw_city_modal(world: &World, rules: &Ruleset, cid: CityId, scroll: &mut usize) -> Option<HudAction> {
    if !world.cities.is_alive(cid) {
        return Some(HudAction::CloseModal);
    }
    let ci = cid as usize;
    let civ = world.cities.civ[ci];
    let cs = &world.civs[civ as usize];
    let y = compute_city_yields(world, rules, cid);
    let title = format!("{}  (Pop {})", world.cities.name[ci], world.cities.pop[ci]);
    let (rect, closed) = draw_modal_frame(&title);
    if closed {
        return Some(HudAction::CloseModal);
    }

    let prod_name = match world.cities.producing[ci] {
        Some(ProductionItem::Unit(u)) => rules.units[u as usize].name.as_str(),
        Some(ProductionItem::Building(b)) => rules.buildings[b as usize].name.as_str(),
        None => "Nothing",
    };
    draw_text(
        &format!(
            "Producing: {}   |   Yields: {:+}F  {}P  {:+}G  {}S  {}C",
            prod_name, y.food, y.prod, y.gold, y.science, y.culture
        ),
        rect.x + 18.0,
        rect.y + 74.0,
        15.0,
        ACCENT_CYAN,
    );

    // Gather all available production items (units + buildings)
    let mut items: Vec<(ProductionItem, String, i32, String)> = Vec::new();
    for u in &rules.units {
        if rules.can_train_unit(cs.tech_bits, u.id) {
            if u.role.is_naval() && !world.map.is_adjacent_to_water(world.cities.pos[ci]) {
                continue;
            }
            let detail = format!("CS {} | Mov {} | Upk {}", u.strength.max(u.ranged_strength), u.moves, u.upkeep);
            items.push((ProductionItem::Unit(u.id), format!("[Unit] {}", u.name), u.cost, detail));
        }
    }
    for b in &rules.buildings {
        if rules.can_build_building(cs.tech_bits, world.cities.buildings[ci], b.id) {
            let detail = format!("+{}F +{}P +{}S +{}G +{}Sup", b.food, b.production, b.science, b.gold, b.supply);
            items.push((ProductionItem::Building(b.id), format!("[Bldg] {}", b.name), b.cost, detail));
        }
    }

    let row_h = 42.0;
    let list_top = rect.y + 90.0;
    let visible_rows = (((rect.h - 144.0) / row_h).floor() as usize).max(3);
    let max_scroll = items.len().saturating_sub(visible_rows);
    let (_, wy_wheel) = mouse_wheel();
    if wy_wheel < -0.1 && *scroll < max_scroll {
        *scroll += 1;
    } else if wy_wheel > 0.1 && *scroll > 0 {
        *scroll -= 1;
    }
    *scroll = (*scroll).min(max_scroll);

    for (slot, (item, label, cost, detail)) in items.iter().skip(*scroll).take(visible_rows).enumerate() {
        let ry = list_top + (slot as f32) * row_h;
        let is_cur = world.cities.producing[ci] == Some(*item);
        draw_rectangle(
            rect.x + 14.0,
            ry,
            rect.w - 28.0,
            row_h - 4.0,
            if is_cur { Color::new(0.16, 0.28, 0.38, 0.96) } else { BG_CARD },
        );
        let turns = ((*cost - world.cities.prod_stored[ci]).max(1) + y.prod.max(1) as i32 - 1) / y.prod.max(1) as i32;
        draw_text(label, rect.x + 22.0, ry + 18.0, 15.0, WHITE);
        draw_text(detail, rect.x + 22.0, ry + 34.0, 12.0, LIGHTGRAY);

        let build_r = Rect::new(rect.x + rect.w - 196.0, ry + 5.0, 88.0, row_h - 14.0);
        if ui_button(
            build_r,
            &format!("{}t", turns.max(1)),
            Color::new(0.14, 0.36, 0.24, 1.0),
            ACCENT_GREEN,
            14.0,
        ) {
            return Some(HudAction::SetProduction(cid, *item));
        }
        let gold_cost = *cost * 2;
        let can_buy = cs.gold >= gold_cost;
        let buy_r = Rect::new(rect.x + rect.w - 102.0, ry + 5.0, 84.0, row_h - 14.0);
        if ui_button(
            buy_r,
            &format!("{}g", gold_cost),
            if can_buy { Color::new(0.42, 0.32, 0.10, 1.0) } else { Color::new(0.16, 0.16, 0.18, 0.8) },
            if can_buy { ACCENT_GOLD } else { BORDER_DIM },
            14.0,
        ) && can_buy
        {
            return Some(HudAction::RushBuy(cid, *item));
        }
    }

    // Touch scroll buttons at bottom of modal
    let by = rect.y + rect.h - 44.0;
    if *scroll > 0 && ui_button(Rect::new(rect.x + 18.0, by, 110.0, 32.0), "SCROLL UP", BG_CARD, BORDER_DIM, 14.0) {
        *scroll = scroll.saturating_sub(2);
    }
    if *scroll < max_scroll
        && ui_button(
            Rect::new(rect.x + 138.0, by, 122.0, 32.0),
            "SCROLL DOWN",
            BG_CARD,
            BORDER_DIM,
            14.0,
        )
    {
        *scroll = (*scroll + 2).min(max_scroll);
    }

    None
}

fn draw_tech_modal(world: &World, rules: &Ruleset, hciv: CivId) -> Option<HudAction> {
    let (rect, closed) = draw_modal_frame("TECHNOLOGY TREE (4 Eras)");
    if closed {
        return Some(HudAction::CloseModal);
    }
    let cs = &world.civs[hciv as usize];
    let col_w = (rect.w - 36.0) / 4.0;

    for era in 0u8..4 {
        let ex = rect.x + 14.0 + (era as f32) * (col_w + 3.0);
        draw_text(Ruleset::era_name(era), ex + 6.0, rect.y + 72.0, 16.0, ACCENT_GOLD);

        let mut row = 0;
        for t in &rules.techs {
            if t.era != era {
                continue;
            }
            let ty = rect.y + 82.0 + (row as f32) * 52.0;
            if ty + 46.0 > rect.y + rect.h - 8.0 {
                break;
            }
            let researched = Ruleset::has_tech(cs.tech_bits, t.id);
            let current = cs.current_tech == Some(t.id);
            let avail = rules.can_research(cs.tech_bits, t.id);

            let (bg, border) = if researched {
                (Color::new(0.12, 0.34, 0.20, 0.95), ACCENT_GREEN)
            } else if current {
                (Color::new(0.12, 0.30, 0.46, 0.98), ACCENT_CYAN)
            } else if avail {
                (BG_CARD, ACCENT_GOLD)
            } else {
                (Color::new(0.08, 0.09, 0.12, 0.85), BORDER_DIM)
            };

            let tag = if t.unlocks_corps {
                " [CORPS]"
            } else if t.unlocks_army {
                " [ARMY]"
            } else if t.unlocks_science_victory {
                " [WIN]"
            } else {
                ""
            };
            let lbl = format!("{}{}", t.name, tag);
            let r = Rect::new(ex, ty, col_w - 6.0, 46.0);
            if ui_button(r, &lbl, bg, border, 12.0) && avail {
                return Some(HudAction::SelectTech(t.id));
            }
            row += 1;
        }
    }
    None
}

fn draw_diplo_modal(world: &World, hciv: CivId) -> Option<HudAction> {
    let (rect, closed) = draw_modal_frame("DIPLOMACY & EMPIRES");
    if closed {
        return Some(HudAction::CloseModal);
    }

    let mut row = 0;
    let row_h = 40.0;
    let top = rect.y + 62.0;
    for other in &world.civs {
        if other.id == hciv || !other.alive || !world.diplo.has_met(hciv, other.id) {
            continue;
        }
        let ry = top + (row as f32) * row_h;
        if ry + row_h > rect.y + rect.h - 10.0 {
            break;
        }
        draw_rectangle(rect.x + 14.0, ry, rect.w - 28.0, row_h - 4.0, BG_CARD);
        let sw_col = Color::from_rgba(other.color[0], other.color[1], other.color[2], 255);
        draw_rectangle(rect.x + 20.0, ry + 8.0, 14.0, 20.0, sw_col);

        let st = world.diplo.status(hciv, other.id);
        let st_lbl = match st {
            DiploStatus::War => "AT WAR",
            DiploStatus::Peace => "PEACE",
            DiploStatus::Alliance => "ALLY",
            DiploStatus::Unmet => "UNMET",
        };
        let st_col = if st == DiploStatus::War { ACCENT_RED } else { ACCENT_GREEN };
        draw_text(
            &format!("{}   (Score {})", other.name, other.score),
            rect.x + 42.0,
            ry + 24.0,
            16.0,
            WHITE,
        );
        draw_text(st_lbl, rect.x + rect.w - 220.0, ry + 24.0, 15.0, st_col);

        let btn_r = Rect::new(rect.x + rect.w - 134.0, ry + 4.0, 114.0, row_h - 12.0);
        if st == DiploStatus::War {
            if ui_button(btn_r, "MAKE PEACE", Color::new(0.14, 0.36, 0.24, 1.0), ACCENT_GREEN, 13.0) {
                return Some(HudAction::MakePeace(other.id));
            }
        } else if ui_button(btn_r, "DECLARE WAR", Color::new(0.46, 0.16, 0.16, 1.0), ACCENT_RED, 13.0) {
            return Some(HudAction::DeclareWar(other.id));
        }
        row += 1;
    }

    if row == 0 {
        draw_text(
            "No other civilizations met yet. Explore the map with your Scout!",
            rect.x + 22.0,
            rect.y + 96.0,
            16.0,
            LIGHTGRAY,
        );
    }
    None
}

fn draw_menu_modal() -> Option<HudAction> {
    let (rect, closed) = draw_modal_frame("GAME MENU & BENCHMARK TOOLS");
    if closed {
        return Some(HudAction::CloseModal);
    }
    let bw = rect.w - 48.0;
    let bx = rect.x + 24.0;
    let mut y = rect.y + 68.0;
    let bh = 46.0;

    if ui_button(Rect::new(bx, y, bw, bh), "SAVE GAME", Color::new(0.14, 0.38, 0.24, 1.0), ACCENT_GREEN, 17.0) {
        return Some(HudAction::SaveGame);
    }
    y += bh + 10.0;
    if ui_button(Rect::new(bx, y, bw, bh), "LOAD SAVED GAME", Color::new(0.16, 0.28, 0.46, 1.0), ACCENT_CYAN, 17.0) {
        return Some(HudAction::LoadGame);
    }
    y += bh + 14.0;

    let third = (bw - 16.0) / 3.0;
    if ui_button(Rect::new(bx, y, third, bh), "SIM +10 TURNS", BG_CARD, ACCENT_GOLD, 15.0) {
        return Some(HudAction::FastForward(10));
    }
    if ui_button(Rect::new(bx + third + 8.0, y, third, bh), "SIM +50 TURNS", BG_CARD, ACCENT_GOLD, 15.0) {
        return Some(HudAction::FastForward(50));
    }
    if ui_button(Rect::new(bx + (third + 8.0) * 2.0, y, third, bh), "SIM +100 TURNS", BG_CARD, ACCENT_GOLD, 15.0) {
        return Some(HudAction::FastForward(100));
    }
    y += bh + 12.0;

    if ui_button(Rect::new(bx, y, bw, bh), "TOGGLE REVEAL FULL MAP", BG_CARD, BORDER_DIM, 16.0) {
        return Some(HudAction::ToggleRevealMap);
    }
    y += bh + 14.0;

    if ui_button(
        Rect::new(bx, y, bw, bh),
        "RETURN TO TITLE SCREEN",
        Color::new(0.46, 0.16, 0.16, 1.0),
        ACCENT_RED,
        17.0,
    ) {
        return Some(HudAction::ReturnToTitle);
    }
    None
}
