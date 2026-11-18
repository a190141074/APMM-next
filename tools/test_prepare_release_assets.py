"""验证发行附件准备流程的隔离、版本和摘要边界。"""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import uuid

SOURCE = Path(__file__).resolve().parents[1]
WORK = Path(os.environ.get("APMM_WORK_DIR", "B:/APMM-next-work"))
PREVIEWS = WORK / "repository-cleanup-20261115/preview-v4-webp/public"
ASSET_CHECKOUT = WORK / "ui-050/publication/assets-repo"
TOOL = SOURCE / "tools/prepare-release-assets.py"


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PrepareReleaseAssetsTests(unittest.TestCase):
    def setUp(self):
        test_work = WORK / "release-workflow-tests"
        test_work.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="release-prepare-", dir=test_work)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "cnb"
        self.github = self.root / "github"
        self.reports = self.root / "reports"
        self.version = (SOURCE / "VERSION").read_text(encoding="utf-8").strip()
        self.distributions = self.root / "distributions.json"
        artifacts = []
        suffixes = {
            "portable": "-portable.zip",
            "msi": ".msi",
            "legacy-bridge": "-legacy-bridge.zip",
            "compat-0.3.39": "-compat-0.3.39.zip",
        }
        package_dir = self.root / "packages"
        package_dir.mkdir()
        for kind, suffix in suffixes.items():
            package = package_dir / f"APMM-{self.version}-windows-x86_64{suffix}"
            package.write_bytes(f"test artifact: {kind} {self.version}".encode())
            artifacts.append({
                "kind": kind,
                "path": str(package),
                "bytes": package.stat().st_size,
                "sha256": sha(package),
            })
        self.distributions.write_text(
            json.dumps({"version": self.version, "artifacts": artifacts}), encoding="utf-8"
        )
        self.repo.mkdir()
        subprocess.run(["git", "init", "-q", "--", str(self.repo)], check=True)
        self.env = {**os.environ, "APMM_REPORT_DIR": str(self.reports)}

    def run_tool(self, *args):
        return subprocess.run(
            [sys.executable, "-X", "utf8", "-B", str(TOOL), "--assets-repo", str(self.repo),
             "--github-dir", str(self.github), *map(str, args)],
            cwd=SOURCE, env=self.env, capture_output=True, text=True, encoding="utf-8",
        )

    def test_application_only_preserves_existing_preview_assets_and_readme(self):
        (self.repo / "bundles").mkdir()
        sentinels = {
            self.repo / "preview-manifest-v4.json": b"modern preview manifest",
            self.repo / "preview-manifest.json": b"legacy preview manifest",
            self.repo / "bundles/README.md": b"preview documentation",
            self.repo / "bundles/previous.zip": b"previous preview bundle",
        }
        for path, content in sentinels.items():
            path.write_bytes(content)
        previous_section = "## 原图预览\n\n历史说明原样保留。\n\n## 自定义章节\n\n也保留。\n"
        (self.repo / "README.md").write_text(
            "# 下载\n\n## 应用下载\n\n旧版本链接\n\n" + previous_section, encoding="utf-8"
        )

        result = self.run_tool("--distributions", self.distributions)
        self.assertEqual(result.returncode, 0, result.stderr)
        for path, content in sentinels.items():
            self.assertEqual(path.read_bytes(), content)
        self.assertIn(previous_section, (self.repo / "README.md").read_text(encoding="utf-8"))
        self.assertFalse((self.github / "previews").exists())

        plan = read_json(self.reports / "publication-plan.json")
        self.assertFalse(plan["remoteApplied"])
        self.assertEqual(len(plan["cnb"]["stages"]), 2)
        self.assertFalse(any("preview-manifest" in item["path"] for item in plan["files"]))
        app = self.github / "application" / f"v{self.version}"
        github_modern = read_json(app / "update-manifest-v2.json")
        cnb_modern = read_json(self.repo / "update-manifest-v2.json")
        github_legacy = read_json(app / "update-manifest.json")
        cnb_legacy = read_json(self.repo / "update-manifest.json")
        self.assertEqual(github_modern["version"], self.version)
        changelog = (SOURCE / "CHANGELOG.md").read_text(encoding="utf-8-sig")
        first_note = next(line[2:] for line in changelog.splitlines() if line.startswith("- "))
        self.assertIn(first_note, github_modern["notes"])
        self.assertIn(first_note, (app / "RELEASE-NOTES.md").read_text(encoding="utf-8"))
        self.assertIn(f"APMM-{self.version}", (self.repo / "README.md").read_text(encoding="utf-8"))
        self.assertIn("astral-mod-manager.exe", (app / "RELEASE-NOTES.md").read_text(encoding="utf-8"))
        for github_asset, cnb_asset in zip(github_modern["assets"], cnb_modern["assets"]):
            self.assertIn(f"/releases/download/v{self.version}/", github_asset["url"])
            self.assertIn(f"/releases/{self.version}/windows-x86_64/", cnb_asset["url"])
            filename = github_asset["url"].rsplit("/", 1)[1]
            self.assertEqual(sha(app / filename), cnb_asset["sha256"])
        self.assertEqual(github_legacy, cnb_legacy)
        self.assertIn("legacy-bridge.zip", github_legacy["assets"][0]["url"])

    def test_existing_cnb_preview_bytes_can_be_reused_for_github_repeatedly(self):
        args = ("--distributions", self.distributions, "--previews", PREVIEWS)
        result = self.run_tool(*args)
        self.assertEqual(result.returncode, 0, result.stderr)
        result = self.run_tool(*args)
        self.assertEqual(result.returncode, 0, result.stderr)

        plan = read_json(self.reports / "publication-plan.json")
        self.assertEqual([item["writtenFiles"] for item in plan["cnb"]["stages"]], [[], [], []])
        self.assertEqual(len(plan["github"]["application"]["attachments"]), 7)
        self.assertEqual(len(plan["github"]["previews"]["attachments"]), 14)
        self.assertEqual(plan["github"]["previews"]["tag"], "preview-v4")

        destination = self.github / "previews/preview-v4"
        for name, count in (("preview-manifest-v4.json", 8), ("preview-manifest.json", 3)):
            manifest = read_json(destination / name)
            self.assertEqual(len(manifest["bundles"]), count)
            for bundle in manifest["bundles"]:
                filename = bundle["bundleUrl"].rsplit("/", 1)[1]
                self.assertIn("/releases/download/preview-v4/", bundle["bundleUrl"])
                self.assertEqual(sha(destination / filename), bundle["sha256"])
                relative = f"bundles/preview-v4/{filename}" if count == 8 else f"bundles/{filename}"
                self.assertEqual(sha(self.repo / relative), bundle["sha256"])

        for item in plan["files"]:
            parent = self.repo if item["channel"] == "cnb" else self.github
            path = parent / item["path"]
            self.assertEqual(path.stat().st_size, item["bytes"])
            self.assertEqual(sha(path), item["sha256"])

    def test_previews_only_needs_no_distribution_and_preserves_bundle_bytes(self):
        result = self.run_tool("--previews-only", "--previews", PREVIEWS)
        self.assertEqual(result.returncode, 0, result.stderr)

        plan = read_json(self.reports / "publication-plan.json")
        self.assertEqual(set(plan["github"]), {"previews"})
        self.assertEqual(plan["manifestsLast"], ["preview-manifest-v4.json", "preview-manifest.json"])
        self.assertEqual([stage["action"] for stage in plan["cnb"]["stages"]], [
            "packages-then-verify-public-size-and-sha256",
            "preview-manifests-then-verify-public-downloads",
            "update-manifests-and-download-indexes-then-verify",
        ])
        paths = {item["path"] for item in plan["files"]}
        self.assertFalse(any(path.startswith("releases/") or path.startswith("application/")
                             or path in {"update-manifest.json", "update-manifest-v2.json"}
                             for path in paths))
        self.assertFalse((self.repo / "releases").exists())
        self.assertFalse((self.repo / "update-manifest.json").exists())
        self.assertFalse((self.repo / "update-manifest-v2.json").exists())
        self.assertFalse((self.github / "application").exists())

        source_packages = {path.name: path for path in (PREVIEWS / "bundles").rglob("*.zip")}
        for item in plan["files"]:
            if not item["path"].endswith(".zip"):
                continue
            filename = item["path"].rsplit("/", 1)[-1]
            target = self.repo if item["channel"] == "cnb" else self.github
            self.assertEqual((target / item["path"]).read_bytes(), source_packages[filename].read_bytes())

    def test_bad_release_version_size_and_sha_fail_before_mutation(self):
        for field in ("version", "bytes", "sha256"):
            with self.subTest(field=field):
                report = read_json(self.distributions)
                if field == "version":
                    report[field] = "1.2.3"
                else:
                    artifact = report["artifacts"][-1]
                    artifact[field] = artifact[field] + 1 if field == "bytes" else "0" * 64
                path = self.root / f"bad-{field}.json"
                path.write_text(json.dumps(report), encoding="utf-8")
                result = self.run_tool("--distributions", path)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((self.repo / "releases").exists())
                self.assertFalse(self.github.exists())
                self.assertFalse((self.reports / "publication-plan.json").exists())

    def test_github_conflict_is_found_before_cnb_assets_are_written(self):
        app = self.github / "application" / f"v{self.version}"
        app.mkdir(parents=True)
        conflict = app / f"APMM-{self.version}-windows-x86_64-compat-0.3.39.zip"
        conflict.write_bytes(b"existing different artifact")
        result = self.run_tool("--distributions", self.distributions)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("不覆盖已有不同内容", result.stderr)
        self.assertEqual(conflict.read_bytes(), b"existing different artifact")
        self.assertFalse((self.repo / "releases").exists())
        self.assertEqual(list(app.iterdir()), [conflict])


if __name__ == "__main__":
    unittest.main(verbosity=2)
