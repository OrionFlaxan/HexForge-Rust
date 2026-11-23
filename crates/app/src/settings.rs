use crate::storage::save_dir;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persistent display, map, and gameplay UI options saved to `options.toml`.
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

/// Screen-density & orientation metrics computed every frame so every button, text label,
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

        // Cap bars so map viewport always retains >= 58% of screen height even at max UI scale
        let raw_top = if is_portrait { 76.0 * scale } else { 56.0 * scale };
        let raw_bot = if is_portrait { 126.0 * scale } else { 88.0 * scale };
        let top_bar_h = raw_top.min(sh * 0.16);
        let bottom_bar_h = raw_bot.min(sh * 0.25);

        Self {
            sw,
            sh,
            scale,
            is_portrait,
            top_bar_h,
            bottom_bar_h,
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
}

/// Shared dark-slate & imperial gold color palette.
pub const BG_PANEL: Color = Color::new(0.06, 0.08, 0.13, 0.95);
pub const BG_CARD: Color = Color::new(0.11, 0.14, 0.21, 0.97);
pub const BORDER_GOLD: Color = Color::new(0.80, 0.65, 0.30, 0.96);
pub const BORDER_DIM: Color = Color::new(0.30, 0.36, 0.48, 0.88);
pub const ACCENT_GOLD: Color = Color::new(0.98, 0.84, 0.36, 1.0);
pub const ACCENT_CYAN: Color = Color::new(0.34, 0.84, 0.98, 1.0);
pub const ACCENT_GREEN: Color = Color::new(0.24, 0.80, 0.44, 1.0);
pub const ACCENT_RED: Color = Color::new(0.88, 0.30, 0.30, 1.0);
pub const ACCENT_PURPLE: Color = Color::new(0.76, 0.48, 0.96, 1.0);

/// Touch-friendly button with crisp border, hover highlight, and automatic text fitting.
pub fn ui_button(rect: Rect, label: &str, bg: Color, border: Color, font_px: f32) -> bool {
    let (mx, my) = mouse_position();
    let hovered = rect.contains(vec2(mx, my));
    let fill = if hovered {
        Color::new((bg.r + 0.08).min(1.0), (bg.g + 0.08).min(1.0), (bg.b + 0.08).min(1.0), bg.a)
    } else {
        bg
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, (rect.h * 0.045).clamp(1.6, 3.2), border);

    // Shrink font slightly only if label exceeds button width
    let mut fs = font_px.max(12.0);
    let mut dims = measure_text(label, None, fs as u16, 1.0);
    let max_w = (rect.w - 10.0).max(20.0);
    if dims.width > max_w && dims.width > 1.0 {
        fs = (fs * (max_w / dims.width)).max(11.0);
        dims = measure_text(label, None, fs as u16, 1.0);
    }
    let tx = rect.x + (rect.w - dims.width) * 0.5;
    let ty = rect.y + (rect.h + dims.height * 0.72) * 0.5;
    draw_text(label, tx.max(rect.x + 4.0), ty, fs, WHITE);

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

/// Draw a responsive modal frame covering 94% of screen on phones and up to 780dp on desktop.
pub fn draw_modal_frame(ui: &UiMetrics, title: &str) -> (Rect, f32, bool) {
    draw_rectangle(0.0, 0.0, ui.sw, ui.sh, Color::new(0.0, 0.0, 0.0, 0.64));
    let w = (ui.sw * 0.94).min(ui.dp(760.0));
    let h = (ui.sh * 0.90).min(ui.dp(620.0));
    let rect = Rect::new((ui.sw - w) * 0.5, (ui.sh - h) * 0.5, w, h);
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, BG_PANEL);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, ui.dp(2.2), BORDER_GOLD);

    let header_h = ui.dp(48.0).min(rect.h * 0.14);
    let title_fs = ui.fs(19.0);
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
        ui.dp(1.4),
        BORDER_DIM,
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
