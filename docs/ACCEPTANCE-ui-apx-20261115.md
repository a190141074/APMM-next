# 界面、关于与 APX 目录验收（2026-11-15）

本次继续完善 0.3.38 源码和浏览器预览，依据当前 CONTRIBUTING.md 的 Rust + Tauri 2 + React + TypeScript 架构。旧 astral-tauri-modding skill 中其他工程的路径、原生界面约定和发布默认值不作为本次工程依据。

## 用户反馈与最终行为

- 总览主按钮为“安装 APX 插件”，明确先安装 APX，再应用 Mod；引导顺序为安装 APX、管理 Mod、按需提取原图。点击后进入渠道选择及游戏版本适配页，保留用户确认安装的步骤。
- 设置下方新增独立“关于”。主页面只列应用信息、NapCatQQ-Desktop/APX 引用和主要界面/桌面依赖，共 9 条鸣谢。完整 521 项依赖及许可原文收进“第三方声明”窗口，可查找和阅读单项，避免把长声明堆在页面中。
- 本工程 LICENSE.txt 原文保持不变，不将第三方 MIT 许可写成本项目许可。NapCatQQ-Desktop 引用标注其 GPL-3.0 许可；此处属于布局和交互参考。
- 搜索模式、资源部位、预览尺寸、主题、提取范围、包格式统一使用 Radix Select，支持浅色/深色、键盘导航、浮层边界和滚动。
- LocalLow、Data、游戏根目录按渠道独立存储。一次修改仅保存指定平台、指定字段；旧全局配置只迁移给原来选中的平台，不覆盖已保存的新配置。
- 三个目录均在“更改”前提供“智能匹配”。匹配只返回存在目录，候选需点“使用此目录”确认；没有候选时可以手动选择。TapTap、哔哩哔哩、WeGame 的 Data 名称相同，需要分别指定各自游戏根目录，不能从同名目录猜渠道。
- APX 状态、安装、还原和打开目录都使用当前平台配置。Android 保留导出入口，本机目录设置禁用。辅助资产工具仅在工作区与当前渠道是同一安装时传递该渠道的 LocalLow 配置。
- 独立“补丁缓存”卡片移除；缓存作为安装和导出内部步骤继续使用。
- 对比中的原图、Mod 图片分别支持滚轮围绕指针缩放、按住拖动平移、双击复位，也支持 Home/0、加减及方向键。显示缩放范围为 25%–800%，两侧状态独立，重新打开复位；监听在关闭或换图时释放。
- 导航名称改为“地块”“鼠标”“临时”。此前完成的紧凑资源工具栏、去除宣传文案和标题栏任务进程小窗仍保留，回归证据见 ACCEPTANCE-compact-ui-20261115.md。

## 本次受影响文件

局部文件树：

```text
apmm-desktop/
├─ src-ui/
│  ├─ app/App.tsx
│  ├─ core/{domain,ipc,services}/
│  ├─ modules/{AboutPage,ApxDirectories,ApxPage,PackageDialog,PluginsPage,
│  │           ResourceDetail,ResourcePage,ResourceSearch,SettingsPage}.tsx
│  ├─ shared/{PanZoomImage,ThemedSelect}.tsx
│  └─ {main.tsx,interface.css}
├─ src-tauri/src/{apx,apx_directories,auxiliary,external_links,game,lib,
│                 settings_commands,storage}.rs
├─ tools/{verify-about-apx-ui,verify-compact-ui}.mjs
├─ docs/{FILE-MAP,ACCEPTANCE-ui-apx-20261115}.md
└─ {package.json,pnpm-lock.yaml,THIRD_PARTY_NOTICES.md,
    verification-about-apx-ui.json,verification-compact-ui.json}
```

