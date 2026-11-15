"""用途：验证异常身份、局部更新及历史基线。作者：待填写。修改记录：2026-11-14，新增分类异常回归。"""

import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from apmm_util.diagnostics import load_diagnostics, reconcile_diagnostics
from apmm_util.errors import ToolError, error_details


def issue(name, code="object_missing"):
    """功能：构造独立异常记录。入参：名称和错误码。返回：异常字典。异常：无。"""
    return {"name": name, "stage": "reference", "code": code, "message": "对象缺失。", "context": {"object_id": "-8858869715767140405"}}


class DiagnosticTests(unittest.TestCase):
    """用异常名单变化验证基线，不依赖异常计数猜测。"""

    def reconcile(self, old, errors, targets=("A", "B"), current=("A", "B"), full=True, now=10):
        """功能：调用异常协调器。入参：旧记录、异常和处理范围。返回：记录和变化。异常：协调失败。"""
        return reconcile_diagnostics(old, "hand_card", SimpleNamespace(fingerprint="catalog"), errors,
                                     list(targets), list(current), full, now)

    def test_first_baseline_and_same_count_different_resources(self):
        """功能：验证首次基线以及同数量不同资源的新增异常。入参：无。返回：无。异常：断言失败。"""
        first, report = self.reconcile(None, [issue("A")])
        self.assertFalse(report["baseline_available"])
        second, report = self.reconcile(first, [issue("B")], now=20)
        self.assertTrue(report["baseline_available"])
        self.assertEqual(report["active_count"], 1)
        self.assertEqual([row["name"] for row in report["new"]], ["B"])
        self.assertEqual([row["name"] for row in report["resolved"]], ["A"])
        self.assertEqual(first["active"][0]["name"], "A")

    def test_single_target_keeps_other_issues_and_first_seen(self):
        """功能：验证单张更新不清除其他异常且持续异常保留首见时间。入参：无。返回：无。异常：断言失败。"""
        first, _ = self.reconcile(None, [issue("A"), issue("B")])
        second, report = self.reconcile(first, [issue("A")], targets=("A",), full=False, now=20)
        self.assertEqual(report["active_count"], 2)
        self.assertEqual(report["continuing"][0]["first_seen_unix"], 10)
        self.assertEqual(report["continuing"][0]["last_seen_unix"], 20)
        self.assertFalse(report["resolved"])
        self.assertEqual(second["active"][1]["last_seen_unix"], 10)

    def test_changed_code_recovery_and_retirement(self):
        """功能：验证错误原因变化、恢复和资源移除分别记录。入参：无。返回：无。异常：断言失败。"""
        first, _ = self.reconcile(None, [issue("A")])
        second, report = self.reconcile(first, [issue("A", "texture_stream_missing")])
        self.assertEqual(report["new"][0]["code"], "texture_stream_missing")
        self.assertEqual(report["resolved"][0]["resolution"], "error_changed")
        _, report = self.reconcile(second, [])
        self.assertEqual(report["resolved"][0]["resolution"], "recovered")
        _, report = self.reconcile(second, [], targets=("B",), current=("B",))
        self.assertEqual(report["resolved"][0]["resolution"], "resource_retired")

    def test_missing_and_corrupt_baseline(self):
        """功能：验证缺失基线正常建立而损坏基线阻止覆盖。入参：无。返回：无。异常：断言失败。"""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.assertIsNone(load_diagnostics(root, "hand_card"))
            directory = root / "diagnostics"
            directory.mkdir()
            path = directory / "hand_card.json"
            for contents in ('null', '{"schema_version":1,"category":"hero","active":[]}'):
                path.write_text(contents)
                with self.assertRaises(ToolError) as caught:
                    load_diagnostics(root, "hand_card")
                self.assertEqual(caught.exception.code, "diagnostics_invalid")
                self.assertEqual(path.read_text(), contents)

    def test_error_context_preserves_full_identifier(self):
        """功能：验证来源上下文合并且对象号不变成浮点数。入参：无。返回：无。异常：断言失败。"""
        error = ToolError("对象缺失。", code="object_missing", stage="reference", context={"object_id": "-8858869715767140405"})
        report = error_details(error, stage="extract", context={"name": "A"})
        self.assertEqual(report["stage"], "reference")
        self.assertEqual(report["context"], {"name": "A", "object_id": "-8858869715767140405"})
        self.assertEqual(error_details(FileNotFoundError("缺包。"))["code"], "bundle_missing")
