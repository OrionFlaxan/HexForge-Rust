use crate::types::{BuildingBits, BuildingId, GameSpeed, PolicyId, TechBits, TechId, UnitKindId, UnitRole};
use serde::Deserialize;
use std::collections::HashMap;

pub const DEFAULT_RULESET_TOML: &str = include_str!("../../../assets/ruleset/default.toml");

#[derive(Debug, Clone)]
pub struct TechDef {
    pub id: TechId,
    pub key: String,
    pub name: String,
    pub era: u8,
    pub cost: i32,
    pub prereq_bits: TechBits,
    pub unlocks_roads: bool,
    pub unlocks_embark: bool,
    pub unlocks_corps: bool,
    pub unlocks_army: bool,
    pub unlocks_ocean_embark: bool,
    pub unlocks_railroads: bool,
    pub unlocks_flight: bool,
    pub unlocks_science_victory: bool,
}

#[derive(Debug, Clone)]
pub struct UnitDef {
    pub id: UnitKindId,
    pub key: String,
    pub name: String,
    pub role: UnitRole,
    pub cost: i32,
    pub strength: i16,
    pub ranged_strength: i16,
    pub range: u8,
    pub moves: u8,
    pub sight: u8,
    pub upkeep: i16,
    pub tech: Option<TechId>,
    pub upgrades_to: Option<UnitKindId>,
    pub city_attack_bonus: i16,
}

#[derive(Debug, Clone)]
pub struct BuildingDef {
    pub id: BuildingId,
    pub key: String,
    pub name: String,
    pub cost: i32,
    pub upkeep: i16,
    pub food: i16,
    pub production: i16,
    pub production_pct: i16,
    pub gold: i16,
    pub gold_pct: i16,
    pub science: i16,
    pub science_pct: i16,
    pub science_per_pop_half: bool,
    pub culture: i16,
    pub defense: i16,
    pub supply: i16,
    pub unit_xp: i16,
    pub rush_buy_discount_pct: i16,
    pub tech: Option<TechId>,
    pub is_wonder: bool,
    pub grants_free_tech: bool,
    pub wins_science: bool,
}

#[derive(Debug, Clone)]
pub struct PolicyDef {
    pub id: PolicyId,
    pub key: String,
    pub name: String,
    pub tree: u8,
    pub tier: u8,
    pub desc: String,
    pub prod_pct: i16,
    pub sci_pct: i16,
    pub gold_pct: i16,
    pub combat_pct: i16,
    pub city_food: i16,
    pub city_prod: i16,
    pub city_gold: i16,
    pub city_science: i16,
    pub city_culture: i16,
    pub city_defense: i16,
    pub supply_bonus: i16,
    pub unit_xp: i16,
    pub rush_discount_pct: i16,
}

#[derive(Debug, Clone)]
pub struct CivDef {
    pub name: String,
    pub color: [u8; 3],
    pub accent: [u8; 3],
    pub aggression: u8,
    pub science_bias: u8,
    pub expansion: u8,
    pub trait_name: String,
    pub trait_desc: String,
    pub prod_pct: i16,
    pub sci_pct: i16,
    pub gold_pct: i16,
    pub combat_pct: i16,
    pub cavalry_pct: i16,
    pub city_food: i16,
    pub city_culture: i16,
    pub supply_bonus: i16,
    pub cities: Vec<String>,
}

/// Compiled, integer-indexed 6-Era ruleset — zero string hashing during simulation.
#[derive(Debug, Clone)]
pub struct Ruleset {
    pub techs: Vec<TechDef>,
    pub units: Vec<UnitDef>,
    pub buildings: Vec<BuildingDef>,
    pub policies: Vec<PolicyDef>,
    pub civs: Vec<CivDef>,
    pub road_tech: TechId,
    pub embark_tech: TechId,
    pub corps_tech: TechId,
    pub army_tech: TechId,
    pub ocean_embark_tech: TechId,
    pub railroad_tech: TechId,
    pub flight_tech: TechId,
    pub science_victory_tech: TechId,
    pub settler_kind: UnitKindId,
    pub worker_kind: UnitKindId,
    pub warrior_kind: UnitKindId,
    pub scout_kind: UnitKindId,
    pub space_center_building: Option<BuildingId>,
}

#[derive(Deserialize)]
struct RawRuleset {
    techs: Vec<RawTech>,
    units: Vec<RawUnit>,
    buildings: Vec<RawBuilding>,
    #[serde(default)]
    policies: Vec<RawPolicy>,
    civs: Vec<RawCiv>,
}

