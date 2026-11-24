use crate::settings::UiTheme;
use hexforge_sim::ruleset::BuildingDef;
use hexforge_sim::types::{CityProcess, Improvement, UnitRole, WorkerOrder};
use macroquad::prelude::*;

#[inline]
fn dim(c: Color, factor: f32) -> Color {
    Color::new(c.r * factor, c.g * factor, c.b * factor, c.a)
}

/// Draw a snapped rectangular pixel block on an `N x N` virtual grid centered at `(cx, cy)`.
#[inline]
fn px_rect(cx: f32, cy: f32, unit: f32, gx: f32, gy: f32, gw: f32, gh: f32, col: Color) {
    draw_rectangle(
        cx + gx * unit,
        cy + gy * unit,
        (gw * unit).max(1.0),
        (gh * unit).max(1.0),
        col,
    );
}

/// Draw a procedural unit sprite on the hex map OR as a miniature card icon!
/// - `UiTheme::PixelArt`: 12×12 crisp retro pixel-art units inside a beveled civ shield (+ gold crown for Unique Units!).
/// - `UiTheme::ModernSimple`: clean minimal geometric NATO/vector glyph inside a flat circular/diamond emblem.
#[allow(clippy::too_many_arguments)]
pub fn draw_unit_sprite(
    theme: UiTheme,
    cx: f32,
    cy: f32,
    radius: f32,
    role: UnitRole,
    era_tier: u8,
    is_unique: bool,
    civ_col: Color,
    accent: Color,
    is_selected: bool,
    no_moves: bool,
) {
    let fill = if no_moves { dim(civ_col, 0.62) } else { civ_col };
    let rim = if is_selected {
        Color::from_rgba(255, 228, 72, 255)
    } else if is_unique {
        Color::from_rgba(255, 206, 68, 255)
    } else {
        accent
    };
    let fg = if no_moves { dim(accent, 0.78) } else { accent };
    let metal = Color::from_rgba(224, 232, 242, 255);
    let wood = Color::from_rgba(182, 128, 68, 255);
    let dark = Color::from_rgba(16, 20, 28, 235);

    match theme {
        UiTheme::PixelArt => {
            let u = (radius * 2.0) / 12.0;
            // Pixel drop shadow
            px_rect(cx, cy, u, -4.5, -4.5, 10.0, 10.0, Color::new(0.0, 0.0, 0.0, 0.45));

            // Stepped octagonal pixel shield (square with cut corners)
            if role.is_civilian() {
                // Diamond-cut pixel tile for Settler / Worker
                px_rect(cx, cy, u, -5.0, -3.0, 10.0, 6.0, dark);
                px_rect(cx, cy, u, -3.0, -5.0, 6.0, 10.0, dark);
                px_rect(cx, cy, u, -4.0, -4.0, 8.0, 8.0, rim);
                px_rect(cx, cy, u, -3.2, -3.2, 6.4, 6.4, fill);
            } else {
                // Chunky RPG octagonal unit badge
                px_rect(cx, cy, u, -5.0, -6.0, 10.0, 12.0, dark);
                px_rect(cx, cy, u, -6.0, -5.0, 12.0, 10.0, dark);
                px_rect(cx, cy, u, -4.2, -5.2, 8.4, 10.4, rim);
                px_rect(cx, cy, u, -5.2, -4.2, 10.4, 8.4, rim);
                px_rect(cx, cy, u, -3.5, -4.4, 7.0, 8.8, fill);
                px_rect(cx, cy, u, -4.4, -3.5, 8.8, 7.0, fill);
                // Top pixel highlight sheen
                px_rect(cx, cy, u, -3.5, -4.4, 7.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.22));
            }

            // Crisp procedural 12x12 pixel art icon by Role & Era!
            match role {
                UnitRole::Settler => {
                    // Covered wagon + fluttering banner
                    px_rect(cx, cy, u, -3.0, -2.5, 6.0, 3.2, WHITE);
                    px_rect(cx, cy, u, -3.5, 0.5, 7.0, 1.5, wood);
                    px_rect(cx, cy, u, -2.6, 2.0, 1.8, 1.8, dark);
                    px_rect(cx, cy, u, 1.0, 2.0, 1.8, 1.8, dark);
                    px_rect(cx, cy, u, -0.5, -4.2, 1.0, 2.0, wood);
                    px_rect(cx, cy, u, 0.5, -4.2, 2.2, 1.3, Color::from_rgba(255, 214, 64, 255));
                }
                UnitRole::Worker => {
                    // Pixel pickaxe / hammer
                    px_rect(cx, cy, u, -3.2, -3.0, 6.4, 1.5, metal);
                    px_rect(cx, cy, u, -3.6, -2.0, 1.4, 1.4, metal);
                    px_rect(cx, cy, u, 2.2, -2.0, 1.4, 1.4, metal);
                    px_rect(cx, cy, u, -0.8, -1.6, 1.6, 5.2, wood);
                }
                UnitRole::Recon => {
                    // Scout compass / hooded cloak & spyglass
                    px_rect(cx, cy, u, -2.0, -3.6, 4.0, 2.4, fg);
                    px_rect(cx, cy, u, -3.2, -1.2, 6.4, 3.2, dark);
                    px_rect(cx, cy, u, -1.2, -0.8, 2.4, 2.4, Color::from_rgba(110, 232, 255, 255));
                    px_rect(cx, cy, u, -2.2, 2.0, 1.6, 1.8, wood);
                    px_rect(cx, cy, u, 0.6, 2.0, 1.6, 1.8, wood);
                }
                UnitRole::Melee => {
                    if era_tier >= 7 {
                        // Future Mech / Giant Death Robot visor!
                        px_rect(cx, cy, u, -3.2, -3.2, 6.4, 4.4, metal);
                        px_rect(cx, cy, u, -2.4, -1.8, 4.8, 1.5, Color::from_rgba(255, 68, 68, 255));
                        px_rect(cx, cy, u, -2.6, 1.2, 2.0, 2.6, dark);
                        px_rect(cx, cy, u, 0.6, 1.2, 2.0, 2.6, dark);
                    } else if era_tier >= 4 {
                        // Modern Infantry rifle + steel helmet
                        px_rect(cx, cy, u, -2.4, -3.6, 4.8, 2.0, metal);
                        px_rect(cx, cy, u, -1.8, -1.6, 3.6, 3.0, fg);
                        px_rect(cx, cy, u, 1.4, -3.2, 1.2, 6.4, dark);
                        px_rect(cx, cy, u, -2.8, 0.2, 5.4, 1.2, wood);
                    } else {
                        // Classic pixel sword & kite shield
                        px_rect(cx, cy, u, -3.2, -2.2, 3.4, 4.2, fg);
                        px_rect(cx, cy, u, -2.4, 2.0, 1.8, 1.2, fg);
                        px_rect(cx, cy, u, 1.0, -3.6, 1.4, 5.2, metal);
                        px_rect(cx, cy, u, 0.0, 1.4, 3.4, 1.0, Color::from_rgba(248, 208, 68, 255));
                        px_rect(cx, cy, u, 1.1, 2.4, 1.2, 1.4, wood);
                    }
                }
                UnitRole::Ranged => {
                    if era_tier >= 5 {
                        // Modern missile / machine gun tripod
                        px_rect(cx, cy, u, -3.4, -1.6, 6.6, 1.8, dark);
                        px_rect(cx, cy, u, 1.6, -2.2, 1.8, 3.0, Color::from_rgba(255, 196, 56, 255));
                        px_rect(cx, cy, u, -2.4, 0.2, 1.2, 3.2, metal);
                        px_rect(cx, cy, u, 0.8, 0.2, 1.2, 3.2, metal);
                    } else {
                        // Pixel recurve bow & arrow
                        px_rect(cx, cy, u, -2.4, -3.6, 1.4, 7.2, wood);
                        px_rect(cx, cy, u, -1.2, -3.6, 1.4, 1.4, wood);
                        px_rect(cx, cy, u, -1.2, 2.2, 1.4, 1.4, wood);
                        px_rect(cx, cy, u, -3.2, -0.6, 6.0, 1.2, metal);
                        px_rect(cx, cy, u, 2.0, -1.6, 1.6, 3.2, Color::from_rgba(255, 214, 72, 255));
                    }
                }
                UnitRole::Cavalry => {
                    if era_tier >= 5 {
                        // Armored pixel Tank (treads + hull + barrel!)
                        px_rect(cx, cy, u, -2.2, -2.6, 4.0, 2.2, metal);
                        px_rect(cx, cy, u, 1.2, -2.2, 2.8, 1.2, metal);
                        px_rect(cx, cy, u, -3.6, -0.4, 7.2, 2.2, fg);
                        px_rect(cx, cy, u, -3.6, 1.8, 7.2, 1.6, dark);
                    } else {
                        // Horse & armored rider silhouette
                        px_rect(cx, cy, u, -1.0, -3.6, 2.2, 2.2, fg);
                        px_rect(cx, cy, u, -3.2, -1.2, 5.2, 2.4, wood);
                        px_rect(cx, cy, u, 1.2, -2.6, 2.2, 2.4, wood);
                        px_rect(cx, cy, u, -3.0, 1.2, 1.4, 2.4, dark);
                        px_rect(cx, cy, u, 1.2, 1.2, 1.4, 2.4, dark);
                    }
                }
                UnitRole::Siege => {
                    // Pixel cannon / artillery barrel + spoked wheels
                    px_rect(cx, cy, u, -2.8, -2.6, 5.6, 2.2, dark);
                    px_rect(cx, cy, u, 1.8, -3.2, 2.0, 1.6, Color::from_rgba(255, 168, 52, 255));
                    px_rect(cx, cy, u, -3.2, -0.2, 6.4, 1.4, wood);
                    px_rect(cx, cy, u, -2.6, 1.0, 2.2, 2.2, wood);
                    px_rect(cx, cy, u, 0.8, 1.0, 2.2, 2.2, wood);
                }
                UnitRole::Naval => {
                    // Pixel ship hull + billowing sail
                    px_rect(cx, cy, u, -0.6, -3.8, 1.2, 5.2, wood);
                    px_rect(cx, cy, u, -2.4, -3.2, 4.8, 3.2, WHITE);
                    px_rect(cx, cy, u, -3.6, 1.0, 7.2, 1.8, wood);
                    px_rect(cx, cy, u, -2.8, 2.6, 5.6, 1.2, Color::from_rgba(92, 198, 255, 255));
                }
                UnitRole::Air => {
                    // Pixel swept-wing fighter aircraft
                    px_rect(cx, cy, u, -0.9, -4.0, 1.8, 8.0, metal);
                    px_rect(cx, cy, u, -4.0, -0.8, 8.0, 1.8, metal);
                    px_rect(cx, cy, u, -2.2, 2.2, 4.4, 1.2, fg);
                    px_rect(cx, cy, u, -0.7, -2.2, 1.4, 1.4, Color::from_rgba(88, 224, 255, 255));
                }
            }

            // Gold star pip on Unique Units!
            if is_unique {
                px_rect(cx, cy, u, -5.4, -5.6, 2.6, 2.6, dark);
                px_rect(cx, cy, u, -5.0, -5.2, 1.8, 1.8, Color::from_rgba(255, 224, 72, 255));
            }
        }
        UiTheme::ModernSimple => {
            let stroke = (radius * if is_selected { 0.22 } else { 0.14 }).clamp(1.4, 3.6);
            if role.is_civilian() {
                draw_poly(cx, cy, 4, radius * 0.98, 45.0, fill);
                draw_poly_lines(cx, cy, 4, radius * 0.98, 45.0, stroke, rim);
            } else if role.is_air() {
                draw_poly(cx, cy, 3, radius * 1.08, -90.0, fill);
                draw_poly_lines(cx, cy, 3, radius * 1.08, -90.0, stroke, rim);
            } else {
                draw_circle(cx, cy, radius, fill);
                draw_circle_lines(cx, cy, radius, stroke, rim);
            }

            // Clean flat geometric vector symbol inside badge
            let s = radius * 0.56;
            let lw = (radius * 0.14).clamp(1.4, 3.2);
            match role {
                UnitRole::Settler => {
                    draw_line(cx - s * 0.4, cy + s * 0.7, cx - s * 0.4, cy - s * 0.7, lw, fg);
                    draw_triangle(
                        vec2(cx - s * 0.3, cy - s * 0.7),
                        vec2(cx + s * 0.7, cy - s * 0.35),
                        vec2(cx - s * 0.3, cy + s * 0.05),
                        fg,
                    );
                }
                UnitRole::Worker => {
                    draw_line(cx, cy - s * 0.55, cx, cy + s * 0.75, lw, fg);
                    draw_line(cx - s * 0.65, cy - s * 0.45, cx + s * 0.65, cy - s * 0.45, lw * 1.2, fg);
                }
                UnitRole::Recon => {
                    draw_circle_lines(cx, cy, s * 0.55, lw, fg);
                    draw_circle(cx, cy, s * 0.18, fg);
                }
                UnitRole::Melee => {
                    draw_line(cx - s * 0.55, cy + s * 0.55, cx + s * 0.55, cy - s * 0.55, lw, fg);
                    draw_line(cx + s * 0.55, cy + s * 0.55, cx - s * 0.55, cy - s * 0.55, lw, fg);
                }
                UnitRole::Ranged => {
                    draw_line(cx - s * 0.55, cy, cx + s * 0.65, cy, lw, fg);
                    draw_line(cx + s * 0.2, cy - s * 0.45, cx + s * 0.68, cy, lw, fg);
                    draw_line(cx + s * 0.2, cy + s * 0.45, cx + s * 0.68, cy, lw, fg);
                }
                UnitRole::Cavalry => {
                    draw_line(cx - s * 0.65, cy + s * 0.4, cx, cy - s * 0.45, lw, fg);
                    draw_line(cx, cy - s * 0.45, cx + s * 0.65, cy + s * 0.4, lw, fg);
                    if era_tier >= 5 {
                        draw_line(cx - s * 0.65, cy + s * 0.6, cx + s * 0.65, cy + s * 0.6, lw, fg);
                    }
                }
                UnitRole::Siege => {
                    draw_circle(cx, cy, s * 0.42, fg);
                    draw_line(cx - s * 0.7, cy, cx + s * 0.7, cy, lw, fg);
                }
                UnitRole::Naval => {
                    draw_line(cx, cy - s * 0.65, cx, cy + s * 0.5, lw, fg);
                    draw_line(cx - s * 0.65, cy + s * 0.2, cx + s * 0.65, cy + s * 0.2, lw, fg);
                    draw_line(cx - s * 0.5, cy + s * 0.55, cx + s * 0.5, cy + s * 0.55, lw, fg);
                }
                UnitRole::Air => {
                    draw_line(cx, cy - s * 0.6, cx, cy + s * 0.6, lw, fg);
                    draw_line(cx - s * 0.65, cy + s * 0.1, cx + s * 0.65, cy + s * 0.1, lw, fg);
                }
            }
            if is_unique {
                draw_circle(
                    cx - radius * 0.72,
                    cy - radius * 0.72,
                    (radius * 0.25).max(2.4),
                    Color::from_rgba(255, 224, 72, 255),
                );
            }
        }
    }
}

