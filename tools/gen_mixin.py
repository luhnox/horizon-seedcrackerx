"""Writes horizon-mixin.toml of SeedCrackerX from the modules src/lib.rs lists, and the hook files."""
import io
import os
import re

ROOT = r'E:\Github\Minecraft\horizon-seedcrackerx'
REV = '3fb4c3138e0c5d8fdd2fdb555cf31368e8897497'
VERSION = open(os.path.join(ROOT, 'Cargo.toml'), encoding='utf-8').read()
VERSION = re.search(r'^version = "([^"]+)"', VERSION, re.M).group(1)

lib = open(os.path.join(ROOT, 'src', 'lib.rs'), encoding='utf-8').read()
mods = re.findall(r'#\[path = "([^"]+)"\]\s*pub mod (\w+);', lib)

out = []
out.append('[mod]')
out.append('id = "seedcrackerx"')
out.append('version = "%s"' % VERSION)
out.append('priority = 90')
out.append('')
out.append('# The Nightly this was written against: the Horizon the instance plays. It reads the game\'s world and the seed the server')
out.append('# sent, so it names the commit it needs; Baritone is pinned to the same one, so the two build together.')
out.append('[compatibility]')
out.append('horizon_revision = "%s"' % REV)
out.append('')
for path, name in mods:
    out.append('[[module]]')
    out.append('id = "mod_%s"' % name)
    out.append('file = "crates/horizon-client/src/game.rs"')
    out.append('name = "%s"' % name)
    out.append('source = "src/%s"' % path)
    out.append('')
out.append('# The part that touches the game.')
out.append('[[module]]')
out.append('id = "mod_scx_game"')
out.append('file = "crates/horizon-client/src/game.rs"')
out.append('name = "scx_game"')
out.append('source = "src/game/seedcracker.rs"')
out.append('')


def hook(id_, target, helper, bindings, cancellable=False, kind='HEAD'):
    out.append('[[injection]]')
    out.append('id = "%s"' % id_)
    out.append('kind = "%s"' % kind)
    if cancellable:
        out.append('cancellable = true')
    out.append('target = "%s"' % target)
    out.append('helper = "hooks/%s.rs"' % helper)
    out.append('bindings = [')
    for b in bindings:
        out.append('  { name = "%s", ty = "%s", value = "%s" },' % b)
    out.append(']')
    out.append('')


hook('tick', "horizon_client::game::native_hooks::Game<'_>::native_tick", 'tick', [('game', '&mut crate::game::Game<\'_>', '&mut *self')])
hook('command', "horizon_client::game::native_hooks::Game<'_>::native_command", 'command', [('game', '&mut crate::game::Game<\'_>', '&mut *self'), ('command', '&str', 'command'), ('ran', '&mut bool', 'ran')])
hook('commands', "horizon_client::game::native_hooks::Game<'_>::native_commands", 'commands', [('commands', "&mut Vec<&'static str>", 'commands')])
hook('gizmos', "horizon_client::game::debug_renderers::Game<'_>::debug_renderer_gizmos", 'gizmos', [('gizmos', '&mut horizon_client::gizmos::Gizmos', 'g')])

open(os.path.join(ROOT, 'horizon-mixin.toml'), 'w', encoding='utf-8', newline='\n').write('\n'.join(out))
os.makedirs(os.path.join(ROOT, 'hooks'), exist_ok=True)


def helper(name, text):
    open(os.path.join(ROOT, 'hooks', name + '.rs'), 'w', encoding='utf-8', newline='\n').write(text)


helper('tick', '''// Once a tick: the chunks that arrived, the seed the server sent, what the cracker has to say, the outlines.
fn scx_tick(game: &mut crate::game::Game<'_>) {
    if game.native_on(crate::game::scx_game::ID) {
        crate::game::scx_game::tick(game);
    }
}
''')
helper('command', '''// /seedcracker, before the server and Horizon's commands see it.
fn scx_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    if !*ran && game.native_on(crate::game::scx_game::ID) {
        crate::game::scx_game::command(game, command, ran);
    }
}
''')
helper('commands', '''// The commands the chat box completes as the game's own.
fn scx_commands(commands: &mut Vec<&'static str>) {
    crate::game::scx_game::commands(commands);
}
''')
helper('gizmos', '''// The outlines of what was found, added to the lines drawn over everything.
fn scx_gizmos(gizmos: &mut horizon_client::gizmos::Gizmos) {
    crate::game::scx_game::add_lines(gizmos);
}
''')
print('ok', len(mods))
