"""读取内置鼠标纹理；仅使用随插件发布的已核验版本类型描述，不推测偏移。"""
import json
from pathlib import Path
from ..errors import ToolError
from ..io_utils import sha256_file, stamp
from ..models import BundleSpec, LocatedBundle, ExtractedImage
from .session import open_session
from .texture import stream_bytes, validate_pixel_budget
from .pixels import decode_pixels

NAMES = ('cursor_up', 'cursor_down')


def locate(config) -> LocatedBundle:
    path = config.data_dir / 'data.unity3d'
    identity = stamp(path)
    digest = sha256_file(path)
    return LocatedBundle(BundleSpec(str(path), path.name, digest), path, digest, identity.size, identity)


def extract_cursors(bundle, budget: int) -> dict[str, ExtractedImage]:
    """按 Texture2D 类型和名称定位；同名冲突、未知 Unity 版本或缺项均拒绝提交。"""
    session = open_session((bundle,), budget)
    result = {}
    for serialized in session.files.values():
        textures = [r for r in serialized.objects.values() if r.type.name == 'Texture2D']
        if not textures:
            continue
        version = serialized.unity_version
        descriptor = Path(__file__).resolve().parents[1] / 'resources' / f'texture2d-{version}.json'
        if not descriptor.is_file():
            raise ToolError(f'内置资源尚未验证 Unity {version} 的 Texture2D 类型描述。')
        from UnityPy.helpers.TypeTreeNode import TypeTreeNode
        node = TypeTreeNode.from_list(json.loads(descriptor.read_text(encoding='utf-8')))
        for reader in textures:
            texture = reader.read_typetree(nodes=node, wrap=True, check_read=True)
            if texture.m_Name not in NAMES:
                continue
            if texture.m_Name in result:
                raise ToolError(f'内置资源存在重复鼠标名称：{texture.m_Name}')
            if texture.m_ImageCount != 1 or texture.m_TextureDimension != 2 or getattr(texture, 'm_MipsStripped', 0):
                raise ToolError('鼠标纹理不是完整二维图像。')
            validate_pixel_budget(texture.m_Width, texture.m_Height, session.loaded_bytes, budget)
            image = decode_pixels(stream_bytes(session, texture), texture.m_Width, texture.m_Height, texture.m_TextureFormat)
            result[texture.m_Name] = ExtractedImage(image, reader.path_id, serialized.name, texture.m_TextureFormat, session.warnings)
    if set(result) != set(NAMES):
        for value in result.values():
            value.image.close()
        raise ToolError(f'内置资源缺少鼠标状态：{sorted(set(NAMES) - set(result))}')
    return result
