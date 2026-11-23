use crate::codex::{draw_codex_modal, CodexState};
use crate::settings::{
    draw_modal_frame, ui_button, AppOptions, TouchScroll, UiMetrics, ACCENT_CYAN, ACCENT_GOLD,
    ACCENT_GREEN, ACCENT_PURPLE, ACCENT_RED, BG_CARD, BG_PANEL, BORDER_DIM, BORDER_GOLD,
};
use hexforge_sim::combat::{can_merge_units, city_defense_strength, find_merge_partner};
use hexforge_sim::economy::{
    best_improvement_for_tile, compute_city_yields, rush_buy_cost, tile_yields,
};
use hexforge_sim::policies::{
    can_adopt_policy, can_afford_any_policy, has_policy, policy_cost, CULTURE_VICTORY_POLICIES,
};
use hexforge_sim::ruleset::Ruleset;
use hexforge_sim::types::{
    CityId, CivId, DiploStatus, GameConfig, Improvement, MapSize, PolicyId, ProductionItem, Road,
    TechId, TileId, UnitId, UnitRole, VictoryKind, INVALID_CITY, VICTORY_CULTURE,
    VICTORY_DOMINATION, VICTORY_SCIENCE, VICTORY_SCORE,
};
use hexforge_sim::world::World;
use hexforge_sim::TurnMetrics;
use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModal {
    None,
    City(CityId),
    Tech,
    Policies,
    Diplo,
    Codex,
    Options,
    Menu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitleTab {
    #[default]
    Play,
    RulesAndCiv,
    Options,
}

#[derive(Debug, Default)]
pub struct TitleUiState {
    pub tab: TitleTab,
    pub show_codex: bool,
    pub codex: CodexState,
    pub scroll: TouchScroll,
}

#[derive(Debug, Default)]
pub struct GameUiState {
    pub modal: ActiveModalDefault,
    pub tech_era_tab: u8,
    pub policy_tree_tab: u8,
    pub city_scroll: TouchScroll,
    pub diplo_scroll: TouchScroll,
    pub codex: CodexState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveModalDefault(pub ActiveModal);
impl Default for ActiveModalDefault {
    fn default() -> Self {
        Self(ActiveModal::None)
    }
}

#[derive(Debug, Clone)]
pub enum HudAction {
    None,
    EndTurn,
    NextUnit,
    MergeUnit(UnitId, UnitId),
    FoundCity(UnitId),
    BuildImprovement(UnitId),
    BuildRoad(UnitId),
    FortifyUnit(UnitId),
    OpenModal(ActiveModal),
    CloseModal,
    SetProduction(CityId, ProductionItem),
    RushBuy(CityId, ProductionItem),
    SelectTech(TechId),
    AdoptPolicy(PolicyId),
    DeclareWar(CivId),
    MakePeace(CivId),
    SaveGame,
    LoadGame,
    FastForward(u32),
    ToggleRevealMap,
    ToggleAutoSpectate,
    OptionsChanged,
    ReturnToTitle,
}

#[derive(Debug, Clone)]
pub enum TitleAction {
    None,
    StartGame(GameConfig),
    ContinueAutosave,
    OptionsChanged,
}

/// Draw the responsive, DPI-scaled Title & Custom Rules Setup Screen.
pub fn draw_title_screen(
    ui: &UiMetrics,
    rules: &Ruleset,
    setup: &mut GameConfig,
    options: &mut AppOptions,
    state: &mut TitleUiState,
    has_autosave: bool,
    status_msg: &str,
) -> TitleAction {
    clear_background(Color::from_rgba(11, 15, 23, 255));

    // Full-screen tiled decorative hex backdrop covering 100% of any aspect ratio
    let hex_r = ui.dp(34.0);
    let cols = (ui.sw / (hex_r * 1.732)).ceil() as i32 + 2;
    let rows = (ui.sh / (hex_r * 1.5)).ceil() as i32 + 2;
    for row in 0..rows {
        for col in 0..cols {
            let cx = col as f32 * hex_r * 1.732 + if row % 2 == 1 { hex_r * 0.866 } else { 0.0 };
            let cy = row as f32 * hex_r * 1.5;
            draw_poly_lines(cx, cy, 6, hex_r, 30.0, ui.dp(1.0), Color::new(0.24, 0.34, 0.48, 0.22));
        }
    }

    // Full-screen Codex overlay when opened from Title Screen
    if state.show_codex {
        if draw_codex_modal(ui, rules, &mut state.codex) {
            state.show_codex = false;
        }
        return TitleAction::None;
    }

    let card_w = (ui.sw * 0.94).min(ui.dp(700.0));
    let card_h = (ui.sh * 0.92).min(ui.dp(560.0));
    let cx = (ui.sw - card_w) * 0.5;
    let cy = (ui.sh - card_h) * 0.5;
    let pad = ui.dp(14.0);
    let inner_w = card_w - pad * 2.0;

    draw_rectangle(cx, cy, card_w, card_h, BG_PANEL);
    draw_rectangle_lines(cx, cy, card_w, card_h, ui.dp(2.4), BORDER_GOLD);

    // Header banner
    let title = "HEXFORGE-RUST  v0.3";
    let title_fs = ui.fs(24.0);
    let td = measure_text(title, None, title_fs as u16, 1.0);
    draw_text(title, cx + (card_w - td.width) * 0.5, cy + ui.dp(30.0), title_fs, ACCENT_GOLD);

    let sub = "100% Pure-Rust 4X  |  6 Eras  |  50 Techs  |  Corps & Armies";
    let sub_fs = ui.fs(12.5);
    let sd = measure_text(sub, None, sub_fs as u16, 1.0);
    draw_text(sub, cx + (card_w - sd.width) * 0.5, cy + ui.dp(48.0), sub_fs, ACCENT_CYAN);

    // 4 Big Navigation Tabs: PLAY | RULES & CIV | OPTIONS | CODEX
    let tab_y = cy + ui.dp(56.0);
    let tab_h = ui.dp(38.0);
    let tab_w = (inner_w - ui.dp(18.0)) / 4.0;
    let nav_items = [
        (Some(TitleTab::Play), "NEW GAME"),
        (Some(TitleTab::RulesAndCiv), "RULES & CIV"),
        (Some(TitleTab::Options), "OPTIONS"),
        (None, "CODEX / ?"),
    ];
    for (idx, &(t_opt, lbl)) in nav_items.iter().enumerate() {
        let r = Rect::new(cx + pad + (idx as f32) * (tab_w + ui.dp(6.0)), tab_y, tab_w, tab_h);
        let active = t_opt.is_some() && Some(state.tab) == t_opt;
        let bg = if active {
            Color::new(0.16, 0.34, 0.48, 1.0)
        } else if t_opt.is_none() {
            Color::new(0.24, 0.18, 0.38, 1.0)
        } else {
            BG_CARD
        };
        let border = if active {
            ACCENT_GOLD
        } else if t_opt.is_none() {
            ACCENT_PURPLE
        } else {
            BORDER_DIM
        };
        if ui_button(r, lbl, bg, border, ui.fs(13.0)) {
            if let Some(t) = t_opt {
                state.tab = t;
                state.scroll.reset();
            } else {
                state.show_codex = true;
                state.codex.scroll.reset();
            }
        }
    }

    let body_top = tab_y + tab_h + ui.dp(10.0);
    let footer_h = ui.dp(56.0);
    let body_h = (cy + card_h - footer_h - body_top).max(ui.dp(100.0));
    let mut result = TitleAction::None;

    match state.tab {
        TitleTab::Play => {
            let mut y = body_top + ui.dp(2.0);
            let btn_h = ui.dp(40.0).min(body_h * 0.21);
            let gap = ui.dp(8.0);
            let lbl_fs = ui.fs(14.5);

            // Map Size
            draw_text("Map Size:", cx + pad, y + ui.dp(14.0), lbl_fs, LIGHTGRAY);
            y += ui.dp(20.0);
            let sizes = [
                (MapSize::Small, "Small 1.2K"),
                (MapSize::Standard, "Std 2.3K"),
                (MapSize::Large, "Large 3.3K"),
                (MapSize::Huge, "Huge 5K"),
            ];
            let sw_btn = (inner_w - ui.dp(18.0)) / 4.0;
            for (idx, &(sz, lbl)) in sizes.iter().enumerate() {
                let r = Rect::new(cx + pad + (idx as f32) * (sw_btn + ui.dp(6.0)), y, sw_btn, btn_h);
                let active = setup.size == sz;
                if ui_button(
                    r,
                    lbl,
                    if active { Color::new(0.18, 0.36, 0.50, 1.0) } else { BG_CARD },
                    if active { ACCENT_GOLD } else { BORDER_DIM },
                    ui.fs(13.5),
                ) {
                    setup.size = sz;
                    setup.num_civs = sz.default_players();
                }
            }
            y += btn_h + gap;

            // Civilizations + Seed side-by-side in Landscape or stacked in Portrait
            let step_w = ui.dp(48.0);
            draw_text(
                &format!("Empires: {} (Max 22)", setup.num_civs),
                cx + pad,
                y + btn_h * 0.64,
                lbl_fs,
                WHITE,
            );
            if ui_button(
                Rect::new(cx + card_w - pad - step_w * 2.0 - ui.dp(6.0), y, step_w, btn_h),
                "-",
                BG_CARD,
                BORDER_DIM,
                ui.fs(20.0),
            ) && setup.num_civs > 2
            {
                setup.num_civs = (setup.num_civs.saturating_sub(2)).max(2);
            }
            if ui_button(
                Rect::new(cx + card_w - pad - step_w, y, step_w, btn_h),
                "+",
                BG_CARD,
                BORDER_DIM,
                ui.fs(20.0),
            ) && setup.num_civs < 22
            {
                setup.num_civs = (setup.num_civs + 2).min(22);
            }
            y += btn_h + gap;

            // Game Mode (Play Chosen Civ vs AI Spectator)
            let half_w = (inner_w - ui.dp(8.0)) * 0.5;
            let is_human = setup.human_civ.is_some();
            let chosen_name = &rules.civs[(setup.rules.human_civ_template as usize).min(rules.civs.len() - 1)].name;
            if ui_button(
                Rect::new(cx + pad, y, half_w, btn_h),
                &format!("Play as {chosen_name}"),
                if is_human { Color::new(0.16, 0.38, 0.24, 1.0) } else { BG_CARD },
                if is_human { ACCENT_GREEN } else { BORDER_DIM },
                ui.fs(14.5),
            ) {
                setup.human_civ = Some(0);
            }
            if ui_button(
                Rect::new(cx + pad + half_w + ui.dp(8.0), y, half_w, btn_h),
                "AI Spectator Mode",
                if !is_human { Color::new(0.32, 0.20, 0.44, 1.0) } else { BG_CARD },
                if !is_human { ACCENT_CYAN } else { BORDER_DIM },
                ui.fs(14.5),
            ) {
                setup.human_civ = None;
            }
            y += btn_h + gap;

            // Active Rules Quick Summary + Seed Randomizer
            let summary = format!(
                "{} | {} | {} | {} | Seed {}",
                setup.rules.difficulty.name(),
                setup.rules.speed.name(),
                setup.rules.map_type.name(),
                Ruleset::era_name(setup.rules.start_era),
                setup.seed
            );
            draw_text(&summary, cx + pad, y + btn_h * 0.62, ui.fs(13.0), ACCENT_GOLD);
            let rnd_w = ui.dp(106.0);
            if ui_button(
                Rect::new(cx + card_w - pad - rnd_w, y, rnd_w, btn_h * 0.9),
                "NEW SEED",
                BG_CARD,
                BORDER_DIM,
                ui.fs(12.5),
            ) {
                setup.seed = ((get_time() * 100_000.0) as u64 ^ 0x5A17) % 99999 + 1;
            }
        }
        TitleTab::RulesAndCiv => {
            let mut y = body_top + ui.dp(2.0);
            let row_h = ui.dp(38.0).min(body_h * 0.19);
            let gap = ui.dp(7.0);

            // 1. Civilization Picker (all 22 Civs with Unique Traits!)
            let tpl_idx = (setup.rules.human_civ_template as usize).min(rules.civs.len() - 1);
            let cdef = &rules.civs[tpl_idx];
            let sw_col = Color::from_rgba(cdef.color[0], cdef.color[1], cdef.color[2], 255);
            draw_rectangle(cx + pad, y, inner_w, row_h + ui.dp(6.0), BG_CARD);
            draw_rectangle(cx + pad, y, ui.dp(8.0), row_h + ui.dp(6.0), sw_col);
            draw_text(
                &format!("Civ: {} — {}", cdef.name, cdef.trait_name),
                cx + pad + ui.dp(14.0),
                y + ui.dp(18.0),
                ui.fs(14.5),
                ACCENT_GOLD,
            );
            draw_text(
                &cdef.trait_desc,
                cx + pad + ui.dp(14.0),
                y + ui.dp(35.0),
                ui.fs(12.5),
                WHITE,
            );
            let nav_w = ui.dp(46.0);
            if ui_button(
                Rect::new(cx + card_w - pad - nav_w * 2.0 - ui.dp(10.0), y + ui.dp(4.0), nav_w, row_h - ui.dp(2.0)),
                "<",
                BG_PANEL,
                BORDER_GOLD,
                ui.fs(18.0),
            ) {
                setup.rules.human_civ_template = if tpl_idx == 0 {
                    (rules.civs.len() - 1) as u8
                } else {
                    (tpl_idx - 1) as u8
                };
            }
            if ui_button(
                Rect::new(cx + card_w - pad - nav_w - ui.dp(4.0), y + ui.dp(4.0), nav_w, row_h - ui.dp(2.0)),
                ">",
                BG_PANEL,
                BORDER_GOLD,
                ui.fs(18.0),
            ) {
                setup.rules.human_civ_template = ((tpl_idx + 1) % rules.civs.len()) as u8;
            }
            y += row_h + ui.dp(12.0);

            // 2. Difficulty + Game Speed
            let half = (inner_w - ui.dp(8.0)) * 0.5;
            if ui_button(
                Rect::new(cx + pad, y, half, row_h),
                &format!("Difficulty: {}", setup.rules.difficulty.name()),
                BG_CARD,
                ACCENT_GOLD,
                ui.fs(13.5),
            ) {
                setup.rules.difficulty = setup.rules.difficulty.next();
            }
            if ui_button(
                Rect::new(cx + pad + half + ui.dp(8.0), y, half, row_h),
                &format!("Speed: {} ({}t)", setup.rules.speed.name(), setup.rules.speed.default_turn_limit()),
                BG_CARD,
                ACCENT_CYAN,
                ui.fs(13.5),
            ) {
                setup.rules.speed = setup.rules.speed.next();
                setup.turn_limit = setup.rules.speed.default_turn_limit();
            }
            y += row_h + gap;

            // 3. Map Type + Start Era
            if ui_button(
                Rect::new(cx + pad, y, half, row_h),
                &format!("Map: {}", setup.rules.map_type.name()),
                BG_CARD,
                BORDER_DIM,
                ui.fs(13.5),
            ) {
                setup.rules.map_type = setup.rules.map_type.next();
            }
            if ui_button(
                Rect::new(cx + pad + half + ui.dp(8.0), y, half, row_h),
                &format!("Start Era: {}", Ruleset::era_name(setup.rules.start_era)),
                BG_CARD,
                BORDER_DIM,
                ui.fs(13.5),
            ) {
                setup.rules.start_era = (setup.rules.start_era + 1) % 5;
            }
            y += row_h + gap;

            // 4. Advanced Rule Toggles (Corps, Fog of War, Turn Limit)
            let third = (inner_w - ui.dp(12.0)) / 3.0;
            if ui_button(
                Rect::new(cx + pad, y, third, row_h),
                if setup.rules.allow_corps_armies { "Corps/Armies: ON" } else { "Corps/Armies: OFF" },
                if setup.rules.allow_corps_armies { Color::new(0.14, 0.34, 0.22, 1.0) } else { BG_CARD },
                if setup.rules.allow_corps_armies { ACCENT_GREEN } else { BORDER_DIM },
                ui.fs(12.5),
            ) {
                setup.rules.allow_corps_armies = !setup.rules.allow_corps_armies;
            }
            if ui_button(
                Rect::new(cx + pad + third + ui.dp(6.0), y, third, row_h),
                if setup.rules.fog_of_war { "Fog of War: ON" } else { "Fog of War: OFF" },
                if setup.rules.fog_of_war { Color::new(0.14, 0.34, 0.22, 1.0) } else { BG_CARD },
                if setup.rules.fog_of_war { ACCENT_GREEN } else { BORDER_DIM },
                ui.fs(12.5),
            ) {
                setup.rules.fog_of_war = !setup.rules.fog_of_war;
            }
            if ui_button(
                Rect::new(cx + pad + (third + ui.dp(6.0)) * 2.0, y, third, row_h),
                if setup.rules.no_turn_limit { "Turn Limit: OFF" } else { "Turn Limit: ON" },
                if !setup.rules.no_turn_limit { Color::new(0.14, 0.34, 0.22, 1.0) } else { BG_CARD },
                if !setup.rules.no_turn_limit { ACCENT_GREEN } else { BORDER_DIM },
                ui.fs(12.5),
            ) {
                setup.rules.no_turn_limit = !setup.rules.no_turn_limit;
            }
            y += row_h + gap;

            // 5. Victory Condition Toggles
            let qtr = (inner_w - ui.dp(18.0)) / 4.0;
            let v_items = [
                (VICTORY_DOMINATION, "Domination"),
                (VICTORY_SCIENCE, "Science"),
                (VICTORY_CULTURE, "Culture"),
                (VICTORY_SCORE, "Score"),
            ];
            for (idx, &(bit, name)) in v_items.iter().enumerate() {
                let on = (setup.rules.victory_mask & bit) != 0;
                let r = Rect::new(cx + pad + (idx as f32) * (qtr + ui.dp(6.0)), y, qtr, row_h * 0.9);
                if ui_button(
                    r,
                    &format!("{}: {}", name, if on { "ON" } else { "OFF" }),
                    if on { Color::new(0.18, 0.30, 0.44, 1.0) } else { BG_CARD },
                    if on { ACCENT_CYAN } else { BORDER_DIM },
                    ui.fs(12.0),
                ) {
                    setup.rules.victory_mask ^= bit;
                    if setup.rules.victory_mask == 0 {
                        setup.rules.victory_mask = VICTORY_DOMINATION;
                    }
                }
            }
        }
        TitleTab::Options => {
            if draw_options_body(
                ui,
                Rect::new(cx + pad, body_top, inner_w, body_h),
                options,
            ) {
                result = TitleAction::OptionsChanged;
            }
        }
    }

    // Footer Launch & Continue Bar
    let fy = cy + card_h - footer_h + ui.dp(4.0);
    let fh = footer_h - ui.dp(12.0);
    if has_autosave {
        let launch_w = inner_w * 0.56;
        let cont_w = inner_w - launch_w - ui.dp(8.0);
        if ui_button(
            Rect::new(cx + pad, fy, launch_w, fh),
            "LAUNCH CAMPAIGN",
            Color::new(0.14, 0.48, 0.26, 1.0),
            ACCENT_GOLD,
            ui.fs(17.0),
        ) {
            return TitleAction::StartGame(*setup);
        }
        if ui_button(
            Rect::new(cx + pad + launch_w + ui.dp(8.0), fy, cont_w, fh),
            "CONTINUE SAVE",
            Color::new(0.16, 0.28, 0.46, 1.0),
            ACCENT_CYAN,
            ui.fs(15.0),
        ) {
            return TitleAction::ContinueAutosave;
        }
    } else if ui_button(
        Rect::new(cx + pad, fy, inner_w, fh),
        "LAUNCH NEW CAMPAIGN",
        Color::new(0.14, 0.48, 0.26, 1.0),
        ACCENT_GOLD,
        ui.fs(18.0),
    ) {
        return TitleAction::StartGame(*setup);
    }

    if !status_msg.is_empty() {
        draw_text(status_msg, cx + pad, cy + card_h - footer_h - ui.dp(4.0), ui.fs(13.0), ACCENT_GOLD);
    }

    result
}

/// Reusable Options Body (used on BOTH Title Screen Options tab AND In-Game Options modal).
/// Returns `true` whenever an option was changed so caller persists `options.toml`.
pub fn draw_options_body(ui: &UiMetrics, area: Rect, options: &mut AppOptions) -> bool {
    let mut changed = false;
    let mut y = area.y + ui.dp(2.0);
    let row_h = ui.dp(38.0).min(area.h * 0.21);
    let gap = ui.dp(8.0);

    // 1. Live UI Scale selector (fixes small buttons on ANY phone/tablet!)
    draw_text(
        &format!("UI Button Scale: {:.0}%  (Live Preview)", options.ui_scale_mult * 100.0),
        area.x,
        y + ui.dp(15.0),
        ui.fs(14.5),
        ACCENT_GOLD,
    );
    y += ui.dp(20.0);
    let scales = [0.75f32, 0.90, 1.00, 1.15, 1.30, 1.50];
    let sbw = (area.w - ui.dp(25.0)) / 6.0;
    for (idx, &sc) in scales.iter().enumerate() {
        let active = (options.ui_scale_mult - sc).abs() < 0.03;
        let r = Rect::new(area.x + (idx as f32) * (sbw + ui.dp(5.0)), y, sbw, row_h);
        if ui_button(
            r,
            &format!("{:.0}%", sc * 100.0),
            if active { Color::new(0.18, 0.38, 0.54, 1.0) } else { BG_CARD },
            if active { ACCENT_GOLD } else { BORDER_DIM },
            ui.fs(13.0),
        ) {
            options.ui_scale_mult = sc;
            changed = true;
        }
    }
    y += row_h + gap;

    // 2. Map & HUD Toggle Grid (2 columns x 3 rows)
    let half = (area.w - ui.dp(8.0)) * 0.5;
    let toggles: [(&str, bool, usize); 6] = [
        ("Tile Yields", options.show_tile_yields, 0),
        ("Hex Grid Lines", options.show_hex_grid, 1),
        ("Resource Badges", options.show_resources, 2),
        ("Rust Perf / FPS", options.show_perf_hud, 3),
        ("Auto Next Unit", options.auto_next_unit, 4),
        (
            if options.autosave_interval == 0 {
                "Autosave: OFF"
            } else if options.autosave_interval == 5 {
                "Autosave: 5 Turns"
            } else if options.autosave_interval == 10 {
                "Autosave: 10 Turns"
            } else {
                "Autosave: 25 Turns"
            },
            options.autosave_interval > 0,
            5,
        ),
    ];

    for (i, &(lbl, on, id)) in toggles.iter().enumerate() {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        let rx = area.x + col * (half + ui.dp(8.0));
        let ry = y + row * (row_h + gap);
        let text = if id == 5 {
            lbl.to_string()
        } else {
            format!("{}: {}", lbl, if on { "ON" } else { "OFF" })
        };
        if ui_button(
            Rect::new(rx, ry, half, row_h),
            &text,
            if on { Color::new(0.14, 0.34, 0.24, 1.0) } else { BG_CARD },
            if on { ACCENT_GREEN } else { BORDER_DIM },
            ui.fs(13.5),
        ) {
            match id {
                0 => options.show_tile_yields = !options.show_tile_yields,
                1 => options.show_hex_grid = !options.show_hex_grid,
                2 => options.show_resources = !options.show_resources,
                3 => options.show_perf_hud = !options.show_perf_hud,
                4 => options.auto_next_unit = !options.auto_next_unit,
                _ => options.cycle_autosave(),
            }
            changed = true;
        }
    }

    changed
}

/// Draw DPI-scaled Top Bar, Bottom Action Bar, and Active Modal Overlay.
#[allow(clippy::too_many_arguments)]
pub fn draw_game_hud(
    ui: &UiMetrics,
    options: &mut AppOptions,
    world: &World,
    rules: &Ruleset,
    viewer_civ: Option<CivId>,
    selected_tile: Option<TileId>,
    selected_unit: Option<UnitId>,
    gui: &mut GameUiState,
    last_metrics: TurnMetrics,
    speculative_ready: bool,
    auto_spectate: bool,
    toast: &str,
) -> (HudAction, bool) {
    let mut action = HudAction::None;
    let (_mx, my) = mouse_position();
    let in_top = my <= ui.top_bar_h;
    let in_bot = my >= ui.sh - ui.bottom_bar_h;
    let modal = gui.modal.0;
    let modal_open = modal != ActiveModal::None || world.victory.is_some();

    // ==================== TOP BAR ====================
    draw_rectangle(0.0, 0.0, ui.sw, ui.top_bar_h, BG_PANEL);
    draw_line(0.0, ui.top_bar_h, ui.sw, ui.top_bar_h, ui.dp(1.8), BORDER_GOLD);

    let active_civ_id = viewer_civ.unwrap_or(0);
    let cs = &world.civs[active_civ_id as usize];
    let era_name = Ruleset::era_name(rules.era_for_techs(cs.tech_bits));
    let pol_ready = viewer_civ.map_or(false, |vc| can_afford_any_policy(world, rules, vc));

    let pad = ui.dp(8.0);
    let top_fs = ui.fs(if ui.is_portrait { 12.5 } else { 14.5 });

    // Top Navigation Buttons
    let btn_h = if ui.is_portrait { ui.top_bar_h * 0.44 } else { ui.top_bar_h - ui.dp(14.0) };
    let btn_y = if ui.is_portrait { ui.top_bar_h - btn_h - ui.dp(4.0) } else { ui.dp(7.0) };

    if viewer_civ.is_some() {
        let btns: [(&str, ActiveModal, Color, Color); 5] = [
            ("TECH", ActiveModal::Tech, BG_CARD, ACCENT_CYAN),
            (
                if pol_ready { "POL !" } else { "POL" },
                ActiveModal::Policies,
                if pol_ready { Color::new(0.36, 0.16, 0.48, 1.0) } else { BG_CARD },
                if pol_ready { ACCENT_GOLD } else { ACCENT_PURPLE },
            ),
            ("DIPLO", ActiveModal::Diplo, BG_CARD, BORDER_DIM),
            ("RULES", ActiveModal::Codex, BG_CARD, BORDER_DIM),
            ("MENU", ActiveModal::Menu, BG_CARD, BORDER_GOLD),
        ];
        if ui.is_portrait {
            let bw = (ui.sw - pad * 2.0 - ui.dp(20.0)) / 5.0;
            for (i, &(lbl, m, bg, bdr)) in btns.iter().enumerate() {
                let r = Rect::new(pad + (i as f32) * (bw + ui.dp(5.0)), btn_y, bw, btn_h);
                if ui_button(r, lbl, bg, bdr, ui.fs(12.5)) {
                    action = HudAction::OpenModal(m);
                }
            }
        } else {
            let bw = ui.dp(66.0);
            let mut rx = ui.sw - pad;
            for &(lbl, m, bg, bdr) in btns.iter().rev() {
                rx -= bw;
                let r = Rect::new(rx, btn_y, bw, btn_h);
                if ui_button(r, lbl, bg, bdr, ui.fs(12.5)) {
                    action = HudAction::OpenModal(m);
                }
                rx -= ui.dp(5.0);
            }
        }

        // Yields & Turn info
        let line1 = format!(
            "{} T{} ({}) | {}g({:+}) | {:+}s | {:+}c",
            cs.name, world.turn, era_name, cs.gold, cs.gold_per_turn, cs.science_per_turn, cs.culture_per_turn
        );
        draw_text(&line1, pad, ui.dp(18.0), top_fs, WHITE);

        let supply_col = if cs.supply_used > cs.supply_cap {
            ACCENT_RED
        } else if cs.supply_used + 1 >= cs.supply_cap {
            ACCENT_GOLD
        } else {
            ACCENT_GREEN
        };
        let tech_str = cs
            .current_tech
            .map(|tid| {
                let t = &rules.techs[tid as usize];
                let cost = Ruleset::scaled_cost(t.cost, world.config.rules.speed);
                let rem = ((cost - cs.tech_progress).max(1) + cs.science_per_turn.max(1) - 1)
                    / cs.science_per_turn.max(1);
                format!("{} ({}t)", t.name, rem)
            })
            .unwrap_or_else(|| "All Techs".into());
        let line2_y = if ui.is_portrait { ui.dp(34.0) } else { ui.dp(40.0) };
        let sup_txt = format!("Sup {}/{}", cs.supply_used, cs.supply_cap);
        draw_text(&sup_txt, pad, line2_y, top_fs - ui.dp(0.8), supply_col);
        draw_text(&format!("Tech: {tech_str}"), pad + ui.dp(92.0), line2_y, top_fs - ui.dp(0.8), ACCENT_CYAN);

        if options.show_perf_hud && !ui.is_portrait {
            let perf = format!("{:.1}ms | {}fps", last_metrics.total_ms, get_fps());
            draw_text(&perf, pad + ui.dp(275.0), line2_y, ui.fs(12.0), ACCENT_GREEN);
        }
    } else {
        // AI Spectator Top Bar
        let (s, c, a) = world.units.formation_counts();
        let cities_alive = world.cities.alive.iter().filter(|&&x| x).count();
        let civs_alive = world.civs.iter().filter(|c| c.alive).count();
        draw_text(
            &format!(
                "AI SPECTATOR T{} | {} Civs | {} Cities | {} Units ({}S/{}C/{}A)",
                world.turn, civs_alive, cities_alive, world.units.alive_count, s, c, a
            ),
            pad,
            ui.dp(20.0),
            top_fs,
            ACCENT_GOLD,
        );
        if options.show_perf_hud {
            draw_text(
                &format!("Rust Turn: {:.2} ms  |  {} FPS", last_metrics.total_ms, get_fps()),
                pad,
                ui.dp(40.0),
                ui.fs(13.0),
                ACCENT_GREEN,
            );
        }
        let bw = ui.dp(74.0);
        let menu_r = Rect::new(ui.sw - bw - pad, btn_y, bw, btn_h);
        let codex_r = Rect::new(menu_r.x - bw - ui.dp(6.0), btn_y, bw, btn_h);
        if ui_button(menu_r, "MENU", BG_CARD, BORDER_GOLD, ui.fs(13.0)) {
            action = HudAction::OpenModal(ActiveModal::Menu);
        }
        if ui_button(codex_r, "RULES", BG_CARD, BORDER_DIM, ui.fs(13.0)) {
            action = HudAction::OpenModal(ActiveModal::Codex);
        }
    }

    // Toast notification banner
    if !toast.is_empty() {
        let tfs = ui.fs(14.5);
        let td = measure_text(toast, None, tfs as u16, 1.0);
        let tw = (td.width + ui.dp(22.0)).min(ui.sw * 0.92);
        let th = ui.dp(30.0);
        let tx = (ui.sw - tw) * 0.5;
        let ty = ui.top_bar_h + ui.dp(6.0);
        draw_rectangle(tx, ty, tw, th, Color::new(0.08, 0.12, 0.19, 0.94));
        draw_rectangle_lines(tx, ty, tw, th, ui.dp(1.4), ACCENT_GOLD);
        draw_text(toast, tx + ui.dp(10.0), ty + th * 0.70, tfs, ACCENT_GOLD);
    }

    // ==================== BOTTOM BAR ====================
    let by = ui.sh - ui.bottom_bar_h;
    draw_rectangle(0.0, by, ui.sw, ui.bottom_bar_h, BG_PANEL);
    draw_line(0.0, by, ui.sw, by, ui.dp(1.8), BORDER_GOLD);

    let end_w = if ui.is_portrait { ui.dp(116.0) } else { ui.dp(144.0) };
    let end_h = if ui.is_portrait { ui.dp(46.0) } else { ui.bottom_bar_h - ui.dp(20.0) };
    let end_r = Rect::new(ui.sw - end_w - pad, by + ui.dp(10.0), end_w, end_h);

    if viewer_civ.is_some() {
        let end_lbl = if speculative_ready { "END TURN *" } else { "END TURN" };
        if ui_button(
            end_r,
            end_lbl,
            Color::new(0.14, 0.48, 0.26, 1.0),
            if speculative_ready { ACCENT_GOLD } else { ACCENT_GREEN },
            ui.fs(16.0),
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
            ui.fs(15.5),
        ) {
            action = HudAction::ToggleAutoSpectate;
        }
        let ff_w = ui.dp(92.0);
        let ff_r = Rect::new(end_r.x - ff_w - ui.dp(6.0), end_r.y, ff_w, end_h);
        if ui_button(ff_r, "+50 TURNS", Color::new(0.18, 0.28, 0.46, 1.0), ACCENT_CYAN, ui.fs(13.5)) {
            action = HudAction::FastForward(50);
        }
    }

    // Bottom Context Info + Large Touch Action Buttons
    let act_h = ui.dp(34.0).min(ui.bottom_bar_h * 0.42);
    let act_y = ui.sh - act_h - ui.dp(8.0);

    if let Some(uid) = selected_unit {
        if world.units.is_alive(uid) {
            let ui_idx = uid as usize;
            let def = &rules.units[world.units.kind[ui_idx] as usize];
            let form = world.units.formation[ui_idx];
            let flbl = if form.label().is_empty() {
                String::new()
            } else {
                format!(" [{}]", form.label())
            };
            draw_text(
                &format!("{}{}", def.name, flbl),
                pad,
                by + ui.dp(20.0),
                ui.fs(16.0),
                ACCENT_GOLD,
            );
            draw_text(
                &format!(
                    "HP {} | CS {} | Mov {:.1}/{} | XP {}",
                    world.units.hp[ui_idx],
                    def.strength.max(def.ranged_strength) as i32 + form.strength_bonus(),
                    world.units.moves_halves[ui_idx] as f32 * 0.5,
                    def.moves,
                    world.units.xp[ui_idx]
                ),
                pad,
                by + ui.dp(39.0),
                ui.fs(13.5),
                LIGHTGRAY,
            );

            if viewer_civ == Some(world.units.civ[ui_idx]) {
                let mut bx = pad;
                let max_x = if ui.is_portrait { ui.sw - pad } else { end_r.x - ui.dp(6.0) };

                if let Some(partner) = find_merge_partner(world, rules, uid) {
                    if let Some(next_f) = can_merge_units(world, rules, uid, partner) {
                        let mw = ui.dp(120.0);
                        if bx + mw <= max_x {
                            if ui_button(
                                Rect::new(bx, act_y, mw, act_h),
                                &format!("MERGE {}", next_f.label().to_uppercase()),
                                Color::new(0.48, 0.36, 0.10, 1.0),
                                ACCENT_GOLD,
                                ui.fs(13.0),
                            ) {
                                action = HudAction::MergeUnit(uid, partner);
                            }
                            bx += mw + ui.dp(6.0);
                        }
                    }
                }

                if def.role == UnitRole::Settler {
                    let fw = ui.dp(108.0);
                    if bx + fw <= max_x {
                        if ui_button(
                            Rect::new(bx, act_y, fw, act_h),
                            "FOUND CITY",
                            Color::new(0.16, 0.44, 0.24, 1.0),
                            ACCENT_GREEN,
                            ui.fs(13.0),
                        ) {
                            action = HudAction::FoundCity(uid);
                        }
                        bx += fw + ui.dp(6.0);
                    }
                }

                if def.role == UnitRole::Worker {
                    let pos = world.units.pos[ui_idx];
                    let imp = best_improvement_for_tile(&world.map, pos);
                    if imp != Improvement::None
                        && world.map.owner[pos as usize] == world.units.civ[ui_idx]
                        && world.map.improvement[pos as usize] != imp
                    {
                        let iw = ui.dp(116.0);
                        if bx + iw <= max_x {
                            if ui_button(
                                Rect::new(bx, act_y, iw, act_h),
                                &format!("BUILD {}", imp.name().to_uppercase()),
                                Color::new(0.18, 0.34, 0.48, 1.0),
                                ACCENT_CYAN,
                                ui.fs(12.5),
                            ) {
                                action = HudAction::BuildImprovement(uid);
                            }
                            bx += iw + ui.dp(6.0);
                        }
                    }
                    // Road / Railroad construction button!
                    let techs = world.civs[world.units.civ[ui_idx] as usize].tech_bits;
                    let can_rail = Ruleset::has_tech(techs, rules.railroad_tech)
                        && world.map.road[pos as usize] != Road::Railroad;
                    let can_road = Ruleset::has_tech(techs, rules.road_tech)
                        && world.map.road[pos as usize] == Road::None;
                    if can_rail || can_road {
                        let rw = ui.dp(92.0);
                        if bx + rw <= max_x {
                            if ui_button(
                                Rect::new(bx, act_y, rw, act_h),
                                if can_rail { "RAILROAD" } else { "ROAD" },
                                Color::new(0.32, 0.26, 0.16, 1.0),
                                ACCENT_GOLD,
                                ui.fs(12.5),
                            ) {
                                action = HudAction::BuildRoad(uid);
                            }
                            bx += rw + ui.dp(6.0);
                        }
                    }
                }

                if def.role.is_combat() && !def.role.is_air() {
                    let fw = ui.dp(82.0);
                    if bx + fw <= max_x {
                        if ui_button(Rect::new(bx, act_y, fw, act_h), "FORTIFY", BG_CARD, BORDER_DIM, ui.fs(13.0)) {
                            action = HudAction::FortifyUnit(uid);
                        }
                        bx += fw + ui.dp(6.0);
                    }
                }

                let nw = ui.dp(92.0);
                if bx + nw <= max_x {
                    if ui_button(Rect::new(bx, act_y, nw, act_h), "NEXT UNIT", BG_CARD, ACCENT_CYAN, ui.fs(13.0)) {
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
                    "{} (Pop {} | Def {} | HP {}/{})",
                    world.cities.name[ci], world.cities.pop[ci], def_s, world.cities.hp[ci], world.cities.max_hp[ci]
                ),
                pad,
                by + ui.dp(20.0),
                ui.fs(15.5),
                ACCENT_GOLD,
            );
            draw_text(
                &format!("{:+}F  {}P  {:+}G  {}S  {}C", y.food, y.prod, y.gold, y.science, y.culture),
                pad,
                by + ui.dp(39.0),
                ui.fs(13.5),
                LIGHTGRAY,
            );
            if viewer_civ == Some(world.cities.civ[ci]) {
                let cw = ui.dp(136.0);
                if ui_button(
                    Rect::new(pad, act_y, cw, act_h),
                    "MANAGE CITY",
                    Color::new(0.18, 0.34, 0.50, 1.0),
                    ACCENT_GOLD,
                    ui.fs(13.5),
                ) {
                    gui.city_scroll.reset();
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
                if res.is_empty() { String::new() } else { format!(" [{res}]") },
                if imp.is_empty() { String::new() } else { format!(" ({imp})") },
            );
            draw_text(&desc, pad, by + ui.dp(20.0), ui.fs(15.0), WHITE);
            draw_text(
                &format!("Yields: {}F  {}P  {}G  {}S", y.food, y.prod, y.gold, y.science),
                pad,
                by + ui.dp(39.0),
                ui.fs(13.5),
                LIGHTGRAY,
            );
            if viewer_civ.is_some()
                && ui_button(
                    Rect::new(pad, act_y, ui.dp(106.0), act_h),
                    "NEXT UNIT",
                    BG_CARD,
                    ACCENT_CYAN,
                    ui.fs(13.0),
                )
            {
                action = HudAction::NextUnit;
            }
        }
    } else if viewer_civ.is_some() {
        draw_text(
            "Tap a unit to move/attack, or tap a city to manage.",
            pad,
            by + ui.dp(24.0),
            ui.fs(14.0),
            LIGHTGRAY,
        );
        if ui_button(
            Rect::new(pad, act_y, ui.dp(118.0), act_h),
            "NEXT UNIT",
            BG_CARD,
            ACCENT_CYAN,
            ui.fs(13.5),
        ) {
            action = HudAction::NextUnit;
        }
    }

    // ==================== VICTORY BANNER ====================
    if let Some(vic) = world.victory {
        let wname = &world.civs[vic.winner as usize].name;
        let vkind = match vic.kind {
            VictoryKind::Domination => "DOMINATION VICTORY",
            VictoryKind::Science => "SCIENCE VICTORY!",
            VictoryKind::Culture => "CULTURE VICTORY!",
            VictoryKind::Score => "SCORE VICTORY",
        };
        let (rect, body_top, _) = draw_modal_frame(ui, vkind);
        draw_text(
            &format!("{} triumphs on Turn {}!", wname, vic.turn),
            rect.x + ui.dp(20.0),
            body_top + ui.dp(46.0),
            ui.fs(19.0),
            WHITE,
        );
        if ui_button(
            Rect::new(
                rect.x + ui.dp(20.0),
                rect.y + rect.h - ui.dp(66.0),
                rect.w - ui.dp(40.0),
                ui.dp(48.0),
            ),
            "RETURN TO TITLE",
            Color::new(0.16, 0.44, 0.26, 1.0),
            ACCENT_GOLD,
            ui.fs(17.0),
        ) {
            action = HudAction::ReturnToTitle;
        }
        return (action, true);
    }

    // ==================== ACTIVE MODAL OVERLAYS ====================
    match modal {
        ActiveModal::None => {}
        ActiveModal::City(cid) => {
            if let Some(a) = draw_city_modal(ui, world, rules, cid, &mut gui.city_scroll) {
                action = a;
            }
        }
        ActiveModal::Tech => {
            if let Some(hciv) = viewer_civ {
                if let Some(a) = draw_tech_modal(ui, world, rules, hciv, &mut gui.tech_era_tab) {
                    action = a;
                }
            }
        }
        ActiveModal::Policies => {
            if let Some(hciv) = viewer_civ {
                if let Some(a) = draw_policies_modal(ui, world, rules, hciv, &mut gui.policy_tree_tab) {
                    action = a;
                }
            }
        }
        ActiveModal::Diplo => {
            if let Some(hciv) = viewer_civ {
                if let Some(a) = draw_diplo_modal(ui, world, rules, hciv, &mut gui.diplo_scroll) {
                    action = a;
                }
            }
        }
        ActiveModal::Codex => {
            if draw_codex_modal(ui, rules, &mut gui.codex) {
                action = HudAction::CloseModal;
            }
        }
        ActiveModal::Options => {
            let (rect, body_top, closed) = draw_modal_frame(ui, "OPTIONS & DISPLAY");
            if closed {
                action = HudAction::CloseModal;
            } else {
                let pad = ui.dp(14.0);
                if draw_options_body(
                    ui,
                    Rect::new(
                        rect.x + pad,
                        body_top + ui.dp(8.0),
                        rect.w - pad * 2.0,
                        rect.y + rect.h - body_top - ui.dp(18.0),
                    ),
                    options,
                ) {
                    action = HudAction::OptionsChanged;
                }
            }
        }
        ActiveModal::Menu => {
            if let Some(a) = draw_menu_modal(ui) {
                action = a;
            }
        }
    }

    let ui_consumed = modal_open || in_top || in_bot;
    (action, ui_consumed)
}

fn draw_city_modal(
    ui: &UiMetrics,
    world: &World,
    rules: &Ruleset,
    cid: CityId,
    scroll: &mut TouchScroll,
) -> Option<HudAction> {
    if !world.cities.is_alive(cid) {
        return Some(HudAction::CloseModal);
    }
    let ci = cid as usize;
    let civ = world.cities.civ[ci];
    let cs = &world.civs[civ as usize];
    let y = compute_city_yields(world, rules, cid);
    let (rect, body_top, closed) = draw_modal_frame(ui, &format!("{} (Pop {})", world.cities.name[ci], world.cities.pop[ci]));
    if closed {
        return Some(HudAction::CloseModal);
    }

    let pad = ui.dp(12.0);
    let prod_name = match world.cities.producing[ci] {
        Some(ProductionItem::Unit(u)) => rules.units[u as usize].name.as_str(),
        Some(ProductionItem::Building(b)) => rules.buildings[b as usize].name.as_str(),
        None => "Nothing",
    };
    draw_text(
        &format!(
            "Building: {} | {:+}F {}P {:+}G {}S {}C",
            prod_name, y.food, y.prod, y.gold, y.science, y.culture
        ),
        rect.x + pad,
        body_top + ui.dp(20.0),
        ui.fs(13.5),
        ACCENT_CYAN,
    );

    let mut items: Vec<(ProductionItem, String, i32, String, bool)> = Vec::new();
    for u in &rules.units {
        if rules.can_train_unit(cs.tech_bits, u.id) {
            if u.role.is_naval() && !world.map.is_adjacent_to_water(world.cities.pos[ci]) {
                continue;
            }
            let cost = Ruleset::scaled_cost(u.cost, world.config.rules.speed);
            let detail = format!(
                "{} | CS {} | Mov {} | Upk {}",
                u.role.name(),
                u.strength.max(u.ranged_strength),
                u.moves,
                u.upkeep
            );
            items.push((ProductionItem::Unit(u.id), u.name.clone(), cost, detail, false));
        }
    }
    for b in &rules.buildings {
        if rules.can_build_building(cs.tech_bits, world.cities.buildings[ci], world.wonders_built, b.id) {
            let cost = Ruleset::scaled_cost(b.cost, world.config.rules.speed);
            let tag = if b.is_wonder { "[WONDER] " } else { "" };
            let detail = format!(
                "+{}F +{}P +{}S +{}G +{}C +{}Sup",
                b.food, b.production, b.science, b.gold, b.culture, b.supply
            );
            items.push((
                ProductionItem::Building(b.id),
                format!("{tag}{}", b.name),
                cost,
                detail,
                b.is_wonder,
            ));
        }
    }

    let list_top = body_top + ui.dp(28.0);
    let footer_h = ui.dp(42.0);
    let list_h = (rect.y + rect.h - footer_h - list_top).max(ui.dp(80.0));
    let list_area = Rect::new(rect.x + pad, list_top, rect.w - pad * 2.0, list_h);
    let row_h = ui.dp(48.0);
    let max_scroll = ((items.len() as f32) * row_h - list_h).max(0.0);
    scroll.update(list_area, max_scroll, ui);

    for (idx, (item, label, cost, detail, is_wonder)) in items.iter().enumerate() {
        let ry = list_top + (idx as f32) * row_h - scroll.offset;
        if ry < list_top - ui.dp(4.0) || ry + row_h > list_top + list_h + ui.dp(4.0) {
            continue;
        }
        let is_cur = world.cities.producing[ci] == Some(*item);
        draw_rectangle(
            list_area.x,
            ry,
            list_area.w,
            row_h - ui.dp(4.0),
            if is_cur { Color::new(0.16, 0.28, 0.40, 0.98) } else { BG_CARD },
        );
        let turns = ((*cost - world.cities.prod_stored[ci]).max(1) + y.prod.max(1) as i32 - 1) / y.prod.max(1) as i32;
        draw_text(
            label,
            list_area.x + ui.dp(8.0),
            ry + ui.dp(18.0),
            ui.fs(14.0),
            if *is_wonder { ACCENT_GOLD } else { WHITE },
        );
        draw_text(detail, list_area.x + ui.dp(8.0), ry + ui.dp(36.0), ui.fs(11.5), LIGHTGRAY);

        let bw = ui.dp(74.0);
        let bh = row_h - ui.dp(14.0);
        let build_x = list_area.x + list_area.w - bw * 2.0 - ui.dp(10.0);
        if ui_button(
            Rect::new(build_x, ry + ui.dp(5.0), bw, bh),
            &format!("{}t", turns.max(1)),
            Color::new(0.14, 0.38, 0.24, 1.0),
            ACCENT_GREEN,
            ui.fs(13.5),
        ) {
            return Some(HudAction::SetProduction(cid, *item));
        }

        if !*is_wonder {
            let gold_cost = rush_buy_cost(world, rules, civ, *item);
            let can_buy = cs.gold >= gold_cost;
            if ui_button(
                Rect::new(build_x + bw + ui.dp(6.0), ry + ui.dp(5.0), bw, bh),
                &format!("{gold_cost}g"),
                if can_buy { Color::new(0.42, 0.32, 0.10, 1.0) } else { Color::new(0.16, 0.16, 0.20, 0.8) },
                if can_buy { ACCENT_GOLD } else { BORDER_DIM },
                ui.fs(13.0),
            ) && can_buy
            {
                return Some(HudAction::RushBuy(cid, *item));
            }
        }
    }

    let fy = rect.y + rect.h - footer_h + ui.dp(4.0);
    let fh = footer_h - ui.dp(10.0);
    let sbw = ui.dp(114.0);
    if scroll.offset > 1.0
        && ui_button(Rect::new(rect.x + pad, fy, sbw, fh), "UP", BG_CARD, BORDER_DIM, ui.fs(13.0))
    {
        scroll.offset = (scroll.offset - list_h * 0.65).max(0.0);
    }
    if scroll.offset < max_scroll - 1.0
        && ui_button(Rect::new(rect.x + pad + sbw + ui.dp(8.0), fy, sbw, fh), "DOWN", BG_CARD, BORDER_DIM, ui.fs(13.0))
    {
        scroll.offset = (scroll.offset + list_h * 0.65).min(max_scroll);
    }
    None
}

/// 6-Era Mobile-Friendly Tech Tree Modal (1 Era per big tab -> huge clickable cards!).
fn draw_tech_modal(
    ui: &UiMetrics,
    world: &World,
    rules: &Ruleset,
    hciv: CivId,
    era_tab: &mut u8,
) -> Option<HudAction> {
    let (rect, body_top, closed) = draw_modal_frame(ui, "TECHNOLOGY TREE (6 ERAS / 50 TECHS)");
    if closed {
        return Some(HudAction::CloseModal);
    }
    let cs = &world.civs[hciv as usize];
    let pad = ui.dp(10.0);
    let tab_h = ui.dp(34.0);
    let tab_w = (rect.w - pad * 2.0 - ui.dp(20.0)) / 6.0;

    for era in 0..6u8 {
        let r = Rect::new(
            rect.x + pad + (era as f32) * (tab_w + ui.dp(4.0)),
            body_top + ui.dp(6.0),
            tab_w,
            tab_h,
        );
        let active = *era_tab == era;
        if ui_button(
            r,
            Ruleset::era_name(era),
            if active { Color::new(0.16, 0.34, 0.48, 1.0) } else { BG_CARD },
            if active { ACCENT_GOLD } else { BORDER_DIM },
            ui.fs(11.5),
        ) {
            *era_tab = era;
        }
    }

    let techs: Vec<_> = rules.techs.iter().filter(|t| t.era == *era_tab).collect();
    let grid_top = body_top + tab_h + ui.dp(14.0);
    let cols = if ui.is_portrait { 2usize } else { 3usize };
    let rows = (techs.len() + cols - 1) / cols;
    let avail_h = (rect.y + rect.h - grid_top - ui.dp(12.0)).max(ui.dp(120.0));
    let card_w = (rect.w - pad * 2.0 - ui.dp(8.0) * (cols as f32 - 1.0)) / (cols as f32);
    let card_h = ((avail_h - ui.dp(8.0) * (rows as f32 - 1.0)) / (rows.max(1) as f32)).clamp(ui.dp(50.0), ui.dp(86.0));

    for (i, t) in techs.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let rx = rect.x + pad + (col as f32) * (card_w + ui.dp(8.0));
        let ry = grid_top + (row as f32) * (card_h + ui.dp(8.0));

        let researched = Ruleset::has_tech(cs.tech_bits, t.id);
        let current = cs.current_tech == Some(t.id);
        let avail = rules.can_research(cs.tech_bits, t.id);

        let (bg, border) = if researched {
            (Color::new(0.12, 0.36, 0.20, 0.96), ACCENT_GREEN)
        } else if current {
            (Color::new(0.12, 0.30, 0.48, 0.98), ACCENT_CYAN)
        } else if avail {
            (BG_CARD, ACCENT_GOLD)
        } else {
            (Color::new(0.08, 0.09, 0.13, 0.88), BORDER_DIM)
        };

        let tag = if t.unlocks_corps {
            " [CORPS]"
        } else if t.unlocks_army {
            " [ARMY]"
        } else if t.unlocks_flight {
            " [AIR]"
        } else if t.unlocks_railroads {
            " [RAIL]"
        } else if t.unlocks_roads {
            " [ROADS]"
        } else if t.unlocks_science_victory {
            " [SCI WIN]"
        } else {
            ""
        };
        let cost = Ruleset::scaled_cost(t.cost, world.config.rules.speed);
        let status = if researched {
            "DONE".into()
        } else if current {
            format!("{}/{}", cs.tech_progress, cost)
        } else {
            format!("{cost} Sci")
        };
        let lbl = format!("{}{} ({})", t.name, tag, status);
        if ui_button(Rect::new(rx, ry, card_w, card_h), &lbl, bg, border, ui.fs(13.0)) && avail {
            return Some(HudAction::SelectTech(t.id));
        }
    }
    None
}

/// 5-Tree Social Policies / Empire Civics Modal (12 policies = Culture Victory!).
fn draw_policies_modal(
    ui: &UiMetrics,
    world: &World,
    rules: &Ruleset,
    hciv: CivId,
    tree_tab: &mut u8,
) -> Option<HudAction> {
    let cs = &world.civs[hciv as usize];
    let adopted = cs.policy_bits.count_ones();
    let cost = policy_cost(adopted, world.config.rules.speed);
    let (rect, body_top, closed) = draw_modal_frame(
        ui,
        &format!("SOCIAL POLICIES ({adopted}/{CULTURE_VICTORY_POLICIES} for Culture Win)"),
    );
    if closed {
        return Some(HudAction::CloseModal);
    }

    let pad = ui.dp(10.0);
    let can_buy = cs.culture_stored >= cost;
    draw_text(
        &format!(
            "Culture: {} / {} (+{}/turn)  {}",
            cs.culture_stored,
            cost,
            cs.culture_per_turn,
            if can_buy { "— TAP AN AVAILABLE POLICY TO ADOPT!" } else { "" }
        ),
        rect.x + pad,
        body_top + ui.dp(20.0),
        ui.fs(13.5),
        if can_buy { ACCENT_GOLD } else { ACCENT_PURPLE },
    );

    let tab_y = body_top + ui.dp(28.0);
    let tab_h = ui.dp(36.0);
    let tab_w = (rect.w - pad * 2.0 - ui.dp(16.0)) / 5.0;
    for tree in 0..5u8 {
        let r = Rect::new(rect.x + pad + (tree as f32) * (tab_w + ui.dp(4.0)), tab_y, tab_w, tab_h);
        let active = *tree_tab == tree;
        if ui_button(
            r,
            Ruleset::policy_tree_name(tree),
            if active { Color::new(0.26, 0.16, 0.42, 1.0) } else { BG_CARD },
            if active { ACCENT_GOLD } else { BORDER_DIM },
            ui.fs(12.0),
        ) {
            *tree_tab = tree;
        }
    }

    let list_top = tab_y + tab_h + ui.dp(12.0);
    let policies: Vec<_> = rules.policies.iter().filter(|p| p.tree == *tree_tab).collect();
    let avail_h = (rect.y + rect.h - list_top - ui.dp(12.0)).max(ui.dp(120.0));
    let card_h = ((avail_h - ui.dp(24.0)) / 4.0).clamp(ui.dp(46.0), ui.dp(82.0));

    for (idx, p) in policies.iter().enumerate() {
        let ry = list_top + (idx as f32) * (card_h + ui.dp(8.0));
        let owned = has_policy(cs.policy_bits, p.id);
        let unlockable = can_adopt_policy(rules, cs.policy_bits, p.id);
        let (bg, border) = if owned {
            (Color::new(0.14, 0.36, 0.22, 0.96), ACCENT_GREEN)
        } else if unlockable && can_buy {
            (Color::new(0.26, 0.18, 0.42, 0.98), ACCENT_GOLD)
        } else if unlockable {
            (BG_CARD, ACCENT_PURPLE)
        } else {
            (Color::new(0.08, 0.09, 0.13, 0.86), BORDER_DIM)
        };
        let status = if owned {
            "[ADOPTED]"
        } else if unlockable && can_buy {
            "[ADOPT NOW]"
        } else if unlockable {
            "[NEXT]"
        } else {
            "[LOCKED]"
        };
        let lbl = format!("Tier {}: {} — {}  {}", p.tier + 1, p.name, p.desc, status);
        if ui_button(Rect::new(rect.x + pad, ry, rect.w - pad * 2.0, card_h), &lbl, bg, border, ui.fs(13.5))
            && unlockable
            && can_buy
        {
            return Some(HudAction::AdoptPolicy(p.id));
        }
    }
    None
}

fn draw_diplo_modal(
    ui: &UiMetrics,
    world: &World,
    rules: &Ruleset,
    hciv: CivId,
    scroll: &mut TouchScroll,
) -> Option<HudAction> {
    let (rect, body_top, closed) = draw_modal_frame(ui, "DIPLOMACY & EMPIRES");
    if closed {
        return Some(HudAction::CloseModal);
    }
    let pad = ui.dp(12.0);
    let met: Vec<_> = world
        .civs
        .iter()
        .filter(|o| o.id != hciv && o.alive && world.diplo.has_met(hciv, o.id))
        .collect();

    if met.is_empty() {
        draw_text(
            "No other civilizations met yet. Explore with your Scout!",
            rect.x + pad,
            body_top + ui.dp(42.0),
            ui.fs(15.0),
            LIGHTGRAY,
        );
        return None;
    }

    let list_top = body_top + ui.dp(10.0);
    let list_h = (rect.y + rect.h - list_top - ui.dp(12.0)).max(ui.dp(100.0));
    let area = Rect::new(rect.x + pad, list_top, rect.w - pad * 2.0, list_h);
    let row_h = ui.dp(50.0);
    let max_scroll = ((met.len() as f32) * row_h - list_h).max(0.0);
    scroll.update(area, max_scroll, ui);

    for (idx, other) in met.iter().enumerate() {
        let ry = list_top + (idx as f32) * row_h - scroll.offset;
        if ry < list_top - ui.dp(4.0) || ry + row_h > list_top + list_h + ui.dp(4.0) {
            continue;
        }
        draw_rectangle(area.x, ry, area.w, row_h - ui.dp(4.0), BG_CARD);
        let sw_col = Color::from_rgba(other.color[0], other.color[1], other.color[2], 255);
        draw_rectangle(area.x + ui.dp(4.0), ry + ui.dp(6.0), ui.dp(10.0), row_h - ui.dp(16.0), sw_col);

        let st = world.diplo.status(hciv, other.id);
        let tpl = &rules.civs[other.template_idx as usize];
        draw_text(
            &format!("{}  (Score {})", other.name, other.score),
            area.x + ui.dp(20.0),
            ry + ui.dp(20.0),
            ui.fs(14.5),
            WHITE,
        );
        draw_text(
            &format!("{} — {}", tpl.trait_name, if st == DiploStatus::War { "AT WAR" } else { "PEACE" }),
            area.x + ui.dp(20.0),
            ry + ui.dp(38.0),
            ui.fs(12.0),
            if st == DiploStatus::War { ACCENT_RED } else { ACCENT_GREEN },
        );

        let bw = ui.dp(116.0);
        let btn_r = Rect::new(area.x + area.w - bw - ui.dp(6.0), ry + ui.dp(6.0), bw, row_h - ui.dp(16.0));
        if st == DiploStatus::War {
            if ui_button(btn_r, "MAKE PEACE", Color::new(0.14, 0.38, 0.24, 1.0), ACCENT_GREEN, ui.fs(12.5)) {
                return Some(HudAction::MakePeace(other.id));
            }
        } else if ui_button(btn_r, "DECLARE WAR", Color::new(0.46, 0.16, 0.16, 1.0), ACCENT_RED, ui.fs(12.5)) {
            return Some(HudAction::DeclareWar(other.id));
        }
    }
    None
}

fn draw_menu_modal(ui: &UiMetrics) -> Option<HudAction> {
    let (rect, body_top, closed) = draw_modal_frame(ui, "GAME MENU");
    if closed {
        return Some(HudAction::CloseModal);
    }
    let pad = ui.dp(16.0);
    let bw = rect.w - pad * 2.0;
    let bx = rect.x + pad;
    let mut y = body_top + ui.dp(12.0);
    let avail = (rect.y + rect.h - y - ui.dp(14.0)).max(ui.dp(180.0));
    let bh = ((avail - ui.dp(40.0)) / 5.5).clamp(ui.dp(38.0), ui.dp(52.0));
    let gap = ui.dp(8.0);
    let half = (bw - ui.dp(8.0)) * 0.5;

    // Row 1: OPTIONS | RULES & CODEX
    if ui_button(
        Rect::new(bx, y, half, bh),
        "OPTIONS & UI SCALE",
        Color::new(0.16, 0.30, 0.46, 1.0),
        ACCENT_GOLD,
        ui.fs(14.5),
    ) {
        return Some(HudAction::OpenModal(ActiveModal::Options));
    }
    if ui_button(
        Rect::new(bx + half + ui.dp(8.0), y, half, bh),
        "RULES & CODEX",
        Color::new(0.24, 0.16, 0.38, 1.0),
        ACCENT_PURPLE,
        ui.fs(14.5),
    ) {
        return Some(HudAction::OpenModal(ActiveModal::Codex));
    }
    y += bh + gap;

    // Row 2: SAVE | LOAD
    if ui_button(
        Rect::new(bx, y, half, bh),
        "SAVE GAME",
        Color::new(0.14, 0.38, 0.24, 1.0),
        ACCENT_GREEN,
        ui.fs(14.5),
    ) {
        return Some(HudAction::SaveGame);
    }
    if ui_button(
        Rect::new(bx + half + ui.dp(8.0), y, half, bh),
        "LOAD SAVED GAME",
        Color::new(0.16, 0.28, 0.46, 1.0),
        ACCENT_CYAN,
        ui.fs(14.5),
    ) {
        return Some(HudAction::LoadGame);
    }
    y += bh + gap;

    // Row 3: Fast-Forward AI stress benchmark buttons
    let third = (bw - ui.dp(12.0)) / 3.0;
    if ui_button(Rect::new(bx, y, third, bh), "SIM +10T", BG_CARD, ACCENT_GOLD, ui.fs(13.5)) {
        return Some(HudAction::FastForward(10));
    }
    if ui_button(Rect::new(bx + third + ui.dp(6.0), y, third, bh), "SIM +50T", BG_CARD, ACCENT_GOLD, ui.fs(13.5)) {
        return Some(HudAction::FastForward(50));
    }
    if ui_button(Rect::new(bx + (third + ui.dp(6.0)) * 2.0, y, third, bh), "SIM +100T", BG_CARD, ACCENT_GOLD, ui.fs(13.5)) {
        return Some(HudAction::FastForward(100));
    }
    y += bh + gap;

    // Row 4: Toggle Map Reveal
    if ui_button(Rect::new(bx, y, bw, bh), "TOGGLE REVEAL FULL MAP", BG_CARD, BORDER_DIM, ui.fs(14.5)) {
        return Some(HudAction::ToggleRevealMap);
    }
    y += bh + gap;

    // Row 5: Return to Title
    if ui_button(
        Rect::new(bx, y, bw, bh),
        "SAVE & RETURN TO TITLE",
        Color::new(0.46, 0.16, 0.16, 1.0),
        ACCENT_RED,
        ui.fs(15.0),
    ) {
        return Some(HudAction::ReturnToTitle);
    }
    None
}
