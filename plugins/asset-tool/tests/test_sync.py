"""用途：验证完整同步、指定提取和失败恢复。作者：待填写。修改记录：2026-11-14，首次创建；补充异常名单及取消回滚验证。"""

import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from PIL import Image

from apmm_util.app import dispatch
from apmm_util.errors import CatalogChanged, ConfirmationRequired, ToolError, Cancelled
from apmm_util.io_utils import load_json, sha256_file
from apmm_util.models import ExtractedImage
from apmm_util.runtime import Runtime
from .helpers import fake_game


class SyncTests(unittest.TestCase):
    """对真实同步流程替换图像解码器，隔离游戏素材依赖。"""

    def setUp(self):
        """功能：准备独立游戏和输出目录。入参：无。返回：无。异常：测试文件写入失败。"""
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)
        game, local = fake_game(self.base)
        self.output = self.base / "output"
        self.bundle = game / "Test_Data/com.unity.addressables/AssetBundles/bundle" / ("a" * 32) / "__data"
        self.bundle.parent.mkdir(parents=True)
        self.bundle.write_bytes(b"fixture")
        self.hash_path = local / "com.unity.addressables/catalog_3.3.0.hash"
        self.args = SimpleNamespace(command="sync", category="hand_card", name=None, game_dir=str(game),
                                    locallow_dir=str(local), cache_dir=None, output=str(self.output),
                                    memory_budget_mib=None, delay_ms=0, yes=True, json=True)

    def tearDown(self):
        """功能：清理独立测试文件。入参：无。返回：无。异常：清理失败。"""
        self.temporary.cleanup()

    def image(self, asset, bundles, budget):
        """功能：生成固定测试图像。入参：资产、包和预算。返回：图像结果。异常：无。"""
        return ExtractedImage(Image.new("RGBA", (2, 2), (255, 0, 0, 128)), 8175208211857592006, "CAB-test", 25)

    def run_sync(self):
        """功能：执行当前测试命令。入参：无。返回：同步报告。异常：处理失败。"""
        return dispatch(self.args, Runtime(), lambda message: None)

    def test_initial_and_repeat_preserve_png_mtime(self):
        """功能：验证首次生成及重复完全复用。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image) as decoder:
            first = self.run_sync()
            self.assertEqual(first["extracted"], 2)
            times = {p.name: p.stat().st_mtime_ns for p in (self.output / "outimg").iterdir()}
            second = self.run_sync()
            self.assertEqual(second["reused"], 2)
            self.assertEqual(second["extracted"], 0)
            self.assertEqual(decoder.call_count, 2)
            self.assertEqual(times, {p.name: p.stat().st_mtime_ns for p in (self.output / "outimg").iterdir()})
            self.assertEqual(len(load_json(self.output / "maps/hand_card.json")), 2)

    def test_single_extract_then_full_sync(self):
        """功能：验证单张提取与后续全分类同步。入参：无。返回：无。异常：断言失败。"""
        self.args.command, self.args.name = "extract", "UT_HandCard_10001"
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            result = self.run_sync()
            self.assertEqual((result["processed"], result["ready"], result["pending"]), (1, 1, 1))
            self.assertEqual(len(list((self.output / "outimg").iterdir())), 1)
            state = load_json(self.output / "state.json")
            self.assertNotIn("last_complete_fingerprint", state["categories"]["hand_card"])
            self.args.command, self.args.name = "sync", None
            result = self.run_sync()
            self.assertEqual((result["extracted"], result["reused"]), (1, 1))

    def test_catalog_change_requires_confirmation(self):
        """功能：验证检测变化后未确认不修改输出。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            self.run_sync()
        old_state = (self.output / "state.json").read_bytes()
        self.hash_path.write_text("b" * 32)
        self.args.yes = False
        with self.assertRaises(ConfirmationRequired):
            self.run_sync()
        self.assertEqual((self.output / "state.json").read_bytes(), old_state)
        self.args.yes = True
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image) as decoder:
            result = self.run_sync()
            self.assertEqual(result["reused"], 2)
            decoder.assert_not_called()

    def test_missing_assets_retry_and_keep_old_images(self):
        """功能：验证缺失不删除旧图且可再次同步。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            self.run_sync()
            png = self.output / "outimg/UT_HandCard_10001.png"
            old_hash = sha256_file(png)
            self.bundle.unlink()
            partial = self.run_sync()
            self.assertEqual((partial["status"], partial["missing"]), ("partial", 2))
            self.assertEqual(sha256_file(png), old_hash)
            self.bundle.write_bytes(b"fixture")
            recovered = self.run_sync()
            self.assertEqual((recovered["ready"], recovered["reused"]), (2, 2))

    def test_changed_bundle_and_deleted_output_regenerate(self):
        """功能：验证内容变化和原图丢失会重新生成。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            self.run_sync()
            self.bundle.write_bytes(b"changed")
            self.assertEqual(self.run_sync()["extracted"], 2)
            (self.output / "outimg/UT_HandCard_10001.png").unlink()
            result = self.run_sync()
            self.assertEqual((result["extracted"], result["reused"]), (1, 1))

    def test_catalog_change_mid_extract_discards_stage(self):
        """功能：验证处理过程中更新不会提交混合版本。入参：无。返回：无。异常：断言失败。"""
        def changed(asset, bundles, budget):
            """功能：模拟提取期间目录变化。入参：资产、包和预算。返回：图像。异常：无。"""
            self.hash_path.write_text("b" * 32)
            return self.image(asset, bundles, budget)
        with patch("apmm_util.unity.texture.extract_image", side_effect=changed):
            with self.assertRaises(CatalogChanged):
                self.run_sync()
        self.assertFalse((self.output / "outimg").exists())
        self.assertFalse((self.output / "state.json").exists())
        self.assertFalse(list(self.output.glob(".apmm-stage-*")))

    def test_diagnostic_replacement_with_equal_counts_keeps_old_images(self):
        """功能：验证等数量不同资源的异常变化且失败保留旧图。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            initial = self.run_sync()
        self.assertFalse(initial["diagnostics"]["baseline_available"])
        images = {p.name: sha256_file(p) for p in (self.output / "outimg").iterdir()}
        failing = "UT_HandCard_10001"
        def decoder(asset, bundles, budget):
            """功能：模拟指定资源的精确定位失败。入参：资产、依赖和预算。返回：图像。异常：对象缺失。"""
            if asset.name == failing:
                raise ToolError("对象缺失。", code="object_missing", stage="reference", context={"object_id": "-9"})
            return self.image(asset, bundles, budget)
        with patch("apmm_util.unity.texture.extract_image", side_effect=decoder):
            self.bundle.write_bytes(b"version-two")
            first = self.run_sync()
            failing = "UT_HandCard_20006_sfw"
            self.bundle.write_bytes(b"version-three")
            second = self.run_sync()
        self.assertEqual((first["failed"], second["failed"]), (1, 1))
        self.assertEqual(second["diagnostics"]["new"][0]["name"], failing)
        self.assertEqual(second["diagnostics"]["resolved"][0]["name"], "UT_HandCard_10001")
        self.assertEqual(images, {p.name: sha256_file(p) for p in (self.output / "outimg").iterdir()})

    def test_single_recovery_only_updates_target_diagnostics(self):
        """功能：验证单张恢复不会清除未处理资源的异常。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=ToolError("对象缺失。", code="object_missing", stage="reference")):
            self.run_sync()
        self.args.command, self.args.name = "extract", "UT_HandCard_10001"
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            result = self.run_sync()
        self.assertEqual(result["diagnostics"]["active_count"], 1)
        self.assertEqual(result["diagnostics"]["resolved"][0]["name"], self.args.name)
        rows = load_json(self.output / "diagnostics/hand_card.json")["active"]
        self.assertEqual([row["name"] for row in rows], ["UT_HandCard_20006_sfw"])

    def test_cancel_keeps_diagnostic_baseline(self):
        """功能：验证取消不会写入新异常基线。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            self.run_sync()
        before = (self.output / "diagnostics/hand_card.json").read_bytes()
        self.bundle.write_bytes(b"changed")
        def changed(asset, bundles, budget):
            """功能：模拟解码期间目录更新。入参：资产、依赖和预算。返回：无。异常：资源处理失败。"""
            self.hash_path.write_text("b" * 32)
            raise ToolError("处理中断。", code="object_missing", stage="reference")
        with patch("apmm_util.unity.texture.extract_image", side_effect=changed), self.assertRaises(CatalogChanged):
            self.run_sync()
        self.assertEqual((self.output / "diagnostics/hand_card.json").read_bytes(), before)
        self.assertFalse(list(self.output.glob(".apmm-stage-*")))

    def test_commit_failure_rolls_back_diagnostics_with_images(self):
        """功能：验证提交后段失败时异常记录与图片一起回滚。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            self.run_sync()
        before = {str(p.relative_to(self.output)): p.read_bytes() for p in self.output.rglob("*") if p.is_file()}
        self.bundle.write_bytes(b"changed")
        replace = os.replace
        def fail_state(source, target):
            """功能：模拟状态提交失败并允许回滚。入参：来源和目标路径。返回：无。异常：模拟写入失败。"""
            if Path(target).name == "state.json" and Path(source).name.endswith(".pending"):
                raise OSError("模拟磁盘写入失败。")
            replace(source, target)
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image),              patch("apmm_util.output.os.replace", side_effect=fail_state), self.assertRaises(OSError):
            self.run_sync()
        self.assertEqual(before, {str(p.relative_to(self.output)): p.read_bytes() for p in self.output.rglob("*") if p.is_file()})

    def test_corrupt_diagnostics_refuses_output_changes(self):
        """功能：验证损坏的异常记录不会被静默覆盖。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            self.run_sync()
        path = self.output / "diagnostics/hand_card.json"
        path.write_text("null")
        before = (self.output / "state.json").read_bytes()
        with self.assertRaises(ToolError) as caught:
            self.run_sync()
        self.assertEqual(caught.exception.code, "diagnostics_invalid")
        self.assertEqual(path.read_text(), "null")
        self.assertEqual((self.output / "state.json").read_bytes(), before)

    def test_user_cancellation_keeps_diagnostics_and_images(self):
        """功能：验证取消信号保持已有异常和图片。入参：无。返回：无。异常：断言失败。"""
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image):
            self.run_sync()
        before = {str(p.relative_to(self.output)): p.read_bytes() for p in self.output.rglob("*") if p.is_file()}
        self.bundle.write_bytes(b"changed")
        with patch("apmm_util.unity.texture.extract_image", side_effect=self.image),              patch("apmm_util.runtime.Runtime.pause", side_effect=Cancelled("已取消。")), self.assertRaises(Cancelled):
            self.run_sync()
        self.assertEqual(before, {str(p.relative_to(self.output)): p.read_bytes() for p in self.output.rglob("*") if p.is_file()})
