# 开发与验证

## 工作流文档与 skill 同步

项目 `skill/` 集中管理工作流 Markdown、skill 入口与补充材料，索引见 [skill/INDEX.md](skill/INDEX.md)。详细工程说明继续维护在本文件和 `docs/`，由 skill 引用，避免重复正文。`skill/apmm-next-modding/` 是权威维护副本；Codex 用户级同名目录只作安装副本。

新会话先读应用 `AGENTS.md`、项目维护版 `SKILL.md` 和本指南，再按任务读取对应文档及本地报告。维护工作流后，从应用根执行：

```powershell
pwsh -File tools/sync-apmm-skill.ps1
pwsh -File tools/sync-apmm-skill.ps1 -Check
```

校验必须确认来源文件的 SHA-256 一致。安装目标额外文件只报告，不自动删除；先核对是否需要迁回维护源。项目和安装副本有差异时先检查实际变更，不盲目采用较新的文件时间。敏感操作的授权只来自当前对话明确覆盖的范围，旧报告和 skill 不授予远端写入权限。

## 路径和版本

新工程位于 `D:\MOD\APMM-next\apmm-desktop`。旧工程、参考工程、demo 不作为构建输出或测试暂存目录。

`VERSION` 为版本真源。每轮独立应用功能或修复交付默认递增 PATCH，同步 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 中主程序条目、`src-tauri/tauri.conf.json`，并更新 `CHANGELOG.md` 当前版本章节。同一轮连续修改或重复构建沿用目标版本。文档、skill 和本地发行工具整理沿用现有版本。发行状态分渠道回读。资产插件独立版本和管理器兼容范围由插件签名清单声明。

禁止把工作目录、依赖缓存或浏览器资料默认塞入 C 盘。本机工作目录为 `B:\APMM-next-work`，Cargo 复用 `src-tauri/target-msvc`，工具链/registry 位于 B 盘。不要新建重复 Cargo target，也不要清理旧工程或共用缓存。

## Windows

需要 Node.js 22、pnpm（锁定版本见 package.json）、Rust stable MSVC、Visual Studio C++ Build Tools、Windows SDK、WebView2。Python 3.12+ 仅用于构建资产插件和验收脚本，主程序运行不需要安装 Python。

```powershell
$env:APMM_WORK_DIR = 'B:\APMM-next-work'
$env:TEMP = 'B:\APMM-next-work\temp'
$env:TMP = $env:TEMP
$env:APMM_RUST_BIN = 'B:\Rust\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin'
$env:APMM_VSDEV = 'B:\VS\BuildTools\Common7\Tools\VsDevCmd.bat'
pnpm.cmd install --frozen-lockfile
pnpm.cmd dev
# 浏览器预览：http://127.0.0.1:1420
pnpm.cmd tauri:dev
```

`tauri:dev` 调用 Tauri CLI，关闭默认的 custom-protocol feature，使用 Vite。正式构建启用该 feature，嵌入构建后的前端。便携构建把前端放入 B 盘唯一目录；Windows 跨盘路径必须转为扩展路径，避免被 Tauri 识别为 URL。脚本检查 `.d` 中确实包含 `index.html` 且不含预览源目录。

## Codex 预览

用户明确要求预览或批注时，从应用目录启动或复用 Vite。先核对端口占用者、进程参数与应用页面。若默认回环端口 1420 已被其他进程占用，从本仓用 `pnpm.cmd dev --port <空闲端口>` 启动新实例。将启动日志和可配置的浏览器测试资料放到 B 盘任务目录。

在 Codex 内置浏览器打开回环 URL，定位目标页面，确认页面和所需数据已加载。用 `APMM_UI_URL` 将同一 URL 交给相关 UI 验收脚本。记录页面、URL、数据来源、启动命令、进程与可批注状态，交付时给出准确 URL 和页面定位。热更新或切页后重新核对当前页面，保留服务供用户批注。用户要求结束时只关闭本任务启动的实例；不要关闭其他任务的共用服务。浏览器开发传输和页面夹具提供界面证据。原生 WebView、IPC、系统文件对话框和游戏目录操作有独立验收步骤。

## Linux

首版发行与原生验收目标是 Windows。Linux 可准备前端和 Python 工具开发环境：Node.js、pnpm、Python 3.12+、Rust GNU；Tauri 原生依赖包括 GTK 3、WebKitGTK 4.1、OpenSSL 和 AppIndicator。参考工程给出的 Debian/Ubuntu 依赖命令如下：

