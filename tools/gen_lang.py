"""Writes src/config/lang.rs from the language files of SeedCrackerX: one table for each language the mod has."""
import json
import os

SRC = r'C:\Users\Administrator\seedcrackerx-java\src\main\resources\assets\seedcrackerx\lang'
OUT = r'E:\Github\Minecraft\horizon-seedcrackerx\src\config\lang.rs'
LANGS = ['en_us', 'hi_in', 'ru_ru', 'tr_tr', 'zh_cn']


def rust_string(s):
    out = ''
    for ch in s:
        if ch == '\\':
            out += '\\\\'
        elif ch == '"':
            out += '\\"'
        elif ch == '\n':
            out += '\\n'
        elif ord(ch) < 32:
            out += '\\u{%x}' % ord(ch)
        else:
            out += ch
    return '"' + out + '"'


lines = [
    '// The language files of SeedCrackerX (assets/seedcrackerx/lang), written out by tools/gen_lang.py: a table for each language,',
    '// the keys in the order of the file. `translate` is `Language.getOrDefault`: the language, then English, then the key.',
    '',
]
for lang in LANGS:
    data = json.load(open(os.path.join(SRC, lang + '.json'), encoding='utf-8'))
    lines.append('static %s: [(&str, &str); %d] = [' % (lang.upper(), len(data)))
    for k, v in data.items():
        lines.append('    (%s, %s),' % (rust_string(k), rust_string(v)))
    lines.append('];')
    lines.append('')
lines.append('/// The table of a language by its code (`en_us`), English where the mod has none for it.')
lines.append('fn table(language: &str) -> &\'static [(&\'static str, &\'static str)] {')
lines.append('    match language {')
for lang in LANGS[1:]:
    lines.append('        "%s" => &%s,' % (lang, lang.upper()))
lines.append('        _ => &EN_US,')
lines.append('    }')
lines.append('}')
lines.append('')
lines.append('/// `Language.getInstance().getOrDefault(key)`.')
lines.append('pub fn translate(language: &str, key: &str) -> String {')
lines.append('    for t in [table(language), &EN_US[..]] {')
lines.append('        if let Some((_, v)) = t.iter().find(|(k, _)| *k == key) {')
lines.append('            return (*v).to_owned();')
lines.append('        }')
lines.append('    }')
lines.append('    key.to_owned()')
lines.append('}')
lines.append('')
lines.append('/// `String.formatted(args)` for the `%s` and `%d` of a message.')
lines.append('pub fn format(template: &str, args: &[String]) -> String {')
lines.append('    let mut out = String::new();')
lines.append('    let mut next = args.iter();')
lines.append('    let mut chars = template.chars().peekable();')
lines.append('    while let Some(c) = chars.next() {')
lines.append("        if c == '%' {")
lines.append('            match chars.peek() {')
lines.append("                Some('s') | Some('d') => {")
lines.append('                    chars.next();')
lines.append('                    out.push_str(next.next().map_or("null", String::as_str));')
lines.append('                    continue;')
lines.append('                }')
lines.append("                Some('%') => {")
lines.append('                    chars.next();')
lines.append("                    out.push('%');")
lines.append('                    continue;')
lines.append('                }')
lines.append('                _ => {}')
lines.append('            }')
lines.append('        }')
lines.append('        out.push(c);')
lines.append('    }')
lines.append('    out')
lines.append('}')
open(OUT, 'w', encoding='utf-8', newline='\n').write('\n'.join(lines) + '\n')
print('ok', len(lines))
