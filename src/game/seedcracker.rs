// SeedCrackerX's side of the game: the world as the finders read it (the chunks the game holds), the seed the server sent, the
// chat the cracker writes to, the /seedcracker commands, and the outlines of what was found. The cracker, the finders and the
// config are the pure modules; this one is the only one that touches the game, and what it reads of the game is read on the
// game's thread (once a tick) and handed to the other threads as values.
use super::scx_config::{Config, RenderType};
use super::scx_cracker_data::{BiomeSourceFactory, DataAddedEvent, HashedSeedData, Host};
use super::scx_cracker_storage::DataStorage;
use super::scx_cracker_time_machine::Phase;
use super::scx_dimension::Dimension;
use super::scx_features::Features;
use super::scx_finder::{Category, FinderContext, FinderQueue, FinderType, SearchPositions};
use super::scx_finders::BUILDERS;
use super::scx_lang;
use super::scx_level::{ChunkPos, Level};
use super::scx_seeds::world_seed;
use super::scx_version::MCVersion;
use super::Game;
use horizon_client::chunk::Chunk;
use horizon_client::gizmos::Line;
use horizon_client::text::{self, ClickEvent, Style, Text};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

pub const ID: &str = "seedcrackerx";
const CONFIG_FILE: &str = "seedcracker.json";

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// A name that lives as long as the game: block and biome names are few, and every finder compares them.
fn intern(s: &str) -> &'static str {
    static NAMES: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut names = lock(&NAMES);
    let set = names.get_or_insert_with(HashSet::new);
    if let Some(found) = set.get(s) {
        return found;
    }
    let leaked: &'static str = Box::leak(s.to_owned().into_boxed_str());
    set.insert(leaked);
    leaked
}

fn bare(name: &str) -> &str {
    name.strip_prefix("minecraft:").unwrap_or(name)
}

// The chat.

/// What the cracker says, from any thread; the game's thread writes it to the chat.
#[derive(Clone)]
enum Message {
    Debug(String),
    Warn(String, Vec<String>),
    Error(String),
    Seed(String, i64),
    Feedback(String, u32),
}

struct GameHost {
    config: Arc<Mutex<Config>>,
    queue: Mutex<Vec<Message>>,
    language: Mutex<String>,
    found: Mutex<Vec<i64>>,
}

impl GameHost {
    fn push(&self, m: Message) {
        lock(&self.queue).push(m);
    }
}

impl Host for GameHost {
    fn debug(&self, message: &str) {
        self.push(Message::Debug(message.to_owned()));
    }

    fn warn(&self, key: &str, args: &[String]) {
        self.push(Message::Warn(key.to_owned(), args.to_vec()));
    }

    fn error(&self, key: &str) {
        self.push(Message::Error(key.to_owned()));
    }

    fn print_seed(&self, key: &str, seed: i64) {
        self.push(Message::Seed(key.to_owned(), seed));
    }

    fn translate(&self, key: &str) -> String {
        scx_lang::translate(&lock(&self.language), key)
    }

    fn version(&self) -> MCVersion {
        lock(&self.config).get_version()
    }

    fn debug_enabled(&self) -> bool {
        lock(&self.config).debug
    }

    fn world_seed_found(&self, seed: i64) {
        lock(&self.found).push(seed);
    }

    fn biome_source_factory(&self) -> Option<BiomeSourceFactory> {
        None
    }
}

fn message_text(host: &GameHost, m: Message) -> Text {
    match m {
        Message::Debug(s) => Text::literal(s),
        Message::Warn(key, args) => Text::colored(scx_lang::format(&host.translate(&key), &args), text::GREEN),
        Message::Error(key) => Text::colored(host.translate(&key), text::RED),
        Message::Feedback(s, color) => Text::colored(s, color),
        Message::Seed(key, seed) => {
            let template = host.translate(&key);
            let (before, after) = match template.split_once("${SEED}") {
                Some((a, b)) => (a.to_owned(), Some(b.to_owned())),
                None => (template, None),
            };
            let number = seed.to_string();
            // ComponentUtils.wrapInSquareBrackets: the seed in green, to be copied with a click.
            let seed_text = Text::literal("[")
                .append(Text::styled(number.clone(), Style { click: Some(ClickEvent::CopyToClipboard(number)), hover: Some(Box::new(Text::literal("Click to copy to clipboard"))), ..Style::color(text::GREEN) }))
                .append(Text::literal("]"));
            let mut out = Text::literal(before).append(seed_text);
            if let Some(after) = after {
                out = out.append(Text::literal(after));
            }
            out
        }
    }
}

