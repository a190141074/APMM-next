"""用途：组装只读资产诊断和一致性检查。作者：待填写。修改记录：2026-11-14，新增单资源检查服务。"""

from .catalog.locator import BundleLocator, validate_bundles
from .catalog.source import assert_unchanged
from .categories.registry import identify_category
from .errors import ToolError, error_details
from .unity.inspection import describe_asset
from .unity.session import open_session


def inspect_asset(config, snapshot, catalog, name: str, runtime) -> dict:
    """功能：只读检查一个已注册资源。入参：配置、目录快照、目录、完整名称和运行上下文。返回：结构报告。异常：缺失、损坏或变化。"""
    category = identify_category(name)
    context = {"name": name, "category": category.key}
    try:
        runtime.checkpoint()
        assert_unchanged(snapshot)
        asset = catalog.asset(name)
        locator, located = BundleLocator(config), []
        for spec in asset.bundles:
            runtime.checkpoint()
            assert_unchanged(snapshot)
            located.append(locator.locate(spec))
        bundles = tuple(located)
        session = open_session(bundles, config.memory_budget)
        description = describe_asset(session, asset, bundles)
        warnings = list(session.warnings)
        del session
        runtime.checkpoint()
        assert_unchanged(snapshot)
        validate_bundles(bundles)
        return {"status": "ok", **context, "resource_type": asset.resource_type,
                "category_sync_supported": category.supported, "location_index": asset.location_index,
                "dependencies": [{"name": b.spec.name, "version_hash": b.spec.version_hash,
                                  "path": str(b.path), "size": b.size, "sha256": b.sha256} for b in bundles],
                **description, "warnings": list(dict.fromkeys([*snapshot.warnings, *warnings, *runtime.warnings])),
                **runtime.metrics()}
    except ToolError as exc:
        exc.context = {**context, **exc.context}
        exc.stage = exc.stage or "inspect"
        raise
    except OSError as exc:
        details = error_details(exc, stage="inspect", context=context)
        raise ToolError(details["message"], code=details["code"], stage=details["stage"], context=details["context"]) from exc
