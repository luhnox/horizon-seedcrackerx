use crate::scx_feature::{DecoratorClass, Feature, FeatureData, Structure, StructureClass};
use crate::scx_lcg::LCG;
use crate::scx_rand::{ChunkRand, JRand};
use crate::scx_seeds::{pillar_seed, structure_seed, world_seed};
use crate::scx_sha256;
use crate::scx_version::{MCVersion, VersionMap};
use crate::scx_worldgen_random::WorldgenRandom;
use std::sync::Arc;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn sha256_matches_the_published_vectors() {
    assert_eq!(hex(&scx_sha256::digest(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(hex(&scx_sha256::digest(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(
        hex(&scx_sha256::digest(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn jrand_is_java_util_random() {
    // The values java.util.Random gives for these seeds.
    assert_eq!(JRand::new(42).next_int(), -1170105035);
    assert_eq!(JRand::new(42).next_long(), -5025562857975149833);
    assert_eq!(JRand::new(42).next_double(), 0.7275636800328681);
    assert_eq!(JRand::new(0).next_int_bound(100), 60);
    assert_eq!(JRand::new(0).next_gaussian(), 0.8025330637390305);
}

#[test]
fn the_static_forms_agree_with_the_generator() {
    let mut r = JRand::new(12345);
    let seed = r.get_seed();
    assert_eq!(JRand::next_int_at(LCG::JAVA.next_seed(seed)), r.next_int());
    let mut r = JRand::new(12345);
    assert_eq!(JRand::next_long_at(LCG::JAVA.next_seed(seed)), r.next_long());
    let mut r = JRand::new(12345);
    assert_eq!(JRand::next_double_at(LCG::JAVA.next_seed(seed)), r.next_double());
    // Not the powers of two: the Java's static form shifts the whole seed there where the generator shifts its 31 bits.
    for bound in [3, 24, 100, 1_000_003] {
        let mut r = JRand::new(999);
        let s = r.get_seed();
        assert_eq!(JRand::next_int_bound_at(LCG::JAVA.next_seed(s), bound), r.next_int_bound(bound));
    }
}

#[test]
fn an_lcg_steps_forward_and_back() {
    let seed = 0x1234_5678_9abc;
    let two = LCG::JAVA.combine_steps(2);
    let back = LCG::JAVA.combine_steps(-2);
    assert_eq!(two.next_seed(seed), LCG::JAVA.next_seed(LCG::JAVA.next_seed(seed)));
    assert_eq!(back.next_seed(two.next_seed(seed)), seed);
    assert_eq!(LCG::JAVA.invert().next_seed(LCG::JAVA.next_seed(seed)), seed);
}

#[test]
fn versions_compare_as_the_java_enum() {
    assert_eq!(MCVersion::latest(), MCVersion::v1_21_3);
    assert!(MCVersion::v1_21.is_older_than(MCVersion::v1_21_1));
    assert!(MCVersion::v1_18.is_newer_than(MCVersion::v1_17_1));
    assert!(MCVersion::v1_12_2.is_older_than(MCVersion::v1_13));
    assert!(MCVersion::v1_16_2.is_between(MCVersion::v1_16, MCVersion::v1_17));
    assert!(!MCVersion::v1_18.is_between(MCVersion::v1_16, MCVersion::v1_17));
    assert_eq!(MCVersion::from_string("1.18.2"), Some(MCVersion::v1_18_2));
    assert_eq!(MCVersion::v1_18.newer(), Some(MCVersion::v1_18_1));
    assert_eq!(MCVersion::v1_18.older(), Some(MCVersion::v1_17_1));
    assert_eq!(MCVersion::values().len(), 141);

    let map = VersionMap::new().add(MCVersion::v1_14, "a").add(MCVersion::v1_18, "b").add(MCVersion::v1_13, "c");
    assert_eq!(map.get_as_of(MCVersion::v1_16), Some(&"a"));
    assert_eq!(map.get_as_of(MCVersion::v1_21), Some(&"b"));
    assert_eq!(map.get_as_of(MCVersion::v1_12_2), None);
    assert_eq!(map.get_oldest_version(), Some(MCVersion::v1_13));
}

#[test]
fn chunk_rand_seedings_match_the_formulas() {
    let mut r = ChunkRand::new();
    // setRegionSeed: regionX * 341873128712 + regionZ * 132897987541 + worldSeed + salt, scrambled.
    let seed = r.set_region_seed(100, 2, 3, 14357617, MCVersion::latest());
    let expected = 2i64 * 341873128712 + 3 * 132897987541 + 100 + 14357617;
    assert_eq!(seed, expected & 0xFFFF_FFFF_FFFF);
    assert_eq!(r.get_seed(), expected ^ 0x5DEECE66D);
    // The population seed: the same chunk, the same world, the same answer; another chunk, another.
    let a = r.set_population_seed(42, 16, 32, MCVersion::latest());
    let b = r.set_population_seed(42, 16, 32, MCVersion::latest());
    let c = r.set_population_seed(42, 32, 16, MCVersion::latest());
    assert_eq!(a, b);
    assert_ne!(a, c);
    // Before 1.13 the two multipliers are made odd by another rule.
    let old = r.set_population_seed(42, 16, 32, MCVersion::v1_12_2);
    assert_ne!(old, a);
}

#[test]
fn pillar_seeds_are_what_the_end_makes_of_a_structure_seed() {
    for pillar in [0i64, 1, 12345, 65535] {
        let heights = pillar_seed::get_pillar_heights(pillar);
        let mut sorted = heights;
        sorted.sort();
        assert_eq!(sorted, [76, 79, 82, 85, 88, 91, 94, 97, 100, 103]);
    }
    // The structure seeds of a pillar seed lead back to it.
    let pillar = 4242;
    let mut it = pillar_seed::get_structure_seeds(pillar);
    for _ in 0..1000 {
        let s = it.next();
        assert_eq!(pillar_seed::from_structure_seed(s), pillar, "structure seed {s}");
    }
    // The heights name the pillar seed, as the End draws them with the structure seed's own random.
    let found = pillar_seed::from_pillar_heights(&pillar_seed::get_pillar_heights(pillar));
    assert!(found.contains(&pillar));
}

#[test]
fn world_seeds_of_a_structure_seed_and_the_hash_that_names_them() {
    let structure = 0x0123_4567_89ab_i64;
    let world = structure_seed::to_world_seed(structure, 0xBEEF);
    assert_eq!(world_seed::to_structure_seed(world), structure);
    let hash = world_seed::to_hash(world);
    assert_eq!(world_seed::from_hash(structure, hash), vec![world]);
    // The shadow seed is the other solution of the mixer.
    assert_ne!(world_seed::get_shadow_seed(world), world);
    assert_eq!(world_seed::get_shadow_seed(world_seed::get_shadow_seed(world)), world);
}

#[test]
fn a_random_world_seed_is_found_from_its_structure_seed() {
    for internal in [1i64, 777, 123456789, 0xABCD_EF12_3456 & 0xFFFF_FFFF_FFFF] {
        let world = JRand::of_internal_seed(internal).next_long();
        let structure = world_seed::to_structure_seed(world);
        let found = structure_seed::to_random_world_seeds(structure);
        assert!(found.contains(&world), "{world} not in {found:?}");
        assert!(world_seed::is_random(world));
    }
}

#[test]
fn worldgen_random_is_the_xoroshiro_of_the_game() {
    // The values Horizon's own XoroshiroRandom (checked against the game's worldgen) gives for these seeds.
    let cases: [(i64, i64, i64, [i32; 6], [i32; 4]); 5] = [
        (0, 3038984756725240190, -3694039286755638414, [15, 2, 2, 4, 14, 15], [308, 58, 48, 99]),
        (1, -1033667707219518978, 6451672561743293322, [6, 3, 3, 10, 2, 8], [129, 62, 61, 213]),
        (12345, -8118485274630516485, 8241557746459281790, [0, 13, 14, 0, 8, 11], [4, 261, 280, 5]),
        (-987654321, -8813114921905719051, -8071674531228274167, [2, 8, 5, 0, 2, 8], [43, 169, 111, 17]),
        (0x1234_5678_9abc_def0, 8158243457423347947, 5232595719026988595, [4, 3, 1, 4, 15, 6], [88, 77, 26, 86]),
    ];
    for (seed, a, b, small, big) in cases {
        let mut r = WorldgenRandom::new(seed);
        assert_eq!((r.next_long(), r.next_long()), (a, b), "seed {seed}");
        let mut r = WorldgenRandom::new(seed);
        assert_eq!([(); 6].map(|_| r.next_int_bound(16)), small, "seed {seed}");
        let mut r = WorldgenRandom::new(seed);
        assert_eq!([(); 4].map(|_| r.next_int_bound(320)), big, "seed {seed}");
    }
}

fn structure(class: StructureClass) -> Arc<Feature> {
    Arc::new(Feature::Structure(Structure::new(class, MCVersion::latest()).expect("a config for the latest version")))
}

/// Where a structure is for a structure seed, then the data at that place must test true for it, and not for most others.
#[test]
fn a_structure_is_started_where_the_seed_puts_it() {
    use StructureClass::*;
    let seed = 0x0000_9a3b_77c1_55d2_i64;
    for class in [Igloo, DesertPyramid, JunglePyramid, SwampHut, Shipwreck, Monument, TrialChambers, BuriedTreasure, Village] {
        let feature = structure(class);
        let Feature::Structure(s) = &*feature else { unreachable!() };
        let mut rand = ChunkRand::new();
        let mut checked = 0;
        for region_x in -3..3 {
            for region_z in -3..3 {
                let Some(pos) = s.get_in_region(seed, region_x, region_z, &mut rand) else { continue };
                let data = FeatureData::structure(&feature, pos.x, pos.z);
                assert!(data.test_start(seed, &mut rand), "{} at {pos:?}", s.name());
                let other = data.test_start(seed ^ 0x1_0000_0001, &mut rand);
                assert!(!other || class == BuriedTreasure, "{} also starts for another seed", s.name());
                checked += 1;
            }
        }
        assert!(checked > 0, "{}", s.name());
    }
}

#[test]
fn region_data_places_chunks_in_their_regions() {
    let feature = structure(StructureClass::Igloo);
    let Feature::Structure(s) = &*feature else { unreachable!() };
    // Spacing 32: chunk (-1, 0) is in region (-1, 0) at offset (31, 0); chunk (33, -33) in region (1, -2).
    let d = s.region_data(-1, 0);
    assert_eq!((d.region_x, d.region_z, d.offset_x, d.offset_z), (-1, 0, 31, 0));
    let d = s.region_data(33, -33);
    assert_eq!((d.region_x, d.region_z, d.offset_x, d.offset_z), (1, -2, 1, 31));
    assert_eq!(s.get_offset(), 24); // 32 - 8
}

#[test]
fn structure_configs_follow_the_version() {
    assert!(Structure::new(StructureClass::Igloo, MCVersion::v1_8).is_none());
    assert_eq!(Structure::new(StructureClass::Igloo, MCVersion::v1_12_2).unwrap().salt(), 14357617);
    assert_eq!(Structure::new(StructureClass::Igloo, MCVersion::v1_13).unwrap().salt(), 14357618);
    let wreck = Structure::new(StructureClass::Shipwreck, MCVersion::v1_16).unwrap();
    assert_eq!((wreck.spacing(), wreck.separation(), wreck.get_offset()), (24, 4, 20));
    assert!(Structure::new(StructureClass::TrialChambers, MCVersion::v1_20_6).is_none());
    assert!(Structure::new(StructureClass::PillagerOutpost, MCVersion::latest()).unwrap().village.is_some());
}

#[test]
fn decorators_exist_by_version() {
    use crate::scx_feature::Decorator;
    assert!(Decorator::new(DecoratorClass::Dungeon, MCVersion::v1_12_2).is_none());
    let d = Decorator::new(DecoratorClass::Dungeon, MCVersion::v1_16_5).unwrap();
    // 1.16: step 3, index 2, but 3 and 3 in the desert and the swamp.
    assert_eq!((d.get_step(None), d.get_index(None)), (3, 2));
    assert_eq!((d.get_step(Some(crate::scx_biome::biomes::DESERT)), d.get_index(Some(crate::scx_biome::biomes::DESERT))), (3, 3));
    let d = Decorator::new(DecoratorClass::Dungeon, MCVersion::latest()).unwrap();
    assert_eq!((d.get_step(None), d.get_index(None)), (3, 2));
}
