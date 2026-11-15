"""按发行清单更新指定测试程序；独立备份程序文件，合并经摘要验证的原图并保护既有数据。"""
import argparse, datetime, hashlib, json, shutil
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / 'dist/APMM-next'
WORK = Path('B:/APMM-next-work')
ALLOWED = {'astral-mod-manager.exe','astral-updater.exe','astral-mod-manager.exe.manifest',
           'WebView2Loader.dll','api-ms-win-core-synch-l1-2-0.dll',
           'licenses/LICENSE.txt','licenses/THIRD_PARTY_NOTICES.md','release-manifest.json'}
LEGACY = {'LICENSE.txt','THIRD_PARTY_NOTICES.md'}
parser=argparse.ArgumentParser()
parser.add_argument('--assets',type=Path,required=True)
parser.add_argument('--apply',action='store_true')
parser.add_argument('--resume-backup',type=Path)
args=parser.parse_args()

def digest(path):
    with path.open('rb') as file: return hashlib.file_digest(file,'sha256').hexdigest()
def no_links(path):
    for parent in [path,*path.parents]:
        assert not parent.is_symlink(), f'拒绝链接路径：{parent}'
        if parent.exists(): assert not parent.stat().st_file_attributes & 0x400, f'拒绝重解析点：{parent}'
        if parent == ROOT or parent == WORK: break

assert TARGET.resolve() == Path('D:/MOD/APMM-next/apmm-desktop/dist/APMM-next').resolve()
no_links(TARGET)
report=json.loads((ROOT/'verification-portable-build.json').read_text('utf-8'))
payload=Path(report['payload'])
assert payload.resolve().is_relative_to(WORK.resolve())
manifest=json.loads((payload/'release-manifest.json').read_text('utf-8'))
assert manifest['version']=='0.3.38'
for record in manifest['files']:
    assert record['path'] in ALLOWED
    file=payload/record['path'];assert file.stat().st_size==record['bytes'];assert digest(file)==record['sha256']
assets=args.assets.resolve();assert assets.is_relative_to(WORK.resolve())
originals=[]
for file in sorted((assets/'maps').glob('*.json')):
    rows=json.loads(file.read_text('utf-8'))
    for row in rows:
        name=row['canonical'];assert '/' not in name and '\\' not in name
        image=assets/'outimg'/f'{name}.png';assert digest(image)==row['sha256']
        with Image.open(image) as im: im.verify()
        originals.append((image,row['sha256']))
assert len(originals)==2179
# 数据保护以既有全部文件摘要为准，不向 library、vault、settings 或 WebView 写入。
protected={str(p.relative_to(TARGET)):digest(p) for p in (TARGET/'data').rglob('*') if p.is_file()}
backup=args.resume_backup or WORK/('local-test-backup-038-'+datetime.datetime.now().strftime('%Y%m%d-%H%M%S'))
assert backup.resolve().is_relative_to(WORK.resolve()) and backup.name.startswith('local-test-backup-038-')
if args.resume_backup: assert (backup/'astral-mod-manager.exe').is_file()
print(json.dumps({'target':str(TARGET),'payload':str(payload),'backup':str(backup),'originals':len(originals),'protectedFiles':len(protected)},ensure_ascii=False),flush=True)
if not args.apply: raise SystemExit(0)
backup.mkdir(exist_ok=bool(args.resume_backup))
for name in ALLOWED | LEGACY:
    current=TARGET/name;no_links(current)
    if current.is_file():
        saved=backup/name;saved.parent.mkdir(parents=True,exist_ok=True)
        if not saved.exists(): shutil.copy2(current,saved);assert digest(current)==digest(saved)
added=0;conflicts=[]
for name in ALLOWED:
    source=payload/name
    if source.is_file():
        target=TARGET/name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target);assert digest(source)==digest(target)
# 旧平铺声明已经独立备份，移出程序根目录，发行包仅采用 licenses 路径。
for name in LEGACY:
    old=TARGET/name
    if old.exists():
        saved=backup/('legacy-root-'+name);shutil.copy2(old,saved);assert digest(old)==digest(saved);old.unlink()
for source,expected in originals:
    target=TARGET/'data/assets/outimg'/source.name;no_links(target)
    if target.exists():
        if digest(target)!=expected: conflicts.append(str(target));continue
    else:
        target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target);added+=1
    assert digest(target)==expected
for source in sorted((assets/'maps').glob('*.json')):
    target=TARGET/'data/assets/maps'/source.name
    if target.exists() and digest(target)!=digest(source): conflicts.append(str(target));continue
    if not target.exists(): target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target)
for relative,expected in protected.items(): assert digest(TARGET/relative)==expected, f'既有数据改变：{relative}'
result={'version':manifest['version'],'target':str(TARGET),'backup':str(backup),'programFiles':manifest['files'],
        'originalsAdded':added,'originalsVerified':len(originals)-len([c for c in conflicts if c.endswith('.png')]),
        'protectedFiles':len(protected),'existingDataPreserved':True,'conflicts':conflicts}
(ROOT/'verification-local-test.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),'utf-8')
print(json.dumps({k:v for k,v in result.items() if k!='programFiles'},ensure_ascii=False))