// The world as the finders read it.

/// What each block state is: its block's name and its properties, looked up by the state's number.
struct StateTable {
    names: Vec<&'static str>,
    properties: Vec<Box<[(&'static str, &'static str)]>>,
}

impl StateTable {
    fn build(game: &Game<'_>) -> StateTable {
        let registry = game.pack.assets.registry;
        let mut names = Vec::with_capacity(registry.len());
        let mut properties = Vec::with_capacity(registry.len());
        for id in 0..registry.len() {
            match registry.state(horizon_world::registry::BlockStateId(id as u32)) {
                Some(state) => {
                    names.push(intern(bare(&state.name)));
                    properties.push(state.properties.iter().map(|(k, v)| (intern(k), intern(v))).collect::<Vec<_>>().into_boxed_slice());
                }
                None => {
                    names.push("air");
                    properties.push(Box::default());
                }
            }
        }
        StateTable { names, properties }
    }
}

/// The chunks of the level, kept for the finders' threads: the game's own handles, refreshed every few ticks.
#[derive(Default)]
struct ChunkStore {
    chunks: RwLock<HashMap<(i32, i32), Arc<Chunk>>>,
}

struct GameLevel {
    store: Arc<ChunkStore>,
    table: Arc<StateTable>,
    biome_names: Arc<Vec<&'static str>>,
    dimension: Dimension,
    air: u32,
}

impl GameLevel {
    fn chunk(&self, x: i32, z: i32) -> Option<Arc<Chunk>> {
        self.store.chunks.read().unwrap_or_else(|e| e.into_inner()).get(&(x >> 4, z >> 4)).cloned()
    }

    fn state(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        self.chunk(x, z).map(|c| c.block(x, y, z).0 as usize)
    }
}

impl Level for GameLevel {
    fn dimension(&self) -> Dimension {
        self.dimension
    }

    fn block(&self, x: i32, y: i32, z: i32) -> &'static str {
        self.state(x, y, z).and_then(|s| self.table.names.get(s).copied()).unwrap_or("air")
    }

    fn property(&self, x: i32, y: i32, z: i32, name: &str) -> Option<&'static str> {
        let s = self.state(x, y, z)?;
        self.table.properties.get(s)?.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
    }

    fn has_block_entity(&self, x: i32, y: i32, z: i32, kind: &str) -> bool {
        // Every chest and spawner the game holds has its block entity.
        self.block(x, y, z) == kind
    }

    fn block_in_tag(&self, x: i32, y: i32, z: i32, tag: &str) -> bool {
        let block = self.block(x, y, z);
        match tag {
            "logs" => block.ends_with("_log") || block.ends_with("_wood") || block.ends_with("_stem") || block.ends_with("_hyphae"),
            // BlockTags.MINEABLE_WITH_AXE: what an axe breaks fastest; the wooden blocks of a wreck are the ones asked about.
            "mineable_with_axe" => block.ends_with("_planks") || block.ends_with("_log") || block.ends_with("_wood") || block.ends_with("_stairs") && !block.contains("stone") && !block.contains("brick") && !block.contains("cobble") || block.ends_with("_fence") || block.ends_with("_slab") && block.contains("wood") || block.ends_with("_door") || block.ends_with("_trapdoor") || block == "chest" || block == "barrel" || block == "bookshelf" || block == "crafting_table",
            _ => false,
        }
    }

    fn noise_biome(&self, qx: i32, y: i32, qz: i32) -> &'static str {
        let (x, z) = (qx << 2, qz << 2);
        match self.chunk(x, z) {
            Some(c) => self.biome_names.get(c.biome(x, y, z) as usize).copied().unwrap_or("the_void"),
            None => "the_void",
        }
    }

    fn biome(&self, x: i32, y: i32, z: i32) -> &'static str {
        self.noise_biome(x >> 2, y, z >> 2)
    }

    fn world_surface_height(&self, x: i32, z: i32) -> i32 {
        match self.chunk(x, z) {
            Some(c) => c.world_surface(x & 15, z & 15, |v| v == self.air),
            None => 0,
        }
    }

    fn is_chunk_loaded(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.store.chunks.read().unwrap_or_else(|e| e.into_inner()).contains_key(&(chunk_x, chunk_z))
    }
}

