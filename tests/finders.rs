// The finders against a made-up world: a structure's blocks are put in the world where the structure would be, a chunk is
// handed to the finder, and the data it gives the storage must be the structure's.
use horizon_seedcrackerx::scx_biome::biomes;
use horizon_seedcrackerx::scx_config::Config;
use horizon_seedcrackerx::scx_cracker_data::{BiomeSourceFactory, Host};
use horizon_seedcrackerx::scx_cracker_storage::DataStorage;
use horizon_seedcrackerx::scx_dimension::Dimension;
use horizon_seedcrackerx::scx_feature::{Detail, Feature};
use horizon_seedcrackerx::scx_features::Features;
use horizon_seedcrackerx::scx_finder::{Finder, FinderContext, SearchPositions};
use horizon_seedcrackerx::scx_finder_piece::{PieceFinder, PieceKind};
use horizon_seedcrackerx::scx_finder_structures as dsl;
use horizon_seedcrackerx::scx_finders::{BuriedTreasureFinder, DesertWellFinder, EndGatewayFinder, IglooFinder, ShipwreckFinder, TempleFinder, TrialChambersFinder};
use horizon_seedcrackerx::scx_level::{BlockPos, ChunkPos, Direction, Level};
use horizon_seedcrackerx::scx_version::MCVersion;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct NoHost;

impl Host for NoHost {
    fn debug(&self, _: &str) {}
    fn warn(&self, _: &str, _: &[String]) {}
    fn error(&self, _: &str) {}
    fn print_seed(&self, _: &str, _: i64) {}
    fn translate(&self, key: &str) -> String {
        key.to_owned()
    }
    fn version(&self) -> MCVersion {
        MCVersion::latest()
    }
    fn debug_enabled(&self) -> bool {
        false
    }
    fn world_seed_found(&self, _: i64) {}
    fn biome_source_factory(&self) -> Option<BiomeSourceFactory> {
        None
    }
}

/// A world of the blocks put in it, air everywhere else, the same biome everywhere.
struct MockLevel {
    dimension: Dimension,
    biome: &'static str,
    blocks: Mutex<HashMap<(i32, i32, i32), &'static str>>,
    properties: Mutex<HashMap<(i32, i32, i32, &'static str), &'static str>>,
}

impl MockLevel {
    fn new(dimension: Dimension, biome: &'static str) -> MockLevel {
        MockLevel { dimension, biome, blocks: Mutex::default(), properties: Mutex::default() }
    }

    fn set(&self, x: i32, y: i32, z: i32, block: &'static str) {
        self.blocks.lock().unwrap().insert((x, y, z), block);
    }

    fn set_property(&self, x: i32, y: i32, z: i32, name: &'static str, value: &'static str) {
        self.properties.lock().unwrap().insert((x, y, z, name), value);
    }
}

impl Level for MockLevel {
    fn dimension(&self) -> Dimension {
        self.dimension
    }
    fn block(&self, x: i32, y: i32, z: i32) -> &'static str {
        self.blocks.lock().unwrap().get(&(x, y, z)).copied().unwrap_or("air")
    }
    fn property(&self, x: i32, y: i32, z: i32, name: &str) -> Option<&'static str> {
        self.properties.lock().unwrap().iter().find(|((px, py, pz, n), _)| (*px, *py, *pz) == (x, y, z) && *n == name).map(|(_, v)| *v)
    }
    fn has_block_entity(&self, x: i32, y: i32, z: i32, _kind: &str) -> bool {
        matches!(self.block(x, y, z), "chest" | "spawner")
    }
    fn block_in_tag(&self, _: i32, _: i32, _: i32, _: &str) -> bool {
        false
    }
    fn noise_biome(&self, _: i32, _: i32, _: i32) -> &'static str {
        self.biome
    }
    fn biome(&self, _: i32, _: i32, _: i32) -> &'static str {
        self.biome
    }
    fn world_surface_height(&self, _: i32, _: i32) -> i32 {
        64
    }
    fn is_chunk_loaded(&self, _: i32, _: i32) -> bool {
        true
    }
}

fn context() -> FinderContext {
    let config = Config::default();
    let features = Features::init(MCVersion::latest(), &config, &|_| {});
    FinderContext {
        features: Arc::new(features),
        search: Arc::new(SearchPositions::reload_height(-64, 320)),
        storage: DataStorage::new(Arc::new(NoHost)),
        anti_xray_bypass: true,
    }
}

