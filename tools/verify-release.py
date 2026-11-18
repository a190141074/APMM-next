"""从实际 ZIP 重新核对文件清单、摘要和发行边界，只检查本轮明确指定的产物。"""
import hashlib
import json
import re
import argparse
from work_paths import build_report, report_dir
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def digest(data):
    return hashlib.sha256(data).hexdigest()

parser = argparse.ArgumentParser()
parser.add_argument("--build-report", type=Path, default=build_report())
parser.add_argument("--plugin-report", type=Path)
args = parser.parse_args()
inputs = [("main", args.build_report, "release-manifest.json")]
if args.plugin_report:
    inputs.append(("plugin", args.plugin_report, "plugin.json"))
reports = []
for kind, report_file, manifest_file in inputs:
    build = json.loads(report_file.read_text(encoding="utf-8-sig"))
    if kind == "main":
        assert build["version"] == (ROOT / "VERSION").read_text().strip()
    archive_path = Path(build["zip"])
    assert digest(archive_path.read_bytes()) == build["sha256"]
    with zipfile.ZipFile(archive_path) as archive:
        names = archive.namelist()
        assert len({name.casefold() for name in names}) == len(names)
        assert not any(".." in Path(name).parts or Path(name).is_absolute() for name in names)
        manifest = json.loads(archive.read(manifest_file))
        if kind == "main":
            assert manifest["version"] == build["version"]
        expected = {record["path"] for record in manifest["files"]} | {manifest_file}
        if kind == "plugin":
            expected.add("plugin.sig")
        assert set(names) == expected
        for record in manifest["files"]:
            payload = archive.read(record["path"])
            assert len(payload) == record["bytes"]
            assert digest(payload) == record["sha256"]
        assert not any(re.search(r"(^|/)(data|outimg|previews|vault|node_modules)(/|$)", n, re.I) for n in names)
        assert not any("private" in n.lower() or n.lower().endswith((".pem", ".key")) for n in names)
        assert not any(re.search(r"(^|/)(UT_|cursor_(up|down)\.)", n) for n in names)
        if kind == "main":
            assert all(("/" not in n or n in {"licenses/LICENSE.txt", "licenses/THIRD_PARTY_NOTICES.md", "licenses/NOTICE.md"}) and not n.lower().endswith((".py", ".pyd", ".png", ".webp")) for n in names)
            assert "THIRD_PARTY_NOTICES.md" not in names
        reports.append({"kind": kind, "path": str(archive_path), "bytes": archive_path.stat().st_size,
                        "sha256": build["sha256"], "entries": names, "verifiedFiles": len(manifest["files"])})

dependency_file = ROOT / "src-tauri/target-msvc/release/astral-mod-manager.d"
dependencies = dependency_file.read_text(encoding="utf-8")
assert "index.html" in dependencies
assert not re.search(r"[/\\]src[/\\]previews[/\\]", dependencies, re.I)

report = {"packages": reports, "frontendEmbedded": True}
output = report_dir() / "release-audit.json"
output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps({"report": str(output), "packages": [{k: v for k, v in r.items() if k != "entries"} for r in reports]}, ensure_ascii=False))