// The state of the mod.

/// What `SeedCracker.INSTANCE` holds, built when the game first asks.
struct State {
    config: Arc<Mutex<Config>>,
    host: Arc<GameHost>,
    storage: Arc<DataStorage>,
    features: Arc<Features>,
    queue: FinderQueue,
    search: Option<Arc<SearchPositions>>,
    table: Arc<StateTable>,
    store: Arc<ChunkStore>,
    level: Option<Arc<GameLevel>>,
    /// The chunks the finders have been given, and the revision of the game's chunks they were read at.
    seen: HashMap<(i32, i32), u64>,
    revision: u64,
    last_refresh: u64,
    /// What the session was when last looked at: the dimension and the hash of the seed.
    session: Option<(String, i64)>,
    lines: Vec<Line>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn config_path(game: &Game<'_>) -> std::path::PathBuf {
    game.native_data_dir(ID).join(CONFIG_FILE)
}

fn save_config(game: &Game<'_>, config: &Config) {
    let dir = game.native_data_dir(ID);
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(CONFIG_FILE), config.to_json());
}

impl State {
    fn new(game: &Game<'_>) -> State {
        let config = std::fs::read_to_string(config_path(game)).ok().and_then(|t| Config::from_json(&t)).unwrap_or_default();
        let version = config.get_version();
        let host_config = Arc::new(Mutex::new(config));
        let host = Arc::new(GameHost { config: host_config.clone(), queue: Mutex::new(Vec::new()), language: Mutex::new("en_us".to_owned()), found: Mutex::new(Vec::new()) });
        let features = Features::init(version, &lock(&host_config), &|m| host.push(Message::Debug(m)));
        let storage = DataStorage::new(host.clone());
        State {
            config: host_config,
            host,
            storage,
            features: Arc::new(features),
            queue: FinderQueue::new(),
            search: None,
            table: Arc::new(StateTable::build(game)),
            store: Arc::new(ChunkStore::default()),
            level: None,
            seen: HashMap::new(),
            revision: u64::MAX,
            last_refresh: 0,
            session: None,
            lines: Vec::new(),
        }
    }

    fn finder_context(&self) -> Option<Arc<FinderContext>> {
        let search = self.search.clone()?;
        let anti_xray_bypass = lock(&self.config).anti_xray_bypass;
        Some(Arc::new(FinderContext { features: self.features.clone(), search, storage: self.storage.clone(), anti_xray_bypass }))
    }

    /// `SeedCracker.reset`: forget what was found.
    fn reset(&mut self) {
        self.storage.clear();
        self.queue.finder_control.delete_finders();
        self.lines.clear();
    }

    /// `ReloadFinders.reloadHeight`, the hash of the seed, and the level the chunks are of.
    fn new_dimension(&mut self, game: &Game<'_>, dimension: &str, hashed_seed: i64, dimension_change: bool) {
        let (min_y, logical_height) = match dimension {
            "minecraft:the_nether" => (0, 128),
            "minecraft:the_end" => (0, 256),
            _ => (-64, 384),
        };
        self.search = Some(Arc::new(SearchPositions::reload_height(min_y, min_y + logical_height)));
        self.store.chunks.write().unwrap_or_else(|e| e.into_inner()).clear();
        self.seen.clear();
        self.revision = u64::MAX;
        let kind = match dimension {
            "minecraft:the_nether" => Dimension::Nether,
            "minecraft:the_end" => Dimension::End,
            _ => Dimension::Overworld,
        };
        let biome_table = match &game.world_biomes {
            Some(b) => b,
            None => game.res.biomes(),
        };
        let names: Vec<&'static str> = biome_table.list.iter().map(|b| intern(bare(&b.name))).collect();
        self.level = Some(Arc::new(GameLevel { store: self.store.clone(), table: self.table.clone(), biome_names: Arc::new(names), dimension: kind, air: game.world.air.0 }));
        let poke: DataAddedEvent = Arc::new(|s: &DataStorage| s.get_time_machine().poke(s, Some(Phase::Biomes)));
        let (active, debug) = {
            let c = lock(&self.config);
            (c.active, c.debug)
        };
        if self.storage.add_hashed_seed_data(HashedSeedData::new(hashed_seed), poke) && active && dimension_change {
            self.host.error("fetchedHashedSeed");
            if debug {
                self.host.debug(&format!("Hashed seed [{hashed_seed}]"));
            }
        }
    }