/// Draw a procedural City castle/skyline sprite on the hex map.
#[allow(clippy::too_many_arguments)]
pub fn draw_city_sprite(
    theme: UiTheme,
    cx: f32,
    cy: f32,
    size: f32,
    era: u8,
    is_capital: bool,
    has_wonder: bool,
    civ_col: Color,
    accent: Color,
) {
    let dark = Color::from_rgba(22, 24, 34, 245);
    let stone = Color::from_rgba(176, 178, 188, 255);
    let stone_hi = Color::from_rgba(218, 220, 228, 255);
    let gold = Color::from_rgba(255, 218, 64, 255);

    match theme {
        UiTheme::PixelArt => {
            let u = (size * 2.0) / 14.0;
            // Ground shadow
            px_rect(cx, cy, u, -6.5, 3.2, 13.0, 2.2, Color::new(0.0, 0.0, 0.0, 0.45));

            if era >= 4 {
                // Modern/Industrial pixel skyline
                px_rect(cx, cy, u, -5.8, -1.8, 3.6, 5.8, stone);
                px_rect(cx, cy, u, -2.0, -5.2, 4.0, 9.2, stone_hi);
                px_rect(cx, cy, u, 2.2, -3.0, 3.6, 7.0, stone);
                // Lit windows
                for wy in [-4.0f32, -2.0, 0.0] {
                    px_rect(cx, cy, u, -1.0, wy, 0.8, 0.9, Color::from_rgba(120, 232, 255, 255));
                    px_rect(cx, cy, u, 0.4, wy, 0.8, 0.9, Color::from_rgba(120, 232, 255, 255));
                }
                px_rect(cx, cy, u, -6.2, 3.4, 12.4, 1.2, civ_col);
            } else {
                // Crenellated medieval/ancient pixel castle keep
                px_rect(cx, cy, u, -6.2, -2.2, 12.4, 6.4, dark);
                px_rect(cx, cy, u, -5.4, -1.4, 10.8, 5.0, stone);
                // Side watchtowers
                px_rect(cx, cy, u, -6.0, -4.2, 2.8, 3.4, stone_hi);
                px_rect(cx, cy, u, 3.2, -4.2, 2.8, 3.4, stone_hi);
                // Central keep
                px_rect(cx, cy, u, -2.2, -5.2, 4.4, 4.4, stone_hi);
                // Battlement notches
                px_rect(cx, cy, u, -5.8, -5.0, 1.1, 1.0, stone_hi);
                px_rect(cx, cy, u, -4.1, -5.0, 1.1, 1.0, stone_hi);
                px_rect(cx, cy, u, 3.4, -5.0, 1.1, 1.0, stone_hi);
                px_rect(cx, cy, u, 5.1, -5.0, 1.1, 1.0, stone_hi);
                // Civ gate & roofs
                px_rect(cx, cy, u, -1.4, 0.8, 2.8, 2.8, dark);
                px_rect(cx, cy, u, -2.4, -6.2, 4.8, 1.2, civ_col);
            }

            // Capital or Wonder golden crown / banner!
            if has_wonder {
                px_rect(cx, cy, u, -2.2, -7.6, 4.4, 1.5, gold);
                px_rect(cx, cy, u, -1.0, -8.8, 2.0, 1.3, gold);
            } else if is_capital {
                px_rect(cx, cy, u, -0.5, -8.2, 0.9, 2.6, stone_hi);
                px_rect(cx, cy, u, 0.4, -8.2, 2.6, 1.5, gold);
            } else {
                px_rect(cx, cy, u, -0.5, -7.4, 0.8, 1.8, stone_hi);
                px_rect(cx, cy, u, 0.3, -7.4, 2.0, 1.2, accent);
            }
        }
        UiTheme::ModernSimple => {
            let bw = size * 1.9;
            let bh = size * 1.25;
            draw_rectangle(cx - bw * 0.5, cy - bh * 0.5, bw, bh, Color::from_rgba(24, 28, 38, 238));
            draw_rectangle(cx - bw * 0.5, cy - bh * 0.5, bw, bh * 0.24, civ_col);
            draw_rectangle_lines(
                cx - bw * 0.5,
                cy - bh * 0.5,
                bw,
                bh,
                (size * 0.14).clamp(1.4, 3.0),
                if has_wonder { gold } else { accent },
            );
            if is_capital || has_wonder {
                draw_poly(cx, cy + bh * 0.12, 5, size * 0.34, -90.0, if has_wonder { gold } else { accent });
            } else {
                draw_circle(cx, cy + bh * 0.12, size * 0.26, accent);
            }
        }
    }
}

