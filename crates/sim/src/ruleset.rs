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
    pub starting_xp: u16,
    /// Template index of the civilization that exclusively trains this Unique Unit.
    pub unique_to_civ: Option<u8>,
    /// Generic base unit replaced by this Unique Unit.
    pub replaces: Option<UnitKindId>,
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
    /// Template index of the civilization that exclusively constructs this Unique Building.
    pub unique_to_civ: Option<u8>,
    /// Generic base building replaced by this Unique Building.
    pub replaces: Option<BuildingId>,
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
    pub leader: String,
    pub color: [u8; 3],
    pub accent: [u8; 3],
    pub aggression: u8,
    pub science_bias: u8,
    pub expansion: u8,
    pub trait_name: String,
    pub trait_desc: String,
    pub prod_pct: i16,
    pub wonder_prod_pct: i16,
    pub sci_pct: i16,
    pub gold_pct: i16,
    pub combat_pct: i16,
    pub cavalry_pct: i16,
    pub ranged_pct: i16,
    pub naval_pct: i16,
    pub city_food: i16,
    pub city_prod: i16,
    pub city_gold: i16,
    pub city_science: i16,
    pub city_culture: i16,
    pub city_defense: i16,
    pub supply_bonus: i16,
    pub unit_xp: i16,
    pub rush_discount_pct: i16,
    pub worker_speed_bonus: u8,
    pub pillage_gold_bonus: i32,
    pub full_strength_when_wounded: bool,
    pub starts_with_tech: Option<TechId>,
    pub unique_unit: Option<UnitKindId>,
    pub unique_building: Option<BuildingId>,
    pub cities: Vec<String>,
}

/// Compiled, integer-indexed 9-Era ruleset — zero string hashing during simulation.
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
    // Precomputed tech IDs for realistic improvements & tech-scaled yields (O(1) bit checks)
    pub mining_tech: Option<TechId>,
    pub animal_husbandry_tech: Option<TechId>,
    pub masonry_tech: Option<TechId>,
    pub calendar_tech: Option<TechId>,
    pub trapping_tech: Option<TechId>,
    pub bronze_working_tech: Option<TechId>,
    pub construction_tech: Option<TechId>,
    pub engineering_tech: Option<TechId>,
    pub guilds_tech: Option<TechId>,
    pub chemistry_tech: Option<TechId>,
    pub economics_tech: Option<TechId>,
    pub scientific_theory_tech: Option<TechId>,
    pub fertilizer_tech: Option<TechId>,
    pub biology_tech: Option<TechId>,
    pub electricity_tech: Option<TechId>,
    pub plastics_tech: Option<TechId>,
    pub radar_tech: Option<TechId>,
    pub ecology_tech: Option<TechId>,
    pub computers_tech: Option<TechId>,
    pub nanotech_tech: Option<TechId>,
    pub settler_kind: UnitKindId,
    pub worker_kind: UnitKindId,
    pub warrior_kind: UnitKindId,
    pub scout_kind: UnitKindId,
    pub space_center_building: Option<BuildingId>,
    pub pyramids_building: Option<BuildingId>,
    pub citizenship_policy: Option<PolicyId>,
}

