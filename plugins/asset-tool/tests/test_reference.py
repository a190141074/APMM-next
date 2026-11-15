"""用途：验证完整 PPtr 和容器的严格定位。作者：待填写。修改记录：2026-11-14，新增定位回归。"""

import unittest
from types import SimpleNamespace

from apmm_util.errors import ToolError
from apmm_util.unity.reference import resolve_pointer, target_object
from .unity_fixtures import FakeReader, texture_session


class ReferenceTests(unittest.TestCase):
    """覆盖引用乱序、对象号冲突和错误回退。"""

    def test_external_order_and_duplicate_identifiers(self):
        """功能：验证外部表顺序独立且同号对象按文件区分。入参：无。返回：无。异常：断言失败。"""
        identifier = 8175208211857592006
        local, first, second = object(), object(), object()
        source = SimpleNamespace(name="CAB-local", externals=[SimpleNamespace(path="archive:/CAB-B"),
                                 SimpleNamespace(path="archive:/CAB-A")], objects={identifier: local})
        session = SimpleNamespace(files={"cab-a": SimpleNamespace(objects={identifier: first}),
                                          "cab-b": SimpleNamespace(objects={identifier: second})})
        for file_id, expected in ((0, local), (1, second), (2, first)):
            self.assertIs(resolve_pointer(session, source, {"m_FileID": file_id, "m_PathID": identifier}), expected)

    def test_negative_identifier_and_error_context(self):
        """功能：验证负对象号原样定位和缺失上下文。入参：无。返回：无。异常：断言失败。"""
        asset, session, bundles = texture_session()
        reader = target_object(session, asset, bundles)
        self.assertEqual(reader.path_id, -8858869715767140405)
        source = reader.assets_file
        source.objects.pop(reader.path_id)
        with self.assertRaises(ToolError) as caught:
            resolve_pointer(session, source, {"m_FileID": 0, "m_PathID": reader.path_id})
        self.assertEqual(caught.exception.code, "object_missing")
        self.assertEqual(caught.exception.context["object_id"], str(reader.path_id))

    def test_null_invalid_and_missing_external(self):
        """功能：验证无效及缺失引用分别报告。入参：无。返回：无。异常：断言失败。"""
        source = SimpleNamespace(name="CAB-local", externals=[SimpleNamespace(path="CAB-missing")], objects={})
        session = SimpleNamespace(files={})
        for pointer, code in (({}, "pointer_invalid"), ({"m_FileID": 0, "m_PathID": 0}, "pointer_null"),
                              ({"m_FileID": -1, "m_PathID": 1}, "pointer_invalid"),
                              ({"m_FileID": 0, "m_PathID": 1 << 63}, "pointer_invalid"),
                              ({"m_FileID": 2, "m_PathID": 1}, "external_index_invalid"),
                              ({"m_FileID": 1, "m_PathID": 1}, "external_file_missing")):
            with self.subTest(code=code), self.assertRaises(ToolError) as caught:
                resolve_pointer(session, source, pointer)
            self.assertEqual(caught.exception.code, code)

    def test_wrong_name_never_uses_first_texture(self):
        """功能：验证错误纹理名称不会回退到第一张。入参：无。返回：无。异常：断言失败。"""
        asset, session, bundles = texture_session()
        next(r for r in session.files["cab-test"].objects.values() if r.type.name == "Texture2D").tree["m_Name"] = "另一资源"
        with self.assertRaises(ToolError) as caught:
            target_object(session, asset, bundles)
        self.assertEqual(caught.exception.code, "container_target_missing")

    def test_ambiguous_container_is_rejected(self):
        """功能：验证多个准确候选会被拒绝。入参：无。返回：无。异常：断言失败。"""
        asset, session, bundles = texture_session()
        _, other, _ = texture_session(asset.name)
        source = other.files.pop("cab-test")
        source.name = "CAB-other"
        session.files["cab-other"] = source
        with self.assertRaises(ToolError) as caught:
            target_object(session, asset, bundles)
        self.assertEqual(caught.exception.code, "container_ambiguous")
