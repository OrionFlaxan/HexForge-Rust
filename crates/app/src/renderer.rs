use crate::camera::MapCamera;
use crate::settings::{draw_panel, AppOptions, UiMetrics, UiTheme};
use crate::sprites::{draw_city_sprite, draw_improvement_sprite, draw_unit_sprite};
use hexforge_sim::economy::tile_yields;
use hexforge_sim::pathing::ReachableSet;
use hexforge_sim::ruleset::Ruleset;
use hexforge_sim::types::{
    CivId, Elevation, Feature, Improvement, Resource, Road, Terrain, TileId, UnitFormation,
    UnitId, INVALID_CITY, INVALID_TILE, INVALID_UNIT, NO_CIV,
};
use hexforge_sim::world::World;
use macroquad::prelude::*;

const HEX_VERTS: [(f32, f32); 6] = [
    (0.8660254, -0.5),
    (0.0, -1.0),
    (-0.8660254, -0.5),
    (-0.8660254, 0.5),
    (0.0, 1.0),
    (0.8660254, 0.5),
];

const DIR_EDGE_VERTS: [(usize, usize); 6] = [
    (5, 0),
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 4),
    (4, 5),
];

#[inline]
fn civ_color(rgb: [u8; 3]) -> Color {
    Color::from_rgba(rgb[0], rgb[1], rgb[2], 255)
}

#[inline]
fn terrain_base_color(t: Terrain, elev: Elevation, theme: UiTheme) -> Color {
    let mut c = match (t, theme) {
        (Terrain::Ocean, UiTheme::PixelArt) => Color::from_rgba(22, 62, 118, 255),
        (Terrain::Coast, UiTheme::PixelArt) => Color::from_rgba(42, 114, 174, 255),
        (Terrain::Grassland, UiTheme::PixelArt) => Color::from_rgba(88, 158, 68, 255),
        (Terrain::Plains, UiTheme::PixelArt) => Color::from_rgba(170, 158, 82, 255),
        (Terrain::Desert, UiTheme::PixelArt) => Color::from_rgba(218, 188, 118, 255),
        (Terrain::Tundra, UiTheme::PixelArt) => Color::from_rgba(138, 152, 144, 255),
        (Terrain::Snow, UiTheme::PixelArt) => Color::from_rgba(226, 236, 244, 255),
        (Terrain::Ocean, UiTheme::ModernSimple) => Color::from_rgba(18, 50, 92, 255),
        (Terrain::Coast, UiTheme::ModernSimple) => Color::from_rgba(36, 94, 146, 255),
        (Terrain::Grassland, UiTheme::ModernSimple) => Color::from_rgba(82, 142, 66, 255),
        (Terrain::Plains, UiTheme::ModernSimple) => Color::from_rgba(156, 148, 84, 255),
        (Terrain::Desert, UiTheme::ModernSimple) => Color::from_rgba(202, 178, 118, 255),
        (Terrain::Tundra, UiTheme::ModernSimple) => Color::from_rgba(128, 142, 136, 255),
        (Terrain::Snow, UiTheme::ModernSimple) => Color::from_rgba(216, 226, 236, 255),
    };
    if elev == Elevation::Hill {
        c.r *= 0.88;
        c.g *= 0.86;
        c.b *= 0.82;
    } else if elev == Elevation::Mountain {
        c = Color::from_rgba(104, 102, 112, 255);
    }
    c
}

#[inline]
fn dim_color(c: Color, factor: f32) -> Color {
    Color::new(c.r * factor, c.g * factor, c.b * factor, c.a)
}

#[inline]
fn draw_filled_hex(cx: f32, cy: f32, r: f32, color: Color) {
    draw_poly(cx, cy, 6, r, 30.0, color);
}

#[inline]
fn draw_hex_outline(cx: f32, cy: f32, r: f32, thickness: f32, color: Color) {
    draw_poly_lines(cx, cy, 6, r, 30.0, thickness, color);
}