/// Draw any of the 16 realistic Tile Improvements (used BOTH on the hex map and inside Worker Action cards!).
pub fn draw_improvement_sprite(
    theme: UiTheme,
    cx: f32,
    cy: f32,
    size: f32,
    imp: Improvement,
    dim_f: f32,
) {
    let u = (size * 2.0) / 10.0;
    let is_px = theme == UiTheme::PixelArt;
    match imp {
        Improvement::Farm => {
            let wheat = dim(Color::from_rgba(238, 206, 78, 245), dim_f);
            let soil = dim(Color::from_rgba(136, 98, 52, 235), dim_f);
            px_rect(cx, cy, u, -4.0, -2.5, 8.0, 5.2, soil);
            for row in [-2.0f32, -0.2, 1.6] {
                px_rect(cx, cy, u, -3.5, row, 7.0, 0.9, wheat);
            }
        }
        Improvement::TerraceFarm => {
            let green = dim(Color::from_rgba(138, 230, 88, 245), dim_f);
            let wall = dim(Color::from_rgba(170, 154, 126, 240), dim_f);
            px_rect(cx, cy, u, -4.2, -2.4, 8.4, 1.3, green);
            px_rect(cx, cy, u, -3.4, -0.6, 6.8, 1.3, wall);
            px_rect(cx, cy, u, -2.6, 1.2, 5.2, 1.3, green);
        }
        Improvement::Mine => {
            let rock = dim(Color::from_rgba(128, 126, 134, 245), dim_f);
            let timber = dim(Color::from_rgba(176, 122, 66, 245), dim_f);
            let ore = dim(Color::from_rgba(246, 212, 86, 245), dim_f);
            px_rect(cx, cy, u, -3.6, -2.6, 7.2, 5.2, rock);
            px_rect(cx, cy, u, -2.0, -1.2, 4.0, 3.8, Color::from_rgba(24, 24, 30, 240));
            px_rect(cx, cy, u, -2.6, -1.8, 5.2, 1.0, timber);
            px_rect(cx, cy, u, 1.8, -2.2, 1.4, 1.4, ore);
        }
        Improvement::Pasture => {
            let fence = dim(Color::from_rgba(196, 146, 86, 245), dim_f);
            let field = dim(Color::from_rgba(116, 198, 86, 235), dim_f);
            let herd = dim(Color::from_rgba(244, 238, 224, 245), dim_f);
            px_rect(cx, cy, u, -3.6, -2.2, 7.2, 4.4, field);
            px_rect(cx, cy, u, -3.8, -2.4, 7.6, 0.8, fence);
            px_rect(cx, cy, u, -3.8, 1.6, 7.6, 0.8, fence);
            px_rect(cx, cy, u, -3.8, -2.4, 0.9, 4.8, fence);
            px_rect(cx, cy, u, 2.9, -2.4, 0.9, 4.8, fence);
            px_rect(cx, cy, u, -1.6, -0.8, 1.8, 1.4, herd);
            px_rect(cx, cy, u, 0.8, -0.2, 1.6, 1.2, herd);
        }
        Improvement::Quarry => {
            let st1 = dim(Color::from_rgba(208, 202, 188, 245), dim_f);
            let st2 = dim(Color::from_rgba(162, 156, 144, 245), dim_f);
            px_rect(cx, cy, u, -3.8, -2.0, 3.4, 2.2, st1);
            px_rect(cx, cy, u, 0.2, -2.0, 3.4, 2.2, st2);
            px_rect(cx, cy, u, -2.8, 0.6, 5.6, 2.2, st1);
        }
        Improvement::Plantation => {
            let leaf = dim(Color::from_rgba(68, 188, 86, 245), dim_f);
            let bloom = dim(Color::from_rgba(248, 168, 88, 245), dim_f);
            for ox in [-2.8f32, 0.0, 2.8] {
                px_rect(cx, cy, u, ox - 1.0, -1.4, 2.0, 3.4, leaf);
                px_rect(cx, cy, u, ox - 0.6, -2.2, 1.2, 1.2, bloom);
            }
        }
        Improvement::Camp => {
            let tent = dim(Color::from_rgba(226, 198, 142, 245), dim_f);
            let flap = dim(Color::from_rgba(92, 62, 38, 245), dim_f);
            if is_px {
                px_rect(cx, cy, u, -1.0, -3.2, 2.0, 1.4, tent);
                px_rect(cx, cy, u, -2.4, -1.8, 4.8, 1.8, tent);
                px_rect(cx, cy, u, -3.6, 0.0, 7.2, 2.6, tent);
                px_rect(cx, cy, u, -1.0, 0.4, 2.0, 2.2, flap);
            } else {
                draw_triangle(
                    vec2(cx - size * 0.72, cy + size * 0.5),
                    vec2(cx, cy - size * 0.65),
                    vec2(cx + size * 0.72, cy + size * 0.5),
                    tent,
                );
            }
        }
        Improvement::LumberMill => {
            let log = dim(Color::from_rgba(168, 108, 56, 245), dim_f);
            let saw = dim(Color::from_rgba(218, 228, 238, 245), dim_f);
            px_rect(cx, cy, u, -3.6, -1.0, 7.2, 3.4, log);
            px_rect(cx, cy, u, -2.8, -2.4, 5.6, 1.6, dim(Color::from_rgba(128, 78, 38, 245), dim_f));
            px_rect(cx, cy, u, -1.0, -0.4, 2.0, 2.0, saw);
        }
        Improvement::TradingPost => {
            let roof = dim(Color::from_rgba(224, 72, 62, 245), dim_f);
            let gold = dim(Color::from_rgba(255, 214, 62, 245), dim_f);
            px_rect(cx, cy, u, -3.8, -2.6, 7.6, 2.0, roof);
            px_rect(cx, cy, u, -3.2, -0.6, 6.4, 3.2, dim(Color::from_rgba(232, 216, 178, 245), dim_f));
            px_rect(cx, cy, u, -1.0, 0.0, 2.0, 2.0, gold);
        }
        Improvement::Fort => {
            let wall = dim(Color::from_rgba(202, 212, 224, 248), dim_f);
            let dark = dim(Color::from_rgba(44, 50, 62, 240), dim_f);
            if is_px {
                px_rect(cx, cy, u, -4.4, -3.0, 8.8, 6.2, dark);
                px_rect(cx, cy, u, -3.8, -2.2, 7.6, 4.8, wall);
                px_rect(cx, cy, u, -4.0, -3.4, 1.8, 1.4, wall);
                px_rect(cx, cy, u, -0.9, -3.4, 1.8, 1.4, wall);
                px_rect(cx, cy, u, 2.2, -3.4, 1.8, 1.4, wall);
                px_rect(cx, cy, u, -1.2, 0.2, 2.4, 2.4, dark);
            } else {
                draw_poly_lines(cx, cy, 5, size * 0.92, -90.0, (size * 0.16).max(1.6), wall);
            }
        }
        Improvement::OilWell => {
            let steel = dim(Color::from_rgba(36, 42, 52, 248), dim_f);
            let oil = dim(Color::from_rgba(255, 198, 52, 240), dim_f);
            px_rect(cx, cy, u, -0.8, -3.8, 1.6, 6.8, steel);
            px_rect(cx, cy, u, -2.6, 0.8, 5.2, 2.2, steel);
            px_rect(cx, cy, u, -2.2, -2.2, 4.4, 1.0, oil);
        }
        Improvement::PowerPlant => {
            let body = dim(Color::from_rgba(128, 146, 168, 248), dim_f);
            let bolt = dim(Color::from_rgba(255, 228, 64, 255), dim_f);
            px_rect(cx, cy, u, -3.8, -1.6, 7.6, 4.4, body);
            px_rect(cx, cy, u, -3.0, -3.4, 1.8, 2.2, body);
            px_rect(cx, cy, u, -0.8, -1.2, 2.0, 1.4, bolt);
            px_rect(cx, cy, u, -1.4, 0.2, 2.0, 1.4, bolt);
        }
        Improvement::SolarFarm => {
            let panel = dim(Color::from_rgba(44, 152, 242, 248), dim_f);
            let grid = dim(Color::from_rgba(255, 228, 84, 245), dim_f);
            px_rect(cx, cy, u, -4.0, -2.2, 8.0, 4.4, panel);
            px_rect(cx, cy, u, -4.0, -0.3, 8.0, 0.6, grid);
            px_rect(cx, cy, u, -0.3, -2.2, 0.6, 4.4, grid);
        }
        Improvement::ResearchOutpost => {
            let dome = dim(Color::from_rgba(222, 244, 255, 248), dim_f);
            let glow = dim(Color::from_rgba(68, 224, 255, 255), dim_f);
            px_rect(cx, cy, u, -2.4, -3.0, 4.8, 2.2, dome);
            px_rect(cx, cy, u, -3.6, -1.0, 7.2, 3.6, dome);
            px_rect(cx, cy, u, -1.4, -0.2, 2.8, 1.6, glow);
        }
        Improvement::Airbase => {
            let pad = dim(Color::from_rgba(72, 82, 96, 248), dim_f);
            let stripe = dim(Color::from_rgba(128, 224, 255, 255), dim_f);
            px_rect(cx, cy, u, -4.4, -2.0, 8.8, 4.2, pad);
            px_rect(cx, cy, u, -3.6, -0.4, 7.2, 0.9, WHITE);
            px_rect(cx, cy, u, 2.2, -3.6, 1.8, 2.2, stripe);
        }
        Improvement::Arcology => {
            let shell = dim(Color::from_rgba(88, 246, 212, 248), dim_f);
            let core = dim(Color::from_rgba(28, 88, 118, 238), dim_f);
            px_rect(cx, cy, u, -2.6, -3.4, 5.2, 2.0, shell);
            px_rect(cx, cy, u, -4.0, -1.6, 8.0, 4.4, shell);
            px_rect(cx, cy, u, -2.8, -0.8, 5.6, 2.8, core);
            px_rect(cx, cy, u, -1.2, -0.2, 2.4, 1.4, shell);
        }
        Improvement::None => {}
    }
}

