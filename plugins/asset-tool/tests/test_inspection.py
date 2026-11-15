"""用途：验证只读诊断和分类能力隔离。作者：待填写。修改记录：2026-11-14，新增结构诊断回归。"""

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from apmm_util.app import dispatch
from apmm_util.errors import CatalogChanged, ToolError
from apmm_util.runtime import Runtime
from apmm_util.unity.inspection import describe_asset
from .helpers import compact_catalog, fake_game
from .unity_fixtures import FakeReader, texture_session


class InspectionTests(unittest.TestCase):
    """确保检查结构不会导图、改变状态或误开放分类。"""

    def setUp(self):
        """功能：准备合成游戏和未创建输出目录。入参：无。返回：无。异常：测试文件写入失败。"""
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)
        game, local = fake_game(self.base)
        self.asset, self.session, self.bundles = texture_session()
        self.session.files["cab-test"].objects[1].tree["m_Container"][0] = ("1", {
            "asset": {"m_FileID": 0, "m_PathID": -8858869715767140405}})
        cached = local / "com.unity.addressables"
        (cached / "catalog_3.3.0.json").write_text(json.dumps(compact_catalog((self.asset.name,))), encoding="utf-8")
        self.hash_path = cached / "catalog_3.3.0.hash"
        self.bundle = game / "Test_Data/com.unity.addressables/AssetBundles/bundle" / ("a" * 32) / "__data"
        self.bundle.parent.mkdir(parents=True)
        self.bundle.write_bytes(b"fixture")
        self.args = SimpleNamespace(command="inspect", name=self.asset.name, output=str(self.base / "output"),
                                    game_dir=str(game), locallow_dir=str(local), cache_dir=None,
                                    memory_budget_mib=None, delay_ms=0, yes=False, json=True)

    def tearDown(self):
        """功能：清理测试目录。入参：无。返回：无。异常：清理失败。"""
        self.temporary.cleanup()

    def run_inspect(self):
        """功能：通过真实命令分发检查合成对象。入参：无。返回：诊断报告。异常：检查失败。"""
        with patch("apmm_util.inspection.open_session", return_value=self.session):
            return dispatch(self.args, Runtime(), lambda message: None)

    def test_inspection_is_read_only_and_does_not_decode(self):
        """功能：验证诊断不创建输出且不会解码。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.pixels.decode_pixels") as decoder:
            result = self.run_inspect()
        decoder.assert_not_called()
        self.assertFalse(Path(self.args.output).exists())
        self.assertTrue(result["category_sync_supported"])
        self.assertFalse(result["pixels_decoded"])
        self.assertEqual(result["object"]["object_id"], "-8858869715767140405")
        self.assertEqual(result["texture"]["stream"]["size"], 16)
        self.assertEqual(result["object_types"], {"AssetBundle": 1, "Texture2D": 1})

    def test_existing_files_remain_unchanged(self):
        """功能：验证已有状态和图片内容、时间保持不变。入参：无。返回：无。异常：断言失败。"""
        root = Path(self.args.output)
        root.mkdir()
        (root / "state.json").write_text('{"schema_version":1,"categories":{}}')
        (root / "old.png").write_bytes(b"existing")
        before = {p.name: (p.read_bytes(), p.stat().st_mtime_ns) for p in root.iterdir()}
        self.run_inspect()
        self.assertEqual(before, {p.name: (p.read_bytes(), p.stat().st_mtime_ns) for p in root.iterdir()})

    def test_catalog_and_bundle_changes_cancel(self):
        """功能：验证检查期间的目录和依赖变化会取消。入参：无。返回：无。异常：断言失败。"""
        for path in (self.hash_path, self.bundle):
            with self.subTest(path=path):
                original = path.read_bytes()
                def changed(config, asset, bundles):
                    """功能：模拟检查途中资源更新。入参：会话、资产和包。返回：结构。异常：无。"""
                    path.write_bytes(b"b" * 32)
                    return describe_asset(config, asset, bundles)
                with patch("apmm_util.inspection.open_session", return_value=self.session),                      patch("apmm_util.inspection.describe_asset", side_effect=changed), self.assertRaises(CatalogChanged):
                    dispatch(self.args, Runtime(), lambda message: None)
                path.write_bytes(original)
        self.assertFalse(Path(self.args.output).exists())

    def test_missing_named_resource_does_not_overwrite_outputs(self):
        """已经支持的分类仍须拒绝当前目录不存在的完整资源名。"""
        self.args.command, self.args.name = 'extract', 'UT_Monster_Bust_999999_0'
        with self.assertRaises(ToolError):
            dispatch(self.args, Runtime(), lambda message: None)
        self.assertFalse((Path(self.args.output) / 'state.json').exists())

    def test_layout_objects_are_reported_without_rendering(self):
        """功能：验证布局对象统计不触发合成。入参：无。返回：无。异常：断言失败。"""
        source = self.session.files["cab-test"]
        source.objects[2] = FakeReader("RectTransform", 2, source, {})
        result = self.run_inspect()
        self.assertTrue(result["layout"]["has_layout_objects"])
        self.assertFalse(result["layout"]["composition_required"])
        self.assertEqual(result["layout"]["object_types"], {"RectTransform": 1})