| 文件 | 职责 |
|---|---|
| `src-ui/app/App.tsx` | 应用导航、标题栏和 APX 优先总览入口 |
| `src-ui/core/domain/credits.ts` | 解析完整第三方声明及主页面主要鸣谢分组 |
| `src-ui/core/domain/resources.ts` | 页面路由和地块、鼠标、临时导航名称 |
| `src-ui/core/ipc/commands.ts` | 生成的 IPC 命令列表，包含目录设置、匹配及外链 |
| `src-ui/core/ipc/generated/PluginDirectories.ts` | ts-rs 生成的平台目录 DTO |
| `src-ui/core/services/app.ts` | 应用服务及系统浏览器外链适配 |
| `src-ui/interface.css` | 主题选择器、目录候选、紧凑关于页及图片缩放布局 |
| `src-ui/main.tsx` | React 启动和界面样式加载顺序 |
| `src-ui/modules/AboutPage.tsx` | 紧凑鸣谢、项目引用和完整许可弹窗 |
| `src-ui/modules/ApxDirectories.tsx` | 按平台独立的目录显示、手动选择和候选确认 |
| `src-ui/modules/ApxPage.tsx` | APX 平台适配、安装、还原及导出界面 |
| `src-ui/modules/PackageDialog.tsx` | 包导入导出向导及主题格式选择器 |
| `src-ui/modules/PluginsPage.tsx` | 工坊及辅助插件界面与主题提取范围选择 |
| `src-ui/modules/ResourceDetail.tsx` | 原图和 Mod 对比，接入独立图片缩放组件 |
| `src-ui/modules/ResourcePage.tsx` | 紧凑资源筛选、主题预览尺寸与部位选择 |
| `src-ui/modules/ResourceSearch.tsx` | 全局搜索和主题搜索模式选择 |
| `src-ui/modules/SettingsPage.tsx` | 设置界面及主题选择器 |
| `src-ui/shared/PanZoomImage.tsx` | 图片指针缩放、捕获拖动、复位及监听释放 |
| `src-ui/shared/ThemedSelect.tsx` | 统一的 Radix 主题选择器和键盘导航 |
| `src-tauri/src/apx.rs` | APX 安装事务及按平台解析目标目录 |
| `src-tauri/src/apx_directories.rs` | 平台目录持久化、旧数据迁移、只读匹配和隔离测试 |
| `src-tauri/src/auxiliary.rs` | 辅助插件生命周期及同安装 LocalLow 配置传递 |
| `src-tauri/src/external_links.rs` | 校验 HTTPS 引用并打开系统浏览器 |
| `src-tauri/src/game.rs` | Steam 库发现及 libraryfolders.vdf 路径兼容 |
| `src-tauri/src/lib.rs` | 领域模块及 IPC 命令注册 |
| `src-tauri/src/settings_commands.rs` | 设置与预设持久化，兼容旧目录 patch 到对应平台 |
| `src-tauri/src/storage.rs` | Library 数据、平台目录配置及载入迁移 |
| `package.json` | 固定 Radix Select 依赖和开发命令 |
| `pnpm-lock.yaml` | 生成的依赖解析锁文件 |
| `THIRD_PARTY_NOTICES.md` | 生成的完整依赖许可与版权原文 |
| `tools/verify-about-apx-ui.mjs` | 关于、主题控件、平台隔离、APX 引导及图片交互验收 |
| `tools/verify-compact-ui.mjs` | 紧凑布局和任务进程浮层验收，适配 Radix 控件 |
| `verification-about-apx-ui.json` | 本轮 7 项浏览器检查结果及截图目录 |
| `verification-compact-ui.json` | 9 项紧凑布局和任务浮层回归结果及几何测量 |
| `docs/FILE-MAP.md` | 更新上述文件职责和行数，登记新增模块 |
| `docs/ACCEPTANCE-ui-apx-20261115.md` | 本次设计取舍、文件清单、验证和边界 |

新建和明显扩展的手写源码文件均未超过 500 行；App.tsx 为 487 行，包向导为 477 行。继承的 apx.rs/storage.rs 仍属于既有大文件，本次把平台目录模型、命令、迁移和检测提取到 apx_directories.rs，避免继续堆入 APX 事务入口。许可证与锁文件属于生成内容。

## 实际验证

| 类别 | 命令或证据 | 结果 |
|---|---|---|
| TypeScript | `pnpm typecheck` | 通过 |
| 前端编译 | Vite 生产构建，输出到 B 盘隔离目录 | 通过；About 许可原文 chunk 约 3.34 MB，gzip 约 152 KB，存在 Vite 大 chunk 提示。About 按页面延迟加载，原文完整保留 |
| Rust 定向测试 | `node tools/cargo.mjs test --lib apx_directories` | 4 项通过、85 项过滤；覆盖原平台迁移、既有覆盖保留、序列化回读、字段/平台隔离、非法请求及只读探测 |
| Rust 静态检查 | `node tools/cargo.mjs check`（all-targets） | 通过；仍有既有 unused/dead-code 警告 |
| 浏览器新交互 | `node tools/verify-about-apx-ui.mjs` | 7 项通过，无 pageerror；包括真实图片缩放/拖动/复位、主题键盘操作、多候选和跨平台操作 |
| 紧凑界面回归 | `node tools/verify-compact-ui.mjs` | 9 项通过，无 pageerror；1089/900/1360 宽度无横向溢出，预览行高和任务控制保留 |
| 许可生成 | `node tools/third-party-notices.mjs` | 生成 521 项完整声明，包含新增 Radix Select 依赖 |

截图与浏览器证据：

- `B:\APMM-next-work\about-apx-validation\c146bdc0-372b-4481-b70e-ba37d4a1de1f`
- `B:\APMM-next-work\compact-ui-validation\a7f9e117-ecfe-40b8-9d47-74b98828c56a`
- 前端构建：`B:\APMM-next-work\about-apx-20261115-003210\frontend`
- 修改前源码快照：`B:\APMM-next-work\about-apx-20261115-003210`

## 未验证范围与剩余边界

本轮浏览器目录操作采用传输边界夹具，Rust 测试使用 B 盘临时目录。它们证明界面请求与平台配置的隔离，不等同于正式 Tauri WebView/IPC、真实游戏安装或多平台同时运行的实机验收。未运行新发行副本的原生集成脚本，未构建、导出或覆盖 EXE/ZIP，未进行 Linux 编译或运行，也未写入实际游戏目录。

最小实机复核：在两套不同渠道安装上分别保存三个目录，切换渠道并重启，确认每个目录保持各自配置；打开目录对应正确安装，再由用户选择渠道和适配版本后执行安装、还原与导出。另在正式 WebView 验证外链、主题下拉和图片滚轮/拖动/双击操作。
