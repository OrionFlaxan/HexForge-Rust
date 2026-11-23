use hexforge_sim::save::{load_from_file, save_to_file, SaveError};
use hexforge_sim::world::World;
use std::path::PathBuf;

pub fn save_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HEXFORGE_SAVE_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("saves");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("hexforge-rust")
                .join("saves");
        }
    }
    PathBuf::from("saves")
}

pub fn slot_path(name: &str) -> PathBuf {
    save_dir().join(format!("{name}.hxfg"))
}

pub fn has_save(name: &str) -> bool {
    slot_path(name).exists()
}

pub fn save_slot(world: &World, name: &str) -> Result<usize, SaveError> {
    save_to_file(world, &slot_path(name))
}

pub fn load_slot(name: &str) -> Result<World, SaveError> {
    load_from_file(&slot_path(name))
}
