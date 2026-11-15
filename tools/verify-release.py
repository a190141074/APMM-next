"""从实际 ZIP 重新核对文件清单、摘要和发行边界，记录磁盘缓存占用。"""
import hashlib
import json
import re
import shutil
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def digest(data):
    return hashlib.sha256(data).hexdigest()

reports = []
for kind, report_file, manifest_file in [
    ("main", "verification-portable-build.json", "release-manifest.json"),
    ("plugin", "verification-plugin-build.json", "plugin.json"),
]:
    build = json.loads((ROOT / report_file).read_text(encoding="utf-8"))
    archive_path = Path(build["zip"])
    assert digest(archive_path.read_bytes()) == build["sha256"]
    with zipfile.ZipFile(archive_path) as archive:
        names = archive.namelist()
        assert len({name.casefold() for name in names}) == len(names)
        assert not any(".." in Path(name).parts or Path(name).is_absolute() for name in names)
        manifest = json.loads(archive.read(manifest_file))
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
            assert all(("/" not in n or n in {"licenses/LICENSE.txt", "licenses/THIRD_PARTY_NOTICES.md"}) and not n.lower().endswith((".py", ".pyd", ".png", ".webp")) for n in names)
            assert "THIRD_PARTY_NOTICES.md" not in names
        reports.append({"kind": kind, "path": str(archive_path), "bytes": archive_path.stat().st_size,
                        "sha256": build["sha256"], "entries": names, "verifiedFiles": len(manifest["files"])})

dependency_file = ROOT / "src-tauri/target-msvc/release/astral-mod-manager.d"
dependencies = dependency_file.read_text(encoding="utf-8")
assert "index.html" in dependencies
assert not re.search(r"[/\\]src[/\\]previews[/\\]", dependencies, re.I)

def directory_size(path):
    return sum(p.stat().st_size for p in path.rglob("*") if p.is_file() and not p.is_symlink()) if path.exists() else 0

caches = []
for directory, reason in [
    (ROOT / "src-tauri/target-msvc", "复用的 MSVC 编译/测试缓存"),
    (ROOT / "node_modules", "锁定的前端开发依赖"),
    (ROOT / "plugins/asset-tool/.venv", "独立资产插件的构建环境"),
    (Path("B:/APMM-next-work"), "源码快照、真实资产结果、隔离 IPC/更新副本、可复现构建暂存和本地签名密钥"),
]:
    caches.append({"path": str(directory), "bytes": directory_size(directory), "retainedReason": reason})
disk = {drive: shutil.disk_usage(drive + ":/").free for drive in "CBD" if Path(drive + ":/").exists()}
report = {"packages": reports, "frontendEmbedded": True, "caches": caches, "freeBytes": disk}
(ROOT / "verification-release-audit.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps({"packages": [{k: v for k, v in r.items() if k != "entries"} for r in reports], "caches": caches, "freeBytes": disk}, ensure_ascii=False))
