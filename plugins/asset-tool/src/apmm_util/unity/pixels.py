"""用途：直接解码纹理像素，避免加载无关音频组件。作者：待填写。修改记录：2026-11-14，首次创建；补充像素解码错误码。"""

from ..errors import ToolError

SUPPORTED_FORMATS = {3, 4, 5, 10, 12, 14, 25, 26, 27}


def decode_pixels(data: bytes, width: int, height: int, texture_format: int, flip: bool = True):
    """功能：解码原始或块压缩像素。入参：数据、尺寸、格式及翻转选项。返回：RGBA 图像。异常：格式或长度无效。"""
    from PIL import Image
    import texture2ddecoder

    if width <= 0 or height <= 0:
        raise ToolError("像素解码尺寸无效。", code="texture_dimensions_invalid", stage="decode")
    raw_formats = {3: ("RGB", "RGB", 3), 4: ("RGBA", "RGBA", 4),
                   5: ("RGBA", "ARGB", 4), 14: ("RGBA", "BGRA", 4)}
    compressed = {10: (texture2ddecoder.decode_bc1, 8), 12: (texture2ddecoder.decode_bc3, 16),
                  25: (texture2ddecoder.decode_bc7, 16), 26: (texture2ddecoder.decode_bc4, 8),
                  27: (texture2ddecoder.decode_bc5, 16)}
    if texture_format in raw_formats:
        mode, order, bytes_per_pixel = raw_formats[texture_format]
        required = width * height * bytes_per_pixel
        if len(data) < required:
            raise ToolError("原始纹理像素数据截断。", code="pixel_data_truncated", stage="decode")
        image = Image.frombytes(mode, (width, height), data[:required], "raw", order)
    elif texture_format in compressed:
        decoder, block_size = compressed[texture_format]
        required = ((width + 3) // 4) * ((height + 3) // 4) * block_size
        if len(data) < required:
            raise ToolError("块压缩纹理像素数据截断。", code="pixel_data_truncated", stage="decode")
        pixels = decoder(data[:required], width, height)
        if len(pixels) != width * height * 4:
            raise ToolError("原生解码器输出的像素数量不正确。", code="pixel_length_mismatch", stage="decode")
        # 原生纹理解码器输出 BGRA，必须转换通道后才能正确保存 PNG。
        image = Image.frombytes("RGBA", (width, height), pixels, "raw", "BGRA")
    else:
        raise ToolError(f"尚未支持纹理格式：{texture_format}", code="texture_format_unsupported", stage="decode",
                        context={"texture_format": texture_format})
    if image.mode != "RGBA":
        image = image.convert("RGBA")
    return image.transpose(Image.Transpose.FLIP_TOP_BOTTOM) if flip else image