```bash
sudo apt install build-essential curl wget file pkg-config libwebkit2gtk-4.1-dev libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf
export APMM_WORK_DIR="$HOME/apmm-work"
export TMPDIR="$APMM_WORK_DIR/temp"
mkdir -p "$TMPDIR"
pnpm install --frozen-lockfile
pnpm dev
pnpm typecheck
pnpm test
```

Windows 原生外拖、凭据、游戏路径检测和更新助手仍有平台专用代码；本次未进行 Linux 编译，也未提供 Linux 包。不要把前端可运行描述为 Linux 桌面已支持。

## 检查命令

| 目标          | 命令                                        |
| ------------- | ------------------------------------------- |
| TypeScript    | `pnpm typecheck`                            |
| 前端规则      | `pnpm test`                                 |
| 浏览器交互    | 启动 Vite 后 `node tools/verify-ui.mjs`     |
| IPC 命令生成  | `node tools/sync-ipc.mjs`                   |
| Rust 类型导出 | `node tools/cargo.mjs test export_bindings` |
| Rust 静态检查 | `pnpm rust:check`                           |
| Rust 对应测试 | `node tools/cargo.mjs test --lib <过滤词>`  |
| Rust 格式     | `node tools/cargo.mjs fmt -- --check`       |
| 便携 ZIP      | `pnpm build:portable`                       |
| 资产插件      | `node tools/build-plugin.mjs`               |
| 真实 IPC 集成 | 构建后 `node tools/verify-native.mjs`       |
| 独立更新助手  | 构建后 `node tools/verify-updater.mjs`      |

资产工具安装依赖使用 `plugins/asset-tool/requirements.lock`；源码测试命令为 `plugins/asset-tool/.venv/Scripts/python.exe -X utf8 -B -m unittest discover -s plugins/asset-tool/tests -t plugins/asset-tool`。设置 `PYTHONPATH=plugins/asset-tool/src`。分类由 `config/sections.json` 生成到插件资源目录，运行 `tools/sync-definitions.py`，不独立编辑生成副本。

`verify-native.mjs` 在 B 盘运行发行副本，将写入全部定向到临时 modimg，实际游戏路径仅用于只读插件检查和提取。脚本中的真实游戏/夹具路径属于本机验收配置，其他机器需要调整。测试不得把旧 data、游戏根或源码根当暂存目录。

## 代码与 PR

### 工坊与无服务器投稿的定向验证

工坊双视图与封面资料使用 `node tools/verify-workshop-cards-ui.mjs` 定向验收；默认预览端口为 1435，可通过 `APMM_UI_URL` 指定。其可复用夹具位于 `tools/workshop-ui-fixture.mjs`，不进入正式后端。`verify-workshop-native.mjs` 验证发行副本的真实封面替换、资料导出恢复和预览界面。两者证据分别记录，不以浏览器夹具证明原生文件选择器。

默认投稿流程是本地导出、人工收稿，维护者流程见 `workshop-service/README.md`。不要要求部署可选 HTTP 服务才能使用作者模式。

- 浏览器：启动 Vite 后，设置 `APMM_UI_URL` 为对应回环地址，再运行 `node tools/verify-workshop-ui.mjs`。
- Rust：使用新的 `ASTRAL_DEV_DATA_DIR` 独立目录，运行 `node tools/cargo.mjs test --lib author -- --test-threads=1 --include-ignored`。工坊测试同理使用 `workshop` 过滤；跨语言用例还需本轮 `APMM_INTEROP_FIXTURE`。
- 离线链：Rust 测试可通过 `APMM_WORKSHOP_EXPORT_FIXTURE` 输出投稿包；Python 测试读取它，并将 v1/v2 目录位置写入 `APMM_INTEROP_FIXTURE`。这些变量必须指向 B 盘任务目录。
- 维护工具：在固定 Python 3.12 环境设置 `PYTHONPATH=workshop-service` 和 B 盘 `APMM_WORK_DIR`，运行 `python -m unittest discover -s workshop-service/tests -p test_local.py`。
- 隔离构建：先确认本轮目标版本并完成版本同步，再设置 `APMM_DIST_DIR` 与 `APMM_BUILD_REPORT` 为新的 B 盘任务路径，执行 `pnpm.cmd build:portable`。未发布或隔离验证不免除新功能交付的版本递增，不覆盖既有 ZIP 和报告。核对新包清单、包名、构建报告与启动快照的版本；旧包及旧验收保留真实版本。
- 原生：`node tools/verify-workshop-native.mjs` 读取 `APMM_BUILD_REPORT` 及 `APMM_WORKSHOP_EXPORT_FIXTURE`，启动 B 盘发行副本验证真实 IPC，退出时关闭本次副本。

