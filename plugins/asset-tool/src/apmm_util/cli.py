"""用途：解析命令并输出用户提示和稳定退出状态。作者：待填写。修改记录：2026-11-14，首次创建；补充诊断命令和结构化错误报告。"""

import argparse
import json
import sys
import threading
import re

from . import __version__
from .app import dispatch
from .errors import Cancelled, CatalogChanged, ConfirmationRequired, ToolError, error_details
from .runtime import Runtime


def common_options(parser) -> None:
    """功能：注册通用参数。入参：参数解析器。返回：无。异常：无。"""
    parser.add_argument("--game-dir", default=argparse.SUPPRESS, help="游戏安装根目录。")
    parser.add_argument("--locallow-dir", default=argparse.SUPPRESS, help="游戏 LocalLow 根目录。")
    parser.add_argument("--cache-dir", action="append", default=argparse.SUPPRESS, help="附加 AssetBundles 缓存根目录，可重复。")
    parser.add_argument("--output", default=argparse.SUPPRESS, help="工具数据输出目录。")
    parser.add_argument("--memory-budget-mib", type=int, default=argparse.SUPPRESS, help="资源解压预算，默认 256 MiB。")
    parser.add_argument("--delay-ms", type=int, default=argparse.SUPPRESS, help="资源之间的处理间隔，默认 20 毫秒。")
    parser.add_argument("--yes", action="store_true", default=argparse.SUPPRESS, help="明确接受检测到的目录更新。")
    parser.add_argument("--json", action="store_true", default=argparse.SUPPRESS, help="输出单个 JSON 报告，适用于管理器调用。")
    parser.add_argument("--jsonl", action="store_true", default=argparse.SUPPRESS, help="输出版本化 JSONL，并从标准输入接收取消指令。")
    parser.add_argument("--quiet", action="store_true", default=argparse.SUPPRESS, help="隐藏逐项处理进度。")


def build_parser() -> argparse.ArgumentParser:
    """功能：创建命令入口。入参：无。返回：参数解析器。异常：无。"""
    parser = argparse.ArgumentParser(prog="apmm-util", description="游戏资产自动索引与增量提取；默认单线程、低优先级。")
    common_options(parser)
    parser.add_argument("--version", action="version", version=f"apmm-util {__version__}")
    commands = parser.add_subparsers(dest="command", required=True)
    for name, help_text in [("check", "只读检查当前目录和同步状态。"), ("categories", "列出分类支持情况。"),
                            ("list", "只读列出指定分类资源。"), ("sync", "增量同步一个分类。"),
                            ("extract", "仅提取指定完整资源名。"), ("inspect", "只读诊断手牌、角色或怪物的单个资源。")]:
        sub = commands.add_parser(name, help=help_text)
        common_options(sub)
        if name in ("list", "sync"):
            sub.add_argument("category", help="分类：hand_card；hero 和 monster 的接口已预留。")
        if name in ("extract", "inspect"):
            sub.add_argument("name", help="完整资源名，例如 UT_HandCard_10001。")
    return parser


def normalize_options(args) -> None:
    """功能：补齐通用参数默认值。入参：解析后的参数。返回：无。异常：无。"""
    for name in ("game_dir", "locallow_dir", "cache_dir", "output", "memory_budget_mib", "delay_ms"):
        if not hasattr(args, name):
            setattr(args, name, None)
    for name in ("yes", "json", "jsonl", "quiet"):
        if not hasattr(args, name):
            setattr(args, name, False)


