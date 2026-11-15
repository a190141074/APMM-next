# 0.4.1 受影响文件

对照实施前源码快照，仅列本轮新增或修改的文件；第三方与生成文件保留既有归属。

```text
config/                       配色与更新源
src-tauri/src/                配色持久化、安装身份、更新器及取色
src-ui/{app,core,hooks,modules,shared}/  页面、配色、设置和动效
tools/                        打包、源码审计及定向验收
docs/                         设计、验收与文件职责
```

| 文件 | 变更 | 职责 | 行数 |
|---|---|---|---:|
| `.gitignore` | 修改 | 版本、许可与构建配置 | 27 |
| `CHANGELOG.md` | 修改 | 版本、许可与构建配置 | 186 |
| `CONTRIBUTING.md` | 修改 | 版本、许可与构建配置 | 76 |
| `LICENSE.txt` | 修改 | 版本、许可与构建配置 | 675 |
| `NOTICE.md` | 新增 | 版本、许可与构建配置 | 10 |
| `README.md` | 修改 | 版本、许可与构建配置 | 38 |
| `THIRD_PARTY_NOTICES.md` | 修改 | 版本、许可与构建配置 | 67654 |
| `VERSION` | 修改 | 版本、许可与构建配置 | 2 |
| `config/cnb.json` | 修改 | 内嵌主题或发布源配置 | 33 |
| `config/themes.json` | 新增 | 内嵌主题或发布源配置 | 5 |
| `docs/ACCEPTANCE-041.md` | 新增 | 开发、验收与职责说明 | 41 |
| `docs/CHANGESET-041.md` | 新增 | 开发、验收与职责说明 | 95 |
| `docs/FILE-MAP.md` | 修改 | 开发、验收与职责说明 | 240 |
| `docs/REFERENCE-REVIEW-041.md` | 新增 | 开发、验收与职责说明 | 16 |
| `docs/UPDATE-041.md` | 新增 | 开发、验收与职责说明 | 29 |
| `docs/images/apx-tabs.png` | 新增 | 开发、验收与职责说明 | 233 |
| `docs/images/settings.png` | 新增 | 开发、验收与职责说明 | 169 |
| `docs/images/theme-editor.png` | 新增 | 开发、验收与职责说明 | 609 |
| `package.json` | 修改 | 版本、许可与构建配置 | 48 |
| `src-tauri/Cargo.lock` | 修改 | 版本、许可与构建配置 | 5239 |
| `src-tauri/Cargo.toml` | 修改 | 版本、许可与构建配置 | 67 |
| `src-tauri/src/app.rs` | 修改 | 配色、状态持久化及 IPC 接入 | 140 |
| `src-tauri/src/appearance.rs` | 新增 | 配色、状态持久化及 IPC 接入 | 106 |
| `src-tauri/src/appearance_commands.rs` | 新增 | 配色、状态持久化及 IPC 接入 | 19 |
| `src-tauri/src/bin/astral-updater.rs` | 修改 | 安装更新、助手交接与恢复 | 83 |
| `src-tauri/src/directory_monitor.rs` | 修改 | 配色、状态持久化及 IPC 接入 | 288 |
| `src-tauri/src/dto.rs` | 修改 | 配色、状态持久化及 IPC 接入 | 137 |
| `src-tauri/src/helper_process.rs` | 新增 | 安装更新、助手交接与恢复 | 80 |
| `src-tauri/src/installation.rs` | 新增 | 安装更新、助手交接与恢复 | 61 |
| `src-tauri/src/lib.rs` | 修改 | 配色、状态持久化及 IPC 接入 | 252 |
| `src-tauri/src/msi_update.rs` | 新增 | 安装更新、助手交接与恢复 | 244 |
| `src-tauri/src/portable_update.rs` | 修改 | 安装更新、助手交接与恢复 | 408 |
| `src-tauri/src/screen_picker.rs` | 新增 | 配色、状态持久化及 IPC 接入 | 80 |
| `src-tauri/src/settings_commands.rs` | 修改 | 配色、状态持久化及 IPC 接入 | 98 |
| `src-tauri/src/storage.rs` | 修改 | 配色、状态持久化及 IPC 接入 | 652 |
| `src-tauri/src/update_channel.rs` | 修改 | 安装更新、助手交接与恢复 | 279 |
| `src-tauri/src/update_gate.rs` | 新增 | 安装更新、助手交接与恢复 | 35 |
| `src-tauri/src/update_install.rs` | 新增 | 安装更新、助手交接与恢复 | 154 |
| `src-tauri/src/updates.rs` | 修改 | 安装更新、助手交接与恢复 | 277 |
| `src-tauri/tauri.conf.json` | 修改 | 版本、许可与构建配置 | 38 |
| `src-ui/app/App.tsx` | 修改 | 页面、服务与导航接入 | 496 |
| `src-ui/appearance.css` | 新增 | 配色、设置、主题控件与页面切换 | 300 |
| `src-ui/compact-workspace.css` | 修改 | 页面、服务与导航接入 | 172 |
| `src-ui/core/domain/appearance.test.ts` | 新增 | 配色、设置、主题控件与页面切换 | 29 |
| `src-ui/core/domain/appearance.ts` | 新增 | 配色、设置、主题控件与页面切换 | 186 |
| `src-ui/core/domain/resources.ts` | 修改 | 页面、服务与导航接入 | 243 |
| `src-ui/core/ipc/commands.ts` | 修改 | 页面、服务与导航接入 | 109 |
| `src-ui/core/ipc/generated/AppearanceSettings.ts` | 新增 | 自动生成的 IPC 契约 | 5 |
| `src-ui/core/ipc/generated/ThemePalette.ts` | 新增 | 自动生成的 IPC 契约 | 4 |
| `src-ui/core/ipc/generated/ThemeScheme.ts` | 新增 | 自动生成的 IPC 契约 | 5 |
| `src-ui/core/ipc/generated/WorkspaceSettings.ts` | 修改 | 自动生成的 IPC 契约 | 6 |
| `src-ui/core/ipc/mock.ts` | 修改 | 页面、服务与导航接入 | 93 |
| `src-ui/core/services/appearance.ts` | 新增 | 配色、设置、主题控件与页面切换 | 42 |
| `src-ui/hooks/useAppearance.tsx` | 新增 | 页面、服务与导航接入 | 167 |
| `src-ui/hooks/usePageNavigation.ts` | 新增 | 配色、设置、主题控件与页面切换 | 105 |
| `src-ui/hooks/useStartup.ts` | 修改 | 页面、服务与导航接入 | 62 |
| `src-ui/interface.css` | 修改 | 页面、服务与导航接入 | 224 |
| `src-ui/main.tsx` | 修改 | 页面、服务与导航接入 | 26 |
| `src-ui/modules/AboutPage.tsx` | 修改 | 页面、服务与导航接入 | 189 |
| `src-ui/modules/AppearanceSettingsPanel.tsx` | 新增 | 配色、设置、主题控件与页面切换 | 62 |
| `src-ui/modules/ApxPage.tsx` | 修改 | 页面、服务与导航接入 | 290 |
| `src-ui/modules/ColorControl.tsx` | 新增 | 配色、设置、主题控件与页面切换 | 203 |
| `src-ui/modules/DebugPage.tsx` | 新增 | 页面、服务与导航接入 | 13 |
| `src-ui/modules/SettingsPage.tsx` | 修改 | 配色、设置、主题控件与页面切换 | 312 |
| `src-ui/modules/ThemeEditor.tsx` | 新增 | 配色、设置、主题控件与页面切换 | 266 |
| `src-ui/modules/ThemesPanel.tsx` | 新增 | 配色、设置、主题控件与页面切换 | 187 |
| `src-ui/modules/UpdateSettings.tsx` | 新增 | 配色、设置、主题控件与页面切换 | 151 |
| `src-ui/settings.css` | 新增 | 页面、服务与导航接入 | 192 |
| `src-ui/shared/SegmentedTabs.tsx` | 新增 | 配色、设置、主题控件与页面切换 | 60 |
| `src-ui/style.css` | 修改 | 页面、服务与导航接入 | 1436 |
| `src-ui/theme-editor.css` | 新增 | 页面、服务与导航接入 | 297 |
| `src-ui/workspace.css` | 修改 | 页面、服务与导航接入 | 492 |
| `tools/audit-source.mjs` | 新增 | 构建、审计与隔离验收 | 19 |
| `tools/build-distributions.mjs` | 新增 | 构建、审计与隔离验收 | 211 |
| `tools/build-portable.mjs` | 修改 | 构建、审计与隔离验收 | 153 |
| `tools/third-party-notices.mjs` | 修改 | 构建、审计与隔离验收 | 56 |
| `tools/verify-appearance-native.mjs` | 新增 | 构建、审计与隔离验收 | 282 |
| `tools/verify-distributions.py` | 新增 | 构建、审计与隔离验收 | 47 |
| `tools/verify-legacy-bridge.mjs` | 新增 | 构建、审计与隔离验收 | 108 |
| `tools/verify-msi.ps1` | 新增 | 构建、审计与隔离验收 | 40 |
| `tools/verify-update-app.mjs` | 修改 | 构建、审计与隔离验收 | 185 |
| `tools/verify-updater.mjs` | 修改 | 构建、审计与隔离验收 | 43 |
| `vendor/wix/LICENSE.txt` | 新增 | 版本、许可与构建配置 | 29 |
| `vendor/wix/README.md` | 新增 | 版本、许可与构建配置 | 6 |

超过 500 行的本轮手写文件按当前职责复核：`storage.rs` 保留同一 Library 持久契约，新增配色校验已独立到 appearance.rs；`App.tsx` 保留导航与页面编排，切页状态独立到 usePageNavigation；样式文件属于声明式配置。第三方许可与 Cargo 锁文件属于生成清单。
