// The port against the Java library itself. `tests/data/java_fixture.txt` is what the SeedFinding libraries (mc_core, mc_seed,
// mc_feature at the commits SeedCrackerX 2.16.2 pins) answered for the same questions, written by tools/Gen.java run on a
// JDK; every line is asked of the Rust port and must come out the same.
use horizon_seedcrackerx::scx_feature::{Decorator, DecoratorClass, Feature, FeatureData, Structure, StructureClass};
use horizon_seedcrackerx::scx_rand::{ChunkRand, JRand};
use horizon_seedcrackerx::scx_seeds::{pillar_seed, structure_seed, world_seed};
use horizon_seedcrackerx::scx_version::MCVersion;
use std::sync::Arc;

const FIXTURE: &str = include_str!("data/java_fixture.txt");

fn class_of(name: &str) -> StructureClass {
    use StructureClass::*;
    match name {
        "buried_treasure" => BuriedTreasure,
        "desert_pyramid" => DesertPyramid,
        "end_city" => EndCity,
        "igloo" => Igloo,
        "jungle_pyramid" => JunglePyramid,
        "monument" => Monument,
        "pillager_outpost" => PillagerOutpost,
        "shipwreck" => Shipwreck,
        "swamp_hut" => SwampHut,
        "village" => Village,
        "trial_chambers" => TrialChambers,
        other => panic!("{other}"),
    }
}

fn version(name: &str) -> MCVersion {
    MCVersion::from_string(name).unwrap_or_else(|| panic!("{name}"))
}

fn list(text: &str) -> Vec<i64> {
    let inner = text.trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() { Vec::new() } else { inner.split(", ").map(|s| s.parse().unwrap()).collect() }
}

