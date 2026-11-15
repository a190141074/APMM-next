"""用途：执行分类增量同步和指定资源提取。作者：待填写。修改记录：2026-11-14，首次创建；补充分类异常的事务提交。"""

import gc
import importlib.metadata

from .catalog.locator import BundleLocator, validate_bundles
from .catalog.source import assert_unchanged
from .config import config_document
from .errors import Cancelled, CatalogChanged, ToolError, error_details
from .diagnostics import load_diagnostics, reconcile_diagnostics
from .io_utils import digest_json, sha256_file
from .output import Transaction
from .state import load_index, manager_map, reconcile, updated_state


def extraction_recipe() -> dict:
    """功能：标记影响解码的依赖版本。入参：无。返回：处理版本信息。异常：缺少依赖。"""
    return {"schema": 2, "decoder": "native_texture2ddecoder", "orientation": "unity_to_top_left", "mode": "RGBA", "png_compression": 3,
            **{name: importlib.metadata.version(name) for name in ("UnityPy", "Pillow", "texture2ddecoder")}}


def asset_fingerprint(asset, bundles, recipe: dict) -> str:
    """功能：计算产物有效性指纹。入参：资产、依赖和处理版本。返回：指纹。异常：无。"""
    return digest_json({"name": asset.name, "internal_id": asset.internal_id, "type": asset.resource_type,
                        "bundles": sorted((b.spec.name, b.spec.version_hash, b.sha256) for b in bundles), "recipe": recipe})


