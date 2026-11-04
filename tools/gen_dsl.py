"""Translates the buildStructure methods of SeedCrackerX's finders to Rust, statement by statement.

A PieceFinder compares the world with the *block* of each state it was given, so a state is reduced to the name of its
block: `Blocks.SANDSTONE_STAIRS.defaultBlockState().setValue(...)` is "sandstone_stairs".
"""
import re
import sys

BASE = r'C:\Users\Administrator\seedcrackerx-java\src\main\java\kaptainwutax\seedcrackerX\finder'
OUT = r'E:\Github\Minecraft\horizon-seedcrackerx\src\finder\structures_dsl.rs'

# (java file, method, rust name, parameters in the java, extra rust parameters)
METHODS = [
    ('structure/DesertPyramidFinder.java', 'buildStructure', 'desert_pyramid', []),
    ('structure/JunglePyramidFinder.java', 'buildStructure', 'jungle_pyramid', []),
    ('structure/SwampHutFinder.java', 'buildStructure', 'swamp_hut', []),
    ('structure/IglooFinder.java', 'buildStructure', 'igloo', []),
    ('structure/MonumentFinder.java', 'buildStructure', 'monument', []),
    ('structure/EndCityFinder.java', 'buildStructure', 'end_city', []),
    ('structure/OutpostFinder.java', 'buildStructure', 'outpost', []),
    ('structure/TrialChambersFinder.java', 'buildOminousVault', 'trial_chambers_ominous_vault', ['x', 'y', 'z']),
    ('structure/TrialChambersFinder.java', 'buildEnd1', 'trial_chambers_end1', []),
    ('structure/TrialChambersFinder.java', 'buildEnd2', 'trial_chambers_end2', []),
]

COLOURED = {'DYED_TERRACOTTA': '{c}_terracotta', 'STAINED_GLASS': '{c}_stained_glass', 'GLAZED_TERRACOTTA': '{c}_glazed_terracotta', 'DYED_CANDLE': '{c}_candle'}
COPPER_STATES = ('unaffected', 'exposed', 'weathered', 'oxidized')


def block_name(expr):
    m = re.search(r'Blocks\.([A-Z0-9_]+)((?:\.[a-z]+\(\))*)', expr)
    if not m:
        raise ValueError('no block in ' + expr)
    base, chain = m.group(1), re.findall(r'\.([a-z]+)\(\)', m.group(2))
    chain = [c for c in chain if c != 'defaultBlockState']
    if base in COLOURED:
        return COLOURED[base].format(c=chain[0])
    if base.startswith(('COPPER_', 'CUT_COPPER')):
        waxed = 'waxed' in chain
        state = next((c for c in chain if c in COPPER_STATES), 'unaffected')
        name = base.lower()
        if base == 'COPPER_BLOCK':
            name = 'copper_block' if state == 'unaffected' else state + '_copper'
        elif state != 'unaffected':
            name = state + '_' + name
        return ('waxed_' if waxed else '') + name
    if chain:
        raise ValueError('unknown accessor in ' + expr)
    return base.lower()


def split_args(text):
    args, depth, cur = [], 0, ''
    for ch in text:
        if ch in '([{':
            depth += 1
        elif ch in ')]}':
            depth -= 1
        if ch == ',' and depth == 0:
            args.append(cur.strip())
            cur = ''
        else:
            cur += ch
    if cur.strip():
        args.append(cur.strip())
    return args


def method_body(source, name):
    m = re.search(r'(?:public|private|protected)\s+(?:static\s+)?void\s+' + name + r'\s*\([^)]*\)\s*\{', source)
    if not m:
        raise ValueError('no method ' + name)
    i, depth = m.end(), 1
    start = i
    while depth:
        ch = source[i]
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
        i += 1
    return source[start:i - 1]


def statements(body):
    """The body as a list of statements and braces, with the statements that run over lines joined."""
    body = re.sub(r'//[^\n]*', '', body)
    out, cur, depth = [], '', 0
    for ch in body:
        if ch in '([':
            depth += 1
        elif ch in ')]':
            depth -= 1
        cur += ch
        if depth == 0 and ch in ';{}':
            s = ' '.join(cur.split())
            if s in ('{', '}'):
                out.append(s)
            elif s.endswith('{') or s.endswith('}'):
                out.append(s)
            else:
                out.append(s)
            cur = ''
    return [s for s in out if s]


def expr(e, variables):
    e = e.replace('finder.width', 'f.width').replace('finder.depth', 'f.depth').replace('finder.height', 'f.height')
    # ternary: a ? b : c
    m = re.fullmatch(r'(.+?)\s*\?\s*(.+?)\s*:\s*(.+)', e)
    if m:
        return 'if %s { %s } else { %s }' % (expr(m.group(1), variables), expr(m.group(2), variables), expr(m.group(3), variables))
    return e


