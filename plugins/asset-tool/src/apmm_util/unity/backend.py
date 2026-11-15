"""用途：启用只使用资产内嵌类型树的低资源解析。作者：待填写。修改记录：2026-11-14，首次创建。"""

import importlib

from ..errors import ToolError
from .common_strings import COMMON_STRINGS

_ready = False


class StrictStrings(dict):
    """拒绝未知公共字段，避免根据不完整名称解释资产。"""

    def get(self, key, default=None):
        """功能：取得已知公共字符串。入参：偏移和默认值。返回：字段名。异常：未知字段。"""
        if key not in self:
            raise ToolError(f"资产使用未知的 Unity 公共字符串偏移：{key}；需要更新解析支持。")
        return super().get(key)


_strings = StrictStrings(COMMON_STRINGS)


def common_strings(version=None) -> dict:
    """功能：提供小型标准字段表。入参：可选 Unity 版本。返回：公共字符串映射。异常：无。"""
    return _strings


def unavailable_typetree():
    """功能：阻止完整类型版本库加载。入参：无。返回：无。异常：资产未提供内嵌类型树。"""
    raise ToolError("资产没有可用的内嵌类型树；低资源模式停止解析，避免加载完整 Unity 类型版本库。")


def prepare_backend():
    """功能：初始化锁定版本的低资源解析适配。入参：无。返回：UnityPy 模块。异常：依赖版本不兼容。"""
    global _ready
    import UnityPy

    if not _ready:
        if UnityPy.__version__ != "1.25.4":
            raise ToolError("低资源解析适配要求 UnityPy 1.25.4，请使用已锁定依赖。")
        tpk = importlib.import_module("UnityPy.helpers.Tpk")
        nodes = importlib.import_module("UnityPy.helpers.TypeTreeNode")
        # 内嵌类型树已经包含字段结构，完整类型资料库只会增加内存且不提供必要信息。
        tpk.get_common_strings = common_strings
        nodes.get_common_strings = common_strings
        tpk.get_typetree = unavailable_typetree
        _ready = True
    return UnityPy

