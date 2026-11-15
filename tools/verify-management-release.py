"""审计本轮管理器 ZIP、实际 EXE、版本文件和原生报告，不借用历史插件报告。"""
import hashlib
import json
import re
import shutil
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sha = lambda data: hashlib.sha256(data).hexdigest()
read_json = lambda name: json.loads((ROOT / name).read_text(encoding="utf-8"))
build = read_json("verification-portable-build.json")
native = read_json("verification-management-native.json")
ui = read_json("verification-management-ui.json")
version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
assert version == build["version"] == native["version"] == ui["version"] == "0.3.39"
assert read_json("package.json")["version"] == version
assert read_json("src-tauri/tauri.conf.json")["version"] == version
assert re.search(r'^version = "' + re.escape(version) + '"$', (ROOT / "src-tauri/Cargo.toml").read_text(encoding="utf-8"), re.M)
assert native["allPassed"] and ui["allPassed"]
archive = Path(build["zip"])
assert sha(archive.read_bytes()) == build["sha256"]
allowed = {"astral-mod-manager.exe", "astral-updater.exe", "astral-mod-manager.exe.manifest", "WebView2Loader.dll", "api-ms-win-core-synch-l1-2-0.dll", "licenses/LICENSE.txt", "licenses/THIRD_PARTY_NOTICES.md", "release-manifest.json"}
with zipfile.ZipFile(archive) as package:
    assert set(package.namelist()) == allowed
    assert len(package.namelist()) == len(allowed)
    manifest = json.loads(package.read("release-manifest.json"))
    assert manifest["version"] == version
    assert {f["path"] for f in manifest["files"]} == allowed - {"release-manifest.json"}
    for f in manifest["files"]:
        data = package.read(f["path"])
        assert len(data) == f["bytes"] and sha(data) == f["sha256"]
        assert sha((Path(build["payload"]) / f["path"]).read_bytes()) == f["sha256"]
    executable_sha = sha(package.read("astral-mod-manager.exe"))
    assert executable_sha == native["executableSha256"]
    assert sha((Path(native["output"]) / "app/astral-mod-manager.exe").read_bytes()) == executable_sha
dependencies = (ROOT / "src-tauri/target-msvc/release/astral-mod-manager.d").read_text(encoding="utf-8")
assert "index.html" in dependencies and not re.search(r"[/\\]src[/\\]previews[/\\]", dependencies, re.I)
# 交付副本存入本轮唯一 B 盘构建目录，并再次核对摘要。
delivery = Path(build["stage"]) / archive.name
shutil.copyfile(archive, delivery)
assert sha(delivery.read_bytes()) == build["sha256"]
report = {"version": version, "zip": str(archive), "delivery": str(delivery), "bytes": archive.stat().st_size, "sha256": build["sha256"], "executableSha256": executable_sha, "files": sorted(allowed), "frontendEmbedded": True, "nativeMatchesArchive": True, "browserChecks": len(ui["checks"]), "nativeChecks": len(native["checks"]), "allPassed": True}
(ROOT / "verification-management-release.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps(report, ensure_ascii=False))