#[test]
fn every_line_of_the_java_fixture_is_answered_the_same() {
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    let mut features: std::collections::HashMap<(String, MCVersion), Option<Arc<Feature>>> = std::collections::HashMap::new();
    let mut rand = ChunkRand::new();
    for line in FIXTURE.lines() {
        let parts: Vec<&str> = line.split(' ').collect();
        let kind = parts[0];
        *counts.entry(kind).or_default() += 1;
        match kind {
            "CFG" => {
                let (name, v) = (parts[1], version(parts[2]));
                let s = Structure::new(class_of(name), v);
                if parts[3] == "none" {
                    // Buried treasure is the one the Java builds with no config before 1.13; a port that has none is the same.
                    assert!(s.is_none(), "{line}");
                } else {
                    let s = s.as_ref().unwrap_or_else(|| panic!("{line}"));
                    assert_eq!((s.spacing(), s.separation(), s.salt()), (parts[3].parse().unwrap(), parts[4].parse().unwrap(), parts[5].parse().unwrap()), "{line}");
                }
                features.insert((name.to_owned(), v), s.map(|s| Arc::new(Feature::Structure(s))));
            }
            "REG" => {
                let (name, v) = (parts[1], version(parts[2]));
                let feature = features[&(name.to_owned(), v)].as_ref().unwrap();
                let Feature::Structure(s) = &**feature else { unreachable!() };
                let got = s.get_in_region(parts[3].parse().unwrap(), parts[4].parse().unwrap(), parts[5].parse().unwrap(), &mut rand);
                let got = got.map_or("none".to_owned(), |p| format!("{},{}", p.x, p.z));
                assert_eq!(got, parts[6], "{line}");
            }
            "START" => {
                let (name, v) = (parts[1], version(parts[2]));
                let feature = features[&(name.to_owned(), v)].as_ref().unwrap();
                let seed: i64 = parts[3].parse().unwrap();
                let data = FeatureData::structure(feature, parts[4].parse().unwrap(), parts[5].parse().unwrap());
                assert_eq!(data.test_start(seed, &mut rand).to_string(), parts[6], "{line}");
                assert_eq!(data.test_start(seed ^ 0x10001, &mut rand).to_string(), parts[7], "{line}");
            }
            "WELL" | "GATE" => {
                if parts[5] == "none" {
                    continue;
                }
                let v = version(parts[1]);
                let class = if kind == "WELL" { DecoratorClass::DesertWell } else { DecoratorClass::EndGateway };
                let feature = Arc::new(Feature::Decorator(Decorator::new(class, v).unwrap()));
                let seed: i64 = parts[2].parse().unwrap();
                let (bx, bz): (i32, i32) = (parts[5].parse().unwrap(), parts[6].parse().unwrap());
                if kind == "WELL" {
                    assert_eq!(FeatureData::desert_well(&feature, bx, bz).test_start(seed, &mut rand).to_string(), parts[7], "{line}");
                    assert_eq!(FeatureData::desert_well(&feature, bx + 1, bz).test_start(seed, &mut rand).to_string(), parts[8], "{line}");
                } else {
                    let height: i32 = parts[7].parse().unwrap();
                    assert_eq!(FeatureData::end_gateway(&feature, bx, bz, height).test_start(seed, &mut rand).to_string(), parts[8], "{line}");
                    assert_eq!(FeatureData::end_gateway(&feature, bx, bz, height + 1).test_start(seed, &mut rand).to_string(), parts[9], "{line}");
                }
            }
            "SEEDING" => {
                let v = version(parts[1]);
                let w: i64 = parts[2].parse().unwrap();
                let (cx, cz): (i32, i32) = (parts[3].parse().unwrap(), parts[4].parse().unwrap());
                let mut cr = ChunkRand::new();
                let mut got: Vec<String> = Vec::new();
                let mut push = |seed: i64, cr: &mut ChunkRand| got.push(format!("{seed}/{}", cr.next_int()));
                let s = cr.set_terrain_seed(cx, cz, v);
                push(s, &mut cr);
                let s = cr.set_population_seed(w, cx << 4, cz << 4, v);
                push(s, &mut cr);
                if v.is_newer_or_equal_to(MCVersion::v1_13) {
                    let s = cr.set_decorator_seed_at_step(w, cx << 4, cz << 4, 3, 2, v);
                    push(s, &mut cr);
                    let s = cr.set_decorator_seed_at_salt(w, cx << 4, cz << 4, 20003, v);
                    push(s, &mut cr);
                }
                let s = cr.set_carver_seed(w, cx, cz, v);
                push(s, &mut cr);
                let s = cr.set_region_seed(w, cx, cz, 14357617, v);
                push(s, &mut cr);
                let s = cr.set_weak_seed(w, cx, cz, v);
                push(s, &mut cr);
                let s = cr.set_slime_seed(w, cx, cz, 987234911, v);
                push(s, &mut cr);
                let s = cr.set_position_seed(cx.wrapping_mul(7), 64, cz.wrapping_mul(3), v);
                push(s, &mut cr);
                let s = cr.set_base_stone_seed(w, cx, 5, cz);
                push(s, &mut cr);
                assert_eq!(got.join(" "), parts[5..].join(" "), "{line}");
            }
            "SEED" => {
                let s: i64 = parts[1].parse().unwrap();
                let structure = s & 0xFFFF_FFFF_FFFF;
                let field = |key: &str| parts.iter().find_map(|p| p.strip_prefix(key)).unwrap_or_else(|| panic!("{key} in {line}"));
                assert_eq!(pillar_seed::from_structure_seed(structure).to_string(), field("pillar="), "{line}");
                assert_eq!(world_seed::to_hash(s).to_string(), field("hash="), "{line}");
                assert_eq!(world_seed::get_shadow_seed(s).to_string(), field("shadow="), "{line}");
                assert_eq!(world_seed::is_random(s).to_string(), field("random="), "{line}");
                assert_eq!(world_seed::is_string(s).to_string(), field("string="), "{line}");
                // randomWorlds=[a, b] and heights=[...] hold spaces, so take them from the whole line.
                let rw = line.split("randomWorlds=").nth(1).unwrap().split(" heights=").next().unwrap();
                assert_eq!(structure_seed::to_random_world_seeds(structure), list(rw), "{line}");
                let heights = line.split("heights=").nth(1).unwrap();
                let got = pillar_seed::get_pillar_heights(structure & 0xFFFF);
                assert_eq!(format!("{got:?}"), heights, "{line}");
            }
            "RAND" => {
                let w: i64 = parts[1].parse().unwrap();
                assert_eq!(world_seed::is_random(w).to_string(), parts[2], "{line}");
                let rest = line.splitn(4, ' ').nth(3).unwrap();
                let (worlds, hash) = rest.rsplit_once(' ').unwrap();
                assert_eq!(structure_seed::to_random_world_seeds(w & 0xFFFF_FFFF_FFFF), list(worlds), "{line}");
                assert_eq!(world_seed::to_hash(w).to_string(), hash, "{line}");
            }
            "NEXTLONG" => {
                let internal: i64 = parts[1].parse().unwrap();
                let w: i64 = parts[2].parse().unwrap();
                assert_eq!(JRand::of_internal_seed(internal).next_long(), w, "{line}");
                let rest = line.splitn(4, ' ').nth(3).unwrap();
                let (worlds, random) = rest.rsplit_once(' ').unwrap();
                assert_eq!(structure_seed::to_random_world_seeds(w & 0xFFFF_FFFF_FFFF), list(worlds), "{line}");
                assert_eq!(world_seed::is_random(w).to_string(), random, "{line}");
            }
            "JRAND" => {
                let s: i64 = parts[1].parse().unwrap();
                let mut j = JRand::new(s);
                let got = format!(
                    "{} {} {} {} {} {} {} {} {}",
                    j.next_int(),
                    j.next_int_bound(24),
                    j.next_long(),
                    j.next_double(),
                    j.next_float(),
                    j.next_boolean(),
                    j.next_int_bound(16),
                    j.next_int_bound(7),
                    j.next_gaussian()
                );
                let expected = parts[2..].join(" ");
                // Java prints floats and doubles with its own shortest form; compare them as numbers.
                let (g, e): (Vec<&str>, Vec<&str>) = (got.split(' ').collect(), expected.split(' ').collect());
                for (g, e) in g.iter().zip(&e) {
                    match (g.parse::<f64>(), e.parse::<f64>()) {
                        (Ok(a), Ok(b)) if g.contains('.') || e.contains('.') => assert!((a - b).abs() <= 1e-12 * b.abs().max(1.0), "{line}: {g} vs {e}"),
                        _ => assert_eq!(g, e, "{line}"),
                    }
                }
            }
            other => panic!("unknown fixture line {other}"),
        }
    }
    // Every kind was in the file, and asked.
    for kind in ["CFG", "REG", "START", "WELL", "GATE", "SEEDING", "SEED", "RAND", "NEXTLONG", "JRAND"] {
        assert!(counts.get(kind).copied().unwrap_or(0) > 0, "no {kind} lines");
    }
}
