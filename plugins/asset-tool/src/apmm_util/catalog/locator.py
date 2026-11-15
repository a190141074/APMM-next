"""用途：根据官方名称和版本定位本地资源包。作者：待填写。修改记录：2026-11-14，首次创建；补充定位错误信息和依赖变化检查。"""

import re
from pathlib import Path

from ..config import safe_component
from ..errors import CatalogChanged, ToolError
from ..io_utils import sha256_file, stamp
from ..models import BundleSpec, Config, LocatedBundle


class BundleLocator:
    """在本次运行中复用小型内容指纹缓存。"""

    def __init__(self, config: Config):
        """功能：创建定位器。入参：运行配置。返回：无。异常：无。"""
        self.config = config
        self._hashes = {}

    def locate(self, spec: BundleSpec) -> LocatedBundle:
        """功能：定位指定版本资源包。入参：官方包描述。返回：文件及指纹。异常：缺失或冲突。"""
        if not safe_component(spec.name) or not re.fullmatch(r"[a-fA-F0-9]{32}", spec.version_hash):
            raise ToolError("资源目录中的 Bundle 名称或版本哈希无效。", code="bundle_identity_invalid", stage="locate")
        runtime = "{UnityEngine.AddressableAssets.Addressables.RuntimePath}"
        if spec.internal_id.startswith(runtime):
            base = (self.config.data_dir / "StreamingAssets/aa").resolve()
            tail = spec.internal_id[len(runtime):].replace("\\", "/").lstrip("/")
            candidate = (base / tail).resolve()
            if not candidate.is_relative_to(base):
                raise ToolError("资源目录的位置越出了 Addressables 根目录。")
            if candidate.is_file():
                return self._describe(spec, candidate)
        candidates = list(dict.fromkeys(root / spec.name / spec.version_hash / "__data" for root in self.config.cache_dirs))
        present = [self._describe(spec, path) for path in candidates if path.is_file()]
        if not present:
            raise FileNotFoundError(f"所需版本尚未下载：{spec.name}/{spec.version_hash}；请先由游戏完成资源更新。")
        if len({item.sha256 for item in present}) != 1:
            raise ToolError(f"多个缓存中同一 Bundle 版本的内容不同：{spec.name}；请用 --cache-dir 明确资源来源。",
                            code="cache_conflict", stage="locate", context={"bundle_name": spec.name, "version_hash": spec.version_hash})
        return present[0]

    def _describe(self, spec: BundleSpec, path: Path) -> LocatedBundle:
        """功能：计算资源包身份。入参：包描述和路径。返回：定位结果。异常：读取失败或写入中。"""
        context = {"bundle_path": str(path), "bundle_name": spec.name, "version_hash": spec.version_hash}
        try:
            before = stamp(path)
            if before.size >= self.config.memory_budget:
                raise ToolError(f"资源包超过读取预算：{path}", code="resource_budget_exceeded", stage="locate", context=context)
            cache_key = (path, before)
            if cache_key not in self._hashes:
                self._hashes[cache_key] = sha256_file(path)
            if stamp(path) != before:
                raise CatalogChanged(f"资源包正在更新，请稍后重试：{path}", stage="locate", context=context)
        except OSError as exc:
            raise CatalogChanged(f"资源包在定位期间无法访问：{path}", stage="locate", context=context) from exc
        return LocatedBundle(spec, path, self._hashes[cache_key], before.size, before)



def validate_bundles(bundles) -> None:
    """功能：确认已定位依赖未变化。入参：定位包集合。返回：无。异常：文件变化或消失。"""
    for bundle in bundles:
        try:
            unchanged = stamp(bundle.path) == bundle.stamp
        except OSError as exc:
            raise CatalogChanged(f"资源包在处理期间消失，已取消提交：{bundle.path}", stage="locate",
                                 context={"bundle_path": str(bundle.path)}) from exc
        if not unchanged:
            raise CatalogChanged(f"资源包在处理期间发生变化，已取消提交：{bundle.path}", stage="locate",
                                 context={"bundle_path": str(bundle.path)})
