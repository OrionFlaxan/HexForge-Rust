use crate::settings::{
    draw_modal_frame, ui_button, TouchScroll, UiMetrics, ACCENT_CYAN, ACCENT_GOLD, ACCENT_GREEN,
    ACCENT_PURPLE, BG_CARD, BORDER_DIM,
};
use hexforge_sim::ruleset::Ruleset;
use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodexTab {
    #[default]
    HowToPlay,
    CombatCorps,
    Units,
    TechsWonders,
    CivsPolicies,
}

#[derive(Debug, Default)]
pub struct CodexState {
    pub tab: CodexTab,
    pub scroll: TouchScroll,
}

/// Draw the full 5-tab Rules & Codex / Civilopedia overlay (accessible from Title AND In-Game!).
/// Returns `true` when the player taps CLOSE.
pub fn draw_codex_modal(ui: &UiMetrics, rules: &Ruleset, state: &mut CodexState) -> bool {
    let (rect, body_top, closed) = draw_modal_frame(ui, "HEXFORGE RULES & CODEX");
    if closed {
        return true;
    }

    let pad = ui.dp(10.0);
    let tab_h = ui.dp(36.0);
    let tabs = [
        (CodexTab::HowToPlay, "RULES"),
        (CodexTab::CombatCorps, "COMBAT"),
        (CodexTab::Units, "UNITS"),
        (CodexTab::TechsWonders, "TECHS"),
        (CodexTab::CivsPolicies, "CIVS"),
    ];
    let tab_w = (rect.w - pad * 2.0 - ui.dp(16.0)) / 5.0;
    for (idx, &(t, lbl)) in tabs.iter().enumerate() {
        let r = Rect::new(
            rect.x + pad + (idx as f32) * (tab_w + ui.dp(4.0)),
            body_top + ui.dp(6.0),
            tab_w,
            tab_h,
        );
        let active = state.tab == t;
        if ui_button(
            r,
            lbl,
            if active { Color::new(0.16, 0.34, 0.48, 1.0) } else { BG_CARD },
            if active { ACCENT_GOLD } else { BORDER_DIM },
            ui.fs(13.0),
        ) {
            state.tab = t;
            state.scroll.reset();
        }
    }

    let content_top = body_top + tab_h + ui.dp(14.0);
    let footer_h = ui.dp(42.0);
    let content_h = (rect.y + rect.h - footer_h - content_top).max(ui.dp(80.0));
    let content_area = Rect::new(rect.x + pad, content_top, rect.w - pad * 2.0, content_h);

    // Build structured lines for the active tab (each entry: (header?, accent_color, text))
    let lines: Vec<(bool, Color, String)> = match state.tab {
        CodexTab::HowToPlay => {
            let mut v = vec![
                (true, ACCENT_GOLD, "1. VICTORY CONDITIONS (4 Paths to Win)".into()),
                (false, WHITE, "- Domination: Be the last surviving empire on the map.".into()),
                (false, WHITE, "- Science: Research Orbital Colonization & build Orbital Launch Complex.".into()),
                (false, WHITE, "- Culture: Adopt 12 Social Policies (3 full Policy Trees) via Culture.".into()),
                (false, WHITE, "- Score: Hold the highest Empire Score at the Turn Limit.".into()),
                (true, ACCENT_GOLD, "2. CONTROLS, UI THEMES & MANDATORY TURN GUARD".into()),
                (false, WHITE, "- Drag with 1 finger to pan; pinch with 2 fingers to zoom.".into()),
                (false, WHITE, "- Tap a unit to highlight Cyan move hexes & Red attack targets.".into()),
                (false, ACCENT_GOLD, "- Smart Turn Guard: bottom-right button shows CHOOSE TECH or BUILD: <City>".into()),
                (false, WHITE, "    so you NEVER forget research or city production before ending a turn!".into()),
                (false, ACCENT_CYAN, "- Options -> UI Theme switches live between PIXEL ART and MODERN SIMPLE!".into()),
                (false, WHITE, "- Tap a friendly City for UNITS | BLDGS | WONDERS | BUILT production tabs!".into()),
                (true, ACCENT_GOLD, "3. REALISTIC WORKERS, ENGINEERING & PILLAGING".into()),
                (false, WHITE, "- Select a Worker -> tap IMPROVE... to choose any realistic tile build,".into()),
                (false, WHITE, "    or toggle AUTO WORK so late-game workers manage tiles automatically!".into()),
                (false, WHITE, "- Multi-turn builds (1-4t) are sped up by Rome/Inca, Pyramids & Citizenship.".into()),
                (false, WHITE, "- Chop Forest (+20 Prod to nearest city!), Clear Jungle, Drain Marsh.".into()),
                (false, WHITE, "- Roads (The Wheel) give 2x speed; Railroads add +15% City Production!".into()),
                (false, WHITE, "- Combat units can PILLAGE enemy tiles (+20G+ & +20 HP); Workers Repair!".into()),
                (true, ACCENT_CYAN, "4. ALL 16 REALISTIC TILE IMPROVEMENTS".into()),
            ];
            for &imp in &hexforge_sim::types::Improvement::ALL {
                let req = hexforge_sim::improvements::improvement_required_tech(rules, imp)
                    .map(|t| rules.techs[t as usize].name.as_str())
                    .unwrap_or("Agriculture");
                v.push((
                    false,
                    WHITE,
                    format!(
                        "- {} ({}): {}",
                        imp.name(),
                        req,
                        hexforge_sim::improvements::improvement_note(imp)
                    ),
                ));
            }
            v
        }
        CodexTab::CombatCorps => vec![
            (true, ACCENT_GOLD, "1. COMBAT STRENGTH, FORTS & TERRAIN".into()),
            (false, WHITE, "- Damage scales exponentially with Strength difference (Civ V formula).".into()),
            (false, WHITE, "- Hills: +25% Defense | Forest/Jungle: +25% Defense | Marsh: -15%.".into()),
            (false, ACCENT_CYAN, "- Fort & Forward Airbase: +50% Defense (can be built on borders!).".into()),
            (false, WHITE, "- Fortify: +20% Defense and heals +18 HP/turn in friendly borders.".into()),
            (false, WHITE, "- Flanking: +8% Attack per additional friendly melee/cavalry adjacent.".into()),
            (false, WHITE, "- Pillaging: Spend 1 Move on an enemy improvement -> +20G (+Civ bonus) & +20 HP!".into()),
            (false, WHITE, "- Promotions: +5% Combat Strength per 30 XP earned (up to +25%).".into()),
            (true, ACCENT_CYAN, "2. CORPS & ARMIES (ANTI-LAG ELITE STACKING)".into()),
            (false, WHITE, "- Unlock Steel (Medieval) -> Merge 2 identical units into CORPS:".into()),
            (false, ACCENT_GOLD, "    ** CORPS: +10 Combat Strength, 1.25x Durability, uses ONLY 1 Supply!".into()),
            (false, WHITE, "- Unlock Replaceable Parts (Modern) -> Merge Corps + 1 unit into ARMY:".into()),
            (false, ACCENT_GOLD, "    *** ARMY: +17 Combat Strength, 1.50x Durability, uses ONLY 1 Supply!".into()),
            (false, WHITE, "- Move two identical units adjacent & tap the gold MERGE button!".into()),
            (true, ACCENT_GREEN, "3. SUPPLY CAP, AIR POWER & FORWARD AIRBASES".into()),
            (false, WHITE, "- Supply Cap = 5 + 2*Cities + TotalPop/2 + Barracks/Armory/Academy/Traits.".into()),
            (false, WHITE, "- Exceeding Supply Cap reduces city Production (-10% each) & adds Gold upkeep.".into()),
            (false, WHITE, "- Air & Orbital Units: Strike 5-10 hexes ignoring terrain; rebase to".into()),
            (false, ACCENT_CYAN, "    friendly Cities OR Forward Airbases (unlocked at Radar)!".into()),
        ],
        CodexTab::Units => {
            let mut v = vec![(
                true,
                ACCENT_GOLD,
                format!("ALL {} UNITS (58 BASE + 24 CIV UNIQUES ACROSS 9 ERAS)", rules.units.len()),
            )];
            for u in &rules.units {
                let tech_name = u
                    .tech
                    .map(|t| rules.techs[t as usize].name.as_str())
                    .unwrap_or("Start");
                let cs_str = if u.ranged_strength > 0 {
                    format!("CS {}/R{} (Rng {})", u.strength, u.ranged_strength, u.range)
                } else {
                    format!("CS {}", u.strength)
                };
                let uniq_tag = u
                    .unique_to_civ
                    .map(|c| format!(" [UNIQUE: {}]", rules.civs[c as usize].name))
                    .unwrap_or_default();
                v.push((
                    false,
                    if u.unique_to_civ.is_some() { ACCENT_GOLD } else { WHITE },
                    format!(
                        "[{}] {}{} — {} | Mov {} | Cost {} | {}",
                        u.role.name(),
                        u.name,
                        uniq_tag,
                        cs_str,
                        u.moves,
                        u.cost,
                        tech_name
                    ),
                ));
            }
            v
        }
        CodexTab::TechsWonders => {
            let wonders_n = rules.buildings.iter().filter(|b| b.is_wonder).count();
            let ubs_n = rules.buildings.iter().filter(|b| b.unique_to_civ.is_some()).count();
            let mut v = vec![(
                true,
                ACCENT_GOLD,
                format!(
                    "{wonders_n} WORLD WONDERS + {ubs_n} CIV UNIQUE BUILDINGS ({} TOTAL)",
                    rules.buildings.len()
                ),
            )];
            for b in &rules.buildings {
                if !b.is_wonder {
                    continue;
                }
                let tname = b.tech.map(|t| rules.techs[t as usize].name.as_str()).unwrap_or("Start");
                let extra = if b.grants_free_tech {
                    " + FREE TECH!"
                } else if b.rush_buy_discount_pct > 0 {
                    " -20% Rush Buy"
                } else if b.key == "wonder_pyramids" {
                    " -1t Worker Speed!"
                } else {
                    ""
                };
                v.push((
                    false,
                    ACCENT_GOLD,
                    format!(
                        "* {} ({} Prod, {}) — +{}F +{}P +{}G +{}S +{}C{}",
                        b.name, b.cost, tname, b.food, b.production, b.gold, b.science, b.culture, extra
                    ),
                ));
            }
            v.push((
                true,
                ACCENT_CYAN,
                format!("9 FULL ERAS & {} TECHNOLOGIES", rules.techs.len()),
            ));
            for era in 0..hexforge_sim::ruleset::NUM_ERAS {
                let names: Vec<String> = rules
                    .techs
                    .iter()
                    .filter(|t| t.era == era)
                    .map(|t| {
                        let tag = if t.unlocks_corps {
                            " [CORPS]"
                        } else if t.unlocks_army {
                            " [ARMY]"
                        } else if t.unlocks_flight {
                            " [AIR]"
                        } else if t.unlocks_railroads {
                            " [RAIL]"
                        } else if t.unlocks_science_victory {
                            " [WIN]"
                        } else {
                            ""
                        };
                        format!("{}{}", t.name, tag)
                    })
                    .collect();
                v.push((false, ACCENT_CYAN, format!("{}:", Ruleset::era_name(era))));
                for chunk in names.chunks(3) {
                    v.push((false, WHITE, format!("   {}", chunk.join("  |  "))));
                }
            }
            v
        }
        CodexTab::CivsPolicies => {
            let mut v = vec![(
                true,
                ACCENT_GOLD,
                format!("{} HISTORICAL CIVILIZATIONS (LEADERS, BUFFS, UU & UB)", rules.civs.len()),
            )];
            for c in &rules.civs {
                let uu = c
                    .unique_unit
                    .map(|id| {
                        let u = &rules.units[id as usize];
                        let rep = u
                            .replaces
                            .map(|r| format!(" ({})", rules.units[r as usize].name))
                            .unwrap_or_default();
                        format!("{}{}", u.name, rep)
                    })
                    .unwrap_or_else(|| "None".into());
                let ub = c
                    .unique_building
                    .map(|id| {
                        let b = &rules.buildings[id as usize];
                        let rep = b
                            .replaces
                            .map(|r| format!(" ({})", rules.buildings[r as usize].name))
                            .unwrap_or_default();
                        format!("{}{}", b.name, rep)
                    })
                    .unwrap_or_else(|| "None".into());
                v.push((
                    false,
                    ACCENT_GOLD,
                    format!("{} ({}) — {}: {}", c.name, c.leader, c.trait_name, c.trait_desc),
                ));
                v.push((
                    false,
                    ACCENT_CYAN,
                    format!("    UU: {}   |   UB: {}", uu, ub),
                ));
            }
            v.push((
                true,
                ACCENT_PURPLE,
                format!("5 SOCIAL POLICY TREES ({} POLICIES — 12 = CULTURE WIN)", rules.policies.len()),
            ));
            for p in &rules.policies {
                v.push((
                    false,
                    WHITE,
                    format!(
                        "[{}] {} — {}",
                        Ruleset::policy_tree_name(p.tree),
                        p.name,
                        p.desc
                    ),
                ));
            }
            v
        }
    };

    let line_h = ui.dp(24.0);
    let total_h = (lines.len() as f32) * line_h + ui.dp(12.0);
    let max_scroll = (total_h - content_h).max(0.0);
    state.scroll.update(content_area, max_scroll, ui);

    draw_rectangle(content_area.x, content_area.y, content_area.w, content_area.h, Color::new(0.05, 0.07, 0.11, 0.92));

    for (idx, (is_hdr, col, txt)) in lines.iter().enumerate() {
        let ly = content_area.y + ui.dp(6.0) + (idx as f32) * line_h - state.scroll.offset;
        if ly < content_area.y - ui.dp(4.0) || ly + line_h > content_area.y + content_area.h + ui.dp(4.0) {
            continue;
        }
        let fs = if *is_hdr { ui.fs(15.0) } else { ui.fs(13.0) };
        draw_text(txt, content_area.x + ui.dp(10.0), ly + line_h * 0.72, fs, *col);
    }

    // Touch-friendly Up / Down scroll buttons in footer
    let fy = rect.y + rect.h - footer_h + ui.dp(4.0);
    let fh = footer_h - ui.dp(10.0);
    let bw = ui.dp(120.0);
    if state.scroll.offset > 1.0
        && ui_button(Rect::new(rect.x + pad, fy, bw, fh), "SCROLL UP", BG_CARD, BORDER_DIM, ui.fs(13.0))
    {
        state.scroll.offset = (state.scroll.offset - content_h * 0.65).max(0.0);
    }
    if state.scroll.offset < max_scroll - 1.0
        && ui_button(
            Rect::new(rect.x + pad + bw + ui.dp(8.0), fy, bw, fh),
            "SCROLL DOWN",
            BG_CARD,
            BORDER_DIM,
            ui.fs(13.0),
        )
    {
        state.scroll.offset = (state.scroll.offset + content_h * 0.65).min(max_scroll);
    }
    let pct = if max_scroll > 1.0 {
        (state.scroll.offset / max_scroll * 100.0).round() as i32
    } else {
        100
    };
    draw_text(
        &format!("Drag to scroll ({pct}%)"),
        rect.x + rect.w - ui.dp(156.0),
        fy + fh * 0.68,
        ui.fs(12.0),
        LIGHTGRAY,
    );

    false
}