工坊原生验收还需显式设置 `APMM_BACKGROUND_LAUNCHER` 为已验证的独立 Windows desktop 启动器、`APMM_TEST_PYTHON` 为对应 Python。未配置时直接停止，避免弹出原生窗口。构建报告版本必须与当前 `VERSION` 一致；运行前核对发行副本的清单摘要。两项工坊卡片验收可用 `APMM_WORKSHOP_WORK` 指定 B 盘证据目录，默认目录跟随当前版本。

Python 依赖声明由其固定虚拟环境执行 `workshop-service/tools/third-party-notices.py` 生成；与桌面 JS/Rust 依赖声明分开管理。

前端遵循“页面 → hooks → 服务 → IPC 传输”依赖；纯资源与任务规则放入 core/domain。Rust 的命令入口只负责适配，事务、下载、提取、更新由各领域负责。跨 IPC 的新 DTO 使用 ts-rs；遗留动态协议只留在适配边界。

新增或明显扩展的手写文件超过 500 行需要职责评审，例外记录于 FILE-MAP。继承的大协议文件逐步按职责拆分，不用机械压缩行数通过检查。新增非平凡注释用简体中文，说明约束和副作用。

PR 描述写清触发条件、行为变化、对应证据和未验证环境。验收分别列源码/测试、编译、实际包、运行日志、游戏行为。无真实拖放或游戏证据时不要宣称通过。

“推送”将同一源码提交同步到 GitHub `a190141074/APMM-next` 与 CNB `astral-weird-egg/APMM-next` 的 `main`。发行标签、附件与清单遵循 [发行规范](docs/RELEASING.md)。远端写入、覆盖旧安装和向游戏部署按本次具体范围取得授权。签名私钥和令牌永不进入源码或 ZIP。

源码提交保留构建必需的资源、锁文件、第三方源码及许可和固定运行库；依赖安装目录、构建输出、临时报告、Tauri 生成的 schema 与 Cargo 缓存标记由 `.gitignore` 排除。发布前运行 `node tools/audit-source.mjs`，并核对待提交文件清单。新增忽略规则后，已经跟踪的文件仍需逐项取消跟踪；只移除 Git 索引记录，保留本机文件。

## 本地产物与公开文档

`APMM_DIST_DIR` 是本轮版本的产物根目录；标准包写入 `windows-x86_64/`，旧版专用包写入 `compatibility/`。`APMM_REPORT_DIR` 保存构建和验收 JSON。默认目录分别为工作盘的 `releases/<版本>/` 与 `reports/<版本>/`，每次重建使用新的隔离目录。构建报告记录程序输入摘要，构建期间输入变化会停止交付。

所有原生验收用 `APMM_BUILD_REPORT` 指定本轮便携构建；发行派生与 MSI 验收用 `APMM_DISTRIBUTIONS_REPORT` 指定本轮报告。详细流程见 [发行与预览资源](docs/RELEASING.md)。原图预览由 `tools/build-preview-bundles.py` 构建，真实安装协议用 Rust 的 `preview_release_artifacts` 过滤验证；发行 WebView 用 `tools/verify-preview-native.mjs` 配合独立桌面启动器验收。

公开 docs 维护稳定指南与展示截图。源码清单由 `tools/source-inventory.py` 输出到工作盘报告目录。

## 任务报告

每个独立任务使用本仓 `reports/<全局四位起递增编号>/YYYYMMDD-HHmmss-任务简称.md`。日期和阶段时间用北京时间（UTC+08:00）。分配编号时取现有编号的最大值加一，从空目录独占创建，遇到并发占用就重取编号。结项、部分交付或遇到明确阻塞时更新报告。续做根据用户给出的编号、任务主题或变更入口找原报告，在原文件追加阶段时间和结果。

报告写明目标、状态、关键取舍、改动文件与职责、实测命令和结果、包与报告路径、提交或远端状态、未验证项和下一步的接续入口。完成时在答复中链接该报告。Git 忽略整个 `reports/`，机器构建 JSON、日志和发行包保持在 B 盘。报告只引用可脱敏的证据路径。
