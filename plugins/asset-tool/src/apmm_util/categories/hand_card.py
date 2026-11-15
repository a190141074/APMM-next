"""用途：识别手牌名称与变体。作者：待填写。修改记录：2026-11-14，首次创建。"""

import re

PATTERN = re.compile(r"^UT_HandCard_(\d+)(_[A-Za-z0-9_]+)?$")


def metadata(name: str) -> dict:
    """功能：解析手牌编号与变体。入参：完整资源名。返回：分类元数据。异常：名称无效。"""
    match = PATTERN.fullmatch(name)
    if not match:
        raise ValueError(f"无效的手牌名称：{name}")
    return {"category": "hand_card", "entity_id": match[1], "variant": (match[2] or "").lstrip("_")}

