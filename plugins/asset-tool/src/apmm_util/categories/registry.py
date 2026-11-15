"""从管理器共享定义注册分类；完整名称、应用槽位和分组分别保存。"""
import json
import re
from dataclasses import dataclass
from pathlib import Path
from ..errors import ToolError

DEFINITIONS = json.loads((Path(__file__).resolve().parents[1] / 'resources/sections.json').read_text(encoding='utf-8'))


@dataclass(frozen=True)
class Category:
    """单一资源部位，名称匹配由共享定义与实体编号共同决定。"""
    key: str
    label: str
    prefix: str
    supported: bool = True

    def parse_metadata(self, name: str) -> dict:
        section, entity = classify(name)
        if section['key'] != self.key:
            raise ValueError(f'资源不属于 {self.key}：{name}')
        group = f"land:{entity}" if self.key in ('land', 'platform') else 'cursor' if self.key == 'cursor' else name
        return {'category': self.key, 'entity_id': entity, 'variant': name.split(entity, 1)[-1].lstrip('_'),
                'canonical': name, 'slot': name, 'group_key': group}

    def names(self, catalog) -> list[str]:
        candidates = catalog.resource_names(self.prefix)
        if self.key == 'monster_profile':
            candidates += catalog.resource_names('UT_Hero_ProfilePhoto_')
        result = []
        for name in sorted(set(candidates)):
            try:
                self.parse_metadata(name)
                result.append(name)
            except (ToolError, ValueError):
                pass
        return result


def prefix_of(definition: dict) -> str:
    return definition['regex'].lstrip('^').split('(')[0]


REGISTRY = {d['key']: Category(d['key'], d['label'], prefix_of(d)) for d in DEFINITIONS}


def classify(name: str) -> tuple[dict, str]:
    """保留官方变体；非数字地块不加入角色分组，怪物头像采用实际名称。"""
    if not re.fullmatch(r'[A-Za-z0-9_]+', name):
        raise ToolError(f'无效资源名：{name}')
    if name in ('cursor_up', 'cursor_down'):
        return next(d for d in DEFINITIONS if d['key'] == 'cursor'), 'cursor'
    for definition in DEFINITIONS:
        match = re.match(definition['regex'], name)
        if not match:
            continue
        entity = match[1]
        if definition['key'] == 'profile_photos' and int(entity) > 999:
            definition = next(d for d in DEFINITIONS if d['key'] == 'monster_profile')
        return definition, entity
    raise ToolError(f'无法识别资源分类：{name}', code='category_unknown', stage='catalog')


def require_category(key: str) -> Category:
    if key not in REGISTRY:
        raise ToolError(f'未知分类：{key}；可选分类：{", ".join(REGISTRY)}')
    return REGISTRY[key]


def identify_category(name: str) -> Category:
    return REGISTRY[classify(name)[0]['key']]


def category_for_name(name: str) -> Category:
    return identify_category(name)


def expand_categories(key: str) -> list[Category]:
    """展开页面分组，保持原始部位作为独立增量同步边界。"""
    groups = {'hero': 'heroes', 'monster': 'monsters', 'cards': 'cards'}
    if key == 'all':
        return list(REGISTRY.values())
    if key == 'lands':
        return [REGISTRY['land'], REGISTRY['platform']]
    if key in groups:
        return [REGISTRY[d['key']] for d in DEFINITIONS if d['category'] == groups[key] and d['key'] not in ('land', 'platform')]
    if key == 'relic':
        return [REGISTRY['relic'], REGISTRY['relic_icon']]
    return [require_category(key)]

