"""用途：维护分类独立状态及动态对应表。作者：待填写。修改记录：2026-11-14，首次创建。"""

import copy
import time

from .errors import ToolError
from .io_utils import load_json


def load_state(root) -> dict:
    """功能：读取同步状态。入参：输出目录。返回：状态对象。异常：版本或结构无效。"""
    state = load_json(root / "state.json", {"schema_version": 1, "categories": {}})
    if state.get("schema_version") != 1 or not isinstance(state.get("categories"), dict):
        raise ToolError("同步状态的格式版本或分类结构无效。")
    return state


def load_index(root, category) -> dict:
    """功能：读取分类索引。入参：输出目录和分类。返回：索引对象。异常：结构损坏。"""
    index = load_json(root / "index" / f"{category.key}.json", {"schema_version": 1, "resources": []})
    if index.get("schema_version") != 1 or not isinstance(index.get("resources"), list):
        raise ToolError("分类索引格式无效。")
    names = [row.get("name") for row in index["resources"]]
    if any(not isinstance(name, str) for name in names) or len(set(names)) != len(names):
        raise ToolError("分类索引包含缺失或重复名称。")
    return index


def reconcile(previous: dict, category, names: list[str], snapshot) -> dict:
    """功能：保留旧产物并建立当前索引。入参：旧索引、分类、名称和快照。返回：新索引。异常：名称无效。"""
    old = {row["name"]: row for row in previous["resources"]}
    rows = []
    same_catalog = previous.get("catalog_fingerprint") == snapshot.fingerprint
    for name in names:
        row = copy.deepcopy(old.get(name, {}))
        row.update({"name": name, **category.parse_metadata(name)})
        if not same_catalog or row.get("status") != "ready":
            row["status"] = "stale" if row.get("artifact") else "pending"
        rows.append(row)
    for name in sorted(set(old) - set(names)):
        row = copy.deepcopy(old[name])
        row["status"] = "retired"
        rows.append(row)
    return {"schema_version": 1, "category": category.key, "version": snapshot.version,
            "catalog_fingerprint": snapshot.fingerprint, "catalog_hash": snapshot.catalog_hash,
            "source_catalog": str(snapshot.path), "resources": rows}


def updated_state(previous: dict, snapshot, index: dict, full_sync: bool) -> dict:
    """功能：更新独立分类状态。入参：旧状态、快照、索引和同步范围。返回：新状态。异常：无。"""
    state = copy.deepcopy(previous)
    state.update({"accepted_catalog_fingerprint": snapshot.fingerprint,
                  "accepted_catalog_hash": snapshot.catalog_hash, "accepted_version": snapshot.version})
    category = index["category"]
    entry = state["categories"].setdefault(category, {})
    current = [row for row in index["resources"] if row["status"] != "retired"]
    ready = sum(row["status"] == "ready" for row in current)
    entry.update({"last_attempt_fingerprint": snapshot.fingerprint, "last_attempt_unix": int(time.time()),
                  "total": len(current), "ready": ready, "complete": ready == len(current)})
    if full_sync and ready == len(current):
        entry["last_complete_fingerprint"] = snapshot.fingerprint
        entry["last_complete_unix"] = int(time.time())
    return state


def manager_map(index: dict) -> list[dict]:
    """功能：输出管理器兼容对应表。入参：分类索引。返回：已验证原图条目。异常：无。"""
    return [{"heroId": row["entity_id"], "section": row["category"], "canonical": row["name"],
             "previewFile": row["artifact"]["file"].removeprefix("outimg/"),
             "sha256": row["artifact"]["sha256"], "width": row["artifact"]["width"],
             "height": row["artifact"]["height"], "groupKey": row.get("group_key", row["name"])}
            for row in index["resources"] if row["status"] == "ready" and row.get("artifact")]