def block_arg(a, variables):
    a = a.strip()
    if a == 'null':
        return 'None'
    m = re.match(r'^\(*(\w+)(?:\.|\)|$)', a)
    if m and m.group(1) in variables:
        return m.group(1)
    return '"%s"' % block_name(a)


def translate(statement_list, params):
    lines, variables, closers = [], {}, []
    indent = 1

    def emit(s):
        lines.append('    ' * indent + s)

    for s in statement_list:
        if s == '}':
            indent -= 1
            closer = closers.pop()
            if closer:
                lines.insert(len(lines), '    ' * (indent + 1) + closer)
            emit('}')
            continue
        s = re.sub(r'^(for|if) ?\(', r'\1 (', s)
        s = re.sub(r'\) ?\{$', ') {', s)
        m = re.fullmatch(r'for \((?:int )?(\w+) = (.+?); (.+?); (\w+)(\+\+| \+= (.+?))\) \{', s)
        if m:
            var, init, cond, uvar, op, step = m.groups()
            declared = 'int ' in s.split(';')[0]
            emit('let mut %s: i32 = %s;' % (var, expr(init, variables)) if declared or True else '')
            emit('while %s {' % expr(cond, variables))
            closers.append('%s += %s;' % (uvar, expr(step, variables) if step else '1'))
            indent += 1
            continue
        m = re.fullmatch(r'if \((.+)\) \{', s)
        if m:
            emit('if %s {' % expr(m.group(1), variables))
            closers.append(None)
            indent += 1
            continue
        m = re.fullmatch(r'(?:final )?(?:BlockState|var) (\w+) = (.+);', s)
        if m:
            name, e = m.groups()
            variables[name] = block_name(e)
            emit('let %s = "%s";' % (name, variables[name]))
            continue
        m = re.fullmatch(r'int (\w+) = (.+);', s)
        if m:
            emit('let %s: i32 = %s;' % (m.group(1), expr(m.group(2), variables)))
            continue
        m = re.fullmatch(r'int (\w+);', s)
        if m:
            emit('let mut %s: i32 = 0;' % m.group(1))
            continue
        m = re.fullmatch(r'finder\.addBlock\((.*)\);', s)
        if m:
            a = split_args(m.group(1))
            if len(a) != 4:
                raise ValueError(s)
            if a[0] == 'null':
                emit('f.remove_block(%s, %s, %s);' % (expr(a[1], variables), expr(a[2], variables), expr(a[3], variables)))
                continue
            emit('f.add_block(%s, %s, %s, %s);' % (block_arg(a[0], variables), expr(a[1], variables), expr(a[2], variables), expr(a[3], variables)))
            continue
        m = re.fullmatch(r'finder\.fillWithOutline\((.*)\);', s)
        if m:
            a = split_args(m.group(1))
            if len(a) != 9:
                raise ValueError(s)
            ints = ', '.join(expr(x, variables) for x in a[:6])
            if 'null' in (a[6], a[7]):
                wrap = lambda b: 'None' if b == 'null' else 'Some(%s)' % block_arg(b, variables)
                emit('f.fill_with_outline_opt(%s, %s, %s, %s);' % (ints, wrap(a[6]), wrap(a[7]), a[8]))
                continue
            emit('f.fill_with_outline(%s, %s, %s, %s);' % (ints, block_arg(a[6], variables), block_arg(a[7], variables), a[8]))
            continue
        m = re.fullmatch(r'buildOminousVault\(finder, (.+?), (.+?), (.+?)\);', s)
        if m:
            emit('trial_chambers_ominous_vault(f, %s, %s, %s);' % tuple(expr(x, variables) for x in m.groups()))
            continue
        raise ValueError('cannot translate: ' + s)
    return lines


def main():
    out = []
    out.append('// The buildStructure methods of the finders, written out from the Java by tools/gen_dsl.py, statement for statement.')
    out.append('// A PieceFinder compares the world with the block of each state it was given, so a state is the name of its block.')
    out.append('use super::scx_finder_piece::PieceFinder;')
    out.append('')
    for file, method, rust, params in METHODS:
        source = open(BASE + '\\' + file.replace('/', '\\'), encoding='utf-8').read()
        body = method_body(source, method)
        lines = translate(statements(body), params)
        sig = ', '.join('%s: i32' % p for p in params)
        out.append('#[allow(clippy::all, non_snake_case, unused_mut, unused_variables, unused_assignments, unused_parens)]')
        out.append('pub fn %s(f: &mut PieceFinder%s) {' % (rust, (', ' + sig) if sig else ''))
        out.extend(lines)
        out.append('}')
        out.append('')
    open(OUT, 'w', encoding='utf-8', newline='\n').write('\n'.join(out))
    print('ok', len(out))


main()
