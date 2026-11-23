use hexforge_sim::hex::HexMapSoA;
use hexforge_sim::types::TileId;
use macroquad::prelude::*;

/// Smooth cylindrical hex map camera supporting both Android multi-touch (drag + pinch)
/// and Desktop mouse/keyboard (drag, wheel zoom, WASD/arrows).
pub struct MapCamera {
    /// World-space center (hex radius = 1.0 in world units).
    pub center_x: f32,
    pub center_y: f32,
    /// Pixels per world unit (i.e., rendered hex outer radius in logical screen pixels).
    pub zoom: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,

    // Pointer / single-touch drag state
    dragging: bool,
    drag_start_screen: Vec2,
    last_screen: Vec2,
    drag_dist: f32,

    // Pinch-to-zoom state
    prev_pinch_dist: Option<f32>,
    prev_pinch_mid: Option<Vec2>,
}

pub struct InputEvent {
    pub tapped_screen: Option<Vec2>,
}

impl MapCamera {
    pub fn new(map: &HexMapSoA) -> Self {
        Self {
            center_x: map.world_width() * 0.5,
            center_y: map.world_height() * 0.5,
            zoom: 34.0,
            min_zoom: 10.0,
            max_zoom: 88.0,
            dragging: false,
            drag_start_screen: Vec2::ZERO,
            last_screen: Vec2::ZERO,
            drag_dist: 0.0,
            prev_pinch_dist: None,
            prev_pinch_mid: None,
        }
    }

    pub fn focus_tile(&mut self, map: &HexMapSoA, tile: TileId) {
        let (wx, wy) = map.tile_world_pos(tile);
        self.center_x = wx;
        self.center_y = wy;
    }

    #[inline]
    pub fn world_to_screen(&self, wx: f32, wy: f32, sw: f32, sh: f32) -> (f32, f32) {
        let sx = (wx - self.center_x) * self.zoom + sw * 0.5;
        let sy = (wy - self.center_y) * self.zoom + sh * 0.5;
        (sx, sy)
    }

    #[inline]
    pub fn screen_to_world(&self, sx: f32, sy: f32, sw: f32, sh: f32) -> (f32, f32) {
        let wx = (sx - sw * 0.5) / self.zoom + self.center_x;
        let wy = (sy - sh * 0.5) / self.zoom + self.center_y;
        (wx, wy)
    }

    pub fn screen_to_tile(&self, map: &HexMapSoA, sx: f32, sy: f32, sw: f32, sh: f32) -> Option<TileId> {
        let (wx, wy) = self.screen_to_world(sx, sy, sw, sh);
        map.world_pos_to_tile(wx, wy)
    }

    /// Update camera from touch + mouse + keyboard input.
    /// `ui_blocks_map` suppresses map pan/tap when a modal overlay or HUD bar consumed the pointer.
    pub fn update(&mut self, map: &HexMapSoA, ui_blocks_map: bool, dt: f32) -> InputEvent {
        let mut tapped_screen = None;
        let sw = screen_width();
        let sh = screen_height();
        let ww = map.world_width();
        let wh = map.world_height();

        if ui_blocks_map {
            self.dragging = false;
            self.prev_pinch_dist = None;
            self.prev_pinch_mid = None;
            return InputEvent { tapped_screen };
        }

        let active_touches = touches();
        if active_touches.len() >= 2 {
            // Multi-touch pinch-to-zoom + 2-finger pan
            self.dragging = false;
            let p0 = active_touches[0].position;
            let p1 = active_touches[1].position;
            let dist = p0.distance(p1).max(1.0);
            let mid = (p0 + p1) * 0.5;

            if let (Some(prev_d), Some(prev_m)) = (self.prev_pinch_dist, self.prev_pinch_mid) {
                let ratio = (dist / prev_d).clamp(0.8, 1.25);
                let (wx_before, wy_before) = self.screen_to_world(mid.x, mid.y, sw, sh);
                self.zoom = (self.zoom * ratio).clamp(self.min_zoom, self.max_zoom);
                let (wx_after, wy_after) = self.screen_to_world(mid.x, mid.y, sw, sh);
                self.center_x += wx_before - wx_after;
                self.center_y += wy_before - wy_after;

                let pan = mid - prev_m;
                self.center_x -= pan.x / self.zoom;
                self.center_y -= pan.y / self.zoom;
            }
            self.prev_pinch_dist = Some(dist);
            self.prev_pinch_mid = Some(mid);
        } else {
            self.prev_pinch_dist = None;
            self.prev_pinch_mid = None;

            // Unified single-finger / left-mouse drag & tap handling
            // macroquad maps primary touch to MouseButton::Left as well
            let mpos: Vec2 = mouse_position().into();
            if is_mouse_button_pressed(MouseButton::Left) {
                self.dragging = true;
                self.drag_start_screen = mpos;
                self.last_screen = mpos;
                self.drag_dist = 0.0;
            } else if is_mouse_button_down(MouseButton::Left) && self.dragging {
                let delta = mpos - self.last_screen;
                self.drag_dist += delta.length();
                if self.drag_dist > 8.0 {
                    self.center_x -= delta.x / self.zoom;
                    self.center_y -= delta.y / self.zoom;
                }
                self.last_screen = mpos;
            } else if is_mouse_button_released(MouseButton::Left) && self.dragging {
                self.dragging = false;
                if self.drag_dist <= 16.0 {
                    tapped_screen = Some(mpos);
                }
            }

            // Mouse wheel zoom (desktop)
            let (_, wy_wheel) = mouse_wheel();
            if wy_wheel.abs() > 0.01 {
                let factor = if wy_wheel > 0.0 { 1.14 } else { 1.0 / 1.14 };
                let (wx_before, wy_before) = self.screen_to_world(mpos.x, mpos.y, sw, sh);
                self.zoom = (self.zoom * factor).clamp(self.min_zoom, self.max_zoom);
                let (wx_after, wy_after) = self.screen_to_world(mpos.x, mpos.y, sw, sh);
                self.center_x += wx_before - wx_after;
                self.center_y += wy_before - wy_after;
            }
        }

        // Keyboard pan & zoom (desktop)
        let pan_speed = (420.0 / self.zoom) * dt;
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            self.center_y -= pan_speed;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            self.center_y += pan_speed;
        }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            self.center_x -= pan_speed;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            self.center_x += pan_speed;
        }
        if is_key_down(KeyCode::Equal) || is_key_down(KeyCode::KpAdd) {
            self.zoom = (self.zoom * (1.0 + 1.5 * dt)).clamp(self.min_zoom, self.max_zoom);
        }
        if is_key_down(KeyCode::Minus) || is_key_down(KeyCode::KpSubtract) {
            self.zoom = (self.zoom * (1.0 - 1.5 * dt)).clamp(self.min_zoom, self.max_zoom);
        }

        // Cylindrical X wrap & clamped Y
        self.center_x = self.center_x.rem_euclid(ww);
        self.center_y = self.center_y.clamp(-1.0, wh + 1.0);

        InputEvent { tapped_screen }
    }
}
