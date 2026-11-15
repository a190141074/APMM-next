"""从唯一分类定义生成插件所需的资源，禁止独立维护副本。"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / 'plugins/asset-tool/src/apmm_util/resources'
TARGET.mkdir(parents=True, exist_ok=True)
definitions = json.loads((ROOT / 'config/sections.json').read_text(encoding='utf-8'))
(TARGET / 'sections.json').write_text(json.dumps(definitions, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(f'Generated {len(definitions)} sections: {TARGET}')