    /// Looks at the game: a new dimension or seed, and the chunks that arrived.
    fn poll(&mut self, game: &Game<'_>) {
        let Some(player) = game.native_player() else {
            if self.session.take().is_some() {
                self.reset();
            }
            return;
        };
        let hashed = match game.remote.as_ref() {
            Some(r) => r.hashed_seed,
            None => world_seed::to_hash(game.level.seed),
        };
        let key = (player.dimension.to_string(), hashed);
        if self.session.as_ref() != Some(&key) {
            let dimension_change = self.session.is_some();
            self.new_dimension(game, &key.0, hashed, dimension_change);
            self.session = Some(key);
        }
        self.sync_chunks(game);
    }

    fn sync_chunks(&mut self, game: &Game<'_>) {
        let revision = game.world.chunks.membership_revision();
        let refresh_due = game.ticks.saturating_sub(self.last_refresh) >= 20;
        if revision == self.revision && !refresh_due {
            return;
        }
        self.last_refresh = game.ticks;
        let mut arrived: Vec<(i32, i32)> = Vec::new();
        {
            let mut store = self.store.chunks.write().unwrap_or_else(|e| e.into_inner());
            if revision != self.revision {
                let present: HashSet<(i32, i32)> = game.world.chunks.keys().copied().collect();
                store.retain(|k, _| present.contains(k));
                self.seen.retain(|k, _| present.contains(k));
                for k in &present {
                    if !self.seen.contains_key(k) {
                        self.seen.insert(*k, game.ticks);
                        arrived.push(*k);
                    }
                }
            }
            for (k, c) in &game.world.chunks {
                match store.get(k) {
                    Some(old) if Arc::ptr_eq(old, c) => {}
                    old => {
                        // A server sends a chunk whole. The game's own world finishes a chunk after it shows (the structures come
                        // with the decoration), so a chunk that changes soon after it arrived is given to the finders again.
                        if old.is_some() && !arrived.contains(k) && game.ticks.saturating_sub(self.seen.get(k).copied().unwrap_or(0)) <= 600 {
                            arrived.push(*k);
                        }
                        store.insert(*k, c.clone());
                    }
                }
            }
        }
        self.revision = revision;
        if arrived.is_empty() {
            return;
        }
        arrived.sort_unstable();
        self.run_finders(&arrived);
    }

    fn run_finders(&self, chunks: &[(i32, i32)]) {
        let (Some(ctx), Some(level)) = (self.finder_context(), self.level.clone()) else { return };
        let config = lock(&self.config);
        for (x, z) in chunks {
            self.queue.on_chunk_data(ctx.clone(), &config, level.clone() as Arc<dyn Level>, ChunkPos::new(*x, *z), &BUILDERS);
        }
    }

    /// Writes what the cracker said to the chat, and works out the outlines to draw.
    fn flush(&mut self, game: &mut Game<'_>) {
        let messages = std::mem::take(&mut *lock(&self.host.queue));
        for m in messages {
            say(game, message_text(&self.host, m));
        }
        let (render, active) = {
            let c = lock(&self.config);
            (c.render, c.active)
        };
        self.lines.clear();
        if render == RenderType::Off || !active {
            publish_lines(Vec::new());
            return;
        }
        let Some(player) = game.native_player() else {
            publish_lines(Vec::new());
            return;
        };
        let dimension = match player.dimension.as_ref() {
            "minecraft:the_nether" => Dimension::Nether,
            "minecraft:the_end" => Dimension::End,
            _ => Dimension::Overworld,
        };
        let render_distance = f64::from(game.streaming.radius * 16 + 16);
        let limit = render_distance * render_distance + 32.0;
        let mut lines = Vec::new();
        self.queue.finder_control.with_active_finders(|finders| {
            for finder in finders {
                if finder.base().dimension != dimension {
                    continue;
                }
                let near = finder.cuboids().iter().any(|c| {
                    let (dx, dz) = (player.x - f64::from(c.center_pos.x), player.z - f64::from(c.center_pos.z));
                    dx * dx + dz * dz <= limit
                });
                if !near {
                    continue;
                }
                for c in finder.cuboids() {
                    push_cuboid_lines(&mut lines, c.min, c.max, c.argb);
                }
            }
        });
        publish_lines(lines);
    }
}

fn say(game: &mut Game<'_>, line: Text) {
    let focused = game.screen == super::Screen::Chat;
    game.ui.chat.add(&game.gui.atlas, line, None, horizon_client::chat::ChatSource::Client, game.ticks, focused);
}