#[derive(Deserialize)]
struct RawTech {
    id: String,
    name: String,
    era: u8,
    cost: i32,
    #[serde(default)]
    prereqs: Vec<String>,
    #[serde(default)]
    unlocks_roads: bool,
    #[serde(default)]
    unlocks_embark: bool,
    #[serde(default)]
    unlocks_corps: bool,
    #[serde(default)]
    unlocks_army: bool,
    #[serde(default)]
    unlocks_ocean_embark: bool,
    #[serde(default)]
    unlocks_railroads: bool,
    #[serde(default)]
    unlocks_flight: bool,
    #[serde(default)]
    unlocks_science_victory: bool,
}

#[derive(Deserialize)]
struct RawUnit {
    id: String,
    name: String,
    role: String,
    cost: i32,
    strength: i16,
    ranged_strength: i16,
    range: u8,
    moves: u8,
    sight: u8,
    upkeep: i16,
    tech: Option<String>,
    upgrades_to: Option<String>,
    #[serde(default)]
    city_attack_bonus: i16,
}

#[derive(Deserialize)]
struct RawBuilding {
    id: String,
    name: String,
    cost: i32,
    upkeep: i16,
    #[serde(default)]
    food: i16,
    #[serde(default)]
    production: i16,
    #[serde(default)]
    production_pct: i16,
    #[serde(default)]
    gold: i16,
    #[serde(default)]
    gold_pct: i16,
    #[serde(default)]
    science: i16,
    #[serde(default)]
    science_pct: i16,
    #[serde(default)]
    science_per_pop_half: bool,
    #[serde(default)]
    culture: i16,
    #[serde(default)]
    defense: i16,
    #[serde(default)]
    supply: i16,
    #[serde(default)]
    unit_xp: i16,
    #[serde(default)]
    rush_buy_discount_pct: i16,
    tech: Option<String>,
    #[serde(default)]
    is_wonder: bool,
    #[serde(default)]
    grants_free_tech: bool,
    #[serde(default)]
    wins_science: bool,
}

#[derive(Deserialize)]
struct RawPolicy {
    id: String,
    name: String,
    tree: u8,
    tier: u8,
    desc: String,
    #[serde(default)]
    prod_pct: i16,
    #[serde(default)]
    sci_pct: i16,
    #[serde(default)]
    gold_pct: i16,
    #[serde(default)]
    combat_pct: i16,
    #[serde(default)]
    city_food: i16,
    #[serde(default)]
    city_prod: i16,
    #[serde(default)]
    city_gold: i16,
    #[serde(default)]
    city_science: i16,
    #[serde(default)]
    city_culture: i16,
    #[serde(default)]
    city_defense: i16,
    #[serde(default)]
    supply_bonus: i16,
    #[serde(default)]
    unit_xp: i16,
    #[serde(default)]
    rush_discount_pct: i16,
}

#[derive(Deserialize)]
struct RawCiv {
    name: String,
    color: [u8; 3],
    accent: [u8; 3],
    aggression: u8,
    science_bias: u8,
    expansion: u8,
    #[serde(default)]
    trait_name: String,
    #[serde(default)]
    trait_desc: String,
    #[serde(default)]
    prod_pct: i16,
    #[serde(default)]
    sci_pct: i16,
    #[serde(default)]
    gold_pct: i16,
    #[serde(default)]
    combat_pct: i16,
    #[serde(default)]
    cavalry_pct: i16,
    #[serde(default)]
    city_food: i16,
    #[serde(default)]
    city_culture: i16,
    #[serde(default)]
    supply_bonus: i16,
    cities: Vec<String>,
}

fn parse_role(s: &str) -> UnitRole {
    match s {
        "settler" => UnitRole::Settler,
        "worker" => UnitRole::Worker,
        "recon" => UnitRole::Recon,
        "melee" => UnitRole::Melee,
        "ranged" => UnitRole::Ranged,
        "cavalry" => UnitRole::Cavalry,
        "siege" => UnitRole::Siege,
        "naval" => UnitRole::Naval,
        "air" => UnitRole::Air,
        other => panic!("Unknown unit role in ruleset: {other}"),
    }
}

impl Ruleset {
    pub fn load_default() -> Self {
        Self::from_toml(DEFAULT_RULESET_TOML).expect("Embedded default.toml must parse cleanly")
    }

