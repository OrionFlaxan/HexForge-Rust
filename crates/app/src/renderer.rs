use crate::camera::MapCamera;
use hexforge_sim::pathing::ReachableSet;
use hexforge_sim::ruleset::Ruleset;
use hexforge_sim::types::{
    CivId, Elevation, Feature, Improvement, Resource, Road, Terrain, TileId, UnitFormation,
    UnitId, UnitRole, INVALID_CITY, INVALID_TILE, INVALID_UNIT, NO_CIV,
};
use hexforge_sim::world::World;
use macroquad::prelude::*;

/// Precomputed unit vertices of a pointy-top hexagon (angles: 30, 90, 150, 210, 270, 330 deg).
/// Edge `d` connecting vertex `(d)` to `(d+1)%6` aligns cleanly with `HexMapSoA` neighbor directions
/// `0=E, 1=NE, 2=NW, 3=W, 4=SW, 5=SE`.
const HEX_VERTS: [(f32, f32); 6] = [
    (0.8660254, -0.5),  // v0: NE corner (between E and NE)
    (0.0, -1.0),        // v1: N corner
    (-0.8660254, -0.5), // v2: NW corner
    (-0.8660254, 0.5),  // v3: SW corner
    (0.0, 1.0),         // v4: S corner
    (0.8660254, 0.5),   // v5: SE corner
];

/// Vertex pair `(a, b)` for each `HexMapSoA` direction `0=E, 1=NE, 2=NW, 3=W, 4=SW, 5=SE`.
const DIR_EDGE_VERTS: [(usize, usize); 6] = [
    (5, 0), // 0: E
    (0, 1), // 1: NE
    (1, 2), // 2: NW
    (2, 3), // 3: W
    (3, 4), // 4: SW
    (4, 5), // 5: SE
];

#[inline]
fn civ_color(rgb: [u8; 3]) -> Color {
    Color::from_rgba(rgb[0], rgb[1], rgb[2], 255)
}

#[inline]
fn terrain_base_color(t: Terrain, elev: Elevation) -> Color {
    let mut c = match t {
        Terrain::Ocean => Color::from_rgba(20, 56, 102, 255),
        Terrain::Coast => Color::from_rgba(38, 102, 156, 255),
        Terrain::Grassland => Color::from_rgba(86, 148, 66, 255),
        Terrain::Plains => Color::from_rgba(162, 154, 82, 255),
        Terrain::Desert => Color::from_rgba(208, 182, 118, 255),
        Terrain::Tundra => Color::from_rgba(134, 148, 138, 255),
        Terrain::Snow => Color::from_rgba(220, 230, 238, 255),
    };
    if elev == Elevation::Hill {
        c.r *= 0.88;
        c.g *= 0.86;
        c.b *= 0.82;
    } else if elev == Elevation::Mountain {
        c = Color::from_rgba(104, 102, 110, 255);
    }
    c
}

#[inline]
fn dim_color(c: Color, factor: f32) -> Color {
    Color::new(c.r * factor, c.g * factor, c.b * factor, c.a)
}

/// Draw a filled pointy-top hex at screen `(cx, cy)` with radius `r`.
#[inline]
fn draw_filled_hex(cx: f32, cy: f32, r: f32, color: Color) {
    draw_poly(cx, cy, 6, r, 30.0, color);
}

/// Draw a stroked pointy-top hex outline.
#[inline]
fn draw_hex_outline(cx: f32, cy: f32, r: f32, thickness: f32, color: Color) {
    draw_poly_lines(cx, cy, 6, r, 30.0, thickness, color);
}

