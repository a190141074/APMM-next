"""用途：解析 Addressables 紧凑目录与官方依赖。作者：待填写。修改记录：2026-11-14，首次创建；补充官方资源位置错误信息。"""

import base64
import json
import struct

from ..errors import ToolError
from ..models import AssetSpec, BundleSpec


def integer(data: bytes, offset: int) -> int:
    """功能：读取紧凑目录整数。入参：字节数据和位置。返回：整数。异常：截断。"""
    if offset < 0 or offset + 4 > len(data):
        raise ToolError("Addressables 目录整数越界。")
    return struct.unpack_from("<i", data, offset)[0]


def table(data: bytes, offset: int, length: int) -> bytes:
    """功能：取得有界目录片段。入参：数据、位置和长度。返回：字节内容。异常：越界。"""
    if offset < 0 or length < 0 or offset + length > len(data):
        raise ToolError("Addressables 目录片段越界。")
    return data[offset:offset + length]


def string_key(data: bytes, offset: int) -> str:
    """功能：解码目录键。入参：键数据和位置。返回：文本标识。异常：字符串损坏。"""
    kind = table(data, offset, 1)[0]
    if kind in (0, 1):
        length = integer(data, offset + 1)
        return table(data, offset + 5, length).decode("ascii" if kind == 0 else "utf-16-le")
    # 数值依赖键通过桶索引连接，不需要猜测它们的文本表达方式。
    return f"@{kind}:{offset}"


class Catalog:
    """提供资源键、位置和依赖的校验后视图。"""

    def __init__(self, document: dict):
        """功能：解析紧凑目录。入参：官方目录对象。返回：无。异常：格式损坏。"""
        try:
            self.ids = document["m_InternalIds"]
            self.providers = document["m_ProviderIds"]
            self.types = document["m_resourceTypes"]
            self.prefixes = document.get("m_InternalIdPrefixes") or []
            key_data = base64.b64decode(document["m_KeyDataString"], validate=True)
            bucket_data = base64.b64decode(document["m_BucketDataString"], validate=True)
            entry_data = base64.b64decode(document["m_EntryDataString"], validate=True)
            self.extra = base64.b64decode(document["m_ExtraDataString"], validate=True)
            count, position = integer(bucket_data, 0), 4
            if count < 0 or count != integer(key_data, 0) or count > len(bucket_data) // 8:
                raise ToolError("Addressables 键和桶的数量无效。")
            self.keys, self.buckets = [], []
            for _ in range(count):
                key_offset, length = integer(bucket_data, position), integer(bucket_data, position + 4)
                position += 8
                values = table(bucket_data, position, length * 4)
                self.keys.append(string_key(key_data, key_offset))
                self.buckets.append(tuple(v[0] for v in struct.iter_unpack("<i", values)))
                position += length * 4
            count = integer(entry_data, 0)
            values = table(entry_data, 4, count * 28)
            self.entries = list(struct.iter_unpack("<7i", values))
            self._validate()
        except (KeyError, ValueError, TypeError, UnicodeError, struct.error) as exc:
            raise ToolError(f"Addressables 目录格式损坏：{exc}") from exc

    def _validate(self) -> None:
        """功能：校验所有表连接。入参：无。返回：无。异常：索引损坏。"""
        for bucket in self.buckets:
            if any(i < 0 or i >= len(self.entries) for i in bucket):
                raise ToolError("Addressables 桶引用了不存在的位置。")
        for internal, provider, dependency, _, extra, primary, resource_type in self.entries:
            ranges = [(internal, len(self.ids)), (provider, len(self.providers)),
                      (primary, len(self.keys)), (resource_type, len(self.types))]
            if any(i < 0 or i >= size for i, size in ranges):
                raise ToolError("Addressables 位置表索引越界。")
            if dependency < -1 or dependency >= len(self.buckets) or extra < -1 or extra >= len(self.extra):
                raise ToolError("Addressables 依赖或附加数据位置无效。")

    def internal_id(self, index: int) -> str:
        """功能：展开官方位置前缀。入参：位置表索引。返回：完整位置。异常：前缀无效。"""
        value = self.ids[self.entries[index][0]]
        if self.prefixes and "#" in value:
            prefix, tail = value.rsplit("#", 1)
            if prefix.isdecimal():
                number = int(prefix)
                if number >= len(self.prefixes):
                    raise ToolError("Addressables 位置前缀越界。")
                value = self.prefixes[number] + tail
        return value

    def options(self, offset: int) -> dict:
        """功能：读取资源包请求选项。入参：附加数据位置。返回：选项字典。异常：选项损坏。"""
        if offset < 0:
            raise ToolError("资源包缺少官方版本和缓存名称信息。")
        if table(self.extra, offset, 1) != b"\x07":
            raise ToolError("资源包选项不是支持的 JSON 对象。")
        position = offset + 1
        for _ in range(2):
            length = table(self.extra, position, 1)[0]
            table(self.extra, position + 1, length)
            position += 1 + length
        length = integer(self.extra, position)
        result = json.loads(table(self.extra, position + 4, length).decode("utf-16-le"))
        if not isinstance(result, dict):
            raise ToolError("资源包选项无效。")
        return result

    def dependencies(self, index: int) -> tuple[BundleSpec, ...]:
        """功能：追踪官方依赖闭包。入参：资源位置索引。返回：去重资源包列表。异常：依赖信息无效。"""
        visited, found, pending = set(), {}, [index]
        while pending:
            current = pending.pop()
            if current in visited:
                continue
            visited.add(current)
            entry = self.entries[current]
            provider = self.providers[entry[1]]
            if provider.endswith(".AssetBundleProvider"):
                options = self.options(entry[4])
                bundle = BundleSpec(self.internal_id(current), options["m_BundleName"], options["m_Hash"], options.get("m_Crc", 0))
                found[(bundle.name, bundle.version_hash)] = bundle
            if entry[2] >= 0:
                pending.extend(self.buckets[entry[2]])
        if not found:
            raise ToolError("资源没有可定位的官方 Bundle 依赖。")
        return tuple(found.values())

    def resource_names(self, prefix: str) -> list[str]:
        """功能：列出分类中的完整名称。入参：分类前缀。返回：排序名称列表。异常：无。"""
        return sorted(set(name for name in self.keys if name.startswith(prefix)))

    def asset(self, name: str) -> AssetSpec:
        """功能：解析唯一资源及其依赖。入参：完整名称。返回：资产描述。异常：缺失或多义。"""
        locations = set()
        for index, key in enumerate(self.keys):
            if key == name:
                locations.update(i for i in self.buckets[index] if self.providers[self.entries[i][1]].endswith(".BundledAssetProvider"))
        if len(locations) != 1:
            raise ToolError(f"资源位置不存在或不唯一：{name}", code="asset_location_invalid", stage="catalog",
                            context={"name": name, "matches": len(locations)})
        index = locations.pop()
        resource_type = self.types[self.entries[index][6]]["m_ClassName"]
        return AssetSpec(name, self.internal_id(index), resource_type, index, self.dependencies(index))
