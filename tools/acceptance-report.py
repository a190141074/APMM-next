"""汇总明确指定的验收记录，不推断测试通过，也不输出到公开文档。"""
import argparse
import json
from pathlib import Path
from work_paths import report_dir

parser = argparse.ArgumentParser()
parser.add_argument("inputs", nargs="+", type=Path)
args = parser.parse_args()
records = [{"source": str(file.resolve()), "result": json.loads(file.read_text("utf-8-sig"))}
           for file in args.inputs]
output = report_dir() / "acceptance.json"
output.write_text(json.dumps(records, ensure_ascii=False, indent=2), encoding="utf-8")
print(output)