/// Render the entire visible viewport of the world (cylindrically wrapped, viewport-culled).
pub fn render_world(
    world: &World,
    rules: &Ruleset,
    cam: &MapCamera,
    viewer_civ: Option<CivId>,
    reveal_all: bool,
    selected_tile: Option<TileId>,
    selected_unit: Option<UnitId>,
    reachable: &ReachableSet,
    attackable: &[TileId],
) {
    let sw = screen_width();
    let sh = screen_height();
    let map = &world.map;
    let r = cam.zoom;
    let ww = map.world_width();

    // Compute visible row bounds in O(1)
    let (_, wy_top) = cam.screen_to_world(0.0, 0.0, sw, sh);
    let (_, wy_bot) = cam.screen_to_world(0.0, sh, sw, sh);
    let row_min = ((wy_top / 1.5).floor() as i32 - 1).clamp(0, map.height as i32 - 1) as u16;
    let row_max = ((wy_bot / 1.5).ceil() as i32 + 1).clamp(0, map.height as i32 - 1) as u16;

    // Horizontal wrap copies needed so cylindrical map fills the screen seamlessly
    let (wx_left, _) = cam.screen_to_world(0.0, 0.0, sw, sh);
    let (wx_right, _) = cam.screen_to_world(sw, 0.0, sw, sh);
    let wrap_min = (wx_left / ww).floor() as i32;
    let wrap_max = (wx_right / ww).floor() as i32;

    let fog_unexplored = Color::from_rgba(14, 18, 26, 255);
    let grid_line = Color::from_rgba(0, 0, 0, 42);

    // PASS 1: Terrain, Elevation, Features, Rivers, Resources, Improvements, Borders
    for wrap in wrap_min..=wrap_max {
        let x_offset = (wrap as f32) * ww;
        for row in row_min..=row_max {
            let y_world = 1.5 * (row as f32);
            let sy = (y_world - cam.center_y) * r + sh * 0.5;
            let row_shift = if (row & 1) == 1 { 0.5 } else { 0.0 };

            // Exact visible column slice for this wrap copy
            let min_c = (((wx_left - x_offset) / 1.7320508 - row_shift).floor() as i32 - 1)
                .clamp(0, map.width as i32 - 1) as u16;
            let max_c = (((wx_right - x_offset) / 1.7320508 - row_shift).ceil() as i32 + 1)
                .clamp(0, map.width as i32 - 1) as u16;

            for col in min_c..=max_c {
                let tid = map.tile_id(col, row);
                let ti = tid as usize;
                let wx = 1.7320508 * (col as f32 + row_shift) + x_offset;
                let sx = (wx - cam.center_x) * r + sw * 0.5;

                let (visible, explored) = match (viewer_civ, reveal_all) {
                    (Some(vc), false) => (
                        world.vision.is_visible(vc, tid),
                        world.vision.is_explored(vc, tid),
                    ),
                    _ => (true, true),
                };

                if !explored {
                    draw_filled_hex(sx, sy, r + 0.5, fog_unexplored);
                    continue;
                }

                let dim = if visible { 1.0 } else { 0.52 };
                let base = dim_color(terrain_base_color(map.terrain[ti], map.elevation[ti]), dim);
                draw_filled_hex(sx, sy, r + 0.5, base);

                // Subtle territory tint on owned land
                let owner = map.owner[ti];
                if owner != NO_CIV {
                    let mut oc = civ_color(world.civs[owner as usize].color);
                    oc.a = if visible { 0.22 } else { 0.14 };
                    draw_filled_hex(sx, sy, r * 0.96, oc);
                }

                if r >= 14.0 {
                    draw_hex_outline(sx, sy, r, 1.0, grid_line);
                }

                // Elevation graphics
                if map.elevation[ti] == Elevation::Hill && r >= 13.0 {
                    let hc = dim_color(Color::from_rgba(128, 114, 82, 200), dim);
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
                } else if map.elevation[ti] == Elevation::Mountain && r >= 11.0 {
                    let mc = dim_color(Color::from_rgba(78, 76, 86, 255), dim);
                    let sc = dim_color(Color::from_rgba(235, 240, 248, 255), dim);
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

                // Features (Forest, Jungle, Marsh)
                if r >= 15.0 {
                    match map.feature[ti] {
                        Feature::Forest => {
                            let tc = dim_color(Color::from_rgba(32, 86, 44, 240), dim);
                            for (ox, oy) in [(-0.28, 0.05), (0.24, 0.08), (0.0, -0.22)] {
                                let tx = sx + r * ox;
                                let ty = sy + r * oy;
                                draw_triangle(
                                    vec2(tx - r * 0.18, ty + r * 0.18),
                                    vec2(tx, ty - r * 0.20),
                                    vec2(tx + r * 0.18, ty + r * 0.18),
                                    tc,
                                );
                            }
                        }
                        Feature::Jungle => {
                            let jc = dim_color(Color::from_rgba(26, 118, 56, 235), dim);
                            for (ox, oy) in [(-0.24, 0.04), (0.24, 0.06), (0.0, -0.18)] {
                                draw_circle(sx + r * ox, sy + r * oy, r * 0.20, jc);
                            }
                        }
                        Feature::Marsh => {
                            let mc = dim_color(Color::from_rgba(52, 118, 108, 210), dim);
                            draw_line(sx - r * 0.32, sy, sx + r * 0.32, sy, 2.0, mc);
                            draw_line(sx - r * 0.22, sy + r * 0.16, sx + r * 0.22, sy + r * 0.16, 2.0, mc);
                        }
                        Feature::None => {}
                    }
                }

                // Rivers along hex edges
                let rmask = map.river_mask[ti];
                if rmask != 0 && r >= 12.0 {
                    let rc = dim_color(Color::from_rgba(58, 168, 242, 240), dim);
                    let thick = (r * 0.11).clamp(1.5, 4.5);
                    for d in 0..6 {
                        if (rmask & (1u8 << d)) != 0 {
                            let (va, vb) = DIR_EDGE_VERTS[d];
                            let (ax, ay) = HEX_VERTS[va];
                            let (bx, by) = HEX_VERTS[vb];
                            draw_line(sx + ax * r, sy + ay * r, sx + bx * r, sy + by * r, thick, rc);
                        }
                    }
                }

                // Roads & Improvements
                if r >= 18.0 {
                    if map.road[ti] != Road::None {
                        let rc = dim_color(Color::from_rgba(182, 152, 112, 220), dim);
                        draw_circle(sx, sy, r * 0.10, rc);
                    }
                    if map.improvement[ti] != Improvement::None && world.cities.tile_city[ti] == INVALID_CITY {
                        let ic = dim_color(Color::from_rgba(244, 218, 128, 225), dim);
                        draw_rectangle_lines(sx - r * 0.24, sy + r * 0.26, r * 0.48, r * 0.20, 1.5, ic);
                    }
                }

                // Resources pip
                if map.resource[ti] != Resource::None && r >= 18.0 {
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
                    draw_circle(sx + r * 0.36, sy - r * 0.30, (r * 0.15).max(3.0), dim_color(rcol, dim));
                    draw_circle_lines(
                        sx + r * 0.36,
                        sy - r * 0.30,
                        (r * 0.15).max(3.0),
                        1.0,
                        Color::from_rgba(15, 18, 24, 200),
                    );
                }

                // Crisp empire border strokes where owner differs from neighbor
                if owner != NO_CIV {
                    let bc = dim_color(civ_color(world.civs[owner as usize].color), dim);
                    let thick = (r * 0.11).clamp(1.8, 4.2);
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
            draw_filled_hex(sx, sy, r * 0.92, Color::from_rgba(56, 214, 245, 70));
            draw_hex_outline(sx, sy, r * 0.92, 2.2, Color::from_rgba(92, 232, 255, 220));
        }
        for &at in attackable {
            let (wx, wy) = map.tile_world_pos(at);
            let (sx, sy) = cam.world_to_screen(wx + x_offset, wy, sw, sh);
            if sx < -r || sx > sw + r || sy < -r || sy > sh + r {
                continue;
            }
            draw_filled_hex(sx, sy, r * 0.92, Color::from_rgba(248, 58, 58, 95));
            draw_hex_outline(sx, sy, r * 0.92, 3.0, Color::from_rgba(255, 92, 92, 245));
        }
        if let Some(st) = selected_tile {
            let (wx, wy) = map.tile_world_pos(st);
            let (sx, sy) = cam.world_to_screen(wx + x_offset, wy, sw, sh);
            if sx >= -r && sx <= sw + r && sy >= -r && sy <= sh + r {
                draw_hex_outline(sx, sy, r * 0.95, 3.2, Color::from_rgba(255, 224, 82, 255));
            }
        }
    }

    // PASS 3: Cities & Units (drawn cleanly on top of terrain)
    for wrap in wrap_min..=wrap_max {
        let x_offset = (wrap as f32) * ww;

        // Cities
        for ci in 0..world.cities.len() {
            if !world.cities.alive[ci] {
                continue;
            }
            let pos = world.cities.pos[ci];
            let explored = match (viewer_civ, reveal_all) {
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

            // Citadel icon
            let sz = (r * 0.44).max(5.0);
            draw_rectangle(sx - sz, sy - sz * 0.7, sz * 2.0, sz * 1.4, Color::from_rgba(32, 34, 42, 235));
            draw_rectangle_lines(sx - sz, sy - sz * 0.7, sz * 2.0, sz * 1.4, 2.0, ac);
            draw_circle(sx, sy, sz * 0.35, cc);

            if r >= 16.0 {
                // City nameplate banner
                let fs = (r * 0.42).clamp(11.0, 20.0);
                let label = if world.cities.is_capital[ci] {
                    format!("* {} [{}]", world.cities.name[ci], world.cities.pop[ci])
                } else {
                    format!("{} [{}]", world.cities.name[ci], world.cities.pop[ci])
                };
                let dims = measure_text(&label, None, fs as u16, 1.0);
                let bw = dims.width + 14.0;
                let bh = fs + 8.0;
                let bx = sx - bw * 0.5;
                let by = sy - r * 0.98 - bh * 0.5;
                draw_rectangle(bx, by, bw, bh, Color::from_rgba(16, 20, 28, 220));
                draw_rectangle(bx, by, 4.0, bh, cc);
                draw_rectangle_lines(bx, by, bw, bh, 1.2, cc);
                draw_text(&label, bx + 8.0, by + bh - 6.0, fs, WHITE);

                // City HP bar if damaged
                if world.cities.hp[ci] < world.cities.max_hp[ci] {
                    let frac = (world.cities.hp[ci].max(0) as f32) / (world.cities.max_hp[ci] as f32);
                    draw_rectangle(bx, by - 5.0, bw, 4.0, Color::from_rgba(30, 30, 36, 220));
                    draw_rectangle(bx, by - 5.0, bw * frac, 4.0, Color::from_rgba(235, 76, 62, 255));
                }
            }
        }

        // Units (combat + civilian)
        for u in 0..world.units.len() {
            if !world.units.alive[u] {
                continue;
            }
            let pos = world.units.pos[u];
            let uciv = world.units.civ[u];
            let visible = match (viewer_civ, reveal_all) {
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
                // Slight corner offset when stacked with a military escort
                sx += r * 0.28;
                sy += r * 0.22;
            }

            let cc = civ_color(world.civs[uciv as usize].color);
            let ac = civ_color(world.civs[uciv as usize].accent);
            let is_sel = selected_unit == Some(u as UnitId);
            let no_moves = world.units.moves_halves[u] == 0;
            let badge_r = (r * 0.40).clamp(5.0, 28.0);
            let fill = if no_moves { dim_color(cc, 0.65) } else { cc };

            // Outer badge shape: circle for combat, diamond/square for civilian
            if def.role.is_civilian() {
                draw_poly(sx, sy, 4, badge_r * 0.95, 45.0, fill);
                draw_poly_lines(
                    sx,
                    sy,
                    4,
                    badge_r * 0.95,
                    45.0,
                    if is_sel { 3.0 } else { 1.6 },
                    if is_sel { YELLOW } else { ac },
                );
            } else {
                draw_circle(sx, sy, badge_r, fill);
                draw_circle_lines(
                    sx,
                    sy,
                    badge_r,
                    if is_sel { 3.2 } else { 1.8 },
                    if is_sel { YELLOW } else { ac },
                );
            }

            // Crisp tactical role letter inside badge
            if r >= 15.0 {
                let glyph = match def.role {
                    UnitRole::Settler => "S",
                    UnitRole::Worker => "W",
                    UnitRole::Recon => "R",
                    UnitRole::Melee => "M",
                    UnitRole::Ranged => "A",
                    UnitRole::Cavalry => "C",
                    UnitRole::Siege => "G",
                    UnitRole::Naval => "N",
                };
                let gfs = (badge_r * 1.15).clamp(10.0, 22.0);
                let gdim = measure_text(glyph, None, gfs as u16, 1.0);
                draw_text(glyph, sx - gdim.width * 0.5, sy + gdim.height * 0.35, gfs, ac);
            }

            // Corps (**) & Army (***) gold chevrons above the unit badge!
            let form = world.units.formation[u];
            if form != UnitFormation::Single && r >= 13.0 {
                let stars = form.stars();
                let spacing = (badge_r * 0.48).max(4.5);
                let start_x = sx - spacing * ((stars as f32 - 1.0) * 0.5);
                let star_y = sy - badge_r - 4.0;
                for s in 0..stars {
                    draw_circle(
                        start_x + (s as f32) * spacing,
                        star_y,
                        (badge_r * 0.22).clamp(2.2, 4.5),
                        Color::from_rgba(255, 218, 64, 255),
                    );
                }
            }

            // Unit HP bar
            if world.units.hp[u] < 100 && r >= 14.0 {
                let bw = badge_r * 1.9;
                let bh = 3.5;
                let frac = (world.units.hp[u].max(0) as f32) / 100.0;
                let hcol = if frac > 0.6 {
                    Color::from_rgba(72, 224, 96, 255)
                } else if frac > 0.3 {
                    Color::from_rgba(244, 196, 48, 255)
                } else {
                    Color::from_rgba(236, 66, 56, 255)
                };
                draw_rectangle(sx - bw * 0.5, sy + badge_r + 2.0, bw, bh, Color::from_rgba(18, 20, 24, 230));
                draw_rectangle(sx - bw * 0.5, sy + badge_r + 2.0, bw * frac, bh, hcol);
            }
        }
    }
}

/// Draw a compact, clickable minimap in the bottom-right corner above the action bar.
/// Returns `Some(TileId)` if the player tapped/clicked inside the minimap.
pub fn render_and_interact_minimap(
    world: &World,
    cam: &MapCamera,
    viewer_civ: Option<CivId>,
    reveal_all: bool,
    rect: Rect,
) -> Option<TileId> {
    draw_rectangle(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0, Color::from_rgba(12, 16, 24, 225));
    draw_rectangle_lines(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0, 1.5, Color::from_rgba(186, 158, 92, 220));

    let map = &world.map;
    let cw = rect.w / (map.width as f32);
    let ch = rect.h / (map.height as f32);
    // Stride sampling on Huge maps keeps minimap rendering < 0.2 ms
    let step = if map.width > 70 { 2usize } else { 1usize };

    for r in (0..map.height as usize).step_by(step) {
        for c in (0..map.width as usize).step_by(step) {
            let tid = map.tile_id(c as u16, r as u16);
            let explored = match (viewer_civ, reveal_all) {
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

    // Camera viewport box indicator on minimap
    let vx = rect.x + (cam.center_x / map.world_width()) * rect.w;
    let vy = rect.y + (cam.center_y / map.world_height()).clamp(0.0, 1.0) * rect.h;
    let vw = ((screen_width() / cam.zoom) / map.world_width() * rect.w).clamp(8.0, rect.w);
    let vh = ((screen_height() / cam.zoom) / map.world_height() * rect.h).clamp(6.0, rect.h);
    draw_rectangle_lines(vx - vw * 0.5, vy - vh * 0.5, vw, vh, 1.5, YELLOW);

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
