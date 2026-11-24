use crate::storage::save_dir;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::path::PathBuf;

/// Live-switchable visual UI theme (persisted in `options.toml`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UiTheme {
    #[default]
    PixelArt,
    ModernSimple,
}

impl UiTheme {
    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            UiTheme::PixelArt => "Pixel Art",
            UiTheme::ModernSimple => "Modern Simple",
        }
    }
}

thread_local! {
    static ACTIVE_THEME: Cell<UiTheme> = const { Cell::new(UiTheme::PixelArt) };
    static ACTIVE_SCALE: Cell<f32> = const { Cell::new(1.0) };
}

#[inline]
fn default_true() -> bool {
    true
}

/// Persistent display, map, theme, and gameplay UI options saved to `options.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppOptions {
    /// User UI scale multiplier (`0.75..=1.50`), multiplied on top of device DPI density.
    pub ui_scale_mult: f32,
    pub show_tile_yields: bool,
    pub show_hex_grid: bool,
    pub show_resources: bool,
    pub show_perf_hud: bool,
    pub auto_next_unit: bool,
    /// Autosave every N turns (`0` = off, `5`, `10`, `25`).
    pub autosave_interval: u32,
    /// Visual style: Retro Pixel Art 4X or Clean Modern Simple UI.
    #[serde(default)]
    pub ui_theme: UiTheme,
    /// Block ending the turn until a Technology and all City Production targets are chosen.
    #[serde(default = "default_true")]
    pub require_turn_choices: bool,
    /// Optionally also prompt to adopt a Social Policy before ending the turn when affordable.
    #[serde(default)]
    pub require_policy_choice: bool,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            ui_scale_mult: 1.0,
            show_tile_yields: false,
            show_hex_grid: true,
            show_resources: true,
            show_perf_hud: true,
            auto_next_unit: false,
            autosave_interval: 10,
            ui_theme: UiTheme::PixelArt,
            require_turn_choices: true,
            require_policy_choice: false,
        }
    }
}

impl AppOptions {
    pub fn path() -> PathBuf {
        save_dir().join("options.toml")
    }

    pub fn load() -> Self {
        let p = Self::path();
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(mut opts) = toml::from_str::<AppOptions>(&text) {
                opts.ui_scale_mult = opts.ui_scale_mult.clamp(0.75, 1.50);
                return opts;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = std::fs::write(&p, text);
        }
    }

    pub fn cycle_autosave(&mut self) {
        self.autosave_interval = match self.autosave_interval {
            0 => 5,
            5 => 10,
            10 => 25,
            _ => 0,
        };
    }
}

/// Screen-density, theme & orientation metrics computed every frame so every button, text label,
/// and modal scales to comfortable thumb-sized proportions on Android AND crisp proportions on Desktop.
#[derive(Debug, Clone, Copy)]
pub struct UiMetrics {
    pub sw: f32,
    pub sh: f32,
    /// Pixels per logical dp (e.g. ~2.2-2.6 on 1080p Android phones, ~1.0 on standard Desktop).
    pub scale: f32,
    pub is_portrait: bool,
    pub top_bar_h: f32,
    pub bottom_bar_h: f32,
    pub theme: UiTheme,
}

impl UiMetrics {
    pub fn compute(options: &AppOptions) -> Self {
        let sw = screen_width().max(320.0);
        let sh = screen_height().max(320.0);
        let is_portrait = sh > sw;
        let short_side = sw.min(sh);

        // Read true Android DisplayMetrics.density exported by MainActivity.java
        let env_dpi = std::env::var("HEXFORGE_DPI_SCALE")
            .ok()
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|&v| v > 0.5 && v < 8.0);