/// Render the viewport-culled cylindrical hex world honoring `UiTheme`, `AppOptions`, and `UiMetrics`.
#[allow(clippy::too_many_arguments)]
pub fn render_world(
    world: &World,
    rules: &Ruleset,
    cam: &MapCamera,
    ui: &UiMetrics,
    options: &AppOptions,
    viewer_civ: Option<CivId>,
    reveal_all: bool,
    selected_tile: Option<TileId>,
    selected_unit: Option<UnitId>,
    reachable: &ReachableSet,
    attackable: &[TileId],
) {
    let sw = ui.sw;
    let sh = ui.sh;
    let map = &world.map;
    let r = cam.zoom;
    let r_dp = r / ui.scale; // zoom expressed in logical dp
    let ww = map.world_width();
    let is_pixel = ui.theme == UiTheme::PixelArt;

    let (_, wy_top) = cam.screen_to_world(0.0, 0.0, sw, sh);
    let (_, wy_bot) = cam.screen_to_world(0.0, sh, sw, sh);
    let row_min = ((wy_top / 1.5).floor() as i32 - 1).clamp(0, map.height as i32 - 1) as u16;
    let row_max = ((wy_bot / 1.5).ceil() as i32 + 1).clamp(0, map.height as i32 - 1) as u16;

    let (wx_left, _) = cam.screen_to_world(0.0, 0.0, sw, sh);
    let (wx_right, _) = cam.screen_to_world(sw, 0.0, sw, sh);
    let wrap_min = (wx_left / ww).floor() as i32;
    let wrap_max = (wx_right / ww).floor() as i32;

    let fog_unexplored = if is_pixel {
        Color::from_rgba(14, 16, 28, 255)
    } else {
        Color::from_rgba(12, 16, 22, 255)
    };
    let grid_line = if is_pixel {
        Color::from_rgba(0, 0, 0, 58)
    } else {
        Color::from_rgba(255, 255, 255, 22)
    };
    let effective_reveal = reveal_all || !world.config.rules.fog_of_war;

    // PASS 1: Terrain, Pixel Dither, Elevation, Features, Rivers, Roads/Railroads, Improvements, Resources, Yields, Borders
    for wrap in wrap_min..=wrap_max {
        let x_offset = (wrap as f32) * ww;
        for row in row_min..=row_max {
            let y_world = 1.5 * (row as f32);
            let sy = (y_world - cam.center_y) * r + sh * 0.5;
            let row_shift = if (row & 1) == 1 { 0.5 } else { 0.0 };

            let min_c = (((wx_left - x_offset) / 1.7320508 - row_shift).floor() as i32 - 1)
                .clamp(0, map.width as i32 - 1) as u16;
            let max_c = (((wx_right - x_offset) / 1.7320508 - row_shift).ceil() as i32 + 1)
                .clamp(0, map.width as i32 - 1) as u16;

            for col in min_c..=max_c {
                let tid = map.tile_id(col, row);
                let ti = tid as usize;
                let wx = 1.7320508 * (col as f32 + row_shift) + x_offset;
                let sx = (wx - cam.center_x) * r + sw * 0.5;

                let (visible, explored) = match (viewer_civ, effective_reveal) {
                    (Some(vc), false) => (
                        world.vision.is_visible(vc, tid),
                        world.vision.is_explored(vc, tid),
                    ),
                    _ => (true, true),
                };

                if !explored {
                    draw_filled_hex(sx, sy, r + 0.6, fog_unexplored);
                    continue;
                }

                let dim = if visible { 1.0 } else { 0.52 };
                let base = dim_color(terrain_base_color(map.terrain[ti], map.elevation[ti], ui.theme), dim);
                draw_filled_hex(sx, sy, r + 0.6, base);

                // Retro 16-bit Pixel Art Terrain Dither & Ocean Wave Glints!
                if is_pixel && r_dp >= 16.0 {
                    let h = ((col as u32).wrapping_mul(374761393) ^ (row as u32).wrapping_mul(668265263)) & 7;
                    let ps = (r * 0.10).clamp(2.0, 5.5);
                    if map.terrain[ti].is_water() {
                        if h < 4 {
                            let wave = dim_color(Color::from_rgba(128, 198, 246, 105), dim);
                            let wxo = ((h as f32) - 1.5) * r * 0.14;
                            let wyo = (((h ^ 3) as f32) - 1.5) * r * 0.12;
                            draw_rectangle(sx + wxo - ps, sy + wyo, ps * 2.4, ps * 0.65, wave);
                            draw_rectangle(sx + wxo + ps * 0.6, sy + wyo + ps * 1.4, ps * 1.8, ps * 0.65, wave);
                        }
                    } else if map.elevation[ti] == Elevation::Flat && h < 5 {
                        let speck = if h % 2 == 0 {
                            Color::new(1.0, 1.0, 1.0, 0.08 * dim)
                        } else {
                            Color::new(0.0, 0.0, 0.0, 0.10 * dim)
                        };
                        let ox = ((h as f32) - 2.0) * r * 0.14;
                        let oy = (((h * 3) % 5) as f32 - 2.0) * r * 0.12;
                        draw_rectangle(sx + ox, sy + oy, ps, ps, speck);
                        draw_rectangle(sx - ox * 0.8, sy - oy * 0.7, ps, ps, speck);
                    }
                }

                let owner = map.owner[ti];
                if owner != NO_CIV {
                    let mut oc = civ_color(world.civs[owner as usize].color);
                    oc.a = if visible { 0.22 } else { 0.14 };
                    draw_filled_hex(sx, sy, r * 0.96, oc);
                }

                if options.show_hex_grid && r_dp >= 13.0 {
                    draw_hex_outline(sx, sy, r, ui.dp(0.9), grid_line);
                }

                // Elevation (Stepped Pixel Blocks in PixelArt vs Smooth Triangles in ModernSimple)
                if map.elevation[ti] == Elevation::Hill && r_dp >= 12.0 {
                    let hc = dim_color(Color::from_rgba(130, 114, 78, 220), dim);
                    let hc_hi = dim_color(Color::from_rgba(164, 148, 104, 210), dim);
                    if is_pixel {
                        draw_rectangle(sx - r * 0.42, sy + r * 0.04, r * 0.84, r * 0.18, hc);
                        draw_rectangle(sx - r * 0.30, sy - r * 0.10, r * 0.56, r * 0.15, hc);
                        draw_rectangle(sx - r * 0.18, sy - r * 0.20, r * 0.30, r * 0.11, hc_hi);
                    } else {
                        draw_triangle(
                            vec2(sx - r * 0.42, sy + r * 0.22),
                            vec2(sx - r * 0.08, sy - r * 0.18),
                            vec2(sx + r * 0.26, sy + r * 0.22),
                            hc,
                        );
                        draw_triangle(
                            vec2(sx - r * 0.12, sy + r * 0.28),
                            vec2(sx + r * 0.20, sy - r * 0.08),
                            vec2(sx + r * 0.48, sy + r * 0.28),
                            hc,
                        );
                    }
                } else if map.elevation[ti] == Elevation::Mountain && r_dp >= 10.0 {
                    let mc = dim_color(Color::from_rgba(78, 76, 88, 255), dim);
                    let mc_dark = dim_color(Color::from_rgba(54, 54, 66, 255), dim);
                    let sc = dim_color(Color::from_rgba(238, 244, 252, 255), dim);
                    if is_pixel {
                        draw_rectangle(sx - r * 0.52, sy + r * 0.12, r * 1.04, r * 0.24, mc_dark);
                        draw_rectangle(sx - r * 0.40, sy - r * 0.08, r * 0.80, r * 0.22, mc);
                        draw_rectangle(sx - r * 0.26, sy - r * 0.26, r * 0.52, r * 0.20, mc);
                        draw_rectangle(sx - r * 0.16, sy - r * 0.42, r * 0.32, r * 0.18, sc);
                        draw_rectangle(sx - r * 0.08, sy - r * 0.54, r * 0.16, r * 0.14, sc);
                    } else {
                        draw_triangle(
                            vec2(sx - r * 0.56, sy + r * 0.36),
                            vec2(sx, sy - r * 0.54),
                            vec2(sx + r * 0.56, sy + r * 0.36),
                            mc,
                        );
                        draw_triangle(
                            vec2(sx - r * 0.19, sy - r * 0.22),
                            vec2(sx, sy - r * 0.54),
                            vec2(sx + r * 0.19, sy - r * 0.22),
                            sc,
                        );
                    }
                }

                // Features (Pixel Pine Trees & Jungles in PixelArt vs Smooth Vector in ModernSimple)
                if r_dp >= 14.0 {
                    match map.feature[ti] {
                        Feature::Forest => {
                            let tc = dim_color(Color::from_rgba(32, 92, 44, 245), dim);
                            let tc_hi = dim_color(Color::from_rgba(52, 128, 62, 245), dim);
                            let trunk = dim_color(Color::from_rgba(112, 74, 42, 240), dim);
                            for (ox, oy) in [(-0.28, 0.04), (0.24, 0.08), (0.0, -0.22)] {
                                let tx = sx + r * ox;
                                let ty = sy + r * oy;
                                if is_pixel {
                                    draw_rectangle(tx - r * 0.05, ty + r * 0.10, r * 0.10, r * 0.12, trunk);
                                    draw_rectangle(tx - r * 0.18, ty + r * 0.00, r * 0.36, r * 0.12, tc);
                                    draw_rectangle(tx - r * 0.13, ty - r * 0.11, r * 0.26, r * 0.12, tc_hi);
                                    draw_rectangle(tx - r * 0.07, ty - r * 0.20, r * 0.14, r * 0.10, tc_hi);
                                } else {
                                    draw_triangle(
                                        vec2(tx - r * 0.18, ty + r * 0.18),
                                        vec2(tx, ty - r * 0.20),
                                        vec2(tx + r * 0.18, ty + r * 0.18),
                                        tc,
                                    );
                                }
                            }
                        }
                        Feature::Jungle => {
                            let jc = dim_color(Color::from_rgba(26, 122, 58, 240), dim);
                            let jc_hi = dim_color(Color::from_rgba(54, 162, 76, 240), dim);
                            for (ox, oy) in [(-0.24, 0.04), (0.24, 0.06), (0.0, -0.18)] {
                                let jx = sx + r * ox;
                                let jy = sy + r * oy;
                                if is_pixel {
                                    draw_rectangle(jx - r * 0.18, jy - r * 0.12, r * 0.36, r * 0.24, jc);
                                    draw_rectangle(jx - r * 0.12, jy - r * 0.18, r * 0.24, r * 0.12, jc_hi);
                                } else {
                                    draw_circle(jx, jy, r * 0.20, jc);
                                }
                            }
                        }
                        Feature::Marsh => {
                            let mc = dim_color(Color::from_rgba(52, 118, 108, 210), dim);
                            draw_line(sx - r * 0.32, sy, sx + r * 0.32, sy, ui.dp(1.8), mc);
                            draw_line(sx - r * 0.22, sy + r * 0.16, sx + r * 0.22, sy + r * 0.16, ui.dp(1.8), mc);
                        }
                        Feature::None => {}
                    }
                }

                // Rivers
                let rmask = map.river_mask[ti];
                if rmask != 0 && r_dp >= 11.0 {
                    let rc = dim_color(Color::from_rgba(58, 168, 242, 240), dim);
                    let thick = (r * 0.11).clamp(ui.dp(1.5), ui.dp(4.2));
                    for d in 0..6 {
                        if (rmask & (1u8 << d)) != 0 {
                            let (va, vb) = DIR_EDGE_VERTS[d];
                            let (ax, ay) = HEX_VERTS[va];
                            let (bx, by) = HEX_VERTS[vb];
                            draw_line(sx + ax * r, sy + ay * r, sx + bx * r, sy + by * r, thick, rc);
                        }
                    }
                }

                // Roads, Railroads & 16 Distinct Procedural Improvement Sprites
                if r_dp >= 15.0 {
                    match map.road[ti] {
                        Road::Road => {
                            let rc = dim_color(Color::from_rgba(196, 162, 114, 235), dim);
                            if is_pixel {
                                draw_rectangle(sx - r * 0.12, sy - r * 0.08, r * 0.24, r * 0.16, rc);
                            } else {
                                draw_circle(sx, sy, r * 0.11, rc);
                            }
                        }
                        Road::Railroad => {
                            let base_r = dim_color(Color::from_rgba(64, 68, 78, 245), dim);
                            let rail = dim_color(Color::from_rgba(238, 204, 88, 245), dim);
                            if is_pixel {
                                draw_rectangle(sx - r * 0.18, sy - r * 0.10, r * 0.36, r * 0.20, base_r);
                                draw_rectangle_lines(sx - r * 0.18, sy - r * 0.10, r * 0.36, r * 0.20, ui.dp(1.4), rail);
                            } else {
                                draw_circle(sx, sy, r * 0.14, base_r);
                                draw_circle_lines(sx, sy, r * 0.14, ui.dp(1.4), rail);
                            }
                        }
                        Road::None => {}
                    }
                    let imp = map.improvement[ti];
                    if imp != Improvement::None && world.cities.tile_city[ti] == INVALID_CITY {
                        let (ix, iy, imp_sz) = if imp.is_fortification() {
                            (sx, sy, r * 0.48)
                        } else {
                            (sx, sy + r * 0.30, r * 0.30)
                        };
                        draw_improvement_sprite(ui.theme, ix, iy, imp_sz, imp, dim);

                        // Pillaged indicator (bright red/orange flame X!)
                        if map.pillaged[ti] {
                            let pc = dim_color(Color::from_rgba(255, 72, 42, 248), dim);
                            draw_line(ix - r * 0.18, iy - r * 0.14, ix + r * 0.18, iy + r * 0.14, ui.dp(2.2), pc);
                            draw_line(ix + r * 0.18, iy - r * 0.14, ix - r * 0.18, iy + r * 0.14, ui.dp(2.2), pc);
                        }
                    }

                    // Active multi-turn construction progress pill on hex
                    if map.work_order[ti].is_some() && map.work_turns_left[ti] > 0 && r_dp >= 18.0 {
                        let lbl = format!("{}t", map.work_turns_left[ti]);
                        let fs = (r * 0.34).clamp(ui.fs(9.5), ui.fs(15.0));
                        let bx = sx - r * 0.44;
                        let by_p = sy - r * 0.48;
                        draw_rectangle(bx, by_p, ui.dp(22.0), ui.dp(14.0), Color::from_rgba(28, 22, 12, 225));
                        draw_rectangle_lines(bx, by_p, ui.dp(22.0), ui.dp(14.0), ui.dp(1.2), Color::from_rgba(250, 206, 62, 245));
                        draw_text(&lbl, bx + ui.dp(3.0), by_p + ui.dp(11.0), fs, Color::from_rgba(255, 224, 86, 255));
                    }
                }

                // Resource badge
                if options.show_resources && map.resource[ti] != Resource::None && r_dp >= 16.0 {
                    let rcol = match map.resource[ti] {
                        Resource::Wheat => Color::from_rgba(245, 206, 66, 255),
                        Resource::Cattle => Color::from_rgba(235, 224, 206, 255),
                        Resource::Iron => Color::from_rgba(168, 184, 204, 255),
                        Resource::Horses => Color::from_rgba(184, 112, 58, 255),
                        Resource::Gold => Color::from_rgba(255, 215, 36, 255),
                        Resource::Gems => Color::from_rgba(88, 232, 214, 255),
                        Resource::Oil => Color::from_rgba(32, 36, 44, 255),
                        Resource::Uranium => Color::from_rgba(116, 255, 84, 255),
                        Resource::None => BLANK,
                    };
                    let pr = (r * 0.15).max(ui.dp(3.0));
                    let rx = sx + r * 0.36;
                    let ry = sy - r * 0.30;
                    if is_pixel {
                        draw_rectangle(rx - pr, ry - pr, pr * 2.0, pr * 2.0, dim_color(rcol, dim));
                        draw_rectangle_lines(rx - pr, ry - pr, pr * 2.0, pr * 2.0, ui.dp(1.2), Color::from_rgba(15, 18, 24, 230));
                    } else {
                        draw_circle(rx, ry, pr, dim_color(rcol, dim));
                        draw_circle_lines(rx, ry, pr, ui.dp(1.0), Color::from_rgba(15, 18, 24, 200));
                    }
                }

                // Optional Tile Yield Dots (Food=Green, Prod=Orange, Gold=Yellow, Sci=Cyan)
                if options.show_tile_yields && r_dp >= 22.0 && map.elevation[ti] != Elevation::Mountain {
                    let owner_techs = if owner != NO_CIV {
                        world.civs[owner as usize].tech_bits
                    } else {
                        0
                    };
                    let y = tile_yields(map, rules, tid, owner_techs);
                    let pip_r = (r * 0.08).max(ui.dp(2.0));
                    let py = sy + r * 0.44;
                    let mut px = sx - r * 0.34;
                    let mut draw_pip = |col: Color| {
                        let c = dim_color(col, dim);
                        if is_pixel {
                            draw_rectangle(px - pip_r, py - pip_r, pip_r * 1.9, pip_r * 1.9, c);
                        } else {
                            draw_circle(px, py, pip_r, c);
                        }
                        px += pip_r * 2.1;
                    };
                    for _ in 0..y.food.clamp(0, 4) {
                        draw_pip(Color::from_rgba(92, 232, 92, 240));
                    }
                    for _ in 0..y.prod.clamp(0, 4) {
                        draw_pip(Color::from_rgba(242, 152, 52, 240));
                    }
                    for _ in 0..y.gold.clamp(0, 3) {
                        draw_pip(Color::from_rgba(255, 224, 58, 240));
                    }
                    for _ in 0..y.science.clamp(0, 3) {
                        draw_pip(Color::from_rgba(72, 220, 255, 240));
                    }
                }

                // Empire Territory Borders
                if owner != NO_CIV {
                    let bc = dim_color(civ_color(world.civs[owner as usize].color), dim);
                    let thick = (r * 0.11).clamp(ui.dp(1.8), ui.dp(4.2));
                    let nbs = map.neighbors_of(tid);
                    for d in 0..6 {
                        let nb = nbs[d];
                        let diff = nb == INVALID_TILE || map.owner[nb as usize] != owner;
                        if diff {
                            let (va, vb) = DIR_EDGE_VERTS[d];
                            let (ax, ay) = HEX_VERTS[va];
                            let (bx, by) = HEX_VERTS[vb];
                            draw_line(
                                sx + ax * r * 0.94,
                                sy + ay * r * 0.94,
                                sx + bx * r * 0.94,
                                sy + by * r * 0.94,
                                thick,
                                bc,
                            );
                        }
                    }
                }
            }
        }
    }

    // PASS 2: Movement / Attack / Selection Highlights
    for wrap in wrap_min..=wrap_max {
        let x_offset = (wrap as f32) * ww;
        for &(rt, _) in &reachable.tiles {
            let (wx, wy) = map.tile_world_pos(rt);
            let (sx, sy) = cam.world_to_screen(wx + x_offset, wy, sw, sh);
            if sx < -r || sx > sw + r || sy < -r || sy > sh + r {
                continue;
            }
            draw_filled_hex(sx, sy, r * 0.92, Color::from_rgba(56, 214, 245, 76));
            draw_hex_outline(sx, sy, r * 0.92, ui.dp(2.2), Color::from_rgba(92, 232, 255, 225));
        }
        for &at in attackable {
            let (wx, wy) = map.tile_world_pos(at);
            let (sx, sy) = cam.world_to_screen(wx + x_offset, wy, sw, sh);
            if sx < -r || sx > sw + r || sy < -r || sy > sh + r {
                continue;
            }
            draw_filled_hex(sx, sy, r * 0.92, Color::from_rgba(248, 58, 58, 98));
            draw_hex_outline(sx, sy, r * 0.92, ui.dp(3.0), Color::from_rgba(255, 92, 92, 245));
        }
        if let Some(st) = selected_tile {
            let (wx, wy) = map.tile_world_pos(st);
            let (sx, sy) = cam.world_to_screen(wx + x_offset, wy, sw, sh);
            if sx >= -r && sx <= sw + r && sy >= -r && sy <= sh + r {
                draw_hex_outline(sx, sy, r * 0.95, ui.dp(3.2), Color::from_rgba(255, 224, 82, 255));
            }
        }
    }

    // PASS 3: Procedural Cities & Units
    for wrap in wrap_min..=wrap_max {
        let x_offset = (wrap as f32) * ww;

        for ci in 0..world.cities.len() {
            if !world.cities.alive[ci] {
                continue;
            }
            let pos = world.cities.pos[ci];
            let explored = match (viewer_civ, effective_reveal) {
                (Some(vc), false) => world.vision.is_explored(vc, pos),
                _ => true,
            };
            if !explored {
                continue;
            }
            let (wx, wy) = map.tile_world_pos(pos);
            let (sx, sy) = cam.world_to_screen(wx + x_offset, wy, sw, sh);
            if sx < -r * 2.0 || sx > sw + r * 2.0 || sy < -r * 2.0 || sy > sh + r * 2.0 {
                continue;
            }
            let cid = world.cities.civ[ci];
            let cc = civ_color(world.civs[cid as usize].color);
            let ac = civ_color(world.civs[cid as usize].accent);
            let has_wonder = (world.cities.buildings[ci] & world.wonders_built) != 0;
            let civ_era = rules.era_for_techs(world.civs[cid as usize].tech_bits);

            let sz = (r * 0.46).max(ui.dp(6.0));
            draw_city_sprite(
                ui.theme,
                sx,
                sy,
                sz,
                civ_era,
                world.cities.is_capital[ci],
                has_wonder,
                cc,
                ac,
            );

            if r_dp >= 15.0 {
                let fs = (r * 0.42).clamp(ui.fs(11.0), ui.fs(20.0));
                let prefix = if has_wonder {
                    "W* "
                } else if world.cities.is_capital[ci] {
                    "* "
                } else {
                    ""
                };
                let label = format!("{}{} [{}]", prefix, world.cities.name[ci], world.cities.pop[ci]);
                let dims = measure_text(&label, None, fs as u16, 1.0);
                let bw = dims.width + ui.dp(16.0);
                let bh = fs + ui.dp(8.0);
                let bx = sx - bw * 0.5;
                let by = sy - r * 1.02 - bh * 0.5;
                draw_panel(ui, Rect::new(bx, by, bw, bh), Color::from_rgba(14, 18, 28, 232), cc);
                draw_rectangle(bx + ui.dp(2.0), by + ui.dp(2.0), ui.dp(4.0), bh - ui.dp(4.0), cc);
                if is_pixel {
                    draw_text(&label, bx + ui.dp(10.0), by + bh - ui.dp(4.0), fs, Color::new(0.0, 0.0, 0.0, 0.7));
                }
                draw_text(&label, bx + ui.dp(9.0), by + bh - ui.dp(5.0), fs, WHITE);

                if world.cities.hp[ci] < world.cities.max_hp[ci] {
                    let frac = (world.cities.hp[ci].max(0) as f32) / (world.cities.max_hp[ci] as f32);
                    let hp_h = ui.dp(4.0);
                    draw_rectangle(bx, by - hp_h - ui.dp(2.0), bw, hp_h, Color::from_rgba(30, 30, 36, 225));
                    draw_rectangle(bx, by - hp_h - ui.dp(2.0), bw * frac, hp_h, Color::from_rgba(235, 76, 62, 255));
                }
            }
        }

        for u in 0..world.units.len() {
            if !world.units.alive[u] {
                continue;
            }
            let pos = world.units.pos[u];
            let uciv = world.units.civ[u];
            let visible = match (viewer_civ, effective_reveal) {
                (Some(vc), false) => vc == uciv || world.vision.is_visible(vc, pos),
                _ => true,
            };
            if !visible {
                continue;
            }
            let (wx, wy) = map.tile_world_pos(pos);
            let (mut sx, mut sy) = cam.world_to_screen(wx + x_offset, wy, sw, sh);
            if sx < -r || sx > sw + r || sy < -r || sy > sh + r {
                continue;
            }
            let def = &rules.units[world.units.kind[u] as usize];
            if def.role.is_civilian() && world.units.tile_mil_unit[pos as usize] != INVALID_UNIT {
                sx += r * 0.28;
                sy += r * 0.22;
            }

            let cc = civ_color(world.civs[uciv as usize].color);
            let ac = civ_color(world.civs[uciv as usize].accent);
            let is_sel = selected_unit == Some(u as UnitId);
            let no_moves = world.units.moves_halves[u] == 0;
            let badge_r = (r * 0.42).clamp(ui.dp(6.0), ui.dp(28.0));
            let era_tier = def.tech.map_or(0, |t| rules.techs[t as usize].era);
            let is_unique = def.unique_to_civ.is_some();

            draw_unit_sprite(
                ui.theme,
                sx,
                sy,
                badge_r,
                def.role,
                era_tier,
                is_unique,
                cc,
                ac,
                is_sel,
                no_moves,
            );

            if world.units.auto_work[u] && r_dp >= 13.0 {
                let ar = (badge_r * 0.28).clamp(ui.dp(2.5), ui.dp(5.5));
                draw_rectangle(
                    sx + badge_r * 0.56,
                    sy - badge_r * 0.92,
                    ar * 1.9,
                    ar * 1.9,
                    Color::from_rgba(68, 232, 126, 255),
                );
            }

            let form = world.units.formation[u];
            if form != UnitFormation::Single && r_dp >= 12.0 {
                let stars = form.stars();
                let spacing = (badge_r * 0.48).max(ui.dp(4.5));
                let start_x = sx - spacing * ((stars as f32 - 1.0) * 0.5);
                let star_y = sy - badge_r - ui.dp(4.0);
                let sr = (badge_r * 0.22).clamp(ui.dp(2.2), ui.dp(4.5));
                for s in 0..stars {
                    let px = start_x + (s as f32) * spacing;
                    if is_pixel {
                        draw_rectangle(px - sr, star_y - sr, sr * 2.0, sr * 2.0, Color::from_rgba(255, 218, 64, 255));
                    } else {
                        draw_circle(px, star_y, sr, Color::from_rgba(255, 218, 64, 255));
                    }
                }
            }

            if world.units.hp[u] < 100 && r_dp >= 13.0 {
                let bw = badge_r * 1.9;
                let bh = ui.dp(3.5);
                let frac = (world.units.hp[u].max(0) as f32) / 100.0;
                let hcol = if frac > 0.6 {
                    Color::from_rgba(72, 224, 96, 255)
                } else if frac > 0.3 {
                    Color::from_rgba(244, 196, 48, 255)
                } else {
                    Color::from_rgba(236, 66, 56, 255)
                };
                draw_rectangle(sx - bw * 0.5, sy + badge_r + ui.dp(2.0), bw, bh, Color::from_rgba(18, 20, 24, 230));
                draw_rectangle(sx - bw * 0.5, sy + badge_r + ui.dp(2.0), bw * frac, bh, hcol);
            }
        }
    }
}

