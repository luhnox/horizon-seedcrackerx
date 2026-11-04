# SeedCrackerX for Horizon

A Rust port of [SeedCrackerX](https://github.com/19MisterX98/SeedcrackerX) 2.16.2 for
[Horizon](https://github.com/horizon-rust/horizon), the Rust rewrite of Minecraft Java Edition 26.3. It is a Horizon **native mod**:
Rust source that `horizon-mixins` splices into the game before it is compiled. SeedCrackerX is a Fabric mod and cannot run in Horizon.

> **Status: the usual route works, the old-version route does not.** [PARITY.md](PARITY.md) lists every Java file and says what is
> ported, partial or missing. Missing: the Dungeon and Warped Fungus cracking of 1.17 and older (needs a lattice solver) and the
> biome-based world seed search. A world seed is found from structures and the hashed seed the server sends.

Pinned to Horizon commit `6475f62f3c834b7489e1816bf47f47ea5eb2b879` (Nightly 2026-11-04 `6475f62`, the newest), the same one Baritone
for Horizon is pinned to, so the two build together. Only the newest Nightly is kept up to date. A later Nightly may need a port.

## Using it

Same as the Java: walk near structures (igloos, temples, monuments, shipwrecks, end cities, buried treasure, outposts, trial
chambers, desert wells, end gateways, emerald ore, dungeons) and they are outlined through walls. With enough of them the seed is
printed in chat. Commands are `/seedcracker ...`:

`render outlines [OFF|XRAY|ON]`, `finder type|category|reload`, `data clear|bits|restore`, `cracker [ON|OFF]`, `cracker debug`,
`version <version>`, `database`, `gui`.

The settings are kept in `seedcracker.json` of the mod's data folder (the Java's layout) and are on the mod's page in Options > Mods.

## Network and processes

- At start the mod reads the Google Sheet of known server seeds (`DATABASE_URL`). When a seed is cracked on a server with more than 10
  players, and you have switched the database submission on, it proves your account to Mojang's session service under the id
  `seedcrackerx` and posts the server address, dimension, seed, version and your name (or a hash) to the sheet's script. Both are
  the Java's behavior; both can be switched off in the settings.
- `/seedcracker database` and the links on the page open your browser with the system's opener.

## Install

A mod is built into the Horizon commit it is pinned to. Only the newest Nightly is supported, so there is one download: the
release is titled `SeedCrackerX <version> for Nightly <date> (<7 letters>)` and its file is named
`seedcrackerx-<version>-nightly-<date>-<7 letters>.hnmod`, the date and the 7 letters the launcher shows in its Versions tab.

| Nightly in the launcher | SeedCrackerX | Download |
|---|---|---|
| 2026-11-04 `6475f62` (up to date) | the latest | [latest release](https://github.com/luhnox/horizon-seedcrackerx/releases/latest), file `seedcrackerx-<version>-nightly-2026-11-04-6475f62.hnmod` |

Install the file in the launcher's native mods and use the same Nightly for the game, so the launcher builds on the source it
already has (10 to 30 minutes the first time). It runs native code with your rights: read the source first.

## Build and test

```
cargo test        # Java diff, cracker, finders
python tools/gen_mixin.py   # rewrites horizon-mixin.toml and hooks/ from src/lib.rs
python tools/pack.py        # dist/seedcrackerx.hnmod
```

MIT, as SeedCrackerX. See [LICENSE](LICENSE).
