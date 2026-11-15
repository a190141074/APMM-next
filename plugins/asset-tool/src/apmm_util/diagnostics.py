"""用途：按具体资源维护分类异常和变化清单。作者：待填写。修改记录：2026-11-14，新增事务内异常记录。"""

import copy
import time

from .errors import ToolError
from .io_utils import load_json


def issue_key(issue: dict) -> tuple:
    """功能：取得稳定异常身份。入参：异常记录。返回：完整名称、阶段和错误码。异常：字段缺失。"""
    return issue["name"], issue["stage"], issue["code"]


def load_diagnostics(root, category: str) -> dict | None:
    """功能：读取旧分类异常基线。入参：数据根和分类。返回：旧记录，缺失时为空。异常：记录损坏。"""
    path = root / "diagnostics" / f"{category}.json"
    previous = load_json(path)
    if previous is None and not path.exists():
        return None
    valid = (isinstance(previous, dict) and previous.get("schema_version") == 1
             and previous.get("category") == category and isinstance(previous.get("active"), list))
    if valid:
        rows = previous["active"]
        valid = all(isinstance(row, dict) and all(isinstance(row.get(k), str) for k in ("name", "stage", "code")) for row in rows)
        if valid:
            valid = len({issue_key(row) for row in rows}) == len(rows)
    if not valid:
        raise ToolError("分类异常记录格式无效，停止覆盖旧记录。", code="diagnostics_invalid", stage="diagnostics",
                        context={"category": category, "path": str(path)})
    return previous


def reconcile_diagnostics(previous, category: str, snapshot, errors: list[dict], targets: list[str],
                          current_names: list[str], full_sync: bool, now=None) -> tuple[dict, dict]:
    """功能：比较异常身份并只更新本次范围。入参：旧记录、分类、快照、异常、目标、现存名称、同步范围及时间。返回：新记录和变化报告。异常：无。"""
    now = int(time.time()) if now is None else now
    old = {issue_key(row): copy.deepcopy(row) for row in (previous or {}).get("active", [])}
    touched, current = set(targets), set(current_names)
    active = {key: row for key, row in old.items() if row["name"] not in touched
              and (not full_sync or row["name"] in current)}
    new, continuing = [], []
    for error in errors:
        key = issue_key(error)
        row = {**copy.deepcopy(error), "first_seen_unix": old.get(key, {}).get("first_seen_unix", now),
               "last_seen_unix": now, "catalog_fingerprint": snapshot.fingerprint}
        active[key] = row
        (continuing if key in old else new).append(copy.deepcopy(row))
    failed_names = {row["name"] for row in errors}
    resolved = []
    for key in sorted(set(old) - set(active)):
        row = old[key]
        reason = "resource_retired" if row["name"] not in current else ("error_changed" if row["name"] in failed_names else "recovered")
        resolved.append({**row, "resolved_unix": now, "resolution": reason})
    document = {"schema_version": 1, "category": category, "catalog_fingerprint": snapshot.fingerprint,
                "initialized_unix": (previous or {}).get("initialized_unix", now), "last_attempt_unix": now,
                "active": [active[key] for key in sorted(active)]}
    summary = {"baseline_available": previous is not None, "active_count": len(active),
               "new": new, "continuing": continuing, "resolved": resolved}
    return document, summary
