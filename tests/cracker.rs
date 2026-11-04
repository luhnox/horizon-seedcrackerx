// The cracker end to end: structures are put where a made-up world has them, handed to the storage as the finders would,
// and the search must come back with that world's seeds.
use horizon_seedcrackerx::scx_cracker_data::{BiomeSourceFactory, DataAddedEvent, HashedSeedData, Host, PillarData};
use horizon_seedcrackerx::scx_cracker_storage::DataStorage;
use horizon_seedcrackerx::scx_cracker_time_machine::Phase;
use horizon_seedcrackerx::scx_feature::{Feature, FeatureData, Structure, StructureClass};
use horizon_seedcrackerx::scx_rand::ChunkRand;
use horizon_seedcrackerx::scx_seeds::{pillar_seed, world_seed};
use horizon_seedcrackerx::scx_version::MCVersion;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct TestHost {
    lines: Mutex<Vec<String>>,
    world_seed: Mutex<Option<i64>>,
}

impl Host for TestHost {
    fn debug(&self, message: &str) {
        self.lines.lock().unwrap().push(message.to_owned());
    }
    fn warn(&self, key: &str, args: &[String]) {
        self.lines.lock().unwrap().push(format!("{key} {args:?}"));
    }
    fn error(&self, key: &str) {
        self.lines.lock().unwrap().push(format!("ERROR {key}"));
    }
    fn print_seed(&self, key: &str, seed: i64) {
        self.lines.lock().unwrap().push(format!("{key} {seed}"));
    }
    fn translate(&self, key: &str) -> String {
        key.to_owned()
    }
    fn version(&self) -> MCVersion {
        MCVersion::latest()
    }
    fn debug_enabled(&self) -> bool {
        false
    }
    fn world_seed_found(&self, seed: i64) {
        *self.world_seed.lock().unwrap() = Some(seed);
    }
    fn biome_source_factory(&self) -> Option<BiomeSourceFactory> {
        None
    }
}

fn structure(class: StructureClass) -> Arc<Feature> {
    Arc::new(Feature::Structure(Structure::new(class, MCVersion::latest()).unwrap()))
}

/// The structures of a world, as the finders would see them: one of each of the classes, around the origin.
fn structures_of(structure_seed: i64, classes: &[StructureClass], regions: &[(i32, i32)]) -> Vec<FeatureData> {
    let mut rand = ChunkRand::new();
    let mut out = Vec::new();
    for &class in classes {
        let feature = structure(class);
        let Feature::Structure(s) = &*feature else { unreachable!() };
        for &(rx, rz) in regions {
            if let Some(pos) = s.get_in_region(structure_seed, rx, rz, &mut rand) {
                out.push(FeatureData::structure(&feature, pos.x, pos.z));
            }
        }
    }
    out
}

fn poke(phase: Phase) -> DataAddedEvent {
    Arc::new(move |s: &DataStorage| s.get_time_machine().poke(s, Some(phase)))
}

#[test]
fn liftable_structures_and_the_hashed_seed_give_the_world_seed() {
    let structure_seed = 0x0000_5c3a_91d7_0e42_i64;
    let world = (0x4b1d_i64 << 48) | structure_seed;
    let host = Arc::new(TestHost::default());
    let storage = DataStorage::new(host.clone());
    storage.get_time_machine().set_threads(8);

    use StructureClass::*;
    let found = structures_of(structure_seed, &[Igloo, DesertPyramid, JunglePyramid, SwampHut, Shipwreck, PillagerOutpost], &[(0, 0), (0, 1)]);
    assert!(found.len() >= 10, "{}", found.len());
    storage.add_hashed_seed_data(HashedSeedData::new(world_seed::to_hash(world)), poke(Phase::Biomes));
    for d in found {
        storage.add_base_data(d, poke(Phase::Lifting));
    }
    assert!(storage.add_base_data(structures_of(structure_seed, &[Monument], &[(0, 0)]).remove(0), poke(Phase::Lifting)));
    storage.tick().unwrap().join().unwrap();

    assert!(storage.get_lifting_bits() >= 40.0, "{}", storage.get_lifting_bits());
    let tm = storage.get_time_machine();
    assert!(tm.structure_seeds().contains(&structure_seed), "{:?}\n{:?}", tm.structure_seeds(), host.lines.lock().unwrap());
    assert_eq!(*host.world_seed.lock().unwrap(), Some(world), "{:?}", host.lines.lock().unwrap());
}

#[test]
fn the_pillars_of_the_end_name_the_pillar_seed() {
    let structure_seed = 0x0000_1234_5678_9abc_i64;
    let host = Arc::new(TestHost::default());
    let storage = DataStorage::new(host.clone());
    let heights = pillar_seed::get_pillar_heights(pillar_seed::from_structure_seed(structure_seed)).to_vec();
    storage.add_pillar_data(PillarData::new(heights), poke(Phase::Pillars));
    storage.tick().unwrap().join().unwrap();
    let pillars = storage.get_time_machine().pillar_seeds().unwrap();
    assert!(pillars.contains(&(pillar_seed::from_structure_seed(structure_seed) as i32)), "{pillars:?}");
}

/// The whole End route: five end cities and the pillars name one structure seed among 2^32 candidates. It scans them all,
/// so it takes a minute: `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn end_cities_and_pillars_give_the_structure_seed() {
    let structure_seed = 0x0000_77aa_0f31_c4d9_i64;
    let host = Arc::new(TestHost::default());
    let storage = DataStorage::new(host.clone());
    storage.get_time_machine().set_threads(8);
    let heights = pillar_seed::get_pillar_heights(pillar_seed::from_structure_seed(structure_seed)).to_vec();
    storage.add_pillar_data(PillarData::new(heights), poke(Phase::Pillars));
    let cities = structures_of(structure_seed, &[StructureClass::EndCity], &[(0, 0), (0, 1), (1, 0), (1, 1), (-1, 0), (0, -1), (-1, -1)]);
    for d in cities {
        storage.add_base_data(d, poke(Phase::Structures));
    }
    storage.tick().unwrap().join().unwrap();
    let seeds = storage.get_time_machine().structure_seeds();
    assert!(seeds.contains(&structure_seed), "{seeds:?}\n{:?}", host.lines.lock().unwrap());
}
