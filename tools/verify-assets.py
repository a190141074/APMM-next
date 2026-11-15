"""核验真实提取产物的尺寸、摘要、透明度、变体和成组关系，并复验独立 EXE 协议。"""
import hashlib
import json
import os
import subprocess
from pathlib import Path
from PIL import Image

project = Path(__file__).resolve().parents[1]
validation = Path((project / 'verification-root.txt').read_text().strip())
assets = validation / 'assets'
build = json.loads((project / 'verification-plugin-build.json').read_text())
exe = Path(build['payload']) / 'apmm-asset-tool.exe'
env = dict(os.environ, APMM_WORK_DIR='B:/APMM-next-work', TEMP='B:/APMM-next-work/temp', TMP='B:/APMM-next-work/temp')
def digest(path):
    return hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()

sections = {}
all_names = {}
fingerprints = {}
for file in sorted((assets / 'maps').glob('*.json')):
    rows = json.loads(file.read_text(encoding='utf-8'))
    if not isinstance(rows, list):
        continue
    sections[file.stem] = len(rows)
    for row in rows:
        name = row['canonical']
        png = assets / 'outimg' / (name + '.png')
        actual = digest(png)
        assert actual == row['sha256'], name
        with Image.open(png) as image:
            assert image.size == (row['width'], row['height']), name
        fingerprints[name] = actual
        all_names[name] = row
assert len(sections) == 22, sections
assert 'UT_Relic_50075_1' in all_names
assert 'cursor_up' in all_names and 'cursor_down' in all_names
lands = {n.removeprefix('UT_Land_') for n in all_names if n.startswith('UT_Land_')}
platforms = {n.removeprefix('UT_Platform_') for n in all_names if n.startswith('UT_Platform_')}
assert lands == platforms
alpha = {}
for name in ['cursor_up', 'cursor_down']:
    with Image.open(assets / 'outimg' / (name + '.png')) as image:
        alpha[name] = {'size': image.size, 'mode': image.mode, 'alpha': image.getchannel('A').getextrema()}
        assert alpha[name]['alpha'] == (0, 255)
args = [str(exe), 'sync', 'all', '--game-dir', 'D:/Steam/steamapps/common/Astral Party/8vJXn6CN', '--locallow-dir', 'B:/feimo/feimo/AstralParty_CN', '--output', str(assets), '--delay-ms', '0', '--jsonl', '--yes']
with (project / 'verification-assets-repeat.jsonl').open('w', encoding='utf-8') as log:
    repeat = subprocess.run(args, stdout=log, stderr=subprocess.PIPE, env=env, encoding='utf-8', check=True)
for name, expected in fingerprints.items():
    assert digest(assets / 'outimg' / (name + '.png')) == expected, name
process = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, text=True, encoding='utf-8')
lines = []
for line in process.stdout:
    lines.append(line)
    if json.loads(line).get('type') == 'progress':
        process.stdin.write('{"protocol":1,"type":"cancel"}\n')
        process.stdin.flush()
        break
rest, error = process.communicate(timeout=120)
lines.extend(rest.splitlines(keepends=True))
(project / 'verification-assets-cancel.jsonl').write_text(''.join(lines), encoding='utf-8')
cancel_report = json.loads(lines[-1])
assert process.returncode != 0 and cancel_report['type'] == 'result', cancel_report
assert cancel_report['result']['status'] == 'cancelled', cancel_report
for name, expected in fingerprints.items():
    assert digest(assets / 'outimg' / (name + '.png')) == expected, name
report = {'sections': sections, 'total': len(all_names), 'cursor': alpha, 'landPairs': len(lands), 'repeatHashesUnchanged': True, 'cancelExit': process.returncode, 'cancelPreservedOriginals': True, 'packagedExecutable': str(exe)}
(project / 'verification-assets-audit.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps(report, ensure_ascii=False))