fn stored(ctx: &FinderContext) -> Vec<(i32, i32, &'static str)> {
    ctx.storage.base_seed_data.lock().unwrap().dump();
    ctx.storage.base_data().iter().map(|d| (d.chunk_x, d.chunk_z, d.feature.name())).collect()
}

/// Puts a piece's blocks in the world with its centre at `center`.
fn place(level: &MockLevel, piece: &PieceFinder, center: BlockPos) {
    for (key, block) in piece.blocks() {
        let p = center.offset_pos(*key);
        level.set(p.x, p.y, p.z, block);
    }
}

fn piece_of(kind: PieceKind, d: Direction, size: [i32; 3], build: fn(&mut PieceFinder)) -> PieceFinder {
    let mut p = PieceFinder::new(kind, Some(d), size);
    build(&mut p);
    p
}

#[test]
fn a_buried_treasure_is_a_chest_in_stone_at_the_middle_of_a_beach_chunk() {
    let ctx = context();
    let level = MockLevel::new(Dimension::Overworld, "beach");
    let chunk = ChunkPos::new(3, -2);
    level.set(chunk.x * 16 + 9, 40, chunk.z * 16 + 9, "chest");
    level.set(chunk.x * 16 + 9, 39, chunk.z * 16 + 9, "sandstone");
    let mut finders = BuriedTreasureFinder::create(&ctx, &level, chunk);
    let found = finders[0].find_in_chunk(&ctx, &level);
    assert_eq!(found, vec![BlockPos::new(chunk.x * 16 + 9, 40, chunk.z * 16 + 9)]);
    assert_eq!(stored(&ctx), vec![(3, -2, "buried_treasure")]);
    assert!(!finders[0].is_useless());

    // A chest in the water, on a block that is not stone, or in a chunk of another biome, is not one.
    let ctx = context();
    level.set_property(chunk.x * 16 + 9, 40, chunk.z * 16 + 9, "waterlogged", "true");
    assert!(BuriedTreasureFinder::create(&ctx, &level, chunk)[0].find_in_chunk(&ctx, &level).is_empty());
    let desert = MockLevel::new(Dimension::Overworld, "desert");
    desert.set(chunk.x * 16 + 9, 40, chunk.z * 16 + 9, "chest");
    desert.set(chunk.x * 16 + 9, 39, chunk.z * 16 + 9, "sandstone");
    assert!(BuriedTreasureFinder::create(&ctx, &desert, chunk)[0].find_in_chunk(&ctx, &desert).is_empty());
    assert!(stored(&ctx).is_empty());
}

#[test]
fn a_desert_well_is_found_from_the_chunk_beside_it() {
    let ctx = context();
    // Config::default has the desert well off, but a finder asked by hand does not look at that.
    let level = MockLevel::new(Dimension::Overworld, "desert");
    let mut well = PieceFinder::new(PieceKind::Piece, Some(Direction::North), [5, 6, 5]);
    well.fill_with_outline(0, 0, 0, 4, 1, 4, "sandstone", "sandstone", false);
    well.fill_with_outline(1, 5, 1, 3, 5, 3, "sandstone_slab", "sandstone_slab", false);
    well.add_block("sandstone", 2, 5, 2);
    well.add_block("water", 2, 1, 2);
    for d in Direction::HORIZONTAL {
        let p = BlockPos::new(2, 1, 2).relative(d, 1);
        well.add_block("water", p.x, p.y, p.z);
    }
    let corner = BlockPos::new(5 * 16 + 14, 63, -3 * 16 + 4);
    place(&level, &well, corner);
    // Seen from the chunk at (5, -3) the corner is in it; the finder of that chunk finds it at the chunk's local position.
    let ctx_features = ctx.features.desert_well.is_some();
    assert!(ctx_features);
    let finders = DesertWellFinder::create(&ctx, &level, ChunkPos::new(5, -3));
    assert_eq!(finders.len(), 9);
    let mut found = 0;
    for mut f in finders {
        found += f.find_in_chunk(&ctx, &level).len();
    }
    assert!(found >= 1);
    let data = stored(&ctx);
    assert_eq!(data.len(), 1, "{data:?}");
    assert_eq!(data[0].2, "desert_well");
}

