# 开发与验证

## 路径和版本

新工程位于 `D:\MOD\APMM-next\apmm-desktop`。旧工程、参考工程、demo 不作为构建输出或测试暂存目录。

`VERSION` 为版本真源；同时同步 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`，并更新 `CHANGELOG.md`。当前管理体验版本为 `0.4.1`。资产插件独立版本 `0.1.0`，签名清单声明管理器范围与协议版本。

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

| 目标 | 命令 |
|---|---|
| TypeScript | `pnpm typecheck` |
| 前端规则 | `pnpm test` |
| 浏览器交互 | 启动 Vite 后 `node tools/verify-ui.mjs` |
| IPC 命令生成 | `node tools/sync-ipc.mjs` |
| Rust 类型导出 | `node tools/cargo.mjs test export_bindings` |
| Rust 静态检查 | `pnpm rust:check` |
| Rust 对应测试 | `node tools/cargo.mjs test --lib <过滤词>` |
| Rust 格式 | `node tools/cargo.mjs fmt -- --check` |
| 便携 ZIP | `pnpm build:portable` |
| 资产插件 | `node tools/build-plugin.mjs` |
| 真实 IPC 集成 | 构建后 `node tools/verify-native.mjs` |
| 独立更新助手 | 构建后 `node tools/verify-updater.mjs` |

资产工具安装依赖使用 `plugins/asset-tool/requirements.lock`；源码测试命令为 `plugins/asset-tool/.venv/Scripts/python.exe -X utf8 -B -m unittest discover -s plugins/asset-tool/tests -t plugins/asset-tool`。设置 `PYTHONPATH=plugins/asset-tool/src`。分类由 `config/sections.json` 生成到插件资源目录，运行 `tools/sync-definitions.py`，不独立编辑生成副本。

`verify-native.mjs` 在 B 盘运行发行副本，将写入全部定向到临时 modimg，实际游戏路径仅用于只读插件检查和提取。脚本中的真实游戏/夹具路径属于本机验收配置，其他机器需要调整。测试不得把旧 data、游戏根或源码根当暂存目录。

## 代码与 PR

前端遵循“页面 → hooks → 服务 → IPC 传输”依赖；纯资源与任务规则放入 core/domain。Rust 的命令入口只负责适配，事务、下载、提取、更新由各领域负责。跨 IPC 的新 DTO 使用 ts-rs；遗留动态协议只留在适配边界。

新增或明显扩展的手写文件超过 500 行需要职责评审，例外记录于 FILE-MAP。继承的大协议文件逐步按职责拆分，不用机械压缩行数通过检查。新增非平凡注释用简体中文，说明约束和副作用。

PR 描述写清触发条件、行为变化、对应证据和未验证环境。验收分别列源码/测试、编译、实际包、运行日志、游戏行为。无真实拖放或游戏证据时不要宣称通过。

公开源码目标为 GitHub `a190141074/APMM-next` 与 CNB `astral-weird-egg/APMM-next`，两端同步同一提交与版本标签。覆盖旧安装、向游戏部署和远端发布遵循本次用户授权；发行包审计后才允许发布，签名私钥和令牌永不进入源码或 ZIP。

源码提交保留构建必需的资源、锁文件、第三方源码及许可和固定运行库；依赖安装目录、构建输出、临时报告、Tauri 生成的 schema 与 Cargo 缓存标记由 `.gitignore` 排除。发布前运行 `node tools/audit-source.mjs`，并核对待提交文件清单。新增忽略规则后，已经跟踪的文件仍需逐项取消跟踪；只移除 Git 索引记录，保留本机文件。