pub fn render_and_interact_minimap(
    world: &World,
    cam: &MapCamera,
    ui: &UiMetrics,
    viewer_civ: Option<CivId>,
    reveal_all: bool,
    rect: Rect,
) -> Option<TileId> {
    let pad = ui.dp(2.0);
    draw_rectangle(rect.x - pad, rect.y - pad, rect.w + pad * 2.0, rect.h + pad * 2.0, Color::from_rgba(12, 16, 24, 225));
    draw_rectangle_lines(rect.x - pad, rect.y - pad, rect.w + pad * 2.0, rect.h + pad * 2.0, ui.dp(1.5), Color::from_rgba(186, 158, 92, 220));

    let map = &world.map;
    let cw = rect.w / (map.width as f32);
    let ch = rect.h / (map.height as f32);
    let step = if map.width > 70 { 2usize } else { 1usize };
    let effective_reveal = reveal_all || !world.config.rules.fog_of_war;

    for r in (0..map.height as usize).step_by(step) {
        for c in (0..map.width as usize).step_by(step) {
            let tid = map.tile_id(c as u16, r as u16);
            let explored = match (viewer_civ, effective_reveal) {
                (Some(vc), false) => world.vision.is_explored(vc, tid),
                _ => true,
            };
            let col = if !explored {
                Color::from_rgba(14, 18, 26, 255)
            } else if map.owner[tid as usize] != NO_CIV {
                civ_color(world.civs[map.owner[tid as usize] as usize].color)
            } else if map.terrain[tid as usize].is_water() {
                Color::from_rgba(22, 54, 96, 255)
            } else {
                Color::from_rgba(88, 126, 74, 255)
            };
            draw_rectangle(
                rect.x + (c as f32) * cw,
                rect.y + (r as f32) * ch,
                cw * (step as f32) + 0.5,
                ch * (step as f32) + 0.5,
                col,
            );
        }
    }

    let vx = rect.x + (cam.center_x / map.world_width()) * rect.w;
    let vy = rect.y + (cam.center_y / map.world_height()).clamp(0.0, 1.0) * rect.h;
    let vw = ((ui.sw / cam.zoom) / map.world_width() * rect.w).clamp(ui.dp(8.0), rect.w);
    let vh = ((ui.sh / cam.zoom) / map.world_height() * rect.h).clamp(ui.dp(6.0), rect.h);
    draw_rectangle_lines(vx - vw * 0.5, vy - vh * 0.5, vw, vh, ui.dp(1.5), YELLOW);

    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        if rect.contains(vec2(mx, my)) {
            let col = (((mx - rect.x) / rect.w) * (map.width as f32)).clamp(0.0, (map.width - 1) as f32) as u16;
            let row = (((my - rect.y) / rect.h) * (map.height as f32)).clamp(0.0, (map.height - 1) as f32) as u16;
            return Some(map.tile_id(col, row));
        }
    }
    None
}