#[test]
fn an_end_gateway_is_read_with_its_height() {
    let ctx = context();
    let level = MockLevel::new(Dimension::End, "end_highlands");
    let chunk = ChunkPos::new(10, 4);
    let (x, z) = (chunk.x * 16 + 6, chunk.z * 16 + 3);
    level.set(x, 75, z, "end_gateway");
    // End stone four blocks below the gateway, bedrock between.
    level.set(x, 71, z, "end_stone");
    for y in 72..75 {
        level.set(x, y, z, "bedrock");
    }
    let mut f = EndGatewayFinder::create(&ctx, &level, chunk);
    assert_eq!(f[0].find_in_chunk(&ctx, &level).len(), 1);
    ctx.storage.base_seed_data.lock().unwrap().dump();
    let base = ctx.storage.base_data();
    assert_eq!(base.len(), 1);
    let Detail::Decorator(horizon_seedcrackerx::scx_feature::DecoratorDetail::EndGateway { height, offset_x, offset_z, .. }) = &base[0].detail else { panic!() };
    // The Java counts the blocks down to the end stone, the end stone itself included, and takes one off: 3.
    assert_eq!((*height, *offset_x, *offset_z), (3, 6, 3));
    let _ = biomes::END_HIGHLANDS;
}

#[test]
fn every_temple_is_found_whichever_way_it_faces() {
    for (name, size, build, finder) in [
        ("desert_pyramid", [21, 15, 21], dsl::desert_pyramid as fn(&mut PieceFinder), TempleFinder::create_desert_pyramid as fn(&FinderContext, &dyn Level, ChunkPos) -> Vec<Box<dyn Finder>>),
        ("jungle_pyramid", [12, 10, 15], dsl::jungle_pyramid, TempleFinder::create_jungle_pyramid),
        ("swamp_hut", [7, 7, 9], dsl::swamp_hut, TempleFinder::create_swamp_hut),
    ] {
        let biome = match name {
            "desert_pyramid" => "desert",
            "jungle_pyramid" => "jungle",
            _ => "swamp",
        };
        for d in Direction::HORIZONTAL {
            let ctx = context();
            let level = MockLevel::new(Dimension::Overworld, biome);
            let chunk = ChunkPos::new(-7, 12);
            let center = BlockPos::new(chunk.x * 16, 70, chunk.z * 16);
            place(&level, &piece_of(PieceKind::Piece, d, size, build), center);
            let mut found = Vec::new();
            for mut f in finder(&ctx, &level, chunk) {
                found.extend(f.find_in_chunk(&ctx, &level));
            }
            assert!(found.contains(&center), "{name} facing {d:?}: {found:?}");
            let data = stored(&ctx);
            assert!(data.contains(&(-7, 12, name)), "{name} facing {d:?}: {data:?}");
        }
    }
}

#[test]
fn an_igloo_and_a_trial_chamber_are_found_in_each_orientation() {
    for d in Direction::HORIZONTAL {
        let ctx = context();
        let level = MockLevel::new(Dimension::Overworld, "snowy_plains");
        let chunk = ChunkPos::new(4, 4);
        let piece = piece_of(PieceKind::Jigsaw, d, [7, 5, 8], dsl::igloo);
        // A jigsaw piece is found where its search position puts it: a chunk corner moved by the rotation point.
        let search = ctx.search.igloo[&d].first().copied().unwrap();
        let center = BlockPos::new(chunk.x * 16 + search.x, 60, chunk.z * 16 + search.z);
        place(&level, &piece, center);
        let mut found = Vec::new();
        for mut f in IglooFinder::create(&ctx, &level, chunk) {
            found.extend(f.find_in_chunk(&ctx, &level));
        }
        assert!(found.contains(&center), "igloo {d:?}: {found:?}");
        assert!(stored(&ctx).contains(&(4, 4, "igloo")));
    }
    let ctx = context();
    let level = MockLevel::new(Dimension::Overworld, "plains");
    let chunk = ChunkPos::new(-3, -3);
    let piece = piece_of(PieceKind::Jigsaw, Direction::North, [19, 20, 19], dsl::trial_chambers_end1);
    place(&level, &piece, BlockPos::new(chunk.x * 16, 0, chunk.z * 16));
    let mut found = Vec::new();
    for mut f in TrialChambersFinder::create(&ctx, &level, chunk) {
        found.extend(f.find_in_chunk(&ctx, &level));
    }
    assert!(!found.is_empty());
    assert!(stored(&ctx).contains(&(-3, -3, "trial_chambers")));
}

#[test]
fn a_shipwreck_is_not_a_chest_in_the_open() {
    let ctx = context();
    let level = MockLevel::new(Dimension::Overworld, "ocean");
    let chunk = ChunkPos::new(1, 1);
    level.set(20, 50, 20, "chest");
    level.set_property(20, 50, 20, "type", "single");
    level.set_property(20, 50, 20, "facing", "north");
    let mut found = 0;
    for mut f in ShipwreckFinder::create(&ctx, &level, chunk) {
        found += f.find_in_chunk(&ctx, &level).len();
    }
    // A chest with no wreck around it: no stairs, no trapdoors, so no known layout and nothing stored.
    assert_eq!(found, 0);
    assert!(stored(&ctx).is_empty());
    let _ = Detail::Region;
    let _ = Feature::Structure;
}

