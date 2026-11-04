// The test crate for the parts of SeedCrackerX that use no game types. In the game the same files are
// modules of game.rs (see horizon-mixin.toml); here they are modules of one crate with the same
// names, so `use super::scx_version::...` reads the same in both.
#[path = "core/version.rs"]
pub mod scx_version;
#[path = "core/mth.rs"]
pub mod scx_mth;
#[path = "core/lcg.rs"]
pub mod scx_lcg;
#[path = "core/sha256.rs"]
pub mod scx_sha256;
#[path = "core/rand.rs"]
pub mod scx_rand;
#[path = "core/seeds.rs"]
pub mod scx_seeds;

#[path = "core/dimension.rs"]
pub mod scx_dimension;
#[path = "core/biome.rs"]
pub mod scx_biome;
#[path = "core/worldgen_random.rs"]
pub mod scx_worldgen_random;
#[path = "feature/feature.rs"]
pub mod scx_feature;

#[path = "cracker/data.rs"]
pub mod scx_cracker_data;
#[path = "cracker/storage.rs"]
pub mod scx_cracker_storage;
#[path = "cracker/time_machine.rs"]
pub mod scx_cracker_time_machine;

#[path = "config/config.rs"]
pub mod scx_config;
#[path = "feature/features.rs"]
pub mod scx_features;
#[path = "finder/level.rs"]
pub mod scx_level;
#[path = "finder/piece.rs"]
pub mod scx_finder_piece;
#[path = "finder/structures_dsl.rs"]
pub mod scx_finder_structures;
#[path = "finder/finder.rs"]
pub mod scx_finder;
#[path = "finder/finders.rs"]
pub mod scx_finders;

#[path = "config/lang.rs"]
pub mod scx_lang;

#[path = "config/structure_save.rs"]
pub mod scx_structure_save;

#[path = "config/database.rs"]
pub mod scx_database;

#[cfg(test)]
mod tests;