fn with<R>(game: &mut Game<'_>, f: impl FnOnce(&mut State, &mut Game<'_>) -> R) -> R {
    let mut state = STATE.with(|s| s.borrow_mut().take()).unwrap_or_else(|| State::new(game));
    let out = f(&mut state, game);
    STATE.with(|s| *s.borrow_mut() = Some(state));
    out
}

// The outlines.

static LINES: Mutex<Vec<Line>> = Mutex::new(Vec::new());

fn publish_lines(lines: Vec<Line>) {
    *lock(&LINES) = lines;
}

/// The twelve edges of a box, 2 pixels wide (`submitShapeOutline(..., argb, 2, true)`).
fn push_cuboid_lines(out: &mut Vec<Line>, min: [f64; 3], max: [f64; 3], argb: u32) {
    let corner = |i: usize| [if i & 1 == 0 { min[0] } else { max[0] }, if i & 2 == 0 { min[1] } else { max[1] }, if i & 4 == 0 { min[2] } else { max[2] }];
    for a in 0..8usize {
        for bit in [1usize, 2, 4] {
            let b = a | bit;
            if b != a {
                out.push(Line { start: corner(a), end: corner(b), color: argb, width: 2.0 });
            }
        }
    }
}

/// The lines for the frame, added to the ones drawn over everything.
pub fn add_lines(gizmos: &mut horizon_client::gizmos::Gizmos) {
    gizmos.on_top.extend(lock(&LINES).iter().copied());
}

// The hooks.

/// Once a tick.
pub fn tick(game: &mut Game<'_>) {
    with(game, |s, game| {
        s.poll(game);
        s.flush(game);
        let _ = s.storage.tick();
    });
}

/// The commands the chat box completes: the root of ours.
pub fn commands(commands: &mut Vec<&'static str>) {
    commands.push("seedcracker");
}

/// `/seedcracker ...`, before the server and Horizon's commands see it.
pub fn command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    let words: Vec<&str> = command.split_whitespace().collect();
    if words.first() != Some(&"seedcracker") {
        return;
    }
    *ran = true;
    with(game, |s, game| run_command(s, game, &words[1..]));
}

fn feedback(game: &mut Game<'_>, message: String, color: u32) {
    say(game, Text::colored(message, color));
}

fn translate(s: &State, key: &str) -> String {
    s.host.translate(key)
}

