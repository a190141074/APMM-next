# 开发与验证

## 路径和版本

新工程位于 `D:\MOD\APMM-next\apmm-desktop`。旧工程、参考工程、demo 不作为构建输出或测试暂存目录。

`VERSION` 为版本真源；每轮独立功能或修复交付默认递增 PATCH，同步 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 中主程序条目、`src-tauri/tauri.conf.json`，并更新 `CHANGELOG.md` 当前版本章节。同一轮连续修改或重复构建不重复递增；仅文档或 skill 整理不自动升版。当前源码版本为 `0.4.10`（未发布）。资产插件独立版本 `0.1.0`，签名清单声明管理器范围与协议版本。

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

公开源码目标为 GitHub `a190141074/APMM-next` 与 CNB `astral-weird-egg/APMM-next`，两端同步同一提交与版本标签。覆盖旧安装、向游戏部署和远端发布遵循本次用户授权；发行包审计后才允许发布，签名私钥和令牌永不进入源码或 ZIP。

源码提交保留构建必需的资源、锁文件、第三方源码及许可和固定运行库；依赖安装目录、构建输出、临时报告、Tauri 生成的 schema 与 Cargo 缓存标记由 `.gitignore` 排除。发布前运行 `node tools/audit-source.mjs`，并核对待提交文件清单。新增忽略规则后，已经跟踪的文件仍需逐项取消跟踪；只移除 Git 索引记录，保留本机文件。

## 本地产物与公开文档

`APMM_DIST_DIR` 是本轮版本的产物根目录；标准包写入 `windows-x86_64/`，旧版专用包写入 `compatibility/`。`APMM_REPORT_DIR` 保存构建和验收 JSON。默认目录分别为工作盘的 `releases/<版本>/` 与 `reports/<版本>/`，每次重建使用新的隔离目录。构建报告记录程序输入摘要，构建期间输入变化会停止交付。

所有原生验收用 `APMM_BUILD_REPORT` 指定本轮便携构建；发行派生与 MSI 验收用 `APMM_DISTRIBUTIONS_REPORT` 指定本轮报告。详细流程见 [发行与预览资源](docs/RELEASING.md)。原图预览由 `tools/build-preview-bundles.py` 构建，真实安装协议用 Rust 的 `preview_release_artifacts` 过滤验证；发行 WebView 用 `tools/verify-preview-native.mjs` 配合独立桌面启动器验收。

公开 docs 只维护稳定指南与展示截图。历史过程材料归档到本地工作盘，生成器不得再次写入 docs；源码清单由 `tools/source-inventory.py` 输出到本地报告目录。
