# 0.3.38 本地测试版验收

当前实现遵循本工程 CONTRIBUTING.md：Rust + Tauri 2 + React + TypeScript，使用 Tailwind CSS 与 Radix UI。原版 skill 中过时的静态前端入口和构建命令不适用。

本轮沿用已经授权的 8 项修正范围，正式构建使用 build:portable。独立源码快照和原测试程序备份用于对照，工作目录没有 Git 仓库。

## 功能对照

| 问题 | 完成内容 | 验证边界 |
|---|---|---|
| 资源缺项和中文名称 | 2,187 项官方资源、22 类；有证据的筹码名称 67 项；保留完整变体和旧导入项 | 目录测试、哈希和实际原图；107 个资源尚缺名称证据，明确标为未收录名称 |
| APX 布局和状态 | 主操作、渠道、状态提示、四项卡片、目录和文件清单 | 实际桌面部分安装、校验失败、旧缓存与缺缓存检查 |
| Logo | SVG、favicon、PNG 和多尺寸 Windows ICO 统一 | 浏览器实际解码，EXE 图标随正式构建嵌入 |
| 根目录第三方声明 | 正式包和测试目录使用 licenses 子目录 | 便携 ZIP 清单、嵌套许可与旧平铺包白名单、恢复测试 |
| 导入导出 | 分类/角色/皮肤/部位勾选、缩略图、冲突状态、终态统计、密码和分卷 | 真实导入向导显示第四候选；APMMOD01/02 头兼容、03 往返与缺卷测试 |
| 搜索 | 智能/ID、昵称/英文/文件名/哈希、建议与键盘定位 | 浏览器智能搜索定位、相同结果重复进入、纯规则测试 |
| 多皮肤、多部位、多候选 | 独立详情工作页，点击应用/原版还原，放大与管理按钮分离 | 真实桌面三个候选、快速点击、文件缺失恢复、第四候选即时更新 |
| 本地预览 | 2,179 张原图及索引单独补入测试目录，正式 ZIP 无原图 | 所有图片哈希核验和浏览器真实解码 2,179/2,179 |

## 已执行验证

- TypeScript 检查和 Rust all-targets 检查通过；Rust 格式检查通过。原有 unused/dead-code 警告保留。
- 前端领域测试 8 项通过：队列失败后继续、搜索边界、多个皮肤/部位/候选、APX 状态。
- Rust 专项：目录 1 项，原图哈希失效 1 项，更新白名单/恢复 2 项，APX 10 项，包分卷及路径 4 项，真实三 PNG 往返 1 项，历史 APMMOD01/02 头 1 项，均通过。
- 浏览器验证 9 组通过，浅深主题、1000px 窄窗口和 2,179 张原图真实解码通过。
- 最终 EXE 的真实桌面验证 19 组通过，写入全部指向 B 盘隔离 modimg，使用真实 React 详情和导入向导。
- 独立更新助手的成功启动、无法启动和中断恢复均通过；嵌套许可与用户文件均保持正确。
- APX 桌面专项 5 组通过：部分安装、校验失败、缓存过期、旧缓存导出、缺文件；旧缓存导出没有改变隔离游戏文件。
- 便携 ZIP、EXE、许可和插件包实际清单/大小/SHA-256 审计通过。

## 未验证和剩余边界

- 本轮没有进行游戏内 Mod 视觉验收，也没有向真实游戏安装 APX。APX 写入事务沿用现有实现，安装落盘实测未在本轮执行。
- Windows 系统文件选择器和真实外拖接收没有在本轮实测；导入向导采用测试文件事件进入，随后走实际 IPC。
- Linux 环境未编译或运行。
- 107 项资源没有可靠名称证据，未按术语表次序或数字编号猜名称，详情见 catalog-audit.json。
- 浏览器为本地元数据和真实图片预览，桌面文件/游戏/APX 命令仍需要正式程序执行。

## 受影响文件与职责

以下文件与本次任务开始时独立源码快照对照；包含前一轮已经写入且本轮继续完成的改动。