fn run_command(s: &mut State, game: &mut Game<'_>, args: &[&str]) {
    match args {
        [] => say(game, Text::colored(translate(s, "Error: please enter a valid seedcracker command"), text::RED)),
        ["render", "outlines"] => {
            let r = lock(&s.config).render;
            feedback(game, format!("{} [{}].", translate(s, "render.getRenderMode"), r.name()), text::AQUA);
        }
        ["render", "outlines", name] => match RenderType::from_name(name) {
            Some(render) => {
                {
                    let mut c = lock(&s.config);
                    c.render = render;
                    save_config(game, &c);
                }
                feedback(game, format!("{} [{}].", translate(s, "render.setRenderMode"), render.name()), text::AQUA);
            }
            None => unknown(game),
        },
        ["finder", "reload"] => reload_finders(s, game),
        ["finder", "type", name] => match FinderType::ALL.into_iter().find(|t| t.name() == *name) {
            Some(t) => print_finder_type(s, game, t),
            None => unknown(game),
        },
        ["finder", "type", name, flag] => match (FinderType::ALL.into_iter().find(|t| t.name() == *name), *flag) {
            (Some(t), "ON") => set_finder_type(s, game, t, true, true),
            (Some(t), "OFF") => set_finder_type(s, game, t, false, true),
            _ => unknown(game),
        },
        ["finder", "category", name] => match Category::ALL.into_iter().find(|c| c.name() == *name) {
            Some(c) => {
                for t in FinderType::get_for_category(c) {
                    print_finder_type(s, game, t);
                }
            }
            None => unknown(game),
        },
        ["finder", "category", name, flag] => match (Category::ALL.into_iter().find(|c| c.name() == *name), *flag) {
            (Some(c), "ON" | "OFF") => {
                for t in FinderType::get_for_category(c) {
                    set_finder_type(s, game, t, *flag == "ON", false);
                }
                save_config(game, &lock(&s.config));
            }
            _ => unknown(game),
        },
        ["data", "clear"] => {
            s.reset();
            feedback(game, translate(s, "data.clearData"), text::GREEN);
        }
        ["data", "bits"] => {
            let base = s.storage.get_base_bits() as i32;
            let wanted = s.storage.get_wanted_bits() as i32;
            let lifting = s.storage.get_lifting_bits() as i32;
            feedback(game, scx_lang::format(&translate(s, "data.collectedBits"), &[base.to_string(), wanted.to_string()]), text::GREEN);
            feedback(game, scx_lang::format(&translate(s, "data.collectedLiftingBits"), &[lifting.to_string(), "40".to_owned()]), text::GREEN);
        }
        ["cracker"] => {
            let flag = {
                let mut c = lock(&s.config);
                c.active = !c.active;
                c.active
            };
            cracker_feedback(s, game, true, flag);
        }
        ["cracker", flag @ ("ON" | "OFF")] => {
            let want = *flag == "ON";
            let changed = {
                let mut c = lock(&s.config);
                let changed = c.active != want;
                c.active = want;
                changed
            };
            cracker_feedback(s, game, changed, want);
        }
        ["cracker", "debug"] => {
            let flag = {
                let mut c = lock(&s.config);
                c.debug = !c.debug;
                c.debug
            };
            cracker_feedback(s, game, true, flag);
        }
        ["cracker", "debug", flag @ ("ON" | "OFF")] => {
            let want = *flag == "ON";
            let changed = {
                let mut c = lock(&s.config);
                let changed = c.debug != want;
                c.debug = want;
                changed
            };
            cracker_feedback(s, game, changed, want);
        }
        ["version", name] => match MCVersion::values().iter().copied().filter(|v| v.is_newer_or_equal_to(MCVersion::v1_8)).find(|v| v.name() == *name) {
            Some(version) => {
                set_version(s, game, version);
                feedback(game, format!("{} {}.", translate(s, "version.setVersion"), version), text::AQUA);
            }
            None => unknown(game),
        },
        _ => unknown(game),
    }
}

fn unknown(game: &mut Game<'_>) {
    say(game, Text::colored("Unknown or incomplete command", text::RED));
}

fn cracker_feedback(s: &State, game: &mut Game<'_>, success: bool, flag: bool) {
    let action = translate(s, if flag { "cracker.enabled" } else { "cracker.disabled" });
    if success {
        feedback(game, format!("{}{action}", translate(s, "cracker.successfully")), text::GREEN);
    } else {
        feedback(game, format!("{}{action}", translate(s, "cracker.already")), text::RED);
    }
    save_config(game, &lock(&s.config));
}

fn print_finder_type(s: &State, game: &mut Game<'_>, t: FinderType) {
    let enabled = t.enabled(&lock(&s.config)).get();
    feedback(game, format!("{} [{}].", scx_lang::format(&translate(s, "finder.isFinder"), &[translate(s, t.name_key())]), enabled.to_string().to_uppercase()), text::AQUA);
}

fn set_finder_type(s: &State, game: &mut Game<'_>, t: FinderType, flag: bool, save: bool) {
    t.enabled(&lock(&s.config)).set(flag);
    if save {
        save_config(game, &lock(&s.config));
    }
    feedback(game, format!("{} [{}].", scx_lang::format(&translate(s, "finder.setFinder"), &[translate(s, t.name_key())]), flag.to_string().to_uppercase()), text::AQUA);
}

/// `Config.setVersion`: the features of the new version, and a fresh start.
fn set_version(s: &mut State, game: &Game<'_>, version: MCVersion) {
    let mut config = lock(&s.config);
    if !config.set_version(version) {
        save_config(game, &config);
        return;
    }
    let host = s.host.clone();
    s.features = Arc::new(Features::init(version, &config, &|m| host.push(Message::Debug(m))));
    save_config(game, &config);
}

/// `ReloadFinders.reload`: the chunks round the player are given to the finders again.
fn reload_finders(s: &mut State, game: &Game<'_>) {
    let Some(player) = game.native_player() else { return };
    let distance = game.streaming.radius;
    let (cx, cz) = ((player.x.round() as i64 >> 4) as i32, (player.z.round() as i64 >> 4) as i32);
    let mut chunks = Vec::new();
    for i in cx - distance..cx + distance {
        for j in cz - distance..cz + distance {
            chunks.push((i, j));
        }
    }
    s.run_finders(&chunks);
}