def print_report(report: dict, machine: bool) -> None:
    """功能：输出机器报告或简洁结果。入参：报告和输出模式。返回：无。异常：标准输出写入失败。"""
    if machine:
        print(json.dumps(report, ensure_ascii=False))
        return
    if report.get("status") not in ("ok", "partial"):
        print(report.get("message", "操作失败。"), file=sys.stderr)
        return
    if "extracted" in report:
        print(f"{report['category']}：共 {report['total']} 项，本次提取 {report['extracted']}，复用 {report['reused']}，"
              f"缺失 {report['missing']}，失败 {report['failed']}。")
        print(f"图片目录：{report['outimg_dir']}\n对应表：{report['map_path']}")
    elif "object" in report:
        obj, texture = report["object"], report["texture"]
        print(f"资源：{report['name']}；官方类型：{report['resource_type']}")
        print(f"容器键：{obj['container_key']}；对象号：{obj['object_id']}；序列化文件：{obj['serialized_file']}")
        print(f"纹理：{texture['width']}×{texture['height']}；格式：{texture['format_name']}（{texture['format']}）")
        print(f"布局对象：{report['layout']['object_types']}；对象类型：{report['object_types']}")
        print(f"像素来源：{texture['stream']}；本次未解码像素。")
        for bundle in report["dependencies"]:
            print(f"依赖：{bundle['name']}/{bundle['version_hash']}；路径：{bundle['path']}")
    elif "resources" in report:
        print("\n".join(report["resources"]))
        print(f"共 {report['total']} 项；版本 {report['version']}。")
    elif "available" in report:
        print(f"当前版本：{report['version']}；需要同步：{'是' if report['needs_update'] else '否'}。")
        print(f"目录：{report['catalog_path']}\n分类资源：{report['available']}")
    else:
        for category in report["categories"]:
            print(f"{category['key']}（{category['label']}）：{'已支持' if category['supported'] else '暂未实现'}")
    if "diagnostics" in report:
        summary = report["diagnostics"]
        if not summary["baseline_available"]:
            print("异常记录：首次建立，无历史基线。")
        print(f"异常：当前 {summary['active_count']}；新增 {len(summary['new'])}；持续 {len(summary['continuing'])}；已解决 {len(summary['resolved'])}。")
    if "duration_seconds" in report:
        print(f"耗时 {report['duration_seconds']} 秒；峰值工作集 {report['peak_memory_mib']} MiB。")
    for warning in report.get("warnings", []):
        print(f"提示：{warning}", file=sys.stderr)
    for error in report.get("errors", []):
        print(f"{error['name']}：{error['message']}（{error['stage']}/{error['code']}）", file=sys.stderr)


def main(argv=None) -> int:
    """功能：运行命令并映射退出码。入参：可选命令参数。返回：退出码。异常：参数错误由解析器处理。"""
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")
    parser = build_parser()
    args = parser.parse_args(argv)
    normalize_options(args)
    runtime = Runtime()
    runtime.install()
    if args.jsonl:
        args.json = True
        def receive_cancel():
            """只接受版本 1 的取消消息，取消由事务安全检查点处理。"""
            for line in sys.stdin:
                try:
                    message = json.loads(line)
                    if message.get('protocol') == 1 and message.get('type') == 'cancel':
                        runtime.cancelled.set()
                except (ValueError, TypeError):
                    continue
        threading.Thread(target=receive_cancel, daemon=True).start()

    def emit(message: str) -> None:
        """功能：输出逐项处理进度。入参：进度文本。返回：无。异常：输出写入失败。"""
        if args.jsonl:
            match = re.search(r'\[(\d+)/(\d+)\]', message)
            print(json.dumps({'protocol': 1, 'type': 'progress', 'message': message,
                              'done': int(match[1]) if match else 0, 'total': int(match[2]) if match else None},
                             ensure_ascii=False), flush=True)
        elif not args.json and not args.quiet:
            print(message, file=sys.stderr, flush=True)

    try:
        report = dispatch(args, runtime, emit)
        code = 2 if report["status"] == "partial" else 0
    except ConfirmationRequired as exc:
        report, code = {"status": "confirmation_required", **error_details(exc), "needs_confirmation": True}, 3
    except CatalogChanged as exc:
        report, code = {"status": "catalog_changed", **error_details(exc)}, 4
    except (Cancelled, KeyboardInterrupt) as exc:
        report, code = {"status": "cancelled", **error_details(exc)}, 130
    except (ToolError, OSError, ValueError, ImportError) as exc:
        report, code = {"status": "error", **error_details(exc)}, 1
    if args.jsonl:
        print(json.dumps({'protocol': 1, 'type': 'result', 'result': report}, ensure_ascii=False), flush=True)
    else:
        print_report(report, args.json)
    return code

