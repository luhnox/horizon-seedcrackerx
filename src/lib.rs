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

#[cfg(test)]
mod tests;
