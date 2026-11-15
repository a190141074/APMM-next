"""用途：选择当前版本目录并检测处理期间的变化。作者：待填写。修改记录：2026-11-14，首次创建。"""

import hashlib
import json
import re

from ..errors import CatalogChanged, ToolError
from ..io_utils import digest_json, read_bounded, stamp
from ..models import CatalogSnapshot, Config
from ..unity.bundle import catalog_document


def current_version(config: Config) -> str:
    """功能：从运行设置取得目录版本。入参：运行配置。返回：版本字符串。异常：版本不唯一。"""
    settings = json.loads(read_bounded(config.data_dir / "StreamingAssets/aa/settings.json", 4 * 1024 * 1024))
    versions = set()
    for location in settings.get("m_CatalogLocations", []):
        match = re.search(r"catalog_([^/\\{}]+)\.hash$", location.get("m_InternalId", ""))
        if match:
            versions.add(match[1])
    if len(versions) != 1:
        raise ToolError("无法从 Addressables 设置确定唯一目录版本。")
    return versions.pop()


def read_hash(path) -> str:
    """功能：读取目录哈希标记。入参：哈希文件路径。返回：规范化标记。异常：标记无效。"""
    value = read_bounded(path, 1024).decode("utf-8-sig").strip()
    if not re.fullmatch(r"[a-fA-F0-9]{32}", value):
        raise ToolError(f"目录哈希标记无效：{path}")
    return value.lower()


def load_current(config: Config) -> tuple[CatalogSnapshot, dict]:
    """功能：读取当前目录的一致性快照。入参：运行配置。返回：快照与目录。异常：损坏或写入中。"""
    settings_path = config.data_dir / "StreamingAssets/aa/settings.json"
    settings_stamp = stamp(settings_path)
    version = current_version(config)
    cached_dir = config.locallow_dir / "com.unity.addressables"
    cached, hash_path = cached_dir / f"catalog_{version}.json", cached_dir / f"catalog_{version}.hash"
    warnings = []
    if cached.exists() != hash_path.exists():
        raise CatalogChanged("当前版本的目录与哈希文件尚未成对写入，请等待游戏更新完成后重试。")
    if cached.is_file() and hash_path.is_file():
        selected, kind = cached, "cached_json"
        token = read_hash(hash_path)
    else:
        selected, kind = config.data_dir / "StreamingAssets/aa/catalog.bundle", "installed_bundle"
        hash_path, token = None, ""
        warnings.append("未发现当前版本的有效更新目录，使用安装目录；尚未下载的新增资源可能不可见。")
    before = stamp(selected)
    raw = read_bounded(selected, min(config.memory_budget // 2, 64 * 1024 * 1024))
    try:
        if kind == "cached_json":
            document = json.loads(raw.decode("utf-8-sig"))
        else:
            document, recovered = catalog_document(raw, config.memory_budget)
            warnings.extend(recovered)
    except (ValueError, UnicodeError) as exc:
        raise ToolError(f"目录无法解析，请确认游戏更新完成：{selected}；{exc}") from exc
    after = stamp(selected)
    if before != after or stamp(settings_path) != settings_stamp or (hash_path and read_hash(hash_path) != token):
        raise CatalogChanged("读取期间目录已变化，请等待游戏更新完成后重试。")
    if not isinstance(document, dict):
        raise ToolError("资源目录不是 JSON 对象。")
    content_hash = hashlib.sha256(raw).hexdigest()
    fingerprint = digest_json({"path": str(selected), "version": version, "hash": token, "sha256": content_hash})
    pending = (cached, cached_dir / f"catalog_{version}.hash") if kind == "installed_bundle" else ()
    snapshot = CatalogSnapshot(selected, hash_path, version, token, content_hash, fingerprint, after, kind,
                               tuple(warnings), settings_path, settings_stamp, pending)
    return snapshot, document


def assert_unchanged(snapshot: CatalogSnapshot) -> None:
    """功能：确认目录仍与快照一致。入参：目录快照。返回：无。异常：目录变化或消失。"""
    try:
        if stamp(snapshot.path) != snapshot.stamp:
            raise CatalogChanged("游戏目录在操作期间发生变化，已取消本次提交。")
        if snapshot.hash_path and read_hash(snapshot.hash_path) != snapshot.catalog_hash:
            raise CatalogChanged("游戏目录哈希在操作期间发生变化，已取消本次提交。")
        if snapshot.settings_path and stamp(snapshot.settings_path) != snapshot.settings_stamp:
            raise CatalogChanged("游戏运行设置在操作期间发生变化，已取消本次提交。")
        if any(path.exists() for path in snapshot.pending_cache_paths):
            raise CatalogChanged("游戏已生成更新目录，安装目录快照失效，请重新运行。")
    except OSError as exc:
        raise CatalogChanged("操作期间目录文件消失或暂时无法读取，已取消本次提交。") from exc

