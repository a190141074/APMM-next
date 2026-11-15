"""用途：有界解压 UnityFS 并恢复序列化节点。作者：待填写。修改记录：2026-11-14，首次创建。"""

import lzma
import struct
from dataclasses import dataclass

from ..errors import ToolError
from .binary import Reader


@dataclass
class DecodedBundle:
    """保存节点数据与兼容性提示。"""

    files: dict[str, bytes]
    warnings: list[str]


def decompress(data: bytes, mode: int, expected: int, budget: int) -> bytes:
    """功能：按预算解压一个块。入参：压缩数据、格式、声明长度和预算。返回：内容。异常：损坏或超限。"""
    if expected < 0 or expected > budget:
        raise ToolError("资源块声明大小超过解压预算。")
    if mode == 0:
        result = data
    elif mode in (2, 3):
        import lz4.block
        result = lz4.block.decompress(data, uncompressed_size=expected)
    elif mode == 1:
        if len(data) < 5:
            raise ToolError("LZMA 数据头截断。")
        props, dictionary = struct.unpack("<BI", data[:5])
        if dictionary > budget or props >= 225:
            raise ToolError("LZMA 字典或参数超过资源预算。")
        remainder, lc = divmod(props, 9)
        pb, lp = divmod(remainder, 5)
        decoder = lzma.LZMADecompressor(format=lzma.FORMAT_RAW, filters=[{
            "id": lzma.FILTER_LZMA1, "dict_size": dictionary, "lc": lc, "lp": lp, "pb": pb}])
        result = decoder.decompress(data[5:], max_length=budget + 1)
        if not decoder.eof and len(result) <= budget:
            raise ToolError("LZMA 数据未完整结束。")
    else:
        raise ToolError(f"暂不支持 UnityFS 压缩格式：{mode}")
    if len(result) > budget:
        raise ToolError("实际解压内容超过资源预算。")
    return result


def serialized_size(data: bytes) -> int | None:
    """功能：读取序列化文件实际声明长度。入参：节点数据。返回：长度或空值。异常：无。"""
    if len(data) < 48:
        return None
    version = struct.unpack_from(">I", data, 8)[0]
    if version == 22:
        return struct.unpack_from(">Q", data, 24)[0]
    if 9 <= version < 22:
        return struct.unpack_from(">I", data, 4)[0]
    return None


def decode_bundle(raw: bytes, budget: int) -> DecodedBundle:
    """功能：解压 UnityFS 并校验节点范围。入参：文件内容和字节预算。返回：节点集合。异常：损坏或超限。"""
    if len(raw) >= budget:
        raise ToolError("压缩文件本身已超过资源预算。")
    reader = Reader(raw)
    if reader.cstring(32) != "UnityFS":
        raise ToolError("资源文件不是支持的 UnityFS Bundle。")
    version = reader.number("I")
    if version not in (6, 7, 8):
        raise ToolError(f"暂不支持 UnityFS 版本：{version}")
    reader.cstring()
    reader.cstring()
    size, compressed, expanded, flags = reader.number("Q"), reader.number("I"), reader.number("I"), reader.number("I")
    if size != len(raw):
        raise ToolError("UnityFS 文件长度不一致，文件可能正在写入或已经损坏。")
    if version >= 7:
        reader.align()
    if flags & 128:
        if compressed > len(raw):
            raise ToolError("UnityFS 元数据范围越界。")
        info_data = raw[size - compressed:]
    else:
        info_data = reader.take(compressed)
    info = Reader(decompress(info_data, flags & 63, expanded, min(budget, 4 * 1024 * 1024)))
    info.take(16)
    count = info.number("I")
    if count > (len(info.data) - info.position) // 10:
        raise ToolError("UnityFS 压缩块数量无效。")
    blocks = [(info.number("I"), info.number("I"), info.number("H")) for _ in range(count)]
    node_count = info.number("I")
    if node_count > (len(info.data) - info.position) // 21:
        raise ToolError("UnityFS 节点数量无效。")
    nodes = [(info.number("Q"), info.number("Q"), info.number("I"), info.cstring()) for _ in range(node_count)]
    if len({node[3] for node in nodes}) != len(nodes):
        raise ToolError("UnityFS 包含重复节点名称。")
    if flags & 512:
        reader.align()
    remaining = budget - len(raw) - len(info.data)
    if sum(block[0] for block in blocks) > remaining:
        raise ToolError("资源包声明的解压总量超过资源预算。")
    chunks, warnings = [], []
    for expected, length, mode in blocks:
        chunk = decompress(reader.take(length), mode & 63, expected, remaining)
        if len(chunk) != expected:
            warnings.append("UnityFS 块长度声明与实际序列化内容不同，已按实际数据校验。")
        chunks.append(chunk)
        remaining -= len(chunk)
    payload = b"".join(chunks)
    files = {}
    for offset, length, _, name in nodes:
        if offset + length > len(payload):
            raise ToolError(f"UnityFS 节点范围越界：{name}")
        actual = serialized_size(payload[offset:offset + 48])
        if actual and actual != length and len(nodes) == 1 and offset == 0 and actual == len(payload):
            # 当前安装目录的目录包保留了旧节点长度，文件内部的完整长度可用于安全恢复。
            length = actual
            warnings.append("已恢复目录包中旧节点长度声明，完整文本将继续由 UnityPy 校验。")
        files[name] = payload[offset:offset + length]
    return DecodedBundle(files, warnings)


def catalog_document(raw: bytes, budget: int) -> tuple[dict, list[str]]:
    """功能：从目录包读取唯一文本目录。入参：包内容和预算。返回：目录对象与提示。异常：目录无效。"""
    import json
    from .backend import prepare_backend

    UnityPy = prepare_backend()

    decoded = decode_bundle(raw, budget)
    documents = []
    for name, data in decoded.files.items():
        environment = UnityPy.load(data)
        for obj in environment.objects:
            if obj.type.name == "TextAsset":
                script = obj.read_typetree().get("m_Script")
                if isinstance(script, bytes):
                    script = script.decode("utf-8-sig")
                try:
                    value = json.loads(script)
                except (ValueError, TypeError):
                    continue
                if isinstance(value, dict) and "m_EntryDataString" in value:
                    documents.append(value)
    if len(documents) != 1:
        raise ToolError("目录包中未找到唯一有效的 Addressables 目录。")
    return documents[0], decoded.warnings