    pub fn from_toml(src: &str) -> Result<Self, toml::de::Error> {
        let raw: RawRuleset = toml::from_str(src)?;
        assert!(raw.techs.len() <= 64, "TechBits supports up to 64 techs");
        assert!(raw.buildings.len() <= 64, "BuildingBits supports up to 64 buildings");
        assert!(raw.policies.len() <= 32, "PolicyBits supports up to 32 policies");

        let mut tech_map = HashMap::with_capacity(raw.techs.len());
        for (i, t) in raw.techs.iter().enumerate() {
            tech_map.insert(t.id.as_str(), i as TechId);
        }

        let mut techs = Vec::with_capacity(raw.techs.len());
        let mut road_tech = 0;
        let mut embark_tech = 0;
        let mut corps_tech = 0;
        let mut army_tech = 0;
        let mut ocean_embark_tech = 0;
        let mut railroad_tech = 0;
        let mut flight_tech = 0;
        let mut science_victory_tech = 0;

        for (i, t) in raw.techs.iter().enumerate() {
            let mut prereq_bits: TechBits = 0;
            for p in &t.prereqs {
                let pid = *tech_map
                    .get(p.as_str())
                    .unwrap_or_else(|| panic!("Unknown tech prereq '{p}' for '{}'", t.id));
                prereq_bits |= 1u64 << pid;
            }
            let id = i as TechId;
            if t.unlocks_roads {
                road_tech = id;
            }
            if t.unlocks_embark {
                embark_tech = id;
            }
            if t.unlocks_corps {
                corps_tech = id;
            }
            if t.unlocks_army {
                army_tech = id;
            }
            if t.unlocks_ocean_embark {
                ocean_embark_tech = id;
            }
            if t.unlocks_railroads {
                railroad_tech = id;
            }
            if t.unlocks_flight {
                flight_tech = id;
            }
            if t.unlocks_science_victory {
                science_victory_tech = id;
            }
            techs.push(TechDef {
                id,
                key: t.id.clone(),
                name: t.name.clone(),
                era: t.era,
                cost: t.cost,
                prereq_bits,
                unlocks_roads: t.unlocks_roads,
                unlocks_embark: t.unlocks_embark,
                unlocks_corps: t.unlocks_corps,
                unlocks_army: t.unlocks_army,
                unlocks_ocean_embark: t.unlocks_ocean_embark,
                unlocks_railroads: t.unlocks_railroads,
                unlocks_flight: t.unlocks_flight,
                unlocks_science_victory: t.unlocks_science_victory,
            });
        }

        let mut unit_map = HashMap::with_capacity(raw.units.len());
        for (i, u) in raw.units.iter().enumerate() {
            unit_map.insert(u.id.as_str(), i as UnitKindId);
        }

        let mut units = Vec::with_capacity(raw.units.len());
        let mut settler_kind = 0;
        let mut worker_kind = 0;
        let mut warrior_kind = 0;
        let mut scout_kind = 0;

        for (i, u) in raw.units.iter().enumerate() {
            let id = i as UnitKindId;
            let role = parse_role(&u.role);
            if u.id == "settler" {
                settler_kind = id;
            } else if u.id == "worker" {
                worker_kind = id;
            } else if u.id == "warrior" {
                warrior_kind = id;
            } else if u.id == "scout" {
                scout_kind = id;
            }
            let tech = u
                .tech
                .as_deref()
                .map(|k| *tech_map.get(k).unwrap_or_else(|| panic!("Unknown tech '{k}'")));
            let upgrades_to = u
                .upgrades_to
                .as_deref()
                .map(|k| *unit_map.get(k).unwrap_or_else(|| panic!("Unknown unit upgrade '{k}'")));
            units.push(UnitDef {
                id,
                key: u.id.clone(),
                name: u.name.clone(),
                role,
                cost: u.cost,
                strength: u.strength,
                ranged_strength: u.ranged_strength,
                range: u.range,
                moves: u.moves,
                sight: u.sight,
                upkeep: u.upkeep,
                tech,
                upgrades_to,
                city_attack_bonus: u.city_attack_bonus,
            });
        }

        let mut buildings = Vec::with_capacity(raw.buildings.len());
        let mut space_center_building = None;
        for (i, b) in raw.buildings.iter().enumerate() {
            let id = i as BuildingId;
            if b.wins_science {
                space_center_building = Some(id);
            }
            let tech = b
                .tech
                .as_deref()
                .map(|k| *tech_map.get(k).unwrap_or_else(|| panic!("Unknown tech '{k}'")));
            buildings.push(BuildingDef {
                id,
                key: b.id.clone(),
                name: b.name.clone(),
                cost: b.cost,
                upkeep: b.upkeep,
                food: b.food,
                production: b.production,
                production_pct: b.production_pct,
                gold: b.gold,
                gold_pct: b.gold_pct,
                science: b.science,
                science_pct: b.science_pct,
                science_per_pop_half: b.science_per_pop_half,
                culture: b.culture,
                defense: b.defense,
                supply: b.supply,
                unit_xp: b.unit_xp,
                rush_buy_discount_pct: b.rush_buy_discount_pct,
                tech,
                is_wonder: b.is_wonder,
                grants_free_tech: b.grants_free_tech,
                wins_science: b.wins_science,
            });
        }

        let policies = raw
            .policies
            .into_iter()
            .enumerate()
            .map(|(i, p)| PolicyDef {
                id: i as PolicyId,
                key: p.id,
                name: p.name,
                tree: p.tree,
                tier: p.tier,
                desc: p.desc,
                prod_pct: p.prod_pct,
                sci_pct: p.sci_pct,
                gold_pct: p.gold_pct,
                combat_pct: p.combat_pct,
                city_food: p.city_food,
                city_prod: p.city_prod,
                city_gold: p.city_gold,
                city_science: p.city_science,
                city_culture: p.city_culture,
                city_defense: p.city_defense,
                supply_bonus: p.supply_bonus,
                unit_xp: p.unit_xp,
                rush_discount_pct: p.rush_discount_pct,
            })
            .collect();

        let civs = raw
            .civs
            .into_iter()
            .map(|c| CivDef {
                name: c.name,
                color: c.color,
                accent: c.accent,
                aggression: c.aggression,
                science_bias: c.science_bias,
                expansion: c.expansion,
                trait_name: c.trait_name,
                trait_desc: c.trait_desc,
                prod_pct: c.prod_pct,
                sci_pct: c.sci_pct,
                gold_pct: c.gold_pct,
                combat_pct: c.combat_pct,
                cavalry_pct: c.cavalry_pct,
                city_food: c.city_food,
                city_culture: c.city_culture,
                supply_bonus: c.supply_bonus,
                cities: c.cities,
            })
            .collect();

        Ok(Self {
            techs,
            units,
            buildings,
            policies,
            civs,
            road_tech,
            embark_tech,
            corps_tech,
            army_tech,
            ocean_embark_tech,
            railroad_tech,
            flight_tech,
            science_victory_tech,
            settler_kind,
            worker_kind,
            warrior_kind,
            scout_kind,
            space_center_building,
        })
    }