        let base_scale = if let Some(density) = env_dpi {
            // Keep logical short edge around 390-460 dp on high-DPI phones so UI fills screen comfortably
            let ideal = short_side / 425.0;
            (density * 0.85).min(ideal * 1.15).max(ideal * 0.88).clamp(1.25, 3.4)
        } else if cfg!(target_os = "android") {
            (short_side / 420.0).clamp(1.3, 3.2)
        } else {
            // Desktop: 1.0 on 760p-1080p, gently scales up on 4K monitors
            (short_side / 800.0).clamp(0.95, 1.8)
        };

        let scale = (base_scale * options.ui_scale_mult).clamp(0.8, 3.8);

        // Cap bars so map viewport always retains >= 60% of screen height even at max UI scale
        let raw_top = if is_portrait { 76.0 * scale } else { 56.0 * scale };
        let raw_bot = if is_portrait { 108.0 * scale } else { 88.0 * scale };
        let top_bar_h = raw_top.min(sh * 0.16);
        let bottom_bar_h = raw_bot.min(sh * 0.22);

        ACTIVE_THEME.with(|t| t.set(options.ui_theme));
        ACTIVE_SCALE.with(|s| s.set(scale));

        Self {
            sw,
            sh,
            scale,
            is_portrait,
            top_bar_h,
            bottom_bar_h,
            theme: options.ui_theme,
        }
    }

    #[inline]
    pub fn dp(&self, val: f32) -> f32 {
        val * self.scale
    }

    #[inline]
    pub fn fs(&self, dp_size: f32) -> f32 {
        (dp_size * self.scale).clamp(12.0, 64.0)
    }

    #[inline]
    pub fn panel_bg(&self) -> Color {
        match self.theme {
            UiTheme::PixelArt => Color::new(0.07, 0.08, 0.15, 0.96),
            UiTheme::ModernSimple => Color::new(0.05, 0.07, 0.10, 0.95),
        }
    }

    #[inline]
    pub fn card_bg(&self) -> Color {
        match self.theme {
            UiTheme::PixelArt => Color::new(0.12, 0.14, 0.24, 0.98),
            UiTheme::ModernSimple => Color::new(0.10, 0.12, 0.17, 0.96),
        }
    }
}

/// Shared dark-slate & imperial gold color palette.
pub const BG_PANEL: Color = Color::new(0.06, 0.08, 0.13, 0.95);
pub const BG_CARD: Color = Color::new(0.11, 0.14, 0.21, 0.97);
pub const BORDER_GOLD: Color = Color::new(0.84, 0.68, 0.30, 0.98);
pub const BORDER_DIM: Color = Color::new(0.30, 0.36, 0.48, 0.88);
pub const ACCENT_GOLD: Color = Color::new(0.98, 0.84, 0.36, 1.0);
pub const ACCENT_CYAN: Color = Color::new(0.34, 0.84, 0.98, 1.0);
pub const ACCENT_GREEN: Color = Color::new(0.24, 0.80, 0.44, 1.0);
pub const ACCENT_RED: Color = Color::new(0.88, 0.30, 0.30, 1.0);
pub const ACCENT_PURPLE: Color = Color::new(0.76, 0.48, 0.96, 1.0);

