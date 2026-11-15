"""用途：组装命令、版本确认和同步服务。作者：待填写。修改记录：2026-11-14，首次创建；补充只读诊断分发。"""

import sys
from typing import Callable

from .catalog.parser import Catalog
from .catalog.source import load_current
from .categories.registry import REGISTRY, category_for_name, require_category, identify_category, expand_categories
from .config import resolve_config
from .errors import Cancelled, ConfirmationRequired
from .output import OutputLock
from .state import load_state


def confirm_update(args, snapshot, state: dict, read_answer: Callable[[str], str] = input) -> None:
    """功能：确认目录变化后是否更新。入参：参数、快照、旧状态和输入函数。返回：无。异常：需确认或取消。"""
    previous = state.get("accepted_catalog_fingerprint")
    if not previous or previous == snapshot.fingerprint or args.yes:
        return
    if args.json or not sys.stdin.isatty():
        raise ConfirmationRequired("游戏资源目录已变化；请确认更新后使用 --yes 重新运行。")
    message = (f"检测到游戏资源目录变化：{state.get('accepted_version', '未知')} → {snapshot.version}；"
               f"目录哈希：{state.get('accepted_catalog_hash') or '安装目录'} → {snapshot.catalog_hash or '安装目录'}。"
               "是否更新所选资源？[y/N] ")
    try:
        answer = read_answer(message).strip().lower()
    except (EOFError, KeyboardInterrupt) as exc:
        raise Cancelled("已取消更新，原有图片与状态保持不变。") from exc
    if answer not in ("y", "yes", "是", "更新"):
        raise Cancelled("已拒绝更新，原有图片与状态保持不变。")


def dispatch(args, runtime, emit) -> dict:
    """功能：执行已解析的命令。入参：参数、运行上下文和进度函数。返回：结构化报告。异常：命令处理失败。"""
    if args.command == "categories":
        return {"status": "ok", "categories": [{"key": c.key, "label": c.label, "prefix": c.prefix,
                                                  "supported": c.supported} for c in REGISTRY.values()]}
    if args.command == "inspect":
        category = identify_category(args.name)
    elif args.command == "extract":
        category = category_for_name(args.name)
        category.parse_metadata(args.name)
        categories = [category]
    elif args.command in ("sync", "list"):
        categories = expand_categories(args.category)
        category = categories[0]
    else:
        category = None
    config = resolve_config(args)
    snapshot, document = load_current(config)
    catalog = Catalog(document)
    state = load_state(config.output_dir)
    needs_update = state.get("accepted_catalog_fingerprint") != snapshot.fingerprint
    common = {"version": snapshot.version, "catalog_hash": snapshot.catalog_hash,
              "catalog_fingerprint": snapshot.fingerprint, "catalog_path": str(snapshot.path),
              "catalog_kind": snapshot.kind, "needs_update": needs_update,
              "needs_confirmation": bool(state.get("accepted_catalog_fingerprint") and needs_update),
              "warnings": list(snapshot.warnings), "output_dir": str(config.output_dir)}
    if args.command == "inspect":
        from .inspection import inspect_asset
        return {**common, "needs_confirmation": False, **inspect_asset(config, snapshot, catalog, args.name, runtime)}
    if args.command == "check":
        return {"status": "ok", **common, "categories": state["categories"],
                "available": {c.key: (2 if c.key == 'cursor' and (config.data_dir / 'data.unity3d').is_file() else len(c.names(catalog))) for c in REGISTRY.values()},
                **runtime.metrics()}
    if args.command == "list":
        names = sorted({name for c in categories for name in (('cursor_up', 'cursor_down') if c.key == 'cursor' else c.names(catalog))})
        return {"status": "ok", **common, "category": category.key, "total": len(names), "resources": names,
                **runtime.metrics()}
    with OutputLock(config.output_dir):
        state = load_state(config.output_dir)
        confirm_update(args, snapshot, state)
        from .sync import sync_category
        from .builtin_sync import sync_cursors
        results = []
        selected_name = args.name if args.command == 'extract' else None
        for current in ([category] if selected_name else categories):
            runtime.checkpoint()
            state = load_state(config.output_dir)
            if current.key == 'cursor':
                result = sync_cursors(config, snapshot, state, runtime, emit, selected_name)
            elif not current.names(catalog) and len(categories) > 1:
                result = {'status': 'ok', 'category': current.key, 'skipped': True, 'reason': '当前目录中没有该类资源'}
            else:
                result = sync_category(config, snapshot, catalog, current, state, runtime, emit, selected_name)
            results.append(result)
        if len(results) == 1:
            return results[0]
        return {'status': 'partial' if any(r['status'] != 'ok' for r in results) else 'ok',
                'categories': results, **runtime.metrics()}