    #[inline]
    pub fn has_tech(tech_bits: TechBits, tech: TechId) -> bool {
        (tech_bits & (1u64 << tech)) != 0
    }

    #[inline]
    pub fn can_research(&self, tech_bits: TechBits, tech: TechId) -> bool {
        let def = &self.techs[tech as usize];
        !Self::has_tech(tech_bits, tech) && (tech_bits & def.prereq_bits) == def.prereq_bits
    }

    #[inline]
    pub fn scaled_cost(base_cost: i32, speed: GameSpeed) -> i32 {
        ((base_cost * speed.cost_pct()) / 100).max(5)
    }

    #[inline]
    pub fn can_train_unit(&self, tech_bits: TechBits, kind: UnitKindId) -> bool {
        let def = &self.units[kind as usize];
        let unlocked = def.tech.map_or(true, |t| Self::has_tech(tech_bits, t));
        if !unlocked {
            return false;
        }
        if let Some(upg) = def.upgrades_to {
            if self.units[upg as usize]
                .tech
                .map_or(true, |t| Self::has_tech(tech_bits, t))
            {
                return false;
            }
        }
        true
    }

    #[inline]
    pub fn can_build_building(
        &self,
        tech_bits: TechBits,
        built_bits: BuildingBits,
        wonders_built: BuildingBits,
        b: BuildingId,
    ) -> bool {
        let mask = 1u64 << b;
        if (built_bits & mask) != 0 {
            return false;
        }
        let def = &self.buildings[b as usize];
        if def.is_wonder && (wonders_built & mask) != 0 {
            return false;
        }
        def.tech.map_or(true, |t| Self::has_tech(tech_bits, t))
    }

    #[inline]
    pub fn era_for_techs(&self, tech_bits: TechBits) -> u8 {
        let mut max_era = 0;
        for t in &self.techs {
            if Self::has_tech(tech_bits, t.id) && t.era > max_era {
                max_era = t.era;
            }
        }
        max_era
    }

    #[inline]
    pub const fn era_name(era: u8) -> &'static str {
        match era {
            0 => "Ancient",
            1 => "Classical",
            2 => "Medieval",
            3 => "Renaissance",
            4 => "Industrial",
            _ => "Modern",
        }
    }

    #[inline]
    pub const fn policy_tree_name(tree: u8) -> &'static str {
        match tree {
            0 => "Tradition",
            1 => "Liberty",
            2 => "Honor",
            3 => "Rationalism",
            _ => "Industry",
        }
    }

    #[inline]
    pub fn best_unit_for_role(&self, tech_bits: TechBits, role: UnitRole) -> Option<UnitKindId> {
        let mut best: Option<(UnitKindId, i16)> = None;
        for u in &self.units {
            if u.role == role && self.can_train_unit(tech_bits, u.id) {
                let score = u.strength.max(u.ranged_strength);
                if best.map_or(true, |(_, bs)| score > bs) {
                    best = Some((u.id, score));
                }
            }
        }
        best.map(|(id, _)| id)
    }
}