/// Theme-aware framed panel:
/// - **PixelArt**: retro RPG double-beveled border, pixel drop shadow, inner highlight & corner studs.
/// - **ModernSimple**: crisp flat minimal card with clean hairline border and subtle top edge.
pub fn draw_panel(ui: &UiMetrics, rect: Rect, bg: Color, border: Color) {
    match ui.theme {
        UiTheme::PixelArt => {
            let px = ui.dp(2.0).round().max(2.0);
            // Crisp dark drop shadow
            draw_rectangle(rect.x + px, rect.y + px, rect.w, rect.h, Color::new(0.01, 0.02, 0.04, 0.68));
            // Panel body
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
            // Top-left pixel highlight bevel
            draw_rectangle(rect.x + px, rect.y + px, (rect.w - px * 2.0).max(0.0), px * 0.75, Color::new(1.0, 1.0, 1.0, 0.10));
            // Bottom inner shadow bevel
            draw_rectangle(rect.x + px, rect.y + rect.h - px * 1.75, (rect.w - px * 2.0).max(0.0), px * 0.75, Color::new(0.0, 0.0, 0.0, 0.36));
            // Outer crisp pixel border
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, px, border);
            // Corner pixel rivets
            let stud = (px * 1.35).min(rect.h * 0.18);
            draw_rectangle(rect.x, rect.y, stud, stud, border);
            draw_rectangle(rect.x + rect.w - stud, rect.y, stud, stud, border);
            draw_rectangle(rect.x, rect.y + rect.h - stud, stud, stud, border);
            draw_rectangle(rect.x + rect.w - stud, rect.y + rect.h - stud, stud, stud, border);
        }
        UiTheme::ModernSimple => {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
            let t = ui.dp(1.25).max(1.2);
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, t, border);
            draw_line(
                rect.x + t,
                rect.y + t,
                rect.x + rect.w - t,
                rect.y + t,
                1.0,
                Color::new(1.0, 1.0, 1.0, 0.07),
            );
        }
    }
}

/// Touch-friendly button that automatically renders in the active `UiTheme` (Pixel Art beveled RPG vs Modern Simple flat).
pub fn ui_button(rect: Rect, label: &str, bg: Color, border: Color, font_px: f32) -> bool {
    let theme = ACTIVE_THEME.with(|t| t.get());
    let scale = ACTIVE_SCALE.with(|s| s.get());
    let (mx, my) = mouse_position();
    let hovered = rect.contains(vec2(mx, my));
    let fill = if hovered {
        Color::new((bg.r + 0.09).min(1.0), (bg.g + 0.09).min(1.0), (bg.b + 0.09).min(1.0), bg.a)
    } else {
        bg
    };

    match theme {
        UiTheme::PixelArt => {
            let px = (2.0 * scale).round().clamp(1.6, 4.0);
            // Offset pixel shadow
            draw_rectangle(rect.x + px * 0.8, rect.y + px, rect.w, rect.h, Color::new(0.01, 0.02, 0.05, 0.65));
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
            // Top pixel highlight & bottom dark lip (classic 16-bit RPG button feel!)
            draw_rectangle(rect.x + px, rect.y + px * 0.7, (rect.w - px * 2.0).max(0.0), px * 0.85, Color::new(1.0, 1.0, 1.0, 0.16));
            draw_rectangle(rect.x + px, rect.y + rect.h - px * 1.55, (rect.w - px * 2.0).max(0.0), px * 0.9, Color::new(0.0, 0.0, 0.0, 0.34));
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, px, border);
            // Crisp pixel corners
            let cs = px * 1.15;
            draw_rectangle(rect.x, rect.y, cs, cs, border);
            draw_rectangle(rect.x + rect.w - cs, rect.y, cs, cs, border);
            draw_rectangle(rect.x, rect.y + rect.h - cs, cs, cs, border);
            draw_rectangle(rect.x + rect.w - cs, rect.y + rect.h - cs, cs, cs, border);
        }
        UiTheme::ModernSimple => {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
            let t = (rect.h * 0.036).clamp(1.2, 2.2);
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, t, border);
            // Subtle modern top accent bar
            draw_line(
                rect.x + t,
                rect.y + t,
                rect.x + rect.w - t,
                rect.y + t,
                1.0,
                Color::new(1.0, 1.0, 1.0, 0.10),
            );
        }
    }

    // Shrink font slightly only if label exceeds button width
    let mut fs = font_px.max(12.0);
    let mut dims = measure_text(label, None, fs as u16, 1.0);
    let max_w = (rect.w - 10.0).max(20.0);
    if dims.width > max_w && dims.width > 1.0 {
        fs = (fs * (max_w / dims.width)).max(11.0);
        dims = measure_text(label, None, fs as u16, 1.0);
    }
    let tx = (rect.x + (rect.w - dims.width) * 0.5).max(rect.x + 4.0);
    let ty = rect.y + (rect.h + dims.height * 0.72) * 0.5;
    if theme == UiTheme::PixelArt {
        draw_text(label, tx + 1.2, ty + 1.2, fs, Color::new(0.0, 0.0, 0.0, 0.65));
    }
    draw_text(label, tx, ty, fs, WHITE);

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

