"""用途：描述目标对象、布局类型和纹理来源。作者：待填写。修改记录：2026-11-14，新增只读结构诊断。"""

from collections import Counter

from ..errors import ToolError
from .pixels import SUPPORTED_FORMATS
from .reference import target_object
from .texture import stream_location


def describe_asset(session, asset, bundles) -> dict:
    """功能：读取结构和纹理元数据而不解码像素。入参：会话、官方资产和依赖。返回：诊断字段。异常：目标或流数据无效。"""
    counts = Counter(reader.type.name for source in session.files.values() for reader in source.objects.values())
    layouts = {kind: counts[kind] for kind in ("RectTransform", "Canvas", "CanvasRenderer") if counts[kind]}
    result = {"object_types": dict(sorted(counts.items())), "loaded_bytes": session.loaded_bytes,
              "layout": {"has_layout_objects": bool(layouts), "object_types": layouts,
                         "composition_required": False, "reason": "目标为独立纹理，原图不需要布局合成。"},
              "pixels_decoded": False}
    if asset.resource_type != "UnityEngine.Texture2D":
        raise ToolError(f"当前只读对象诊断仅支持 Texture2D，实际类型：{asset.resource_type}",
                        code="asset_type_unsupported", stage="inspect", context={"resource_type": asset.resource_type})
    reader = target_object(session, asset, bundles)
    texture = reader.read()
    if texture.m_Width <= 0 or texture.m_Height <= 0:
        raise ToolError("纹理尺寸无效。", code="texture_dimensions_invalid", stage="inspect")
    from UnityPy.enums import TextureFormat
    try:
        format_name = TextureFormat(texture.m_TextureFormat).name
    except ValueError:
        format_name = "未知格式"
    result.update({"object": {"type": reader.type.name, "name": texture.m_Name,
                              "object_id": str(reader.path_id), "serialized_file": reader.assets_file.name,
                              "container_key": asset.internal_id},
                   "texture": {"width": texture.m_Width, "height": texture.m_Height,
                               "format": texture.m_TextureFormat, "format_name": format_name,
                               "decoder_supported": texture.m_TextureFormat in SUPPORTED_FORMATS,
                               "image_count": texture.m_ImageCount, "dimension": texture.m_TextureDimension,
                               "stream": stream_location(session, texture)}})
    return result