def sync_category(config, snapshot, catalog, category, state, runtime, emit, selected_name=None) -> dict:
    """功能：串行生成分类索引与图片。入参：配置、目录、分类、旧状态、运行上下文和目标。返回：结果报告。异常：取消或提交失败。"""
    from .unity.texture import extract_image

    names = category.names(catalog)
    if not names:
        raise ToolError(f"当前目录中没有分类 {category.key} 的资源，停止覆盖旧对应表。")
    if selected_name and selected_name not in names:
        raise ToolError(f"当前目录中不存在资源：{selected_name}")
    index = reconcile(load_index(config.output_dir, category), category, names, snapshot)
    targets = [row for row in index["resources"] if row["name"] in names and (not selected_name or row["name"] == selected_name)]
    previous_diagnostics = load_diagnostics(config.output_dir, category.key)
    recipe, locator = extraction_recipe(), BundleLocator(config)
    transaction = Transaction(config.output_dir)
    warnings, errors, checked_bundles = list(snapshot.warnings) + runtime.warnings, [], {}
    counts = {"extracted": 0, "reused": 0, "missing": 0, "failed": 0}
    try:
        for number, row in enumerate(targets, 1):
            runtime.checkpoint()
            assert_unchanged(snapshot)
            name = row["name"]
            stage, context = "catalog", {"name": name, "category": category.key}
            try:
                asset = catalog.asset(name)
                context.update({"container_key": asset.internal_id, "location_index": asset.location_index})
                stage, located = "locate", []
                for spec in asset.bundles:
                    context.update({"bundle_name": spec.name, "version_hash": spec.version_hash})
                    located.append(locator.locate(spec))
                bundles = tuple(located)
                context.pop("bundle_name", None)
                context.pop("version_hash", None)
                context["bundle_paths"] = [str(bundle.path) for bundle in bundles]
                for bundle in bundles:
                    checked_bundles[bundle.path] = bundle
                stage = "reuse"
                fingerprint = asset_fingerprint(asset, bundles, recipe)
                relative = f"outimg/{name}.png"
                existing = config.output_dir / relative
                artifact = row.get("artifact") or {}
                reusable = (artifact.get("fingerprint") == fingerprint and artifact.get("file") == relative
                            and existing.is_file() and sha256_file(existing) == artifact.get("sha256"))
                if reusable:
                    counts["reused"] += 1
                    action = "复用"
                    artifact["validated_catalog_fingerprint"] = snapshot.fingerprint
                else:
                    stage = "extract"
                    extracted = extract_image(asset, bundles, config.memory_budget)
                    stage = "output"
                    context.update({"serialized_file": extracted.serialized_file, "object_id": str(extracted.object_id)})
                    staged = transaction.png(relative, extracted.image)
                    artifact = {"file": relative, "sha256": sha256_file(staged), "fingerprint": fingerprint,
                                "width": extracted.image.width, "height": extracted.image.height, "mode": "RGBA",
                                "object_id": str(extracted.object_id), "serialized_file": extracted.serialized_file,
                                "texture_format": extracted.texture_format, "source_version": snapshot.version,
                                "validated_catalog_fingerprint": snapshot.fingerprint, "recipe": recipe}
                    warnings.extend(extracted.warnings)
                    extracted.image.close()
                    del extracted
                    counts["extracted"] += 1
                    action = "提取"
                row.pop("error_details", None)
                row.update({"status": "ready", "artifact": artifact, "error": None,
                            "location_index": asset.location_index, "internal_id": asset.internal_id,
                            "resource_type": asset.resource_type, "version": snapshot.version,
                            "bundles": [{"name": b.spec.name, "version_hash": b.spec.version_hash,
                                         "path": str(b.path), "sha256": b.sha256, "size": b.size} for b in bundles]})
            except (Cancelled, CatalogChanged):
                raise
            except Exception as exc:
                transaction.discard(f"outimg/{name}.png")
                kind = "missing" if isinstance(exc, FileNotFoundError) else "failed"
                counts[kind] += 1
                row.update({"status": kind, "error": str(exc), "version": snapshot.version})
                details = {"name": name, "status": kind, **error_details(exc, stage=stage, context=context)}
                row["error_details"] = details
                errors.append(details)
                action = "缺失" if kind == "missing" else "失败"
            emit(f"[{number}/{len(targets)}] {action} {name}")
            gc.collect()
            runtime.pause(config.delay_ms)
        current = [row for row in index["resources"] if row["status"] != "retired"]
        new_state = updated_state(state, snapshot, index, not selected_name)
        diagnostic_document, diagnostic_summary = reconcile_diagnostics(
            previous_diagnostics, category.key, snapshot, errors, [row["name"] for row in targets], names, not selected_name)
        transaction.json(f"diagnostics/{category.key}.json", diagnostic_document)
        transaction.json(f"index/{category.key}.json", index)
        transaction.json(f"maps/{category.key}.json", manager_map(index))
        transaction.json("config.json", config_document(config))
        transaction.json("state.json", new_state)

        def checkpoint() -> None:
            """功能：检查提交的一致性。入参：无。返回：无。异常：取消、目录或依赖变化。"""
            runtime.checkpoint()
            assert_unchanged(snapshot)
            validate_bundles(checked_bundles.values())

        transaction.commit(checkpoint)
        return {"status": "partial" if errors else "ok", "category": category.key,
                "version": snapshot.version, "catalog_hash": snapshot.catalog_hash,
                "catalog_fingerprint": snapshot.fingerprint, "total": len(current), "processed": len(targets),
                "ready": sum(row["status"] == "ready" for row in current),
                "pending": sum(row["status"] in ("pending", "stale") for row in current),
                "retired": len(index["resources"]) - len(current), **counts,
                "output_dir": str(config.output_dir), "outimg_dir": str(config.output_dir / "outimg"),
                "map_path": str(config.output_dir / "maps" / f"{category.key}.json"),
                "index_path": str(config.output_dir / "index" / f"{category.key}.json"),
                "diagnostics": diagnostic_summary,
                "diagnostics_path": str(config.output_dir / "diagnostics" / f"{category.key}.json"),
                "errors": errors, "warnings": list(dict.fromkeys(warnings)), **runtime.metrics()}
    finally:
        transaction.cleanup()

