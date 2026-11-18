"""核对已审计的应用和预览包，在隔离目录准备 CNB 与 GitHub 的发布材料。"""

import argparse
import hashlib
import json
import re
import shutil
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from tempfile import NamedTemporaryFile
from urllib.parse import quote

from work_paths import distributions_report, report_dir

ROOT = Path(__file__).resolve().parents[1]
CNB_UPDATE = json.loads((ROOT / "config/cnb.json").read_text("utf-8"))["update"]
PREFIX = CNB_UPDATE["rawPrefix"]
GITHUB_APP = CNB_UPDATE["fallback"].split("/releases/", 1)[0]
GITHUB_PREVIEWS = "https://github.com/a190141074/AstralPartyModManager-Previews"
UPDATE_NAMES = ("update-manifest-v2.json", "update-manifest.json")
PREVIEW_NAMES = ("preview-manifest-v4.json", "preview-manifest.json")
PACKAGE_SUFFIXES = {
    "portable": "-portable.zip",
    "msi": ".msi",
    "legacy-bridge": "-legacy-bridge.zip",
    "compat-0.3.39": "-compat-0.3.39.zip",
}


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def sha(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def inside(root, relative):
    """验证现有路径与未来输出都保持在各自的目录里。"""
    path = root / relative
    if not path.resolve().is_relative_to(root):
        raise ValueError(f"路径越界：{relative}")
    return path


def release_notes(version):
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8-sig")
    sections = re.finditer(r"^##[ \t]+(?P<title>[^\n]+)\n(?P<body>.*?)(?=^##[ \t]|\Z)",
                           changelog, re.MULTILINE | re.DOTALL)
    title = re.compile(rf"v?{re.escape(version)}(?:[ \t]*(?:[（(][^\n]+[）)]|—[^\n]+))?")
    bodies = [section["body"].strip() for section in sections if title.fullmatch(section["title"].strip())]
    if len(bodies) != 1 or not bodies[0]:
        raise ValueError(f"CHANGELOG.md 必须有一段非空的 {version} 更新说明。")
    body = bodies[0]
    bullets = [line.strip() for line in re.findall(r"^[-*][ \t]+(.+)$", body, re.MULTILINE)]
    summary = " ".join(bullets[:3]) if bullets else next(line.strip() for line in body.splitlines() if line.strip())
    return body, summary


@dataclass(frozen=True)
class Package:
    source: Path
    size: int
    sha256: str

    @classmethod
    def checked(cls, source, record):
        """对照发行报告或预览清单的大小与 SHA-256 验证本轮附件。"""
        size, expected = record["bytes"], record["sha256"]
        if (type(size) is not int or size <= 0 or not re.fullmatch(r"[a-f0-9]{64}", expected)
                or not source.is_file() or source.stat().st_size != size or sha(source) != expected):
            raise ValueError(f"输入包大小或摘要不符：{source.name}")
        return cls(source, size, expected)


@dataclass
class Output:
    path: Path
    content: Package | bytes
    mutable: bool = False

    @property
    def size(self):
        return self.content.size if isinstance(self.content, Package) else len(self.content)

    @property
    def digest(self):
        return (self.content.sha256 if isinstance(self.content, Package)
                else hashlib.sha256(self.content).hexdigest())

    def needs_write(self):
        for parent in self.path.parents:
            if parent.exists() and not parent.is_dir():
                raise ValueError(f"发行目录被文件占用：{parent}")
        if self.path.exists():
            if self.path.is_file() and self.path.stat().st_size == self.size and sha(self.path) == self.digest:
                return False
            if not self.mutable or not self.path.is_file():
                raise ValueError(f"不覆盖已有不同内容的发行文件：{self.path}")
        elif self.path.is_symlink():
            raise ValueError(f"发行路径指向不存在的文件：{self.path}")
        return True

    def write(self):
        """先在同目录校验完整副本，再替换可更新的索引。"""
        if not self.needs_write():
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with NamedTemporaryFile(dir=self.path.parent, delete=False) as handle:
            stage = Path(handle.name)
        try:
            if isinstance(self.content, Package):
                shutil.copyfile(self.content.source, stage)
            else:
                stage.write_bytes(self.content)
            if stage.stat().st_size != self.size or sha(stage) != self.digest:
                raise ValueError(f"写入前复核失败：{self.path.name}")
            if self.mutable:
                stage.replace(self.path)
            else:
                # 同一目录原子创建，避免并发准备时覆盖另一轮已存在的不可变附件。
                try:
                    self.path.hardlink_to(stage)
                except FileExistsError:
                    self.needs_write()
        finally:
            stage.unlink(missing_ok=True)


def load_application(report_path, version):
    report = read_json(report_path)
    if report["version"] != version:
        raise ValueError("发行报告不是当前源码版本。")
    records = {item["kind"]: item for item in report["artifacts"]}
    if records.keys() != PACKAGE_SUFFIXES.keys() or len(report["artifacts"]) != len(PACKAGE_SUFFIXES):
        raise ValueError("发行报告应包含且仅包含四种 Windows 发行包。")
    application = {}
    for kind, suffix in PACKAGE_SUFFIXES.items():
        source = Path(records[kind]["path"])
        if source.name != f"APMM-{version}-windows-x86_64{suffix}":
            raise ValueError(f"发行包文件名与当前版本和包类型不符：{source.name}")
        package = Package.checked(source, records[kind])
        group = "windows-x86_64" if kind in {"portable", "msi"} else "compatibility"
        application[kind] = (f"releases/{version}/{group}/{source.name}", package)
    return application


def load_previews(public):
    """校验两套预览共享同一预览版本，沿用构建器记录的编码来源和包摘要。"""
    public = public.resolve()
    provenance = read_json(public.parent / "preview-build.json")
    if provenance.get("encoding") != {"format": "WEBP", "lossless": False, "quality": 50,
                                      "method": 4, "resized": False, "alpha": "lossless"}:
        raise ValueError("预览必须来自已核验的质量 50 有损 WebP 构建。")
    version = str(provenance["previewVersion"])
    if not re.fullmatch(r"[0-9]+", version):
        raise ValueError("预览构建报告的版本必须是整数。")
    tag = f"preview-v{version}"
    manifests = {name: read_json(public / name) for name in PREVIEW_NAMES}
    packages = {}
    github_prefix = f"{GITHUB_PREVIEWS}/releases/download/{tag}/"
    for name, role in zip(PREVIEW_NAMES, ("modern", "legacy")):
        manifest = manifests[name]
        if manifest != provenance["manifests"][role] or manifest["schema"] != 2 or not manifest["bundles"]:
            raise ValueError(f"预览清单与构建报告不一致：{name}")
        names = set()
        for bundle in manifest["bundles"]:
            key = bundle["key"]
            if (not re.fullmatch(r"[a-z0-9-]+", key) or key in names
                    or str(bundle["version"]) != version):
                raise ValueError(f"预览清单存在重复分类或版本不符：{key}")
            names.add(key)
            if role == "modern":
                relative = f"bundles/{tag}/APMM-previews-{key}-{version}.zip"
                expected_url = PREFIX + relative
            else:
                filename = f"astral-party-previews-{key}-{tag}-compat.zip"
                relative = "bundles/" + filename
                expected_url = github_prefix + filename
            if bundle["bundleUrl"] != expected_url:
                raise ValueError(f"预览清单链接不符：{name} / {key}")
            package = Package.checked(inside(public, relative), bundle)
            packages[relative] = package
    return tag, manifests, packages, provenance


def app_manifests(version, notes, packages, github=False):
    tag_prefix = f"{GITHUB_APP}/releases/download/v{version}/"
    assets = []
    for kind in ("portable", "msi"):
        relative, package = packages[kind]
        url = tag_prefix + quote(package.source.name) if github else PREFIX + relative
        assets.append({"platform": "win-x86_64", "kind": kind, "url": url,
                       "sha256": package.sha256, "bytes": package.size})
    relative, bridge = packages["legacy-bridge"]
    # 历史客户端的 schema 1 通过已验证的 CNB 桥接 URL 更新。
    legacy = {"platform": "win-x86_64", "url": PREFIX + relative,
              "sha256": bridge.sha256, "bytes": bridge.size}
    return {UPDATE_NAMES[0]: {"schema": 2, "version": version, "notes": notes, "assets": assets},
            UPDATE_NAMES[1]: {"schema": 1, "version": version, "notes": notes, "assets": [legacy]}}


def cnb_documents(repo, version, preview_info, previews_only=False):
    """更新下载入口和本轮提供的预览说明，保留资产仓其他内容。"""
    readme = repo / "README.md"
    text = readme.read_text(encoding="utf-8-sig") if readme.exists() else "# APMM 下载与资源\n"

    def section(heading, body):
        pattern = re.compile(rf"^## {re.escape(heading)}[ \t]*\n.*?(?=^## |\Z)", re.MULTILINE | re.DOTALL)
        if pattern.search(text):
            return pattern.sub(lambda _: body + "\n", text, count=1).rstrip() + "\n"
        return text.rstrip() + "\n\n" + body

    documents = {}
    if not previews_only:
        text = section("应用下载", f"""## 应用下载

| 适用场景 | 下载 |
| --- | --- |
| 便携版 | [APMM {version} 便携版](releases/{version}/windows-x86_64/APMM-{version}-windows-x86_64-portable.zip)：解压后打开 `APMM-{version}` 文件夹运行 |
| 固定安装 | [APMM {version} 安装版](releases/{version}/windows-x86_64/APMM-{version}-windows-x86_64.msi) |

旧版自动更新使用 [兼容包](releases/{version}/compatibility/)，每个发行包附带 ".sha256" 校验文件。
""")
        release_index = f"""# 应用发行

当前版本：[APMM {version}]({version}/windows-x86_64/)。旧程序升级专用包位于 [兼容目录]({version}/compatibility/)。

历史平铺文件保留旧下载地址，新发行包使用版本目录。
"""
        documents["releases/README.md"] = release_index
    if preview_info:
        tag, _, _, provenance = preview_info
        retained = len(provenance["retainedLegacy"])
        game = provenance["gameVersion"]
        text = section("原图预览", f"""## 原图预览

[完整 {tag} 预览包](bundles/{tag}/)使用质量 50 的有损 WebP，保留原图尺寸和透明通道。当前客户端读取 "preview-manifest-v4.json"，旧客户端读取独立的固定资源集合清单 "preview-manifest.json"。

预览版本独立于应用版本。{retained} 个历史项目沿用已核验的旧预览，其他原图来自游戏 {game} 的提取结果。
""")
        documents["bundles/README.md"] = f"""# 原图预览

- [完整 {tag} 预览包]({tag}/)按分类下载。
- 兼容包使用平铺文件名，保持旧客户端可识别的固定资源集合。
- 已发布历史包保留原文件和旧下载地址。
"""
    documents["README.md"] = text
    return {name: content.encode("utf-8") for name, content in documents.items()}


def github_outputs(directory, endpoint, tag, title, notes, packages, manifests):
    """生成 GitHub 草稿的扁平附件清单及供审阅的正文。"""
    entries = {item.source.name: item for item in packages}
    if len(entries) != len(packages):
        raise ValueError("GitHub Release 附件名重复。")
    entries["SHA256SUMS.txt"] = "".join(f"{item.sha256}  {name}\n"
                                         for name, item in sorted(entries.items())).encode("ascii")
    entries.update({name: json_bytes(value) for name, value in manifests.items()})
    body = "RELEASE-NOTES.md"
    files = [Output(inside(directory, name), content) for name, content in entries.items()]
    files.append(Output(inside(directory, body), notes.encode("utf-8")))
    steps = [{"action": "upload-draft-assets", "files": list(entries)},
             {"action": "verify-draft-size-and-sha256", "files": list(entries)},
             {"action": "publish-and-verify-public-downloads", "files": list(entries)}]
    return files, {"repository": endpoint, "tag": tag, "title": title, "directory": str(directory),
                   "notesFile": body, "attachments": list(entries), "stages": steps}


def prepare(args):
    """先核对所有输入和输出冲突，再整理可审阅的本地发布计划。"""
    repo = args.assets_repo.resolve()
    github_dir = args.github_dir.resolve() if args.github_dir else None
    if not (repo / ".git").exists() or repo == ROOT or ROOT in repo.parents or repo in ROOT.parents:
        raise ValueError("请指定独立的资产仓库副本。")
    if github_dir and any(a == b or a in b.parents or b in a.parents for a, b in
                          [(github_dir, repo), (github_dir, ROOT)]):
        raise ValueError("GitHub 暂存目录必须与源码和 CNB 资产仓分开。")
    version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("VERSION 必须是三段数字。")
    changelog, notes = release_notes(version)
    previews_only = args.previews_only
    application = {} if previews_only else load_application(args.distributions, version)
    previews = load_previews(args.previews) if args.previews else None
    artifacts = {relative: package for relative, package in application.values()}
    if previews:
        artifacts.update(previews[2])
    artifact_files = {}
    for relative, package in artifacts.items():
        artifact_files[relative] = package
        artifact_files[relative + ".sha256"] = f"{package.sha256}  {package.source.name}\n".encode("ascii")
    cnb = []
    github = {}
    outputs = []

    def cnb_stage(action, files, mutable=False):
        stage = [Output(inside(repo, name), item, mutable) for name, item in files.items()]
        cnb.append({"action": action, "files": list(files),
                    "writtenFiles": [name for name, output in zip(files, stage) if output.needs_write()]})
        outputs.extend(stage)

    if artifact_files:
        cnb_stage("packages-then-verify-public-size-and-sha256", artifact_files)
    if previews:
        cnb_stage("preview-manifests-then-verify-public-downloads",
                  {name: json_bytes(value) for name, value in previews[1].items()}, mutable=True)
    update_files = {} if previews_only else {
        name: json_bytes(value) for name, value in app_manifests(version, notes, application).items()}
    update_files.update(cnb_documents(repo, version, previews, previews_only))
    cnb_stage("update-manifests-and-download-indexes-then-verify", update_files, mutable=True)
    if github_dir and not previews_only:
        tag = f"v{version}"
        title = f"APMM {version}"
        body = (f"{changelog}\n\n## 下载\n\n"
                f"- 便携 ZIP 解压后打开 `APMM-{version}` 文件夹，运行其中的 `astral-mod-manager.exe`。\n- MSI 用于固定安装和系统卸载。\n"
                "- legacy-bridge 用于已发布的 0.3.36 升级；compat-0.3.39 仅用于对应测试版。\n"
                "- 下载后可使用 SHA256SUMS.txt 核对摘要。\n")
        files, github["application"] = github_outputs(
            inside(github_dir, f"application/{tag}").resolve(), GITHUB_APP, tag, title, body,
            [package for _, package in application.values()], app_manifests(version, notes, application, github=True))
        outputs.extend(files)
    if github_dir and previews:
        tag, preview_manifests, preview_packages, _ = previews
        prefix = f"{GITHUB_PREVIEWS}/releases/download/{tag}/"
        manifests = {name: {**manifest, "bundles": [
            {**bundle, "bundleUrl": prefix + quote(PurePosixPath(bundle["bundleUrl"]).name)}
            for bundle in manifest["bundles"]]} for name, manifest in preview_manifests.items()}
        title = f"Preview bundles {tag.removeprefix('preview-')}"
        body = (f"# {title}\n\n包含当前客户端的完整分类包和旧客户端的固定集合兼容包。\n"
                "完整清单和兼容清单均指向此标签下的附件。\n")
        files, github["previews"] = github_outputs(
            inside(github_dir, f"previews/{tag}").resolve(), GITHUB_PREVIEWS, tag, title, body,
            list(preview_packages.values()), manifests)
        outputs.extend(files)

    def record(output):
        target = repo if output.path.is_relative_to(repo) else github_dir
        return {"channel": "cnb" if target == repo else "github",
                "path": output.path.relative_to(target).as_posix(), "bytes": output.size, "sha256": output.digest}

    plan = {"version": version, "repository": str(repo), "remoteApplied": False,
            "files": [record(output) for output in outputs],
            "manifestsLast": [*(PREVIEW_NAMES if previews else ()), *( () if previews_only else UPDATE_NAMES)],
            "cnb": {"repository": CNB_UPDATE["ownerRepo"], "branch": "main", "stages": cnb},
            "github": github}
    report = Output(report_dir() / "publication-plan.json", json_bytes(plan), mutable=True)
    for output in [*outputs, report]:
        output.needs_write()
    for output in [*outputs, report]:
        output.write()
    print(json.dumps({"version": version, "files": len(outputs), "remoteApplied": False}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets-repo", type=Path, required=True, help="独立的 CNB 资产仓副本。")
    parser.add_argument("--previews", type=Path, help="已审计预览构建中的 public 目录。")
    parser.add_argument("--previews-only", action="store_true", help="仅准备预览包，不生成应用发行材料。")
    parser.add_argument("--distributions", type=Path, help="本轮 distributions.json。")
    parser.add_argument("--github-dir", type=Path, help="独立的 GitHub Release 附件暂存根目录。")
    options = parser.parse_args()
    if options.previews_only and options.previews is None:
        parser.error("--previews-only 必须同时指定 --previews。")
    if options.distributions is None and not options.previews_only:
        options.distributions = distributions_report()
    prepare(options)