/// Draw a miniature icon for any `WorkerOrder` inside the Worker Actions Sheet.
pub fn draw_worker_order_icon(theme: UiTheme, cx: f32, cy: f32, size: f32, order: WorkerOrder) {
    let u = (size * 2.0) / 10.0;
    match order {
        WorkerOrder::BuildImprovement(imp) => draw_improvement_sprite(theme, cx, cy, size, imp, 1.0),
        WorkerOrder::BuildRoad => {
            px_rect(cx, cy, u, -4.0, -1.4, 8.0, 2.8, Color::from_rgba(196, 162, 112, 255));
            px_rect(cx, cy, u, -3.0, -0.4, 2.0, 0.8, Color::from_rgba(235, 216, 178, 255));
            px_rect(cx, cy, u, 1.0, -0.4, 2.0, 0.8, Color::from_rgba(235, 216, 178, 255));
        }
        WorkerOrder::BuildRailroad => {
            px_rect(cx, cy, u, -4.2, -1.8, 8.4, 3.6, Color::from_rgba(74, 80, 92, 255));
            px_rect(cx, cy, u, -4.0, -1.2, 8.0, 0.7, Color::from_rgba(244, 210, 88, 255));
            px_rect(cx, cy, u, -4.0, 0.6, 8.0, 0.7, Color::from_rgba(244, 210, 88, 255));
        }
        WorkerOrder::ChopForest => {
            px_rect(cx, cy, u, -1.0, -3.0, 2.0, 5.8, Color::from_rgba(164, 108, 58, 255));
            px_rect(cx, cy, u, -3.4, -2.6, 3.2, 2.2, Color::from_rgba(218, 226, 236, 255));
            px_rect(cx, cy, u, 1.2, 0.6, 2.8, 2.0, Color::from_rgba(62, 172, 74, 255));
        }
        WorkerOrder::ClearJungle => {
            px_rect(cx, cy, u, -3.4, -2.4, 6.8, 4.8, Color::from_rgba(44, 152, 74, 255));
            px_rect(cx, cy, u, -3.2, -0.5, 6.4, 1.0, Color::from_rgba(248, 206, 72, 255));
        }
        WorkerOrder::DrainMarsh => {
            px_rect(cx, cy, u, -3.8, -2.0, 7.6, 4.2, Color::from_rgba(58, 154, 168, 255));
            px_rect(cx, cy, u, -2.8, 0.6, 5.6, 1.6, Color::from_rgba(112, 204, 86, 255));
        }
        WorkerOrder::Repair => {
            let wrench = Color::from_rgba(255, 196, 68, 255);
            px_rect(cx, cy, u, -3.4, -1.0, 6.8, 2.0, wrench);
            px_rect(cx, cy, u, -1.0, -3.4, 2.0, 6.8, wrench);
        }
    }
}

