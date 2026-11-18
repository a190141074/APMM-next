"""在隔离资产仓库准备版本目录和当前索引；不提交、不推送，也不改旧包字节。"""
import argparse
import hashlib
import json
import shutil
from pathlib import Path
from work_paths import distributions_report, report_dir

ROOT = Path(__file__).resolve().parents[1]
PREFIX = json.loads((ROOT / "config/cnb.json").read_text("utf-8"))["update"]["rawPrefix"]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(args):
    repo = args.assets_repo.resolve()
    if not (repo / ".git").exists() or repo == ROOT or ROOT in repo.parents:
        raise ValueError("请指定独立的资产仓库副本。")
    report = json.loads(args.distributions.read_text("utf-8-sig"))
    version = (ROOT / "VERSION").read_text().strip()
    if report["version"] != version:
        raise ValueError("发行报告不是当前源码版本。")
    modern_preview = json.loads((args.previews / "preview-manifest-v4.json").read_text())
    legacy_preview = json.loads((args.previews / "preview-manifest.json").read_text())
    provenance = json.loads((args.previews.parent / "preview-build.json").read_text("utf-8"))
    if provenance.get("encoding") != {"format": "WEBP", "lossless": False, "quality": 50,
                                      "method": 4, "resized": False, "alpha": "lossless"}:
        raise ValueError("预览必须来自已核验的质量 50 有损 WebP 构建，不能发布旧的无损大包。")
    retained = len(provenance["retainedLegacy"])
    game_version = provenance["gameVersion"]
    entries = []
    def copy(source, relative, expected, size):
        target = repo / relative
        if not target.resolve().is_relative_to(repo):
            raise ValueError("目标路径越界。")
        if source.stat().st_size != size or sha(source) != expected:
            raise ValueError(f"输入摘要不符：{source.name}")
        if target.exists() and sha(target) != expected:
            raise ValueError(f"不覆盖已有不同内容的发行文件：{relative}")
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        if sha(target) != expected:
            raise ValueError(f"复制后摘要不符：{relative}")
        target.with_name(target.name + ".sha256").write_text(expected + "  " + target.name + "\n", encoding="utf-8")
        entries.append({"path": relative, "sha256": expected, "bytes": size})

    artifacts = {}
    for item in report["artifacts"]:
        source = Path(item["path"])
        group = "windows-x86_64" if item["kind"] in {"portable", "msi"} else "compatibility"
        relative = f"releases/{version}/{group}/{source.name}"
        copy(source, relative, item["sha256"], item["bytes"])
        artifacts[item["kind"]] = {"platform": "win-x86_64", "kind": item["kind"],
                                    "url": PREFIX + relative, "sha256": item["sha256"], "bytes": item["bytes"]}
    for manifest in [modern_preview, legacy_preview]:
        for bundle in manifest["bundles"]:
            url = bundle["bundleUrl"]
            relative = url.removeprefix(PREFIX) if url.startswith(PREFIX) else "bundles/" + url.rsplit("/", 1)[-1]
            copy(args.previews / relative, relative, bundle["sha256"], bundle["bytes"])
    notes = "修复资源智能匹配、导入替换和旧库迁移；优化启动扫描、APX 与工坊布局，并恢复原图定位。"
    modern_update = {"schema": 2, "version": version, "notes": notes,
                     "assets": [artifacts["portable"], artifacts["msi"]]}
    bridge = {k: v for k, v in artifacts["legacy-bridge"].items() if k != "kind"}
    legacy_update = {"schema": 1, "version": version, "notes": notes, "assets": [bridge]}
    manifests = {"preview-manifest-v4.json": modern_preview, "preview-manifest.json": legacy_preview,
                 "update-manifest-v2.json": modern_update, "update-manifest.json": legacy_update}
    for name, content in manifests.items():
        (repo / name).write_text(json.dumps(content, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (repo / "README.md").write_text(f'''# APMM 下载与资源

## 应用下载

| 适用场景 | 下载 |
|---|---|
| 解压即用 | [APMM {version} 便携版](releases/{version}/windows-x86_64/APMM-{version}-windows-x86_64-portable.zip) |
| 固定安装 | [APMM {version} 安装版](releases/{version}/windows-x86_64/APMM-{version}-windows-x86_64.msi) |

旧版自动更新使用 [兼容包](releases/{version}/compatibility/)，普通安装请使用上面的标准包。每个包附带 `.sha256` 校验文件。

## 原图预览

[完整 preview-v4 预览包](bundles/preview-v4/)由官方资源提取后以质量 50 编码为有损 WebP，保留尺寸和透明度，包含角色、卡面、怪物、芯片和鼠标。完整清单为 `preview-manifest-v4.json`；旧版通过 `preview-manifest.json` 获取固定资源集合的兼容包。PNG 原图不进入预览下载包。

预览更新独立于软件更新。{retained} 个当前游戏目录已移除的历史项目保留旧预览，其他原图来自游戏 {game_version} 的本次提取。

## 目录

- `releases/<版本>/windows-x86_64/`：标准应用发行。
- `releases/<版本>/compatibility/`：旧程序升级兼容包。
- `bundles/preview-v4/`：完整原图预览包。
- `screenshots/<版本>/`：界面展示图，与原图预览包分别维护。
- `workshop/`：签名工坊目录和作品资源。

根目录四份 JSON 是客户端固定读取入口。`releases/`、`bundles/` 中既有的平铺文件只保留旧直链兼容，不再作为新标准包的发布位置。旧文件保持原摘要，避免旧清单或下载中断恢复失效。
''', encoding="utf-8")
    (repo / "releases/README.md").write_text(f'''# 应用发行

当前版本：[APMM {version}]({version}/windows-x86_64/)。旧程序升级专用包位于 [{version}/compatibility/]({version}/compatibility/)。

新版本均使用相同的两级目录。此目录中原有平铺文件是兼容旧下载地址的保留项，不是推荐下载入口，也不继续添加新的常规发行包。
''', encoding="utf-8")
    (repo / "bundles/README.md").write_text('''# 原图预览

- [preview-v4/](preview-v4/)：当前完整资源，按分类下载。
- `*-preview-v4-compat.zip`：旧客户端固定资源白名单的刷新包。
- `*-preview-v3.zip`：原始历史版本，保留摘要和旧直链。

应用按对应的根目录清单选择，不需要手动拼接文件。界面截图另见 `screenshots/`。
''', encoding="utf-8")
    plan = {"version": version, "repository": str(repo), "remoteApplied": False,
            "files": entries, "manifestsLast": list(manifests),
            "githubPreviewManifests": ["preview-manifest.json", "preview-manifest-v4.json"],
            "githubPreviewAssets": [x["bundleUrl"] for x in legacy_preview["bundles"]]}
    (report_dir() / "publication-plan.json").write_text(json.dumps(plan, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"version": version, "files": len(entries), "remoteApplied": False}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--assets-repo", type=Path, required=True)
    parser.add_argument("--previews", type=Path, required=True)
    parser.add_argument("--distributions", type=Path, default=distributions_report())
    prepare(parser.parse_args())
