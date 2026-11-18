"""统一本地验收输出目录，避免生成报告进入公开源码。"""
import os
from pathlib import Path


def report_dir() -> Path:
    """返回源码之外的报告目录；调用者可以按本轮任务显式指定。"""
    root = Path(__file__).resolve().parents[1]
    version = (root / "VERSION").read_text(encoding="utf-8").strip()
    work = Path(os.environ.get("APMM_WORK_DIR", "B:/APMM-next-work"))
    directory = Path(os.environ.get("APMM_REPORT_DIR", work / "reports" / version)).resolve()
    if directory == root or root in directory.parents:
        raise ValueError("验收报告必须保存到源码目录之外。")
    directory.mkdir(parents=True, exist_ok=True)
    return directory


def build_report() -> Path:
    return Path(os.environ.get("APMM_BUILD_REPORT", report_dir() / "portable-build.json"))


def distributions_report() -> Path:
    return Path(os.environ.get("APMM_DISTRIBUTIONS_REPORT", report_dir() / "distributions.json"))
