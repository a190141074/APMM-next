"""复核本版本三个发行物的实际字节、清单和旧版白名单，不扫描无关缓存。"""
import hashlib
import json
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
read = lambda name: json.loads((ROOT / name).read_text(encoding="utf-8-sig"))
digest = lambda value: hashlib.sha256(value).hexdigest()
report = read("verification-distributions.json")
version = (ROOT / "VERSION").read_text().strip()
assert report["version"] == version
legacy_allowed = {
    "astral-mod-manager.exe", "astral-mod-manager.exe.manifest",
    "WebView2Loader.dll", "api-ms-win-core-synch-l1-2-0.dll",
    "astral-updater.exe", "astral-updater-next.exe", "release-manifest.json",
    "LICENSE.txt", "THIRD_PARTY_NOTICES.md",
}
checks = []
for artifact in report["artifacts"]:
    path = Path(artifact["path"])
    payload = path.read_bytes()
    assert len(payload) == artifact["bytes"] and digest(payload) == artifact["sha256"], path.name
    if artifact["kind"] != "msi":
        with zipfile.ZipFile(path) as archive:
            names = archive.namelist()
            assert len(names) == len({n.lower() for n in names})
            manifest = json.loads(archive.read("release-manifest.json"))
            assert manifest["version"] == version and manifest["dataSchema"] == 1
            assert set(names) == {f["path"] for f in manifest["files"]} | {"release-manifest.json"}
            for item in manifest["files"]:
                assert ".." not in Path(item["path"]).parts
                data = archive.read(item["path"])
                assert len(data) == item["bytes"] and digest(data) == item["sha256"], item["path"]
            assert not any(n.split("/")[0].lower() in {"data", "vault", "previews", "credentials"} for n in names)
            if artifact["kind"] in {"legacy-bridge", "compat-0.3.39"}:
                assert set(names) == (legacy_allowed if artifact["kind"] == "legacy-bridge" else legacy_allowed - {"astral-updater-next.exe"})
                assert (ROOT / "NOTICE.md").read_bytes() in archive.read("THIRD_PARTY_NOTICES.md")
            else:
                assert json.loads(archive.read("installation.json"))["kind"] == "portable"
            checks.append({**artifact, "filesVerified": len(manifest["files"])})
    else:
        checks.append(artifact)
result = {"version": version, "artifacts": checks}
(ROOT / "verification-distribution-audit.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps(result, ensure_ascii=False))
