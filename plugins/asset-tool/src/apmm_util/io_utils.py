"""用途：提供有界读取及文件指纹。作者：待填写。修改记录：2026-11-14，首次创建；补充资源预算错误码。"""

import hashlib
import json
from pathlib import Path
from typing import Any

from .errors import ToolError
from .models import FileStamp


def stamp(path: Path) -> FileStamp:
    """功能：读取文件身份。入参：文件路径。返回：大小、时间和身份。异常：文件访问异常。"""
    info = path.stat()
    return FileStamp(info.st_size, info.st_mtime_ns, info.st_ino)


def read_bounded(path: Path, limit: int) -> bytes:
    """功能：限制单次文件读取。入参：路径和字节上限。返回：文件内容。异常：超限或访问失败。"""
    if path.stat().st_size > limit:
        raise ToolError(f"文件超过资源预算：{path}", code="resource_budget_exceeded", context={"path": str(path)})
    data = bytearray()
    with path.open("rb") as stream:
        while True:
            chunk = stream.read(min(65536, limit - len(data) + 1))
            if not chunk:
                break
            data.extend(chunk)
            if len(data) > limit:
                raise ToolError(f"文件写入后超过资源预算：{path}", code="resource_budget_exceeded", context={"path": str(path)})
    if len(data) > limit:
        raise ToolError(f"文件写入后超过资源预算：{path}", code="resource_budget_exceeded", context={"path": str(path)})
    return bytes(data)


def sha256_file(path: Path) -> str:
    """功能：流式计算内容指纹。入参：文件路径。返回：十六进制指纹。异常：文件访问异常。"""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def digest_json(value: Any) -> str:
    """功能：计算结构化内容指纹。入参：可序列化对象。返回：稳定指纹。异常：序列化失败。"""
    data = json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()
    return hashlib.sha256(data).hexdigest()


def load_json(path: Path, default: Any = None) -> Any:
    """功能：读取小型状态文件。入参：路径和缺失默认值。返回：解析结果。异常：损坏或访问失败。"""
    if not path.exists():
        return default
    try:
        return json.loads(read_bounded(path, 32 * 1024 * 1024).decode("utf-8-sig"))
    except (ValueError, UnicodeError) as exc:
        raise ToolError(f"JSON 文件损坏：{path}；{exc}") from exc

