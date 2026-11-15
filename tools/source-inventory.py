"""生成本次源码职责清单，并核对旧版只读基线。"""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ROLES = {
    "app": "应用启动、WebView、便携目录和协议生命周期",
    "lib": "兼容导出与 IPC 注册入口",
    "dto": "Rust 到 TypeScript 的工作台契约",
    "tasks": "统一任务状态、修订号、取消与事件",
    "task_dispatch": "后台任务排队与领域调用适配",
    "task_history": "任务持久化、重启中断和密码边界",
    "migration": "旧库复制、源变化校验和路径重写",
    "migration_journal": "复制提交日志与重启恢复",
    "downloads": "有界下载、续传、范围和摘要校验",
    "auxiliary": "可选插件的安装、卸载、运行与取消",
    "aux_manifest": "插件签名清单、ZIP 边界与文件验证",
    "portable_update": "发行清单、更新日志和整批恢复",
    "astral-updater": "更新助手进程与启动确认",
    "updates": "更新检查、收据、任务闸门与助手调度",
    "workspace": "工作台快照与成组应用适配",
    "resource_transaction": "游戏资源整批修改与回滚",
    "resource_rules": "完整官方名称、变体及新增类型",
    "originals": "本地原图校验、远端回退和 URI 访问边界",
    "thumbnails": "有界后台缩略图解码与独立内容缓存",
    "image_import": "PNG 识别、目标及冲突预览",
    "preview_install": "远端预览安全展开与事务安装",
    "settings_commands": "设置与预设持久化和整批应用",
    "packages": "历史包编解码、分卷、队列、导入导出事务",
    "resource_mutation": "遗留资源导入、扫描协调、启禁和导出适配",
    "apx": "APX 平台限制、版本确认、缓存、安装和还原",
    "workshop": "工坊签名目录、安装及文件归属保护",
    "legacy_tests": "旧库、路径与包协议行为夹具",
    "native_drag": "继承的 Windows OLE 外拖边界",
    "naming": "旧名称解析兼容规则",
    "storage": "Library 数据、设置和原子持久化",
    "staging": "工作盘、暂存白名单及跨盘提交",
    "ResourcePage": "图片库、筛选、列表状态和批量操作",
    "ResourceDetail": "原图、Mod 比较及显式应用",
    "PackageDialog": "PNG/包统一导入和导出向导",
    "PluginsPage": "APX、工坊及辅助插件生命周期界面",
    "SettingsPage": "主题、游戏目录、凭据、预设及更新设置",
    "TasksPage": "任务快照、进度、结果与重试界面",
    "WelcomeDialog": "首次迁移检测和确认",
    "useApp": "查询、事件、窗口及操作状态 hooks",
    "useStartup": "启动确认、间隔更新检查和偏好恢复",
}
EXCLUDED = {"node_modules", ".venv", "__pycache__", "target-msvc", "target", "src-release", "dist", "vendor", "THIRD_PARTY_LICENSES"}
files = []
for path in ROOT.rglob("*"):
    rel = path.relative_to(ROOT)
    if any(part in EXCLUDED or part.endswith(".egg-info") for part in rel.parts):
        continue
    if path.is_file() and path.suffix in {".rs", ".tsx", ".ts", ".css", ".py", ".mjs", ".json", ".toml"} and not path.name.startswith("verification-"):
        files.append(path)

rows = ["# 受影响文件与职责", "", "新工程内的下列源码/配置均为本次建立或迁入；旧工程文件基线另行核对。第三方 vendored 依赖保留原许可，生成文件由对应脚本维护。", "", "| 文件 | 行数 | 职责 |", "|---|---:|---|"]
oversized = []
for path in sorted(files):
    rel = path.relative_to(ROOT).as_posix()
    count = len(path.read_text(encoding="utf-8-sig").splitlines())
    if "generated" in path.parts:
        role = "ts-rs 生成的 IPC 类型"
    elif rel.startswith("plugins/asset-tool/tests/"):
        role = "资产解析、像素或同步的自动化夹具"
    elif rel.startswith("plugins/asset-tool/src/"):
        role = "独立资产工具的 " + path.stem + " 领域实现"
    elif rel.startswith("tools/"):
        role = "构建、生成或隔离验收入口"
    else:
        role = ROLES.get(path.stem, "所属模块的配置、领域模型或兼容辅助")
    rows.append(f"| `{rel}` | {count} | {role} |")
    if count > 500 and path.suffix in {".rs", ".tsx", ".ts", ".py", ".mjs"} and "generated" not in path.parts:
        oversized.append((rel, count, role))

rows.extend(["", "## 超过 500 行的职责评审", "", "以下迁入文件暂时保留兼容边界，不代表拆分已全部结束；对新责任优先提取独立模块。本次已把任务、工作台、资产、下载、迁移恢复、成组事务和便携更新从遗留入口拆出。未通过压缩行数规避评审。", ""])
for rel, count, role in oversized:
    rows.append(f"- `{rel}`（{count} 行）：{role}。保留理由：现有协议/测试或页面领域内耦合较高；后续按责任迁移，避免同时改写历史兼容行为。")
(ROOT / "docs/FILE-MAP.md").write_text("\n".join(rows) + "\n", encoding="utf-8")

baseline = json.loads((ROOT / "docs/legacy-baseline.json").read_text(encoding="utf-8"))
changed = []
for name, expected in baseline["files"].items():
    source = Path(baseline["source"]) / name
    if not source.is_file() or hashlib.sha256(source.read_bytes()).hexdigest() != expected:
        changed.append(name)
report = {"files": len(files), "oversized": oversized, "legacyBaselineFiles": len(baseline["files"]), "legacyChanged": changed}
(ROOT / "verification-source-inventory.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps(report, ensure_ascii=False))
if changed:
    raise SystemExit("旧版基线变化，需要核对，禁止声明旧工程未改动。")
