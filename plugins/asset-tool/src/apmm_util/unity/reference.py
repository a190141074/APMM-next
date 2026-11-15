"""用途：解析容器和跨文件 PPtr 引用。作者：待填写。修改记录：2026-11-14，补充精确定位错误信息。"""

from ..errors import ToolError
from ..models import AssetSpec, LocatedBundle
from .session import AssetSession, basename


def resolve_pointer(session: AssetSession, source, pointer: dict):
    """功能：按来源文件和完整对象号解析引用。入参：会话、来源文件和指针。返回：对象读取器。异常：指针或目标无效。"""
    context = {"serialized_file": str(getattr(source, "name", ""))}
    try:
        file_id, object_id = int(pointer["m_FileID"]), int(pointer["m_PathID"])
    except (KeyError, TypeError, ValueError) as exc:
        raise ToolError("PPtr 缺少有效文件号或对象号。", code="pointer_invalid", stage="reference", context=context) from exc
    context.update({"file_id": file_id, "object_id": str(object_id)})
    if object_id == 0:
        raise ToolError("资源指向空对象。", code="pointer_null", stage="reference", context=context)
    if file_id < 0 or not -(1 << 63) <= object_id < (1 << 63):
        raise ToolError("资源指针的文件号或对象号无效。", code="pointer_invalid", stage="reference", context=context)
    target = source
    if file_id:
        if file_id > len(source.externals):
            raise ToolError("PPtr 外部文件号越界。", code="external_index_invalid", stage="reference", context=context)
        external = source.externals[file_id - 1]
        context["external_file"] = str(external.path)
        target = session.files.get(basename(external.path))
        if target is None:
            raise ToolError(f"官方依赖中缺少外部序列化文件：{external.path}",
                            code="external_file_missing", stage="reference", context=context)
    reader = target.objects.get(object_id)
    if reader is None:
        raise ToolError(f"序列化文件缺少完整对象标识：{object_id}",
                        code="object_missing", stage="reference", context=context)
    return reader


def target_object(session: AssetSession, asset: AssetSpec, bundles: tuple[LocatedBundle, ...]):
    """功能：通过官方容器键定位唯一纹理。入参：会话、资产和包。返回：纹理对象。异常：容器多义或名称不符。"""
    found = {}
    expected_names = {bundle.spec.name.removesuffix(".bundle") for bundle in bundles}
    context = {"name": asset.name, "container_key": asset.internal_id}
    for source in session.files.values():
        for reader in source.objects.values():
            if reader.type.name != "AssetBundle":
                continue
            tree = reader.read_typetree()
            bundle_name = tree.get("m_AssetBundleName", tree.get("m_Name", "")).removesuffix(".bundle")
            if bundle_name not in expected_names:
                raise ToolError(f"实际 Bundle 名称与官方目录不一致：{bundle_name}", code="bundle_name_mismatch",
                                stage="reference", context={**context, "bundle_name": bundle_name})
            for key, entry in tree.get("m_Container", []):
                if str(key) == asset.internal_id:
                    target = resolve_pointer(session, source, entry["asset"])
                    # 小型数字容器键会在不同包中重复，完整资源名可消除合法的键冲突。
                    if target.type.name != "Texture2D" or target.peek_name() != asset.name:
                        continue
                    found[(basename(target.assets_file.name), target.path_id)] = target
    if len(found) != 1:
        code = "container_ambiguous" if found else "container_target_missing"
        raise ToolError(f"容器中未找到唯一匹配的 Texture2D：{asset.name}", code=code,
                        stage="reference", context={**context, "matches": len(found)})
    return next(iter(found.values()))