pub const NUM_ERAS: u8 = 9;

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
    #[serde(default)]
    starting_xp: u16,
    #[serde(default)]
    unique_to_civ: Option<String>,
    #[serde(default)]
    replaces: Option<String>,
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
    #[serde(default)]
    unique_to_civ: Option<String>,
    #[serde(default)]
    replaces: Option<String>,
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
    #[serde(default)]
    leader: String,
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
    wonder_prod_pct: i16,
    #[serde(default)]
    sci_pct: i16,
    #[serde(default)]
    gold_pct: i16,
    #[serde(default)]
    combat_pct: i16,
    #[serde(default)]
    cavalry_pct: i16,
    #[serde(default)]
    ranged_pct: i16,
    #[serde(default)]
    naval_pct: i16,
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
    #[serde(default)]
    worker_speed_bonus: u8,
    #[serde(default)]
    pillage_gold_bonus: i32,
    #[serde(default)]
    full_strength_when_wounded: bool,
    #[serde(default)]
    starts_with_tech: Option<String>,
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
        assert!(raw.techs.len() <= 128, "TechBits (u128) supports up to 128 techs");
        assert!(raw.buildings.len() <= 128, "BuildingBits (u128) supports up to 128 buildings");
        assert!(raw.policies.len() <= 32, "PolicyBits supports up to 32 policies");
        assert!(raw.civs.len() <= 64, "Ruleset supports up to 64 civ templates");

        // Build civ name lookup first so unique_to_civ on units & buildings resolves in O(1)
        let mut civ_name_map: HashMap<&str, u8> = HashMap::with_capacity(raw.civs.len());
        for (i, c) in raw.civs.iter().enumerate() {
            civ_name_map.insert(c.name.as_str(), i as u8);
        }

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
                prereq_bits |= 1u128 << pid;
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

        let mining_tech = tech_map.get("mining").copied();
        let animal_husbandry_tech = tech_map.get("animal_husbandry").copied();
        let masonry_tech = tech_map.get("masonry").copied();
        let calendar_tech = tech_map.get("calendar").copied();
        let trapping_tech = tech_map.get("trapping").copied();
        let bronze_working_tech = tech_map.get("bronze_working").copied();
        let construction_tech = tech_map.get("construction").copied();
        let engineering_tech = tech_map.get("engineering").copied();
        let guilds_tech = tech_map.get("guilds").copied();
        let chemistry_tech = tech_map.get("chemistry").copied();
        let economics_tech = tech_map.get("economics").copied();
        let scientific_theory_tech = tech_map.get("scientific_theory").copied();
        let fertilizer_tech = tech_map.get("fertilizer").copied();
        let biology_tech = tech_map.get("biology").copied();
        let electricity_tech = tech_map.get("electricity").copied();
        let plastics_tech = tech_map.get("plastics").copied();
        let radar_tech = tech_map.get("radar").copied();
        let ecology_tech = tech_map.get("ecology").copied();
        let computers_tech = tech_map.get("computers").copied();
        let nanotech_tech = tech_map.get("nanotechnology").copied();

        let mut unit_map = HashMap::with_capacity(raw.units.len());
        for (i, u) in raw.units.iter().enumerate() {
            unit_map.insert(u.id.as_str(), i as UnitKindId);
        }

        let mut civ_uu: Vec<Option<UnitKindId>> = vec![None; raw.civs.len()];
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
            let unique_to_civ = u.unique_to_civ.as_deref().map(|cname| {
                *civ_name_map
                    .get(cname)
                    .unwrap_or_else(|| panic!("Unknown unique_to_civ '{cname}' for unit '{}'", u.id))
            });
            let replaces = u.replaces.as_deref().map(|rk| {
                *unit_map
                    .get(rk)
                    .unwrap_or_else(|| panic!("Unknown replaces unit '{rk}' for unit '{}'", u.id))
            });
            if let Some(ctpl) = unique_to_civ {
                civ_uu[ctpl as usize] = Some(id);
            }
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
                starting_xp: u.starting_xp,
                unique_to_civ,
                replaces,
            });
        }

        let mut building_map = HashMap::with_capacity(raw.buildings.len());
        for (i, b) in raw.buildings.iter().enumerate() {
            building_map.insert(b.id.as_str(), i as BuildingId);
        }

        let mut civ_ub: Vec<Option<BuildingId>> = vec![None; raw.civs.len()];
        let mut buildings = Vec::with_capacity(raw.buildings.len());
        let mut space_center_building = None;
        let mut pyramids_building = None;
        for (i, b) in raw.buildings.iter().enumerate() {
            let id = i as BuildingId;
            if b.wins_science {
                space_center_building = Some(id);
            }
            if b.id == "wonder_pyramids" {
                pyramids_building = Some(id);
            }
            let tech = b
                .tech
                .as_deref()
                .map(|k| *tech_map.get(k).unwrap_or_else(|| panic!("Unknown tech '{k}'")));
            let unique_to_civ = b.unique_to_civ.as_deref().map(|cname| {
                *civ_name_map
                    .get(cname)
                    .unwrap_or_else(|| panic!("Unknown unique_to_civ '{cname}' for building '{}'", b.id))
            });
            let replaces = b.replaces.as_deref().map(|rk| {
                *building_map
                    .get(rk)
                    .unwrap_or_else(|| panic!("Unknown replaces building '{rk}' for building '{}'", b.id))
            });
            if let Some(ctpl) = unique_to_civ {
                civ_ub[ctpl as usize] = Some(id);
            }
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
                unique_to_civ,
                replaces,
            });
        }

        let mut citizenship_policy = None;
        let policies: Vec<PolicyDef> = raw
            .policies
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                let id = i as PolicyId;
                if p.id == "lib_citizenship" {
                    citizenship_policy = Some(id);
                }
                PolicyDef {
                    id,
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
                }
            })
            .collect();

        let civs: Vec<CivDef> = raw
            .civs
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let starts_with_tech = c.starts_with_tech.as_deref().map(|tk| {
                    *tech_map
                        .get(tk)
                        .unwrap_or_else(|| panic!("Unknown starts_with_tech '{tk}' for civ '{}'", c.name))
                });
                CivDef {
                    name: c.name.clone(),
                    leader: c.leader.clone(),
                    color: c.color,
                    accent: c.accent,
                    aggression: c.aggression,
                    science_bias: c.science_bias,
                    expansion: c.expansion,
                    trait_name: c.trait_name.clone(),
                    trait_desc: c.trait_desc.clone(),
                    prod_pct: c.prod_pct,
                    wonder_prod_pct: c.wonder_prod_pct,
                    sci_pct: c.sci_pct,
                    gold_pct: c.gold_pct,
                    combat_pct: c.combat_pct,
                    cavalry_pct: c.cavalry_pct,
                    ranged_pct: c.ranged_pct,
                    naval_pct: c.naval_pct,
                    city_food: c.city_food,
                    city_prod: c.city_prod,
                    city_gold: c.city_gold,
                    city_science: c.city_science,
                    city_culture: c.city_culture,
                    city_defense: c.city_defense,
                    supply_bonus: c.supply_bonus,
                    unit_xp: c.unit_xp,
                    rush_discount_pct: c.rush_discount_pct,
                    worker_speed_bonus: c.worker_speed_bonus,
                    pillage_gold_bonus: c.pillage_gold_bonus,
                    full_strength_when_wounded: c.full_strength_when_wounded,
                    starts_with_tech,
                    unique_unit: civ_uu[i],
                    unique_building: civ_ub[i],
                    cities: c.cities.clone(),
                }
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
            mining_tech,
            animal_husbandry_tech,
            masonry_tech,
            calendar_tech,
            trapping_tech,
            bronze_working_tech,
            construction_tech,
            engineering_tech,
            guilds_tech,
            chemistry_tech,
            economics_tech,
            scientific_theory_tech,
            fertilizer_tech,
            biology_tech,
            electricity_tech,
            plastics_tech,
            radar_tech,
            ecology_tech,
            computers_tech,
            nanotech_tech,
            settler_kind,
            worker_kind,
            warrior_kind,
            scout_kind,
            space_center_building,
            pyramids_building,
            citizenship_policy,
        })
    }

    #[inline]
    pub fn has_tech(tech_bits: TechBits, tech: TechId) -> bool {
        (tech_bits & (1u128 << tech)) != 0
    }

    #[inline]
    pub fn has_opt_tech(tech_bits: TechBits, tech: Option<TechId>) -> bool {
        tech.map_or(true, |t| Self::has_tech(tech_bits, t))
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

    /// Resolve upgrade target for `kind` owned by `civ_tpl` (auto-upgrading into Civ's Unique Unit when applicable!).
    #[inline]
    pub fn upgrade_target_for_civ(&self, civ_tpl: u8, kind: UnitKindId) -> Option<UnitKindId> {
        let base_upg = self.units[kind as usize].upgrades_to?;
        if let Some(uu_id) = self.civs.get(civ_tpl as usize).and_then(|c| c.unique_unit) {
            if self.units[uu_id as usize].replaces == Some(base_upg) {
                return Some(uu_id);
            }
        }
        Some(base_upg)
    }

    /// Non-civ-specific unit check (only considers generic base units).
    #[inline]
    pub fn can_train_unit(&self, tech_bits: TechBits, kind: UnitKindId) -> bool {
        let def = &self.units[kind as usize];
        if def.unique_to_civ.is_some() {
            return false;
        }
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

    /// Civ-aware unit check: enforces Civ-exclusive Unique Units & automatically replaces their generic counterpart!
    #[inline]
    pub fn can_train_unit_for_civ(&self, tech_bits: TechBits, civ_tpl: u8, kind: UnitKindId) -> bool {
        let def = &self.units[kind as usize];
        if let Some(owner_tpl) = def.unique_to_civ {
            if owner_tpl != civ_tpl {
                return false;
            }
        } else if let Some(uu_id) = self.civs.get(civ_tpl as usize).and_then(|c| c.unique_unit) {
            if self.units[uu_id as usize].replaces == Some(kind) {
                return false; // Replaced by this civ's Unique Unit!
            }
        }
        let unlocked = def.tech.map_or(true, |t| Self::has_tech(tech_bits, t));
        if !unlocked {
            return false;
        }
        if let Some(upg) = self.upgrade_target_for_civ(civ_tpl, kind) {
            if self.units[upg as usize]
                .tech
                .map_or(true, |t| Self::has_tech(tech_bits, t))
            {
                return false;
            }
        }
        true
    }

    /// Non-civ-specific building check (uses `u128` `BuildingBits`, excludes civ-unique structures).
    #[inline]
    pub fn can_build_building(
        &self,
        tech_bits: TechBits,
        built_bits: BuildingBits,
        wonders_built: BuildingBits,
        b: BuildingId,
    ) -> bool {
        let def = &self.buildings[b as usize];
        if def.unique_to_civ.is_some() {
            return false;
        }
        let mask = 1u128 << b;
        if (built_bits & mask) != 0 {
            return false;
        }
        if def.is_wonder && (wonders_built & mask) != 0 {
            return false;
        }
        def.tech.map_or(true, |t| Self::has_tech(tech_bits, t))
    }

    /// Civ-aware building check: enforces Civ-exclusive Unique Buildings & avoids duplicate base/UB structures!
    #[inline]
    pub fn can_build_building_for_civ(
        &self,
        tech_bits: TechBits,
        built_bits: BuildingBits,
        wonders_built: BuildingBits,
        civ_tpl: u8,
        b: BuildingId,
    ) -> bool {
        let def = &self.buildings[b as usize];
        if let Some(owner_tpl) = def.unique_to_civ {
            if owner_tpl != civ_tpl {
                return false;
            }
        } else if let Some(ub_id) = self.civs.get(civ_tpl as usize).and_then(|c| c.unique_building) {
            if self.buildings[ub_id as usize].replaces == Some(b) {
                return false; // Replaced by this civ's Unique Building!
            }
        }
        let mask = 1u128 << b;
        if (built_bits & mask) != 0 {
            return false;
        }
        // Prevent double-building a base building and its UB (or two UBs sharing the same base) in conquered cities
        let base_slot = def.replaces.unwrap_or(b);
        if (built_bits & (1u128 << base_slot)) != 0 {
            return false;
        }
        if built_bits != 0 {
            for other in &self.buildings {
                if other.replaces == Some(base_slot) && (built_bits & (1u128 << other.id)) != 0 {
                    return false;
                }
            }
        }
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
            5 => "Modern",
            6 => "Atomic",
            7 => "Information",
            _ => "Future",
        }
    }

    #[inline]
    pub const fn era_short_name(era: u8) -> &'static str {
        match era {
            0 => "Ancient",
            1 => "Classic",
            2 => "Mediev",
            3 => "Renais",
            4 => "Indust",
            5 => "Modern",
            6 => "Atomic",
            7 => "Info",
            _ => "Future",
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

    #[inline]
    pub fn best_unit_for_role_civ(
        &self,
        tech_bits: TechBits,
        civ_tpl: u8,
        role: UnitRole,
    ) -> Option<UnitKindId> {
        let mut best: Option<(UnitKindId, i32)> = None;
        for u in &self.units {
            if u.role == role && self.can_train_unit_for_civ(tech_bits, civ_tpl, u.id) {
                let unique_bonus = if u.unique_to_civ == Some(civ_tpl) { 2 } else { 0 };
                let score = (u.strength.max(u.ranged_strength) as i32) * 10 + unique_bonus;
                if best.map_or(true, |(_, bs)| score > bs) {
                    best = Some((u.id, score));
                }
            }
        }
        best.map(|(id, _)| id)
    }
}
