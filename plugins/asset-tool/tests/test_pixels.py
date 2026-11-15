"""用途：验证无损原图、透明边界及分配预算。作者：待填写。修改记录：2026-11-14，新增像素和 PNG 回归。"""

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from PIL import Image

from apmm_util.errors import ToolError
from apmm_util.output import Transaction
from apmm_util.unity.pixels import decode_pixels
from apmm_util.unity.texture import extract_image, validate_pixel_budget
from .unity_fixtures import texture_session


class PixelTests(unittest.TestCase):
    """覆盖半透明像素和全尺寸透明边界的实际输出。"""

    def test_png_keeps_alpha_colors_direction_and_empty_border(self):
        """功能：验证解码后经实际 PNG 写入仍保留全部像素。入参：无。返回：无。异常：断言失败。"""
        pixels = [(0, 0, 0, 0)] * 12
        pixels[4:8] = [(255, 0, 0, 128), (0, 255, 0, 64), (0, 0, 255, 255), (255, 255, 255, 1)]
        data = bytes(value for pixel in pixels for value in pixel)
        image = decode_pixels(data, 4, 3, 4)
        with tempfile.TemporaryDirectory() as tmp:
            tx = Transaction(Path(tmp))
            tx.png("outimg/test.png", image)
            tx.commit(lambda: None)
            with Image.open(Path(tmp) / "outimg/test.png") as saved:
                self.assertEqual(saved.size, (4, 3))
                self.assertEqual(saved.mode, "RGBA")
                self.assertEqual(saved.tobytes(), image.tobytes())
                self.assertEqual(saved.getpixel((0, 1)), (255, 0, 0, 128))
                self.assertEqual(saved.getpixel((0, 0)), (0, 0, 0, 0))
                self.assertEqual(saved.getpixel((3, 2)), (0, 0, 0, 0))
            tx.cleanup()
        image.close()

    def test_png_orientation_is_preserved(self):
        """功能：验证颜色标记在导图后仍处于正确上下位置。入参：无。返回：无。异常：断言失败。"""
        image = decode_pixels(bytes([255, 0, 0, 128, 0, 0, 255, 64]), 1, 2, 4)
        self.assertEqual(image.getpixel((0, 0)), (0, 0, 255, 64))
        self.assertEqual(image.getpixel((0, 1)), (255, 0, 0, 128))
        image.close()

    def test_oversized_texture_is_rejected_before_decode(self):
        """功能：验证超大尺寸不会进入原生解码器。入参：无。返回：无。异常：断言失败。"""
        asset, session, bundles = texture_session()
        reader = next(r for r in session.files["cab-test"].objects.values() if r.type.name == "Texture2D")
        reader.tree["m_Width"], reader.tree["m_Height"] = 12000, 12000
        with patch("apmm_util.unity.texture.open_session", return_value=session),              patch("apmm_util.unity.texture.decode_pixels") as decoder, self.assertRaises(ToolError) as caught:
            extract_image(asset, bundles, 256 * 1024 * 1024)
        self.assertEqual(caught.exception.code, "resource_budget_exceeded")
        decoder.assert_not_called()

    def test_invalid_dimensions_and_budget_boundary(self):
        """功能：验证非法尺寸及精确预算边界。入参：无。返回：无。异常：断言失败。"""
        for width, height in ((0, 1), (-1, 1), (1, 0), (1, -1)):
            with self.subTest(size=(width, height)), self.assertRaises(ToolError) as caught:
                validate_pixel_budget(width, height, 0, 100)
            self.assertEqual(caught.exception.code, "texture_dimensions_invalid")
        validate_pixel_budget(2, 2, 100, 148)
        with self.assertRaises(ToolError):
            validate_pixel_budget(2, 2, 100, 147)
