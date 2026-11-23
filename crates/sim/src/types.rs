use serde::{Deserialize, Serialize};

pub type TileId = u32;
pub type UnitId = u32;
pub type CityId = u32;
pub type CivId = u16;
pub type TechId = u8;
pub type UnitKindId = u8;
pub type BuildingId = u8;
pub type TechBits = u64;
pub type BuildingBits = u64;

pub const INVALID_TILE: TileId = u32::MAX;
pub const INVALID_UNIT: UnitId = u32::MAX;
pub const INVALID_CITY: CityId = u32::MAX;
pub const NO_CIV: CivId = u16::MAX;
pub const MAX_CIVS: usize = 24;
pub const MAX_TURN_LIMIT: u32 = 350;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MapSize {
    Small = 0,
    Standard = 1,
    Large = 2,
    Huge = 3,
}

impl MapSize {
    #[inline]
    pub const fn dimensions(self) -> (u16, u16) {
        match self {
            MapSize::Small => (48, 26),    // 1,248 tiles
            MapSize::Standard => (64, 36), // 2,304 tiles
            MapSize::Large => (76, 44),    // 3,344 tiles
            MapSize::Huge => (96, 52),     // 4,992 tiles
        }
    }

    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            MapSize::Small => "Small",
            MapSize::Standard => "Standard",
            MapSize::Large => "Large",
            MapSize::Huge => "Huge",
        }
    }

    #[inline]
    pub const fn default_players(self) -> u16 {
        match self {
            MapSize::Small => 6,
            MapSize::Standard => 10,
            MapSize::Large => 16,
            MapSize::Huge => 22,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Terrain {
    Ocean = 0,
    Coast = 1,
    Grassland = 2,
    Plains = 3,
    Desert = 4,
    Tundra = 5,
    Snow = 6,
}

impl Terrain {
    #[inline]
    pub const fn is_water(self) -> bool {
        matches!(self, Terrain::Ocean | Terrain::Coast)
    }

    #[inline]
    pub const fn is_land(self) -> bool {
        !self.is_water()
    }

    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            Terrain::Ocean => "Ocean",
            Terrain::Coast => "Coast",
            Terrain::Grassland => "Grassland",
            Terrain::Plains => "Plains",
            Terrain::Desert => "Desert",
            Terrain::Tundra => "Tundra",
            Terrain::Snow => "Snow",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Elevation {
    Flat = 0,
    Hill = 1,
    Mountain = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Feature {
    None = 0,
    Forest = 1,
    Jungle = 2,
    Marsh = 3,
}

impl Feature {
    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            Feature::None => "",
            Feature::Forest => "Forest",
            Feature::Jungle => "Jungle",
            Feature::Marsh => "Marsh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Resource {
    None = 0,
    Wheat = 1,
    Cattle = 2,
    Iron = 3,
    Horses = 4,
    Gold = 5,
    Gems = 6,
    Oil = 7,
    Uranium = 8,
}

impl Resource {
    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            Resource::None => "",
            Resource::Wheat => "Wheat",
            Resource::Cattle => "Cattle",
            Resource::Iron => "Iron",
            Resource::Horses => "Horses",
            Resource::Gold => "Gold",
            Resource::Gems => "Gems",
            Resource::Oil => "Oil",
            Resource::Uranium => "Uranium",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Improvement {
    None = 0,
    Farm = 1,
    Mine = 2,
    Pasture = 3,
    TradingPost = 4,
    LumberMill = 5,
    OilWell = 6,
}

impl Improvement {
    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            Improvement::None => "",
            Improvement::Farm => "Farm",
            Improvement::Mine => "Mine",
            Improvement::Pasture => "Pasture",
            Improvement::TradingPost => "Trading Post",
            Improvement::LumberMill => "Lumber Mill",
            Improvement::OilWell => "Oil Well",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Road {
    None = 0,
    Road = 1,
    Railroad = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum UnitRole {
    Settler = 0,
    Worker = 1,
    Recon = 2,
    Melee = 3,
    Ranged = 4,
    Cavalry = 5,
    Siege = 6,
    Naval = 7,
}

impl UnitRole {
    #[inline]
    pub const fn is_civilian(self) -> bool {
        matches!(self, UnitRole::Settler | UnitRole::Worker)
    }

    #[inline]
    pub const fn is_combat(self) -> bool {
        !self.is_civilian()
    }

    #[inline]
    pub const fn is_naval(self) -> bool {
        matches!(self, UnitRole::Naval)
    }

    #[inline]
    pub const fn is_ranged_attacker(self) -> bool {
        matches!(self, UnitRole::Ranged | UnitRole::Siege)
    }

    #[inline]
    pub const fn can_capture_city(self) -> bool {
        matches!(self, UnitRole::Melee | UnitRole::Cavalry | UnitRole::Recon)
    }
}

/// Corps & Army formation stacking (Pillar 2):
/// Merging 2 units -> Corps (+10 CS, 1.25x durability, 1 supply slot)
/// Merging Corps + 1 unit -> Army (+17 CS, 1.50x durability, 1 supply slot)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum UnitFormation {
    Single = 0,
    Corps = 1,
    Army = 2,
}

impl UnitFormation {
    #[inline]
    pub const fn strength_bonus(self) -> i32 {
        match self {
            UnitFormation::Single => 0,
            UnitFormation::Corps => 10,
            UnitFormation::Army => 17,
        }
    }

    /// Damage reduction multiplier (percentage of incoming damage taken).
    #[inline]
    pub const fn incoming_damage_pct(self) -> i32 {
        match self {
            UnitFormation::Single => 100,
            UnitFormation::Corps => 80, // 1.25x durability
            UnitFormation::Army => 67,  // 1.50x durability
        }
    }

    #[inline]
    pub const fn stars(self) -> u8 {
        match self {
            UnitFormation::Single => 1,
            UnitFormation::Corps => 2,
            UnitFormation::Army => 3,
        }
    }

    #[inline]
    pub const fn label(self) -> &'static str {
        match self {
            UnitFormation::Single => "",
            UnitFormation::Corps => "Corps",
            UnitFormation::Army => "Army",
        }
    }

    #[inline]
    pub const fn unit_count_equiv(self) -> u8 {
        match self {
            UnitFormation::Single => 1,
            UnitFormation::Corps => 2,
            UnitFormation::Army => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Yields {
    pub food: i16,
    pub prod: i16,
    pub gold: i16,
    pub science: i16,
    pub culture: i16,
}

impl Yields {
    #[inline]
    pub const fn new(food: i16, prod: i16, gold: i16, science: i16, culture: i16) -> Self {
        Self {
            food,
            prod,
            gold,
            science,
            culture,
        }
    }

    #[inline]
    pub fn add_assign(&mut self, other: Yields) {
        self.food += other.food;
        self.prod += other.prod;
        self.gold += other.gold;
        self.science += other.science;
        self.culture += other.culture;
    }

    #[inline]
    pub const fn total(self) -> i32 {
        self.food as i32 + self.prod as i32 + self.gold as i32 + self.science as i32 + self.culture as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductionItem {
    Unit(UnitKindId),
    Building(BuildingId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiploStatus {
    Unmet = 0,
    Peace = 1,
    War = 2,
    Alliance = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VictoryKind {
    Domination,
    Science,
    Score,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VictoryState {
    pub winner: CivId,
    pub kind: VictoryKind,
    pub turn: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameConfig {
    pub size: MapSize,
    pub num_civs: u16,
    pub seed: u64,
    /// `Some(0)` for human controlling Civ 0, `None` for pure AI spectator mode.
    pub human_civ: Option<CivId>,
    pub turn_limit: u32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            size: MapSize::Large,
            num_civs: 12,
            seed: 42,
            human_civ: Some(0),
            turn_limit: MAX_TURN_LIMIT,
        }
    }
}