#[test]
fn messages_are_the_language_files() {
    use horizon_seedcrackerx::scx_lang::{format, translate};
    assert_eq!(translate("en_us", "tmachine.lookingForPillarSeed"), "Looking for pillar seeds...");
    // A key a language lacks falls back to English, and a key nobody has is itself.
    assert_eq!(translate("zh_cn", "no.such.key"), "no.such.key");
    assert_ne!(translate("ru_ru", "settings.active"), "");
    assert_eq!(format(&translate("en_us", "data.collectedBits"), &["12".into(), "32".into()]), "You currently have collected 12 bits out of 32.");
}

#[test]
fn structures_are_saved_and_restored_by_name() {
    use horizon_seedcrackerx::scx_feature::FeatureData;
    use horizon_seedcrackerx::scx_structure_save::{file_name, from_text, to_text};
    let features = Features::init(MCVersion::latest(), &Config::default(), &|_| {});
    let igloo = features.igloo.clone().unwrap();
    let wreck = features.shipwreck.clone().unwrap();
    let data = vec![FeatureData::structure(&igloo, 12, -7), FeatureData::structure(&wreck, -300, 41)];
    let text = to_text(&data);
    assert_eq!(text, "igloo;12;-7\nshipwreck;-300;41\n");
    let back = from_text(&format!("{text}nonsense\nvillage;1;2;3\nigloo;x;2\nwhale;5;5\n"), &features.structure_types);
    assert_eq!(back.len(), 2);
    assert_eq!((back[0].chunk_x, back[0].chunk_z, back[0].feature.name()), (12, -7, "igloo"));
    assert_eq!(file_name("/127.0.0.1:25565"), "_127.0.0.1_25565.txt");
}

#[test]
fn the_database_sheet_is_read_by_address_then_by_hash() {
    use horizon_seedcrackerx::scx_database::{post_body, Database};
    let mut db = Database::default();
    db.parse_csv("ip,dim,seed,version,user,hash,hashed\nplay.example.net/1.2.3.4:25565,overworld,123456789L,1.21,x,0,-777\nshort,row\nbad,overworld,xL,1.21,x,0,5\nother,overworld,-42L,1.21,x,0,31337\n");
    assert_eq!(db.len(), 2);
    assert_eq!(db.get_seed("play.example.net/1.2.3.4:25565", 0), Some(123456789));
    assert_eq!(db.get_seed("elsewhere", 31337), Some(-42));
    assert_eq!(db.get_seed("elsewhere", 1), None);
    assert_eq!(post_body("a b/1:2", "overworld", -5, "1.21.3", "Steve", true), "serverIp=a+b%2F1%3A2&dimension=overworld&seed=-5L&version=1.21.3&username=Steve&hash=1");
}

/// A development helper: the blocks of a pyramid the game generated (HORIZON_PYRAMID_BLOCKS, from horizon-ctl blocks) against
/// what the finder expects for each facing at the given origin.
#[test]
#[ignore]
fn pyramid_of_the_game_against_the_finder() {
    let Ok(path) = std::env::var("HORIZON_PYRAMID_BLOCKS") else { return };
    let text = std::fs::read_to_string(path).unwrap();
    let mut world: HashMap<(i32, i32, i32), String> = HashMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() == 4 {
            world.insert((f[0].parse().unwrap(), f[1].parse().unwrap(), f[2].parse().unwrap()), f[3].to_owned());
        }
    }
    let origin = (-2464, 70, 1664);
    for d in Direction::HORIZONTAL {
        let piece = piece_of(PieceKind::Piece, d, [21, 15, 21], dsl::desert_pyramid);
        let mut bad = 0;
        let mut shown = Vec::new();
        for (key, block) in piece.blocks() {
            let p = (origin.0 + key.x, origin.1 + key.y, origin.2 + key.z);
            let actual = world.get(&p).map(String::as_str).unwrap_or("air");
            if actual != *block {
                bad += 1;
                if shown.len() < 12 {
                    shown.push(format!("{key:?} expects {block} has {actual}"));
                }
            }
        }
        println!("{d:?}: {} blocks expected, {bad} differ {shown:#?}", piece.blocks().len());
    }
}
