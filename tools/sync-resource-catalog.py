"""合并官方资源目录和有证据的名称；只生成元数据，不复制游戏图片。"""
import argparse
import hashlib
import json
import re
from pathlib import Path
from PIL import Image
from work_paths import report_dir

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--assets', type=Path, required=True)
parser.add_argument('--reference', type=Path, required=True)
args = parser.parse_args()

def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

def pixels(path):
    im = Image.open(path).convert('RGBA')
    return hashlib.sha256(str(im.size).encode() + im.tobytes()).hexdigest()

heroes = read(ROOT / 'config/heroes.json')
monsters = read(ROOT / 'config/monsters.json')
cards = read(ROOT / 'config/card_names.json')
nicknames = read(ROOT / 'config/role_nicknames.json')
sections = {s['key']: s for s in read(ROOT / 'config/sections.json')}
translations = read(args.reference / 'localization/zhs/relics.json')
english = read(args.reference / 'localization/eng/relics.json')
matches = {}
for path in (args.reference / 'images/relic').glob('token_*.png'):
    key = 'RE_ASTRAL_PARTY_MOD_RELIC_' + path.stem.upper() + '.title'
    if key not in translations:
        continue
    matches.setdefault(pixels(path), []).append({
        'name': translations[key].strip('【】'),
        'english': english.get(key, '').strip('[]【】'),
        'file': path.name,
    })

# 同一纹理被多种品质共用，只标注共用的名称，不从图片推断品质。
shared = {'50010': '美工刀', '50030': '手持风扇', '50032': '标记喷罐',
          '50044': '瞄具', '50078': '飞星', 'Attack': '拳击手套',
          'Defend': '摩托头盔', 'Enmity': '肾上腺素', 'Gold': '银行卡',
          'Heal': '医疗箱', 'Hp': '夹心饼干', 'KillReward': '手电筒', 'Speed': '速度轮滑'}
terms = {'50027': '维生素药丸', '50042': 'ATM机', '50066': '探天卫星', '50075': '自适应核心'}
chips = {}
for path in sorted((args.assets / 'outimg').glob('UT_Relic_*.png')):
    evidence = matches.get(pixels(path), [])
    if not evidence:
        continue
    chip_id = path.stem.removeprefix('UT_Relic_')
    chips[chip_id] = {
        'cn': terms.get(chip_id, shared.get(chip_id, evidence[0]['name'])),
        'aliases': sorted({v for e in evidence for v in [e['name'], e['english']] if v}),
        'evidence': {'method': 'equal-rgba-pixels', 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                     'references': [e['file'] for e in evidence]},
    }
if '50075' in chips:
    chips['50075_1'] = {'cn': '自适应核心 · 变体 1', 'aliases': chips['50075']['aliases'],
                      'evidence': {'method': 'official-resource-variant', 'base': 'UT_Relic_50075'}}
save(ROOT / 'config/chip_names.json', chips)

official = {}
for value in read(ROOT / 'config/manifest.json'):
    if 'preview' not in value:
        official[value['canonical']] = value
for path in sorted((args.assets / 'maps').glob('*.json')):
    for value in read(path):
        image = args.assets / 'outimg' / (value['canonical'] + '.png')
        if hashlib.sha256(image.read_bytes()).hexdigest() != value['sha256']:
            raise ValueError(f'原图摘要不符：{image}')
        official[value['canonical']] = value

records = []
for cn, value in sorted(official.items()):
    section, entity = value['section'], str(value['heroId'])
    if section == 'profile_photos' and entity.isdecimal() and int(entity) > 999:
        section = 'monster_profile'
    definition = sections[section]
    category = ('lands' if section in ('land', 'platform') else
                'chips' if section in ('relic', 'relic_icon') else
                'cursors' if section == 'cursor' else definition['category'])
    meta = heroes.get(entity, {}) if category in ('heroes', 'lands') else monsters.get(entity, {})
    name = meta.get('cn', '')
    aliases = [meta.get('en', ''), meta.get('titleCn', ''), meta.get('titleEn', '')]
    if category in ('heroes', 'lands'):
        aliases += nicknames.get(entity, [])
    elif category == 'cards':
        name = cards.get(section, {}).get(entity, '')
        aliases = []
    elif category == 'chips':
        chip = chips.get(cn.removeprefix('UT_Relic_'), {})
        name, aliases = chip.get('cn', ''), chip.get('aliases', [])
    elif category == 'cursors':
        name, aliases = '默认鼠标', []
    variant, variant_label, order = 'base', '原皮', 0
    common = section in ('account_background', 'item_icons')
    if category == 'heroes' and not common:
        tail = cn.split('_' + entity, 1)[-1]
        found = re.match(r'_(Max|\d{2})(?:_|$)', tail)
        if found:
            variant = found[1]
            variant_label = '契约皮' if variant == 'Max' else '皮肤 ' + str(int(variant))
            order = 1 if variant == 'Max' else int(variant) + 1
    if common:
        variant, variant_label, order = 'shared', '公共资源', 1000
    label = definition['label']
    if section == 'busts' and re.search(r'_\d$', cn):
        label += ' ' + str(int(cn[-1]) + 1)
    elif section == 'account_background':
        numbered = re.match(r'UT_AccountBackground_\d+_(\d+)(.*)$', cn)
        if numbered:
            label += ' ' + str(int(numbered[1])) + (' · ' + numbered[2].strip('_') if numbered[2] else '')
    elif section == 'cursor':
        label = '抬起' if cn.endswith('_up') else '按下'
    records.append({
        'key': section + '/' + cn, 'category': category, 'entityId': entity,
        'entityName': name or '未收录名称 · ' + entity, 'nameKnown': bool(name),
        'aliases': sorted(set(a for a in aliases if a)), 'section': section,
        'sectionLabel': definition['label'], 'resourceLabel': label,
        'canonical': cn, 'fullName': cn, 'applicationSlot': cn,
        'groupKey': 'land:' + entity if category == 'lands' else 'cursor' if category == 'cursors' else section + '/' + cn,
        'variantKey': variant, 'variantLabel': variant_label, 'variantOrder': order,
        'originalUrl': None, 'originalStatus': 'not-extracted', 'files': [],
        'activeHash': None, 'disabled': False,
        'previewFile': value.get('previewFile', cn + '.webp'), 'sha256': value.get('sha256'),
    })
save(ROOT / 'config/resources.json', records)
summary = {'resources': len(records), 'sections': len({r['section'] for r in records}),
           'namedChipResources': len(chips),
           'unknownNames': [{'key': r['key'], 'category': r['category']} for r in records if not r['nameKnown']]}
save(report_dir() / 'catalog-audit.json', summary)
print(json.dumps({k: v for k, v in summary.items() if k != 'unknownNames'}, ensure_ascii=False))
print('未确认名称：', len(summary['unknownNames']))
