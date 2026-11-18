"""按 Git 候选文件生成本地源码清单，可选择复核旧工程基线。"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from work_paths import report_dir

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--baseline", type=Path)
args = parser.parse_args()
names = subprocess.check_output(
    ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=root
).decode("utf-8").split(chr(0))
records = []
for name in sorted(set(names) - {""}):
    file = root / name
    if not file.is_file():
        continue
    data = file.read_bytes()
    records.append({"path": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
                    "lines": len(data.splitlines()) if bytes([0]) not in data else None})
changed = []
if args.baseline:
    baseline = json.loads(args.baseline.read_text(encoding="utf-8-sig"))
    for name, expected in baseline["files"].items():
        file = Path(baseline["source"]) / name
        if not file.is_file() or hashlib.sha256(file.read_bytes()).hexdigest() != expected:
            changed.append(name)
report = {"version": (root / "VERSION").read_text().strip(), "files": records,
          "legacyBaseline": str(args.baseline) if args.baseline else None, "legacyChanged": changed}
output = report_dir() / "source-inventory.json"
output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps({"files": len(records), "report": str(output), "legacyChanged": changed}))
if changed:
    raise SystemExit("旧工程与指定基线不符。")
