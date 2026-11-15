"""用途：发现游戏、更新缓存和工具配置。作者：待填写。修改记录：2026-11-14，首次创建。"""

import os
import sys
from pathlib import Path
from typing import Any

from .errors import ToolError
from .io_utils import load_json, read_bounded
from .models import Config



def default_output() -> Path:
    """功能：确定工具旁的数据目录。入参：无。返回：绝对输出路径。异常：无。"""
    base = Path(sys.executable).parent if getattr(sys, "frozen", False) else Path(__file__).resolve().parents[2]
    return base / "data"


def resolve_config(args: Any) -> Config:
    """功能：合并参数和配置并发现资源根目录。入参：命令参数。返回：运行配置。异常：配置无效。"""
    output = Path(args.output or default_output()).expanduser().resolve()
    saved = load_json(output / "config.json", {})
    game_value = args.game_dir or saved.get("game_dir")
    if not game_value:
        raise ToolError("请通过 --game-dir 指定游戏安装目录，或由 APMM 传入检测结果。")
    game = Path(game_value).expanduser().resolve()
    candidates = sorted(p for p in game.glob("*_Data") if (p / "StreamingAssets/aa/settings.json").is_file())
    if len(candidates) != 1:
        raise ToolError(f"游戏目录必须包含唯一的 Unity 数据目录：{game}")
    data = candidates[0]
    info = read_bounded(data / "app.info", 4096).decode("utf-8-sig").splitlines()
    if len(info) < 2 or any(not safe_component(s.strip()) for s in info[:2]):
        raise ToolError("app.info 中的公司名或游戏名无效。")
    user = Path(os.environ.get("USERPROFILE", str(Path.home())))
    discovered = user / "AppData/LocalLow" / info[0].strip() / info[1].strip()
    locallow = Path(args.locallow_dir or saved.get("locallow_dir") or discovered).expanduser().resolve()
    overrides = args.cache_dir if args.cache_dir is not None else saved.get("cache_dirs", [])
    if args.game_dir and str(game) != saved.get("game_dir") and args.cache_dir is None:
        overrides = []
    defaults = [data / "com.unity.addressables/AssetBundles", locallow / "com.unity.addressables/AssetBundles"]
    caches = tuple(dict.fromkeys(Path(p).expanduser().resolve() for p in (overrides or defaults)))
    budget = args.memory_budget_mib if args.memory_budget_mib is not None else saved.get("memory_budget_mib", 256)
    delay = args.delay_ms if args.delay_ms is not None else saved.get("delay_ms", 20)
    if not 16 <= budget <= 4096 or not 0 <= delay <= 10000:
        raise ToolError("资源预算须为 16～4096 MiB，处理间隔须为 0～10000 毫秒。")
    if output == game or output.is_relative_to(data) or output.is_relative_to(locallow):
        raise ToolError("输出目录不能覆盖游戏数据或更新缓存。")
    return Config(game, data, locallow, caches, output, budget * 1024 * 1024, delay)


def safe_component(value: str) -> bool:
    """功能：校验单段目录名。入参：字符串。返回：是否安全。异常：无。"""
    return bool(value) and value not in (".", "..") and not any(c in value for c in '/\\:\x00')


def config_document(config: Config) -> dict:
    """功能：生成可保存配置。入参：运行配置。返回：配置字典。异常：无。"""
    return {"schema_version": 1, "game_dir": str(config.game_dir), "locallow_dir": str(config.locallow_dir),
            "cache_dirs": [str(p) for p in config.cache_dirs],
            "memory_budget_mib": config.memory_budget // (1024 * 1024), "delay_ms": config.delay_ms}

