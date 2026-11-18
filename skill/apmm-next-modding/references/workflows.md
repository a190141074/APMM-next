# 开发、验证与交付入口

所有命令以 `D:\MOD\APMM-next\apmm-desktop` 为工作目录。先读当前 CONTRIBUTING.md 和对应脚本，以下是定位用的本机配置，不是跨机器路径保证。

## 环境与启动

Windows 使用 Node.js 22、package.json 指定的 pnpm、Rust stable MSVC、Visual Studio C++ Build Tools、Windows SDK、WebView2。资产插件构建使用 Python，主程序运行不要求系统安装 Python。

```powershell
$env:APMM_WORK_DIR = 'B:\APMM-next-work'
$env:TEMP = 'B:\APMM-next-work\temp'
$env:TMP = $env:TEMP
$env:APMM_RUST_BIN = 'B:\Rust\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin'
$env:APMM_VSDEV = 'B:\VS\BuildTools\Common7\Tools\VsDevCmd.bat'
pnpm.cmd install --frozen-lockfile
pnpm.cmd dev
```

预览通常在 `http://127.0.0.1:1420/`。原生开发用 `pnpm.cmd tauri:dev`，它经 `tools/cargo.mjs` 调用 Tauri CLI 并关闭默认 custom-protocol；正式构建启用该 feature。不要用正式 bundle 配置冒充开发预览。

Linux 准备和 GTK/WebKitGTK 依赖按 CONTRIBUTING.md；目前已有记录不包含 Linux 桌面编译/运行证据。可准备环境，但支持状态必须由新证据确认。

## 定向检查

| 改动 | 当前入口 |
|---|---|
| 前端类型 | `pnpm.cmd typecheck` |
| 前端纯规则 | `pnpm.cmd exec vitest run <测试文件> -t <相关用例>` |
| UI 与目录操作 | 启动 Vite 后 `node tools/verify-about-apx-ui.mjs` |
| 紧凑资源页与任务浮窗 | `node tools/verify-compact-ui.mjs` |
| 卡面列表与布局切换 | `node tools/verify-resource-list-ui.mjs` |
| 总览版本卡与入场动画 | `node tools/verify-overview-ui.mjs` |
| 工坊卡片与双视图 | `node tools/verify-workshop-cards-ui.mjs` |
| 其他资源交互 | `node tools/verify-ui.mjs` |
| Rust 静态检查 | `node tools/cargo.mjs check`（脚本当前检查 all-targets） |
| 平台目录迁移与隔离 | `node tools/cargo.mjs test --lib apx_directories` |
| 其他 Rust 领域 | `node tools/cargo.mjs test --lib <相关过滤词>` |
| IPC 常量 | `node tools/sync-ipc.mjs` |
| DTO 类型 | `node tools/cargo.mjs test export_bindings`，可缩小至相关生成用例 |
| 格式 | Prettier 检查改动前端文件；Rust 用 `node tools/cargo.mjs fmt -- --check`，继承的无关格式差异不引发全仓重写 |
| 第三方许可 | `node tools/third-party-notices.mjs` |
| 原生 IPC | 检查隔离路径和夹具后，构建副本运行 `node tools/verify-native.mjs` |
| 原生 APX 状态与缓存 | 同上，`node tools/verify-apx-runtime.mjs` |
| 更新助手 | 同上，`node tools/verify-updater.mjs` |
| 启动与增量扫描 | 按 `updates-and-startup.md` 核对夹具和启动器后，`node tools/verify-incremental-native.mjs` |
| 压缩预览的真实安装 | `node tools/cargo.mjs test --lib preview_release_artifacts`，包路径按测试读取的环境变量配置 |
| 正式包预览显示 | 构建报告与独立桌面配置后，`node tools/verify-preview-native.mjs` |

只验证本次改动及必要调用链；不要默认重跑全套无关测试。报告过滤掉的测试数量，既有警告和新失败分开说明。

PowerShell 下 `pnpm.cmd` 经 cmd 转发时可能改变带 `|` 的正则参数。Vitest 多用例过滤可直接用 `node node_modules/vitest/vitest.mjs run <文件> --testNamePattern '用例甲|用例乙'`。

资产源码测试使用现有 `plugins/asset-tool/.venv/Scripts/python.exe -X utf8 -B -m unittest ...` 和 `PYTHONPATH=plugins/asset-tool/src`；优先相关模块。分类生成用 `tools/sync-definitions.py`。不要为了普通 UI 任务重新打包整个资产运行时。

