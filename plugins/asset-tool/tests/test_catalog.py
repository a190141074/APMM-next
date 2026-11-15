"""用途：验证目录解析与热更新一致性。作者：待填写。修改记录：2026-11-14，首次创建。"""

import base64
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from apmm_util.catalog.parser import Catalog
from apmm_util.catalog.source import assert_unchanged, load_current
from apmm_util.categories.registry import category_for_name, require_category
from apmm_util.config import resolve_config
from apmm_util.errors import CatalogChanged, ToolError
from .helpers import compact_catalog, fake_game


class CatalogTests(unittest.TestCase):
    """验证官方目录格式和分类边界。"""

    def test_keys_variants_and_dependency(self):
        """功能：验证完整名称及官方依赖。入参：无。返回：无。异常：断言失败。"""
        catalog = Catalog(compact_catalog())
        self.assertEqual(catalog.resource_names("UT_HandCard_"), ["UT_HandCard_10001", "UT_HandCard_20006_sfw"])
        asset = catalog.asset("UT_HandCard_20006_sfw")
        self.assertEqual(asset.internal_id, "2")
        self.assertEqual(asset.bundles[0].version_hash, "a" * 32)
        self.assertEqual(category_for_name(asset.name).parse_metadata(asset.name)["variant"], "sfw")

    def test_corrupt_indexes_rejected(self):
        """功能：验证损坏连接无法越界。入参：无。返回：无。异常：断言失败。"""
        document = compact_catalog()
        document["m_EntryDataString"] = base64.b64encode(b"\xff" * 8).decode()
        with self.assertRaises(ToolError):
            Catalog(document)

    def test_shared_categories_available(self):
        """验证共享分类和官方变体已经开放。"""
        from apmm_util.categories.registry import expand_categories, category_for_name
        self.assertGreater(len(expand_categories('hero')), 1)
        self.assertGreater(len(expand_categories('monster')), 1)
        self.assertEqual(category_for_name('UT_Hero_ProfilePhoto_1001').key, 'monster_profile')
        self.assertEqual(category_for_name('UT_Relic_50075_1').parse_metadata('UT_Relic_50075_1')['canonical'], 'UT_Relic_50075_1')
        with self.assertRaises(ToolError):
            category_for_name('UT_Platform_Start')

    def test_hash_and_json_changes_detected(self):
        """功能：验证目录和哈希变化检测。入参：无。返回：无。异常：断言失败。"""
        with tempfile.TemporaryDirectory() as tmp:
            game, local = fake_game(Path(tmp))
            args = SimpleNamespace(output=str(Path(tmp) / "out"), game_dir=str(game), locallow_dir=str(local),
                                   cache_dir=None, memory_budget_mib=None, delay_ms=None)
            config = resolve_config(args)
            snapshot, _ = load_current(config)
            self.assertEqual(snapshot.kind, "cached_json")
            hash_path = local / "com.unity.addressables/catalog_3.3.0.hash"
            hash_path.write_text("b" * 32)
            with self.assertRaises(CatalogChanged):
                assert_unchanged(snapshot)
            newer, _ = load_current(config)
            self.assertNotEqual(snapshot.fingerprint, newer.fingerprint)
            newer.path.write_bytes(newer.path.read_bytes() + b" ")
            with self.assertRaises(CatalogChanged):
                assert_unchanged(newer)

    def test_half_written_pair_rejected(self):
        """功能：验证不完整热更新不会回退到旧目录。入参：无。返回：无。异常：断言失败。"""
        with tempfile.TemporaryDirectory() as tmp:
            game, local = fake_game(Path(tmp))
            (local / "com.unity.addressables/catalog_3.3.0.hash").unlink()
            args = SimpleNamespace(output=str(Path(tmp) / "out"), game_dir=str(game), locallow_dir=str(local),
                                   cache_dir=None, memory_budget_mib=None, delay_ms=None)
            with self.assertRaises(CatalogChanged):
                load_current(resolve_config(args))

    def test_game_settings_change_invalidates_snapshot(self):
        """功能：验证游戏版本设置变化会取消旧目录快照。入参：无。返回：无。异常：断言失败。"""
        with tempfile.TemporaryDirectory() as tmp:
            game, local = fake_game(Path(tmp))
            args = SimpleNamespace(output=str(Path(tmp) / "out"), game_dir=str(game), locallow_dir=str(local),
                                   cache_dir=None, memory_budget_mib=None, delay_ms=None)
            snapshot, _ = load_current(resolve_config(args))
            snapshot.settings_path.write_bytes(snapshot.settings_path.read_bytes() + b" ")
            with self.assertRaises(CatalogChanged):
                assert_unchanged(snapshot)
