"""对比本轮只读快照，生成全部受影响文件和职责清单，不遍历共用缓存。"""
import hashlib
import json
import os
import sys
from pathlib import Path

from work_paths import report_dir

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = report_dir()
VERSION = (ROOT / "VERSION").read_text().strip()
BASELINE = Path(sys.argv[1])
SKIP = {"node_modules", ".venv", "__pycache__", "target-msvc", "target", "src-release", "dist", "vendor", ".git"}
SUFFIXES = {".rs", ".tsx", ".ts", ".css", ".py", ".mjs", ".json", ".toml", ".lock", ".yaml", ".md"}
ROLES = {
    "app.rs": "应用监听生命周期、图片修订协议与目录读取边界",
    "candidate_management.rs": "候选信息编辑、归属合并和跨盘文件回滚",
    "directory_files.rs": "递归 PNG 枚举、禁用后缀与持久文件标识",
    "directory_monitor.rs": "目录通知、稳定等待、任务互斥和退出取消",
    "directory_sync.rs": "被动游戏读取、实际应用计算与外部源绑定",
    "vault_sync.rs": "库候选编辑和重命名关联、规则归类及冲突提示",
    "management_types.rs": "候选信息、来源、归属确认和预设持久契约",
    "preset_domain.rs": "固定内容与范围捕获、问题预览和整批事务",
    "presets.rs": "预设管理 IPC 和旧预设命令兼容入口",
    "management_tests.rs": "预设兼容、局部原版、问题聚合及事务回滚测试",
    "sync_management_tests.rs": "外部扫描、独立信息、重命名和启禁同步测试",
    "storage.rs": "Library 新字段、默认监听和旧数据加载",
    "dto.rs": "工作区候选信息与监听设置 DTO",
    "workspace.rs": "资源快照、独立候选信息和实际状态展示",
    "settings_commands.rs": "监听开关和既有设置兼容入口",
    "lib.rs": "新领域模块、IPC 命令及测试注册",
    "image_import.rs": "完整 PNG 校验、逐项归属预览和入库事务",
    "legacy_commands.rs": "旧 PNG 导入入口委托新流程",
    "task_dispatch.rs": "后台 PNG 导入和被动扫描协调",
    "resource_mutation.rs": "扫描和临时迁移委托新领域，保留维护入口",
    "packages.rs": "可选来源和手动身份分享、重复补空及 PNG 载荷校验",
    "legacy_tests.rs": "历史包构造兼容和真实 PNG 迁移夹具",
    "workshop.rs": "既有候选构造适配兼容默认字段",
    "originals.rs": "已连接游戏目录只读预览和禁用 PNG 读取",
    "App.tsx": "独立预设导航和路由",
    "PresetsPage.tsx": "预设详情、修复、锁定、范围编辑和应用清单",
    "CreatePresetDialog.tsx": "全局、角色全变体和固定部位创建范围",
    "ResourceSlotSelect.tsx": "分类、对象、变体、部位四级选择",
    "CandidateComparison.tsx": "模组信息左侧对比和独立缩放、平移、复位及放大",
    "CandidateCreditsFields.tsx": "作者、可选画师与各自多链接的紧凑编辑",
    "candidateInfo.ts": "兼容信息默认字段、隐藏图片后缀与安全导出命名",
    "management-ui.css": "模组信息左右分栏、紧凑字段和窄窗堆叠",
    "main.tsx": "加载模组信息的独立样式职责",
    "ResourceFileActions.tsx": "模组信息、来源外链和归属冲突确认",
    "PackageSelection.tsx": "PNG 逐项目标选择和错误项禁用",
    "PackageDialog.tsx": "逐项归属传输与导入任务反馈",
    "ResourceSearch.tsx": "相似标记、稳定结果与键盘定位",
    "SettingsPage.tsx": "目录监听开关与预设页导航",
    "ResourcePage.tsx": "候选缩略图的内容修订刷新",
    "ResourceDetail.tsx": "候选和放大图片的内容修订刷新",
    "AboutPage.tsx": "半宽参考项目与工具鸣谢、JASM/APX/Codex 链接",
    "Ui.tsx": "图片修订请求和失败状态刷新",
    "PanZoomImage.tsx": "修订换图时重置缩放并释放监听",
    "resources.ts": "实际分类槽位映射、名称容错和稳定搜索排序",
    "importClassification.ts": "导入目标预览与目标槽位重复判断",
    "management.test.ts": "名称搜索、归属预览和实际分类映射测试",
    "transport.ts": "图片 URL 的哈希修订参数",
    "management.ts": "预设管理和候选管理 IPC 服务",
    "useManagement.ts": "预设详情查询与命令服务 hooks",
    "useApp.ts": "目录事件刷新工作区和预设详情快照",
    "app.ts": "目录变化订阅和现有服务整合",
    "mock.ts": "开发浏览器管理入口与版本适配",
    "mockManagement.ts": "浏览器内存预设夹具，生产仍使用 Rust",
    "commands.ts": "sync-ipc 生成的新管理命令与事件常量",
    "sync-ipc.mjs": "生成目录变化事件常量",
    "style.css": "预设、信息、归属布局及 800 宽窗口支持",
    "tauri.conf.json": "版本同步和原生窗口最小宽度",
    "management-fixtures.mjs": "大小固定、完整 CRC 的真实 PNG 夹具",
    "management-about-ui.mjs": "关于页同宽布局、链接和窄深主题验收",
    "management-image-refresh.mjs": "同路径时间戳不变编辑的 WebView 像素验收",
    "management-native-ui.mjs": "正式管理界面与真实 PNG IPC 的导入向导验收",
    "management-mod-info-ui.mjs": "对比缩放、平移、复位、Enter 改名和作者画师多链接界面验收",
    "verify-management-release.py": "本轮 ZIP 白名单、摘要、版本与原生副本一致性审计",
    "management-native-temporary.mjs": "正式外部临时启禁、只读预览和本机选择保持验收",
    "verify-management-native.mjs": "B 盘发行 EXE、目录和事务整体验收编排",
    "verify-management-ui.mjs": "B 盘浏览器管理、搜索和关于页交互验收",
    "management-source-inventory.py": "本轮快照对比、变更职责和行数审计",
}

