"""用途：验证事务回滚、确认和独立分类状态。作者：待填写。修改记录：2026-11-14，首次创建。"""

import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from apmm_util.app import confirm_update
from apmm_util.errors import Cancelled, CatalogChanged, ConfirmationRequired, ToolError
from apmm_util.output import OutputLock, Transaction
from apmm_util.state import updated_state


class OutputStateTests(unittest.TestCase):
    """验证取消、确认与输出事务不破坏旧产物。"""

    def test_transaction_rolls_back_after_catalog_change(self):
        """功能：验证提交中变化可恢复全部旧内容。入参：无。返回：无。异常：断言失败。"""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "first.json").write_text("旧内容", encoding="utf-8")
            tx = Transaction(root)
            tx.json("first.json", {"new": True})
            tx.json("second.json", {"new": True})
            calls = 0
            def changed():
                """功能：在提交后模拟目录变化。入参：无。返回：无。异常：目录变化。"""
                nonlocal calls
                calls += 1
                if calls == 3:
                    raise CatalogChanged("变化")
            with self.assertRaises(CatalogChanged):
                tx.commit(changed)
            self.assertEqual((root / "first.json").read_text(encoding="utf-8"), "旧内容")
            self.assertFalse((root / "second.json").exists())
            tx.cleanup()

    def test_paths_and_exclusive_lock(self):
        """功能：验证路径边界与重复运行锁。入参：无。返回：无。异常：断言失败。"""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            tx = Transaction(root)
            with self.assertRaises(ToolError):
                tx.path("../escape.png")
            tx.cleanup()
            with OutputLock(root):
                with self.assertRaises(ToolError):
                    with OutputLock(root):
                        pass

    def test_noninteractive_confirmation_and_decline(self):
        """功能：验证版本变化必须确认。入参：无。返回：无。异常：断言失败。"""
        snapshot = SimpleNamespace(fingerprint="new", version="3.3.0", catalog_hash="b")
        state = {"accepted_catalog_fingerprint": "old", "accepted_version": "3.3.0", "accepted_catalog_hash": "a"}
        args = SimpleNamespace(yes=False, json=True)
        with self.assertRaises(ConfirmationRequired):
            confirm_update(args, snapshot, state)
        args.json = False
        with patch("sys.stdin.isatty", return_value=True):
            with self.assertRaises(Cancelled):
                confirm_update(args, snapshot, state, lambda message: "n")
            confirm_update(args, snapshot, state, lambda message: "y")
        args.yes = True
        confirm_update(args, snapshot, state)

    def test_extract_keeps_complete_marker_and_other_categories(self):
        """功能：验证单张提取不推进全分类完成标记。入参：无。返回：无。异常：断言失败。"""
        snapshot = SimpleNamespace(fingerprint="new", catalog_hash="b", version="3.3.0")
        previous = {"schema_version": 1, "categories": {"hero": {"ready": 9},
                    "hand_card": {"last_complete_fingerprint": "old"}}}
        index = {"category": "hand_card", "resources": [{"status": "ready"}, {"status": "pending"}]}
        state = updated_state(previous, snapshot, index, False)
        self.assertEqual(state["categories"]["hero"], {"ready": 9})
        self.assertEqual(state["categories"]["hand_card"]["last_complete_fingerprint"], "old")
        self.assertEqual(previous["categories"]["hand_card"], {"last_complete_fingerprint": "old"})