## 浏览器证据的范围

`tools/local-preview.ts` 可以读取受限的本地元数据和真实图片；测试可以在传输边界注入平台目录和任务夹具。真实图片解码和 DOM 操作不代表浏览器已经执行原生游戏写入。

夹具只放在验收脚本或开发边界，不把假目录、假任务或假安装状态加入正式后端以让截图好看。浏览器 profile 与 screenshots 在 B 盘隔离目录；文件选择器、正式 WebView、系统外链和外部拖放需要原生证据。

收到明确的 Codex 预览或批注要求时，先核对端口占用者、启动参数和返回页面，再复用正确实例。找不到对应实例就从应用根以隐藏进程启动 Vite，将日志重定向到 B 盘任务目录。端口被占用时用 `pnpm.cmd dev --port <空闲端口>`，并给验收传同一 `APMM_UI_URL`。用内置浏览器打开回环地址，查看可见页面、目标页面和数据加载状态。确认后保留页面与服务。HTTP 成功或面板排队只可报告这一步。浏览器证据范围见上文。

原生验收默认使用独立 Windows desktop；运行前读对应脚本，显式设置它要求的 `APMM_BACKGROUND_LAUNCHER`、`APMM_TEST_PYTHON` 与隔离目录。缺少配置不回退为可见测试窗口。只有用户明确要求展示的应用/预览才保持可见。

`TEMP/TMP` 指向 B 盘只约束继承它们的进程，不代表 Codex 内置浏览器缓存、聊天日志或 Windows 分页文件也已迁走。用户报告 C 盘突然吃满时，先只读核对余量、近期创建/修改的大文件、进程内存、pagefile 当前/峰值和 Codex 错误日志；不能把内存耗尽产生的系统占用归因于源码文件，也不能未经授权删除缓存或改变分页设置。

## 构建、发行与继续任务

读 [releases-and-preview.md](releases-and-preview.md) 和工程 `docs/RELEASING.md`：覆盖当前构建报告路径、版本目录、源码输入摘要、MSI 打包副作用、预览编码与新旧清单发布顺序。软件更新的身份、退出交接与恢复边界读 [updates-and-startup.md](updates-and-startup.md)。

- `VERSION` 是版本真源，同步 package.json、Cargo 主程序版本、tauri.conf.json 和 CHANGELOG 当前章节。界面正式版本来自 `CARGO_PKG_VERSION`，浏览器来自已同步的 package.json；不全局替换依赖或旧兼容版本。
- 普通前端编译可直接输出到 B 盘任务目录；完整包使用当前 `build:portable`。ZIP/MSI 名称、内部元数据、清单与启动快照均核对目标版本；`includes(version)` 不能证明本轮升版或包包含当前源码。
- 所有报告入口先读脚本。当前 `verify-release.py` 默认审计本轮主包，只有显式传入 `--plugin-report` 才审计独立插件；需要时核对插件报告与其已构建版本，不为普通 UI 修改默认重打资产运行时，也不用旧应用报告为新包背书。

## 任务报告与接续

格式以当前工作区应用根 `CONTRIBUTING.md#任务报告` 为准。新会话遇到“继续”先在应用根 `reports/` 找用户指定编号或最近匹配主题，必要时对照用户链接的聊天。读最后阶段的阻塞和接续入口，再用当前 Git、版本和产物验证。独立任务使用新编号，续做在原文件追加小节。写入后用 `git check-ignore` 和 `git ls-files -- reports/` 确认隔离。

```markdown
# 0001 · 任务标题

- 开始与最后更新：YYYY-MM-DD HH:mm:ss UTC+08:00
- 目标与状态：进行中 / 部分交付 / 阻塞 / 完成
- 版本、分支与提交：实际值

## YYYY-MM-DD HH:mm:ss UTC+08:00 · 阶段名称

- 本阶段完成、关键取舍、受影响文件及职责
- 实际验证命令、结果及未验证事项
- 实际产物、独立证据路径和分渠道发布状态
- 下一阶段入口或剩余阻塞
```

`APMM_REPORT_DIR` 默认在 B 盘 `reports/<版本>/`，给机器报告和构建证据使用。人工报告链接相应文件。公开 `docs/` 保持长期指南和展示内容。
- 保留用户数据、签名材料和有复现价值的快照。清理前核对固定绝对路径与授权范围，不把整个 B 盘工作根、源码根、旧工程或共用 Cargo 缓存当作可删除暂存。