/// Draw a responsive modal frame covering 94% of screen on phones and up to 760dp on desktop.
pub fn draw_modal_frame(ui: &UiMetrics, title: &str) -> (Rect, f32, bool) {
    draw_rectangle(0.0, 0.0, ui.sw, ui.sh, Color::new(0.0, 0.0, 0.0, 0.66));
    let w = (ui.sw * 0.94).min(ui.dp(760.0));
    let h = (ui.sh * 0.90).min(ui.dp(620.0));
    let rect = Rect::new((ui.sw - w) * 0.5, (ui.sh - h) * 0.5, w, h);
    draw_panel(ui, rect, ui.panel_bg(), BORDER_GOLD);

    let header_h = ui.dp(48.0).min(rect.h * 0.14);
    let title_fs = ui.fs(18.5);
    if ui.theme == UiTheme::PixelArt {
        draw_rectangle(
            rect.x + ui.dp(2.0),
            rect.y + ui.dp(2.0),
            rect.w - ui.dp(4.0),
            header_h - ui.dp(2.0),
            Color::new(0.11, 0.13, 0.24, 0.95),
        );
        draw_text(
            title,
            rect.x + ui.dp(15.2),
            rect.y + header_h * 0.68 + 1.2,
            title_fs,
            Color::new(0.0, 0.0, 0.0, 0.70),
        );
    }
    draw_text(title, rect.x + ui.dp(14.0), rect.y + header_h * 0.68, title_fs, ACCENT_GOLD);

    let close_w = ui.dp(82.0);
    let close_h = header_h - ui.dp(10.0);
    let close_r = Rect::new(
        rect.x + rect.w - close_w - ui.dp(8.0),
        rect.y + ui.dp(5.0),
        close_w,
        close_h,
    );
    let closed = ui_button(
        close_r,
        "CLOSE",
        Color::new(0.45, 0.16, 0.16, 1.0),
        ACCENT_RED,
        ui.fs(14.0),
    );
    draw_line(
        rect.x,
        rect.y + header_h,
        rect.x + rect.w,
        rect.y + header_h,
        ui.dp(if ui.theme == UiTheme::PixelArt { 2.0 } else { 1.2 }),
        if ui.theme == UiTheme::PixelArt { BORDER_GOLD } else { BORDER_DIM },
    );
    (rect, rect.y + header_h, closed)
}

/// Simple drag-and-wheel vertical scroll helper for touch lists inside modals.
#[derive(Debug, Default, Clone, Copy)]
pub struct TouchScroll {
    pub offset: f32,
    dragging: bool,
    last_y: f32,
}

impl TouchScroll {
    pub fn reset(&mut self) {
        self.offset = 0.0;
        self.dragging = false;
    }

    pub fn update(&mut self, area: Rect, max_offset: f32, ui: &UiMetrics) {
        let (mx, my) = mouse_position();
        let inside = area.contains(vec2(mx, my));
        let (_, wy) = mouse_wheel();
        if inside && wy.abs() > 0.05 {
            self.offset -= wy * ui.dp(42.0);
        }
        if inside && is_mouse_button_pressed(MouseButton::Left) {
            self.dragging = true;
            self.last_y = my;
        } else if self.dragging && is_mouse_button_down(MouseButton::Left) {
            let dy = my - self.last_y;
            if dy.abs() > 1.5 {
                self.offset -= dy;
                self.last_y = my;
            }
        } else if is_mouse_button_released(MouseButton::Left) {
            self.dragging = false;
        }
        self.offset = self.offset.clamp(0.0, max_offset.max(0.0));
    }
}
