"""用途：读取纹理流并解码原尺寸透明图片。作者：待填写。修改记录：2026-11-14，复用流范围和缓冲预算校验。"""

from ..errors import ToolError
from ..models import AssetSpec, ExtractedImage, LocatedBundle
from .reference import target_object
from .session import AssetSession, basename, open_session
from .pixels import SUPPORTED_FORMATS, decode_pixels


def stream_location(session: AssetSession, texture) -> dict:
    """功能：验证像素来源范围而不复制数据。入参：会话与纹理。返回：来源、路径、偏移和长度。异常：流缺失或越界。"""
    info = getattr(texture, "m_StreamData", None)
    if info and (info.size or info.offset < 0):
        context = {"stream_path": info.path, "offset": info.offset, "size": info.size}
        data = session.streams.get(basename(info.path))
        if data is None:
            raise ToolError(f"官方依赖中缺少纹理数据流：{info.path}", code="texture_stream_missing", stage="stream", context=context)
        if info.offset < 0 or info.size < 0 or info.offset + info.size > len(data):
            raise ToolError("纹理数据流的偏移或长度越界。", code="texture_stream_range_invalid", stage="stream", context=context)
        return {"kind": "external", "path": info.path, "offset": info.offset, "size": info.size}
    data = getattr(texture, "image_data", None)
    if not data:
        raise ToolError("纹理既没有内置像素，也没有有效流数据。", code="texture_data_missing", stage="stream")
    return {"kind": "embedded", "path": "", "offset": 0, "size": len(data)}


def stream_bytes(session: AssetSession, texture) -> bytes:
    """功能：读取纹理的准确数据范围。入参：会话与纹理。返回：编码像素。异常：缺流或范围无效。"""
    location = stream_location(session, texture)
    if location["kind"] == "embedded":
        return bytes(texture.image_data)
    data = session.streams[basename(location["path"])]
    start = location["offset"]
    return data[start:start + location["size"]]


def validate_pixel_budget(width: int, height: int, loaded_bytes: int, budget: int) -> None:
    """功能：在分配解码缓冲前验证尺寸和预算。入参：宽高、已加载字节和预算。返回：无。异常：尺寸无效或超限。"""
    context = {"width": width, "height": height, "loaded_bytes": loaded_bytes, "budget": budget}
    if width <= 0 or height <= 0:
        raise ToolError("纹理尺寸无效。", code="texture_dimensions_invalid", stage="decode", context=context)
    if loaded_bytes + width * height * 12 > budget:
        raise ToolError("图像解码缓冲区超过资源预算。", code="resource_budget_exceeded", stage="decode", context=context)


def extract_image(asset: AssetSpec, bundles: tuple[LocatedBundle, ...], budget: int) -> ExtractedImage:
    """功能：提取目标原图。入参：资产、依赖和预算。返回：图像与对象标识。异常：格式无效或超限。"""
    if asset.resource_type != "UnityEngine.Texture2D":
        raise ToolError(f"当前仅支持 Texture2D，实际类型：{asset.resource_type}", code="asset_type_unsupported", stage="decode")
    session = open_session(bundles, budget)
    reader = target_object(session, asset, bundles)
    texture = reader.read()
    width, height = texture.m_Width, texture.m_Height
    validate_pixel_budget(width, height, session.loaded_bytes, budget)
    if texture.m_TextureFormat not in SUPPORTED_FORMATS:
        raise ToolError(f"尚未支持纹理格式：{texture.m_TextureFormat}", code="texture_format_unsupported", stage="decode",
                        context={"texture_format": texture.m_TextureFormat})
    if texture.m_ImageCount != 1 or texture.m_TextureDimension != 2 or getattr(texture, "m_MipsStripped", 0):
        raise ToolError("当前不支持纹理数组、多维纹理或剥离顶层的纹理。", code="texture_shape_unsupported", stage="decode")
    data = stream_bytes(session, texture)
    image = decode_pixels(data, width, height, texture.m_TextureFormat)
    if image.size != (width, height):
        image.close()
        raise ToolError("解码结果尺寸与纹理元数据不同。", code="pixel_dimensions_mismatch", stage="decode")
    if image.mode != "RGBA":
        image = image.convert("RGBA")
    return ExtractedImage(image, reader.path_id, reader.assets_file.name, texture.m_TextureFormat, session.warnings)