/// Draw a miniature procedural icon for any Building, World Wonder, or Unique Building in the City Production Tabs!
pub fn draw_building_icon(theme: UiTheme, cx: f32, cy: f32, size: f32, b: &BuildingDef, civ_col: Color) {
    let u = (size * 2.0) / 10.0;
    let dark = Color::from_rgba(16, 20, 30, 240);
    let gold = Color::from_rgba(255, 216, 68, 255);
    let is_ub = b.unique_to_civ.is_some();

    let accent = if b.is_wonder {
        gold
    } else if is_ub {
        civ_col
    } else if b.science > 0 || b.science_pct > 0 {
        Color::from_rgba(74, 214, 255, 255)
    } else if b.gold > 0 || b.gold_pct > 0 {
        gold
    } else if b.defense > 0 || b.supply > 0 || b.unit_xp > 0 {
        Color::from_rgba(232, 92, 82, 255)
    } else if b.production > 0 || b.production_pct > 0 {
        Color::from_rgba(242, 162, 62, 255)
    } else if b.food > 0 {
        Color::from_rgba(88, 224, 108, 255)
    } else {
        Color::from_rgba(198, 128, 248, 255)
    };

    if theme == UiTheme::PixelArt {
        px_rect(cx, cy, u, -4.6, -4.6, 9.2, 9.2, dark);
        px_rect(cx, cy, u, -4.0, -4.0, 8.0, 8.0, if b.is_wonder || is_ub { gold } else { accent });
        px_rect(cx, cy, u, -3.3, -3.3, 6.6, 6.6, Color::from_rgba(24, 30, 44, 255));
        if b.is_wonder {
            // Golden stepped wonder pyramid/monument
            px_rect(cx, cy, u, -1.0, -2.8, 2.0, 1.4, gold);
            px_rect(cx, cy, u, -2.0, -1.4, 4.0, 1.6, gold);
            px_rect(cx, cy, u, -3.0, 0.2, 6.0, 2.4, gold);
        } else {
            // Classical columned building or Unique Palace
            px_rect(cx, cy, u, -2.8, -2.4, 5.6, 1.2, accent);
            px_rect(cx, cy, u, -2.4, -1.0, 1.1, 3.0, WHITE);
            px_rect(cx, cy, u, -0.5, -1.0, 1.1, 3.0, WHITE);
            px_rect(cx, cy, u, 1.4, -1.0, 1.1, 3.0, WHITE);
            px_rect(cx, cy, u, -2.8, 1.8, 5.6, 1.1, accent);
        }
    } else {
        draw_rectangle(cx - size * 0.85, cy - size * 0.85, size * 1.7, size * 1.7, Color::from_rgba(22, 28, 40, 240));
        draw_rectangle_lines(
            cx - size * 0.85,
            cy - size * 0.85,
            size * 1.7,
            size * 1.7,
            (size * 0.14).max(1.2),
            if b.is_wonder || is_ub { gold } else { accent },
        );
        if b.is_wonder {
            draw_poly(cx, cy, 5, size * 0.56, -90.0, gold);
        } else {
            draw_rectangle(cx - size * 0.45, cy - size * 0.45, size * 0.9, size * 0.9, accent);
        }
    }
}

