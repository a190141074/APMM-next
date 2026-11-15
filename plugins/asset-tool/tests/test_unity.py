"""用途：验证解压预算、完整引用与像素方向。作者：待填写。修改记录：2026-11-14，首次创建。"""

import lzma
import struct
import unittest
from types import SimpleNamespace

from apmm_util.errors import ToolError
from apmm_util.unity.bundle import decode_bundle, decompress
from apmm_util.unity.reference import resolve_pointer
from apmm_util.unity.texture import stream_bytes
from .helpers import unityfs


class UnityTests(unittest.TestCase):
    """验证格式解析及像素解码的关键约束。"""

    def test_plain_and_stale_node_length(self):
        """功能：验证目录包旧长度的受限恢复。入参：无。返回：无。异常：断言失败。"""
        payload = bytearray(64)
        struct.pack_into(">I", payload, 8, 22)
        struct.pack_into(">Q", payload, 24, 64)
        result = decode_bundle(unityfs(bytes(payload), declared_size=32), 16384)
        self.assertEqual(len(result.files["CAB-test"]), 64)
        self.assertTrue(result.warnings)

    def test_truncated_and_oversized_bundle(self):
        """功能：验证截断和超限资源被拒绝。入参：无。返回：无。异常：断言失败。"""
        raw = unityfs(b"test")
        for data, budget in [(raw[:-1], 16384), (raw, 16)]:
            with self.assertRaises(ToolError):
                decode_bundle(data, budget)

    def test_lzma_actual_expansion_bounded(self):
        """功能：验证错误长度声明无法突破预算。入参：无。返回：无。异常：断言失败。"""
        options = {"id": lzma.FILTER_LZMA1, "dict_size": 4096, "lc": 3, "lp": 0, "pb": 2}
        encoded = bytes([93]) + struct.pack("<I", 4096) + lzma.compress(b"a" * 100000, format=lzma.FORMAT_RAW, filters=[options])
        with self.assertRaisesRegex(ToolError, "超过资源预算"):
            decompress(encoded, 1, 10, 8192)

    def test_external_pointer_preserves_64_bits(self):
        """功能：验证外部文件号和完整对象号。入参：无。返回：无。异常：断言失败。"""
        identifier = 8175208211857592006
        expected = object()
        source = SimpleNamespace(externals=[SimpleNamespace(path="archive:/CAB-external")], objects={})
        external = SimpleNamespace(objects={identifier: expected})
        session = SimpleNamespace(files={"cab-external": external})
        self.assertIs(resolve_pointer(session, source, {"m_FileID": 1, "m_PathID": identifier}), expected)
        with self.assertRaises(ToolError):
            resolve_pointer(session, source, {"m_FileID": 2, "m_PathID": identifier})
        with self.assertRaises(ToolError):
            resolve_pointer(session, source, {"m_FileID": 1, "m_PathID": identifier & 0xffffffff})

    def test_stream_range_and_missing_dependency(self):
        """功能：验证纹理流只能读取准确范围。入参：无。返回：无。异常：断言失败。"""
        session = SimpleNamespace(streams={"cab.resS".lower(): b"abcdefgh"})
        texture = SimpleNamespace(m_StreamData=SimpleNamespace(offset=2, size=3, path="archive:/CAB/CAB.resS"))
        self.assertEqual(stream_bytes(session, texture), b"cde")
        texture.m_StreamData.offset = 7
        with self.assertRaises(ToolError):
            stream_bytes(session, texture)

    def test_rgba_direction_and_alpha(self):
        """功能：验证 Unity 底部原点转换和通道顺序。入参：无。返回：无。异常：断言失败。"""
        from apmm_util.unity.pixels import decode_pixels
        data = bytes([255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 255, 255, 255, 255])
        image = decode_pixels(data, 2, 2, 4)
        self.assertEqual(image.getpixel((0, 0)), (0, 0, 255, 0))
        self.assertEqual(image.getpixel((1, 1)), (0, 255, 0, 128))

    def test_bc7_constant_red_fixture(self):
        """功能：使用独立编码的固定色块验证 BC7 通道。入参：无。返回：无。异常：断言失败。"""
        from apmm_util.unity.pixels import decode_pixels
        value, position = 0, 0
        def bits(number, width):
            """功能：写入测试色块位段。入参：值和位数。返回：无。异常：无。"""
            nonlocal value, position
            value |= number << position
            position += width
        bits(64, 7)
        for color in (127, 0, 0, 127):
            bits(color, 7)
            bits(color, 7)
        bits(1, 1)
        bits(1, 1)
        bits(0, 3)
        for _ in range(15):
            bits(0, 4)
        self.assertEqual(position, 128)
        image = decode_pixels(value.to_bytes(16, "little"), 4, 4, 25)
        self.assertEqual(image.getpixel((0, 0)), (255, 1, 1, 255))
        self.assertEqual(image.getextrema(), ((255, 255), (1, 1), (1, 1), (255, 255)))

    def test_low_resource_backend_never_loads_full_type_library(self):
        """功能：验证缺少类型树时不会加载完整版本库。入参：无。返回：无。异常：断言失败。"""
        from apmm_util.unity.backend import prepare_backend
        from UnityPy.helpers import Tpk
        prepare_backend()
        with self.assertRaisesRegex(ToolError, "低资源模式"):
            Tpk.get_typetree()

