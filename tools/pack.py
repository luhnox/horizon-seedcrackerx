"""Builds dist/seedcrackerx.hnmod from this folder.

The .hnmod is a zip of horizon-native.json (the registry entry), horizon-mixin.toml and every file the
toml names as a `source` or a `helper`, and the LICENSE, under mixins/seedcrackerx/. Run it from the repository root:

    python tools/pack.py
"""
import json
import re
import shutil
import zipfile

toml = open('horizon-mixin.toml', encoding='utf-8').read()
version = re.search(r'^version = "([^"]+)"', toml, re.M).group(1)
sources = re.findall(r'^(?:source|helper) = "([^"]+)"', toml, re.M)
files = ['horizon-mixin.toml'] + sorted(set(sources))
meta = json.load(open('horizon-native.json', encoding='utf-8'))
meta['version'] = version
stamp = (2026, 1, 1, 0, 0, 0)  # a fixed time, so the same sources always give the same file
with zipfile.ZipFile('dist/seedcrackerx.hnmod', 'w', zipfile.ZIP_DEFLATED) as z:
    z.writestr(zipfile.ZipInfo('horizon-native.json', stamp), json.dumps(meta, indent=2, sort_keys=True) + '\n')
    for f in files + ['LICENSE']:
        z.writestr(zipfile.ZipInfo('mixins/seedcrackerx/' + f, stamp), open(f, 'rb').read())
print(version, len(files) + 1, 'files ->', 'dist/seedcrackerx.hnmod')

# The Horizon commit this build is pinned to is a Nightly: the copy that is uploaded to a release says which one in its name, so
# a player on that Nightly picks the file by the date the launcher shows. A commit that is not listed is named by its hash.
NIGHTLIES = {
    '142388e1c136e622652ec7505af15d8ad195e041': '2026-10-28',
    'ba6344a59cb9e755b730dcf02285fc0d33b97d3c': '2026-10-30',
    '3fb4c3138e0c5d8fdd2fdb555cf31368e8897497': '2026-11-04',
}
pin = meta['horizon']
named = f"dist/seedcrackerx-{version}-nightly-{NIGHTLIES[pin]}.hnmod" if pin in NIGHTLIES else f"dist/seedcrackerx-{version}-horizon-{pin[:7]}.hnmod"
shutil.copyfile('dist/seedcrackerx.hnmod', named)
print('and', named)