def source_files(root):
    result = {}
    for folder, dirs, names in os.walk(root):
        dirs[:] = [d for d in dirs if d not in SKIP and not d.endswith(".egg-info")]
        for name in names:
            p = Path(folder) / name
            if name.startswith("verification-") or not (p.suffix in SUFFIXES or name == "VERSION"):
                continue
            result[p.relative_to(root).as_posix()] = p
    return result

old, current = source_files(BASELINE), source_files(ROOT)
records = []
for rel, p in sorted(current.items()):
    data = p.read_bytes()
    if rel in old and data == old[rel].read_bytes():
        continue
    role = "ts-rs 生成的管理 DTO" if "/generated/" in rel else ROLES.get(p.name)
    if not role:
        role = "版本与依赖锁同步" if p.name in {"VERSION", "package.json", "Cargo.toml", "Cargo.lock"} else "本轮开发、迁移、变更或验收文档"
    records.append({"path": rel, "status": "modified" if rel in old else "added", "lines": len(data.decode("utf-8-sig").splitlines()), "role": role, "sha256": hashlib.sha256(data).hexdigest()})
report = {"version": VERSION, "baseline": str(BASELINE), "files": records, "removed": sorted(set(old) - set(current))}
rows = [f"# {VERSION} 源码差异", "", f"对比只读源码快照 `{BASELINE}`。以下为全部手写代码、生成契约和配置文档差异；构建缓存及验收 JSON 不作为源码。", "", "| 文件 | 变更 | 行数 | 职责 |", "|---|---|---:|---|"]
rows += [f"| `{r['path']}` | {r['status']} | {r['lines']} | {r['role']} |" for r in records]
(OUTPUT / "changed-source.md").write_text("\n".join(rows) + "\n", encoding="utf-8")
# 清单写入后再记录自身摘要，避免把上一轮清单的摘要作为本轮证据。
for record in records:
    data = (ROOT / record["path"]).read_bytes()
    record["sha256"] = hashlib.sha256(data).hexdigest()
    record["lines"] = len(data.decode("utf-8-sig").splitlines())
(OUTPUT / "changed-files.json").write_text(json.dumps([r["path"] for r in records], ensure_ascii=False, indent=2), encoding="utf-8")
(OUTPUT / "management-source.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps({"files": len(records), "removed": report["removed"], "review": [{"path": r["path"], "lines": r["lines"]} for r in records if r["lines"] > 500 and Path(r["path"]).suffix in {".rs", ".tsx", ".ts", ".mjs", ".py"} and "/generated/" not in r["path"]]}, ensure_ascii=False))