```text
apmm-desktop/
  CHANGELOG.md
  VERSION
  config/chip_names.json
  config/resources.json
  index.html
  package.json
  public/logo.svg
  src-tauri/Cargo.lock
  src-tauri/Cargo.toml
  src-tauri/icons/128x128.png
  src-tauri/icons/128x128@2x.png
  src-tauri/icons/32x32.png
  src-tauri/icons/icon.ico
  src-tauri/icons/icon.png
  src-tauri/src/apx.rs
  src-tauri/src/catalog.rs
  src-tauri/src/dto.rs
  src-tauri/src/image_import.rs
  src-tauri/src/lib.rs
  src-tauri/src/originals.rs
  src-tauri/src/packages.rs
  src-tauri/src/portable_update.rs
  src-tauri/src/resource_rules.rs
  src-tauri/src/task_dispatch.rs
  src-tauri/src/workspace.rs
  src-tauri/src/workspace_catalog.rs
  src-tauri/tauri.conf.json
  src-ui/app/App.tsx
  src-ui/core/domain/applyQueue.ts
  src-ui/core/domain/apx.ts
  src-ui/core/domain/domain.test.ts
  src-ui/core/domain/packageResults.ts
  src-ui/core/domain/resources.ts
  src-ui/core/ipc/generated/ResourceRecord.ts
  src-ui/core/ipc/generated/WorkspaceSettings.ts
  src-ui/core/ipc/mock.ts
  src-ui/core/services/app.ts
  src-ui/hooks/useApp.ts
  src-ui/main.tsx
  src-ui/modules/ApxPage.tsx
  src-ui/modules/MaintenancePanel.tsx
  src-ui/modules/PackageDialog.tsx
  src-ui/modules/PackageSelection.tsx
  src-ui/modules/ResourceDetail.tsx
  src-ui/modules/ResourceFileActions.tsx
  src-ui/modules/ResourcePage.tsx
  src-ui/modules/ResourceSearch.tsx
  src-ui/modules/SettingsPage.tsx
  src-ui/shared/Ui.tsx
  src-ui/style.css
  src-ui/workspace.css
  tools/build-portable.mjs
  tools/deploy-local-test.py
  tools/generate-icons.mjs
  tools/local-preview.ts
  tools/sync-resource-catalog.py
  tools/verify-apx-runtime.mjs
  tools/verify-native.mjs
  tools/verify-release.py
  tools/verify-ui.mjs
  tools/verify-updater.mjs
  vite.config.ts
```

