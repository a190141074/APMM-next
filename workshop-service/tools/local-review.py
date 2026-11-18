"""本地收稿、审核证据校验和签名目录准备，不需要服务器。"""
import argparse
import json
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from workshop_service.local_review import receive, prepare, read_key, verify_public, check_base


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    receive_args = commands.add_parser("inspect")
    receive_args.add_argument("archive", type=Path)
    for command in (receive_args,):
        command.add_argument("--index", required=True, type=Path)
        command.add_argument("--output", required=True, type=Path)
    prepare_args = commands.add_parser("prepare")
    for name in ("review", "evidence", "catalog", "output", "key", "index"):
        prepare_args.add_argument("--" + name, required=True, type=Path)
    verify_args = commands.add_parser("verify-public")
    verify_args.add_argument("--prepared", required=True, type=Path)
    verify_args.add_argument("--base", required=True)
    check_args = commands.add_parser("check-base")
    check_args.add_argument("--prepared", required=True, type=Path)
    check_args.add_argument("--catalog", required=True, type=Path)
    args = parser.parse_args()
    if args.action == "check-base":
        check_base(args.prepared, args.catalog)
        print("资产仓库基线及待发布文件未改变。")
        return
    if args.action == "verify-public":
        verify_public(args.prepared, args.base)
        print("公开目录与本地待发布清单一致。")
        return
    allowed = set(json.loads(args.index.read_text(encoding="utf-8-sig")))
    if args.action == "inspect":
        result = receive(args.archive, args.output, allowed)
        print("本地收稿校验通过，请填写审核证据。SHA-256：" + result["snapshotHash"])
    else:
        result = prepare(args.review, args.evidence, args.catalog, args.output, read_key(args.key), allowed)
        print("签名目录已准备，尚未发布。目录摘要：" + result["catalogHash"])


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError) as error:
        print("本地审核失败：" + str(error), file=sys.stderr)
        raise SystemExit(1)