/// Draw a miniature procedural icon for a repeatable `CityProcess` inside the City FOCUS tab.
pub fn draw_process_icon(theme: UiTheme, cx: f32, cy: f32, size: f32, proc: CityProcess) {
    let u = (size * 2.0) / 10.0;
    let dark = Color::from_rgba(16, 20, 30, 240);
    let accent = match proc {
        CityProcess::FoodFocus => Color::from_rgba(88, 224, 108, 255),
        CityProcess::ProductionReserve => Color::from_rgba(244, 166, 62, 255),
        CityProcess::Wealth => Color::from_rgba(255, 214, 66, 255),
        CityProcess::Research => Color::from_rgba(74, 214, 255, 255),
        CityProcess::Culture => Color::from_rgba(204, 128, 250, 255),
        CityProcess::Nothing => Color::from_rgba(154, 166, 186, 255),
    };

    if theme == UiTheme::PixelArt {
        px_rect(cx, cy, u, -4.6, -4.6, 9.2, 9.2, dark);
        px_rect(cx, cy, u, -4.0, -4.0, 8.0, 8.0, accent);
        px_rect(cx, cy, u, -3.3, -3.3, 6.6, 6.6, Color::from_rgba(22, 28, 42, 255));
        match proc {
            CityProcess::FoodFocus => {
                // Pixel wheat sheaf / fruit emblem
                px_rect(cx, cy, u, -0.6, -2.6, 1.2, 5.4, Color::from_rgba(240, 204, 82, 255));
                px_rect(cx, cy, u, -2.2, -1.8, 1.6, 1.4, accent);
                px_rect(cx, cy, u, 0.6, -1.8, 1.6, 1.4, accent);
                px_rect(cx, cy, u, -2.2, 0.0, 1.6, 1.4, accent);
                px_rect(cx, cy, u, 0.6, 0.0, 1.6, 1.4, accent);
            }
            CityProcess::ProductionReserve => {
                // Pixel anvil / hammer reserve
                px_rect(cx, cy, u, -2.6, -2.2, 5.2, 1.8, accent);
                px_rect(cx, cy, u, -0.8, -0.4, 1.6, 3.0, Color::from_rgba(190, 144, 96, 255));
                px_rect(cx, cy, u, -2.4, 1.8, 4.8, 1.0, Color::from_rgba(220, 228, 240, 255));
            }
            CityProcess::Wealth => {
                // Pixel gold coin stack
                px_rect(cx, cy, u, -2.4, -2.2, 4.8, 1.4, accent);
                px_rect(cx, cy, u, -2.4, -0.4, 4.8, 1.4, Color::from_rgba(232, 178, 42, 255));
                px_rect(cx, cy, u, -2.4, 1.4, 4.8, 1.4, accent);
            }
            CityProcess::Research => {
                // Pixel science flask
                px_rect(cx, cy, u, -1.0, -2.6, 2.0, 1.6, WHITE);
                px_rect(cx, cy, u, -2.2, -1.0, 4.4, 1.6, accent);
                px_rect(cx, cy, u, -2.8, 0.6, 5.6, 2.0, accent);
            }
            CityProcess::Culture => {
                // Pixel star / festival emblem
                px_rect(cx, cy, u, -0.9, -2.8, 1.8, 5.6, accent);
                px_rect(cx, cy, u, -2.8, -0.9, 5.6, 1.8, accent);
                px_rect(cx, cy, u, -1.4, -1.4, 2.8, 2.8, WHITE);
            }
            CityProcess::Nothing => {
                // Pixel pause bars (Standby)
                px_rect(cx, cy, u, -2.2, -2.4, 1.6, 4.8, accent);
                px_rect(cx, cy, u, 0.6, -2.4, 1.6, 4.8, accent);
            }
        }
    } else {
        draw_circle(cx, cy, size * 0.86, Color::from_rgba(22, 28, 40, 240));
        draw_circle_lines(cx, cy, size * 0.86, (size * 0.14).max(1.2), accent);
        match proc {
            CityProcess::FoodFocus => draw_circle(cx, cy, size * 0.44, accent),
            CityProcess::ProductionReserve => {
                draw_rectangle(cx - size * 0.44, cy - size * 0.36, size * 0.88, size * 0.72, accent);
            }
            CityProcess::Wealth => draw_poly(cx, cy, 6, size * 0.48, 0.0, accent),
            CityProcess::Research => draw_poly(cx, cy, 3, size * 0.50, -90.0, accent),
            CityProcess::Culture => draw_poly(cx, cy, 5, size * 0.50, -90.0, accent),
            CityProcess::Nothing => {
                draw_rectangle(cx - size * 0.36, cy - size * 0.38, size * 0.24, size * 0.76, accent);
                draw_rectangle(cx + size * 0.12, cy - size * 0.38, size * 0.24, size * 0.76, accent);
            }
        }
    }
}