| 文件 | 职责 |
|---|---|
| `CHANGELOG.md` | 版本、构建入口或资源引用同步 |
| `VERSION` | 版本、构建入口或资源引用同步 |
| `config/chip_names.json` | 名称证据与完整资源元数据 |
| `config/resources.json` | 名称证据与完整资源元数据 |
| `index.html` | 版本、构建入口或资源引用同步 |
| `package.json` | 版本、构建入口或资源引用同步 |
| `public/logo.svg` | 唯一 Logo 矢量源与 favicon |
| `src-tauri/Cargo.lock` | 版本、构建入口或资源引用同步 |
| `src-tauri/Cargo.toml` | 版本、构建入口或资源引用同步 |
| `src-tauri/icons/128x128.png` | 统一紫色星光 PNG/ICO 图标 |
| `src-tauri/icons/128x128@2x.png` | 统一紫色星光 PNG/ICO 图标 |
| `src-tauri/icons/32x32.png` | 统一紫色星光 PNG/ICO 图标 |
| `src-tauri/icons/icon.ico` | 统一紫色星光 PNG/ICO 图标 |
| `src-tauri/icons/icon.png` | 统一紫色星光 PNG/ICO 图标 |
| `src-tauri/src/apx.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/catalog.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/dto.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/image_import.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/lib.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/originals.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/packages.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/portable_update.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/resource_rules.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/task_dispatch.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/workspace.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/src/workspace_catalog.rs` | 资源目录、名称规则、预览、任务结果、APX 与更新边界 |
| `src-tauri/tauri.conf.json` | 版本、构建入口或资源引用同步 |
| `src-ui/app/App.tsx` | 导航、搜索模式、Mod 包菜单与主题外壳 |
| `src-ui/core/domain/applyQueue.ts` | 资源分组、搜索、应用顺序与任务结果统计 |
| `src-ui/core/domain/apx.ts` | 资源分组、搜索、应用顺序与任务结果统计 |
| `src-ui/core/domain/domain.test.ts` | 资源分组、搜索、应用顺序与任务结果统计 |
| `src-ui/core/domain/packageResults.ts` | 资源分组、搜索、应用顺序与任务结果统计 |
| `src-ui/core/domain/resources.ts` | 资源分组、搜索、应用顺序与任务结果统计 |
| `src-ui/core/ipc/generated/ResourceRecord.ts` | 兼容传输和生成的资源 DTO |
| `src-ui/core/ipc/generated/WorkspaceSettings.ts` | 兼容传输和生成的资源 DTO |
| `src-ui/core/ipc/mock.ts` | 兼容传输和生成的资源 DTO |
| `src-ui/core/services/app.ts` | 等待任务终态、串行应用与 IPC 服务 |
| `src-ui/hooks/useApp.ts` | 版本、构建入口或资源引用同步 |
| `src-ui/main.tsx` | 版本、构建入口或资源引用同步 |
| `src-ui/modules/ApxPage.tsx` | 资源详情、搜索、APX、导入导出、候选管理和维护页面 |
| `src-ui/modules/MaintenancePanel.tsx` | 资源详情、搜索、APX、导入导出、候选管理和维护页面 |
| `src-ui/modules/PackageDialog.tsx` | 资源详情、搜索、APX、导入导出、候选管理和维护页面 |
| `src-ui/modules/PackageSelection.tsx` | 资源详情、搜索、APX、导入导出、候选管理和维护页面 |
| `src-ui/modules/ResourceDetail.tsx` | 资源详情、搜索、APX、导入导出、候选管理和维护页面 |
| `src-ui/modules/ResourceFileActions.tsx` | 资源详情、搜索、APX、导入导出、候选管理和维护页面 |
| `src-ui/modules/ResourcePage.tsx` | 资源详情、搜索、APX、导入导出、候选管理和维护页面 |
| `src-ui/modules/ResourceSearch.tsx` | 资源详情、搜索、APX、导入导出、候选管理和维护页面 |
| `src-ui/modules/SettingsPage.tsx` | 资源详情、搜索、APX、导入导出、候选管理和维护页面 |
| `src-ui/shared/Ui.tsx` | 原图状态和加载失败展示 |
| `src-ui/style.css` | 版本、构建入口或资源引用同步 |
| `src-ui/workspace.css` | 版本、构建入口或资源引用同步 |
| `tools/build-portable.mjs` | 资源元数据生成、构建、部署和隔离验证 |
| `tools/deploy-local-test.py` | 资源元数据生成、构建、部署和隔离验证 |
| `tools/generate-icons.mjs` | 资源元数据生成、构建、部署和隔离验证 |
| `tools/local-preview.ts` | 资源元数据生成、构建、部署和隔离验证 |
| `tools/sync-resource-catalog.py` | 资源元数据生成、构建、部署和隔离验证 |
| `tools/verify-apx-runtime.mjs` | 资源元数据生成、构建、部署和隔离验证 |
| `tools/verify-native.mjs` | 资源元数据生成、构建、部署和隔离验证 |
| `tools/verify-release.py` | 资源元数据生成、构建、部署和隔离验证 |
| `tools/verify-ui.mjs` | 资源元数据生成、构建、部署和隔离验证 |
| `tools/verify-updater.mjs` | 资源元数据生成、构建、部署和隔离验证 |
| `vite.config.ts` | 版本、构建入口或资源引用同步 |

新增/扩展手写文件均低于 500 行；PackageDialog 的任务结果解释已移入纯领域模块。原有 apx.rs 与 packages.rs 分别只调整校验和补充协议夹具，其兼容/事务职责仍保留，未做全仓重构。

## 产物和数据保护

最终部署及数据摘要见 verification-local-test.json；包摘要见 verification-portable-build.json；ZIP 审计见 verification-release-audit.json。
当前长期复用的 MSVC 缓存约 12.23 GiB；依据 CONTRIBUTING.md 保留。B 盘源码备份、原图、隔离验证和两轮构建暂存保留供本地反馈复现，未清理旧工程或共用缓存。

新增文档和验收附件：

- `docs/ACCEPTANCE-0.3.38.md`：功能对照、边界、全部受影响文件和分层验证。
- `docs/changed-files-0.3.38.json`：与独立源码快照相比的完整文件列表。
- `docs/catalog-audit.json`：资源覆盖、缺名和筹码证据审计。
- `verification-local-test.json`：实际测试目录、备份和 1,431 个既有文件数据保护结果。
- `verification-native.json`、`verification-apx-runtime.json`、`verification-updater.json`、`verification-ui.json`：真实桌面、APX、更新助手和浏览器验证证据。
- `verification-portable-build.json`、`verification-release-audit.json`：实际发行包、EXE 与 ZIP 摘要、内容边界和缓存状态。
