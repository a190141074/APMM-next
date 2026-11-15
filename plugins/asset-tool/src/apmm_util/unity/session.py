"""用途：只加载目标资产所需的资源包。作者：待填写。修改记录：2026-11-14，首次创建；补充资源变化取消和定位上下文。"""

import hashlib
from dataclasses import dataclass

from ..errors import CatalogChanged, ToolError
from ..io_utils import read_bounded, stamp
from ..models import LocatedBundle
from .bundle import decode_bundle


@dataclass
class AssetSession:
    """保存一次提取的序列化文件、流数据和依赖来源。"""

    environment: object
    files: dict
    streams: dict
    bundle_sources: dict
    loaded_bytes: int
    warnings: list[str]


def basename(value: str) -> str:
    """功能：规范化 Unity 归档文件名。入参：归档路径。返回：小写末段文件名。异常：无。"""
    return value.replace("\\", "/").rsplit("/", 1)[-1].lower()


def open_session(bundles: tuple[LocatedBundle, ...], budget: int) -> AssetSession:
    """功能：有界加载明确依赖。入参：定位后的包和预算。返回：提取会话。异常：损坏、冲突或超限。"""
    from .backend import prepare_backend

    UnityPy = prepare_backend()
    from UnityPy.files import SerializedFile

    environment = UnityPy.Environment()
    files, streams, bundle_sources, warnings = {}, {}, {}, []
    loaded_bytes = 0
    for bundle in bundles:
        context = {"bundle_name": bundle.spec.name, "bundle_path": str(bundle.path), "version_hash": bundle.spec.version_hash}
        try:
            if stamp(bundle.path) != bundle.stamp:
                raise CatalogChanged(f"资源包在定位后发生变化：{bundle.path}", stage="bundle", context=context)
            raw = read_bounded(bundle.path, budget - loaded_bytes)
            if hashlib.sha256(raw).hexdigest() != bundle.sha256 or stamp(bundle.path) != bundle.stamp:
                raise CatalogChanged(f"资源包在读取期间发生变化：{bundle.path}", stage="bundle", context=context)
            decoded = decode_bundle(raw, budget - loaded_bytes)
        except OSError as exc:
            raise CatalogChanged(f"资源包在读取期间无法访问：{bundle.path}", stage="bundle", context=context) from exc
        except ToolError as exc:
            exc.context = {**context, **exc.context}
            exc.stage = exc.stage or "bundle"
            if exc.code == "processing_error" and not isinstance(exc, CatalogChanged):
                exc.code = "bundle_invalid"
            raise
        warnings.extend(decoded.warnings)
        for name, content in decoded.files.items():
            key = basename(name)
            if key in files or key in streams:
                raise ToolError(f"依赖资源包中存在同名序列化节点：{name}", code="serialized_file_conflict",
                                stage="bundle", context={**context, "serialized_file": name})
            node = environment.load_file(content, name=name)
            if isinstance(node, SerializedFile):
                files[key] = node
                bundle_sources[key] = bundle.path
            else:
                streams[key] = content
            loaded_bytes += len(content)
        del raw, decoded
    return AssetSession(environment, files, streams, bundle_sources, loaded_bytes, warnings)

