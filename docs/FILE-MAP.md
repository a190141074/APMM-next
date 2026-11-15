# 受影响文件与职责

当前 `0.3.39` 的完整变更范围见 [CHANGED-MANAGEMENT-0.3.39.md](CHANGED-MANAGEMENT-0.3.39.md)，以本轮 B 盘只读快照差异为准。下方早期迁入清单保留历史定位，新管理职责按下面的模块边界组织。

## 0.3.39 管理职责

| 入口 | 职责 |
|---|---|
| `src-tauri/src/management_types.rs`、`presets.rs`、`preset_domain.rs` | 持久契约、预设 IPC、范围与内容校验和整批应用事务 |
| `src-tauri/src/candidate_management.rs` | 具体候选的来源和备注编辑、归属合并、跨盘备份与回滚 |
| `src-tauri/src/directory_files.rs`、`vault_sync.rs` | PNG 递归枚举和文件标识、库候选编辑关联、规则迁移和待确认结果 |
| `src-tauri/src/directory_sync.rs`、`directory_monitor.rs` | 被动游戏读取及实际应用计算、原生通知与稳定等待、任务互斥和退出取消 |
| `src-tauri/src/management_tests.rs`、`sync_management_tests.rs` | 预设、归属、旧数据、独立信息、外部编辑与事务失败夹具 |
| `src-ui/modules/PresetsPage.tsx`、`CreatePresetDialog.tsx` | 预设详情、编辑与修复、应用清单、创建全局或固定范围 |
| `src-ui/modules/ResourceFileActions.tsx`、`shared/ResourceSlotSelect.tsx` | 模组信息、四级归属和规则冲突确认 |
| `src-ui/core/domain/importClassification.ts`、`resources.ts` | 导入目标与重复预览、实际分类映射、名称容错与稳定排序 |
| `src-ui/core/services/management.ts`、`hooks/useManagement.ts` | 管理 IPC 服务和只读展示查询 |
| `src-ui/core/ipc/transport.ts`、`shared/Ui.tsx`、`shared/PanZoomImage.tsx` | 内容哈希修订图片请求、失败刷新与缩放生命周期 |
| `src-ui/shared/CandidateComparison.tsx`、`CandidateCreditsFields.tsx`、`core/domain/candidateInfo.ts` | 模组信息的左侧只读对比、作者和画师多链接、无后缀名称与导出命名 |
| `src-ui/management-ui.css` | 模组信息的两栏布局、紧凑字段与窄窗堆叠 |
| `tools/verify-management-native.mjs`、`management-native-ui.mjs`、`management-image-refresh.mjs` | 发行 EXE 的隔离 IPC、管理界面和同路径图片像素验收 |
| `tools/verify-management-ui.mjs`、`management-about-ui.mjs` | 浏览器交互、主题、键盘定位和关于页布局验收 |
| `tools/management-fixtures.mjs`、`management-source-inventory.py` | 真实 PNG、源码快照差异与文件职责审计 |

### 本轮 500 行职责复核

新增预设、归属、目录同步和监听均为独立模块，各手写模块保持在 500 行内。继承文件按本轮所涉及的窄入口修改，未顺带重构历史协议。

- `packages.rs`：仍拥有历史 APMMOD01/02/03 与分卷事务。本轮只在既有文件元信息、归一化和提交处加入来源、手动身份与 PNG 校验，没有将预设或监听塞入包协议。
- `resource_mutation.rs`、`legacy_commands.rs`：扫描、临时迁移和导入委托新领域，保留旧兼容入口；新职责已独立，遗留资源维护流程保持原边界。
- `storage.rs`：新字段与默认值仍属于 Library 持久契约；预设应用、候选归类和目录监听流程均在独立模块。
- `legacy_tests.rs`、`workshop.rs`：仅适配新候选默认字段和真实 PNG 旧夹具；未加入新的页面或业务流程。
- `tools/verify-management-native.mjs`：超过 500 行但仅编排同一个发行副本的完整验收旅程；PNG、页面布局和像素检查已拆出，保留单一副本、调用顺序和失败证据，避免多个脚本各自维护可变索引。

新工程内的下列源码/配置均为本次建立或迁入；旧工程文件基线另行核对。第三方 vendored 依赖保留原许可，生成文件由对应脚本维护。

| 文件 | 行数 | 职责 |
|---|---:|---|
| `config/auxiliary.json` | 6 | 可选插件的安装、卸载、运行与取消 |
| `config/card_names.json` | 143 | 所属模块的配置、领域模型或兼容辅助 |
| `config/cnb.json` | 32 | 所属模块的配置、领域模型或兼容辅助 |
| `config/heroes.json` | 37 | 所属模块的配置、领域模型或兼容辅助 |
| `config/monsters.json` | 450 | 所属模块的配置、领域模型或兼容辅助 |
| `config/role_nicknames.json` | 1 | 所属模块的配置、领域模型或兼容辅助 |
| `config/sections.json` | 179 | 所属模块的配置、领域模型或兼容辅助 |
| `docs/legacy-baseline.json` | 28 | 所属模块的配置、领域模型或兼容辅助 |
| `package.json` | 46 | 固定 Radix Select 依赖和开发命令 |
| `plugins/asset-tool/apmm-util.py` | 7 | 所属模块的配置、领域模型或兼容辅助 |
| `plugins/asset-tool/pyproject.toml` | 20 | 所属模块的配置、领域模型或兼容辅助 |
| `plugins/asset-tool/src/apmm_util/__init__.py` | 4 | 独立资产工具的 __init__ 领域实现 |
| `plugins/asset-tool/src/apmm_util/app.py` | 91 | 独立资产工具的 app 领域实现 |
| `plugins/asset-tool/src/apmm_util/builtin_sync.py` | 61 | 独立资产工具的 builtin_sync 领域实现 |
| `plugins/asset-tool/src/apmm_util/catalog/__init__.py` | 2 | 独立资产工具的 __init__ 领域实现 |
| `plugins/asset-tool/src/apmm_util/catalog/locator.py` | 70 | 独立资产工具的 locator 领域实现 |
| `plugins/asset-tool/src/apmm_util/catalog/parser.py` | 144 | 独立资产工具的 parser 领域实现 |
| `plugins/asset-tool/src/apmm_util/catalog/source.py` | 87 | 独立资产工具的 source 领域实现 |
| `plugins/asset-tool/src/apmm_util/categories/__init__.py` | 2 | 独立资产工具的 __init__ 领域实现 |
| `plugins/asset-tool/src/apmm_util/categories/hand_card.py` | 14 | 独立资产工具的 hand_card 领域实现 |
| `plugins/asset-tool/src/apmm_util/categories/registry.py` | 91 | 独立资产工具的 registry 领域实现 |
| `plugins/asset-tool/src/apmm_util/cli.py` | 149 | 独立资产工具的 cli 领域实现 |
| `plugins/asset-tool/src/apmm_util/config.py` | 62 | 独立资产工具的 config 领域实现 |
| `plugins/asset-tool/src/apmm_util/diagnostics.py` | 60 | 独立资产工具的 diagnostics 领域实现 |
| `plugins/asset-tool/src/apmm_util/errors.py` | 36 | 独立资产工具的 errors 领域实现 |
| `plugins/asset-tool/src/apmm_util/inspection.py` | 44 | 独立资产工具的 inspection 领域实现 |
| `plugins/asset-tool/src/apmm_util/io_utils.py` | 59 | 独立资产工具的 io_utils 领域实现 |
| `plugins/asset-tool/src/apmm_util/models.py` | 89 | 独立资产工具的 models 领域实现 |
| `plugins/asset-tool/src/apmm_util/output.py` | 148 | 独立资产工具的 output 领域实现 |
| `plugins/asset-tool/src/apmm_util/resources/sections.json` | 179 | 独立资产工具的 sections 领域实现 |
| `plugins/asset-tool/src/apmm_util/resources/texture2d-2022.3.62f3.json` | 434 | 独立资产工具的 texture2d-2022.3.62f3 领域实现 |
| `plugins/asset-tool/src/apmm_util/runtime.py` | 79 | 独立资产工具的 runtime 领域实现 |
| `plugins/asset-tool/src/apmm_util/state.py` | 73 | 独立资产工具的 state 领域实现 |
| `plugins/asset-tool/src/apmm_util/sync.py` | 141 | 独立资产工具的 sync 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/__init__.py` | 2 | 独立资产工具的 __init__ 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/backend.py` | 50 | 独立资产工具的 backend 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/binary.py` | 41 | 独立资产工具的 binary 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/builtin.py` | 50 | 独立资产工具的 builtin 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/bundle.py` | 146 | 独立资产工具的 bundle 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/common_strings.py` | 121 | 独立资产工具的 common_strings 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/inspection.py` | 39 | 独立资产工具的 inspection 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/pixels.py` | 42 | 独立资产工具的 pixels 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/reference.py` | 62 | 独立资产工具的 reference 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/session.py` | 71 | 独立资产工具的 session 领域实现 |
| `plugins/asset-tool/src/apmm_util/unity/texture.py` | 67 | 独立资产工具的 texture 领域实现 |
| `plugins/asset-tool/tests/__init__.py` | 2 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/helpers.py` | 58 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/test_catalog.py` | 84 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/test_diagnostics.py` | 78 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/test_inspection.py` | 98 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/test_output_state.py` | 76 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/test_pixels.py` | 65 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/test_reference.py` | 67 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/test_sync.py` | 215 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/test_unity.py` | 101 | 资产解析、像素或同步的自动化夹具 |
| `plugins/asset-tool/tests/unity_fixtures.py` | 45 | 资产解析、像素或同步的自动化夹具 |
| `src/card_names.json` | 143 | 所属模块的配置、领域模型或兼容辅助 |
| `src/heroes.json` | 37 | 所属模块的配置、领域模型或兼容辅助 |
| `src/manifest.json` | 10316 | 所属模块的配置、领域模型或兼容辅助 |
| `src/monsters.json` | 450 | 所属模块的配置、领域模型或兼容辅助 |
| `src/role_nicknames.json` | 1 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/bindings/serde_json/JsonValue.ts` | 3 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/build.rs` | 6 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/capabilities/default.json` | 15 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/Cargo.toml` | 60 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/gen/schemas/acl-manifests.json` | 1 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/gen/schemas/capabilities.json` | 1 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/gen/schemas/desktop-schema.json` | 2358 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/gen/schemas/windows-schema.json` | 2358 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/app.rs` | 114 | 应用启动、WebView、便携目录和协议生命周期 |
| `src-tauri/src/apx.rs` | 1636 | APX 安装事务及按平台解析目标目录 |
| `src-tauri/src/asset_catalog.rs` | 303 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/aux_manifest.rs` | 186 | 插件签名清单、ZIP 边界与文件验证 |
| `src-tauri/src/auxiliary.rs` | 322 | 辅助插件生命周期及同安装 LocalLow 配置传递 |
| `src-tauri/src/bin/astral-updater.rs` | 113 | 更新助手进程与启动确认 |
| `src-tauri/src/catalog.rs` | 268 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/classification.rs` | 20 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/commands.rs` | 6 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/diagnostics.rs` | 142 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/downloads.rs` | 329 | 有界下载、续传、范围和摘要校验 |
| `src-tauri/src/dto.rs` | 110 | Rust 到 TypeScript 的工作台契约 |
| `src-tauri/src/game.rs` | 101 | Steam 库发现及 libraryfolders.vdf 路径兼容 |
| `src-tauri/src/image_import.rs` | 27 | PNG 识别、目标及冲突预览 |
| `src-tauri/src/legacy_commands.rs` | 890 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/legacy_tests.rs` | 1043 | 旧库、路径与包协议行为夹具 |
| `src-tauri/src/lib.rs` | 216 | 领域模块及 IPC 命令注册 |
| `src-tauri/src/library.rs` | 13 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/logs.rs` | 48 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/main.rs` | 6 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/migration.rs` | 287 | 旧库复制、源变化校验和路径重写 |
| `src-tauri/src/migration_journal.rs` | 239 | 复制提交日志与重启恢复 |
| `src-tauri/src/naming.rs` | 406 | 旧名称解析兼容规则 |
| `src-tauri/src/native_drag.rs` | 296 | 继承的 Windows OLE 外拖边界 |
| `src-tauri/src/originals.rs` | 107 | 本地原图校验、远端回退和 URI 访问边界 |
| `src-tauri/src/package.rs` | 12 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/packages.rs` | 2640 | 历史包编解码、分卷、队列、导入导出事务 |
| `src-tauri/src/portable_update.rs` | 247 | 发行清单、更新日志和整批恢复 |
| `src-tauri/src/presets.rs` | 3 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/preview_engine.rs` | 826 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/preview_install.rs` | 141 | 远端预览安全展开与事务安装 |
| `src-tauri/src/remote_source.rs` | 280 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/resource_mutation.rs` | 2169 | 遗留资源导入、扫描协调、启禁和导出适配 |
| `src-tauri/src/resource_rules.rs` | 128 | 完整官方名称、变体及新增类型 |
| `src-tauri/src/resource_transaction.rs` | 73 | 游戏资源整批修改与回滚 |
| `src-tauri/src/scan.rs` | 15 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/scanner.rs` | 43 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/settings_commands.rs` | 241 | 设置与预设持久化，兼容旧目录 patch 到对应平台 |
| `src-tauri/src/staging.rs` | 138 | 工作盘、暂存白名单及跨盘提交 |
| `src-tauri/src/storage.rs` | 651 | Library 数据、平台目录配置及载入迁移 |
| `src-tauri/src/task_dispatch.rs` | 188 | 后台任务排队与领域调用适配 |
| `src-tauri/src/task_history.rs` | 80 | 任务持久化、重启中断和密码边界 |
| `src-tauri/src/tasks.rs` | 339 | 统一任务状态、修订号、取消与事件 |
| `src-tauri/src/thumbnails.rs` | 81 | 有界后台缩略图解码与独立内容缓存 |
| `src-tauri/src/update_channel.rs` | 240 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/updates.rs` | 356 | 更新检查、收据、任务闸门与助手调度 |
| `src-tauri/src/vault.rs` | 19 | 所属模块的配置、领域模型或兼容辅助 |
| `src-tauri/src/workshop.rs` | 795 | 工坊签名目录、安装及文件归属保护 |
| `src-tauri/src/workspace.rs` | 207 | 工作台快照与成组应用适配 |
| `src-tauri/tauri.conf.json` | 37 | 所属模块的配置、领域模型或兼容辅助 |
| `src-ui/app/App.tsx` | 487 | 应用导航、标题栏和 APX 优先总览入口 |
| `src-ui/core/domain/domain.test.ts` | 55 | 所属模块的配置、领域模型或兼容辅助 |
| `src-ui/core/domain/packages.ts` | 18 | 历史包编解码、分卷、队列、导入导出事务 |
| `src-ui/core/domain/resources.ts` | 133 | 页面路由和地块、鼠标、临时导航名称 |
| `src-ui/core/domain/tasks.ts` | 15 | 统一任务状态、修订号、取消与事件 |
| `src-ui/core/ipc/commands.ts` | 98 | 生成的 IPC 命令列表，包含目录设置、匹配及外链 |
| `src-ui/core/ipc/generated/ModFile.ts` | 3 | ts-rs 生成的 IPC 类型 |
| `src-ui/core/ipc/generated/PresetSummary.ts` | 3 | ts-rs 生成的 IPC 类型 |
| `src-ui/core/ipc/generated/ResourceAction.ts` | 3 | ts-rs 生成的 IPC 类型 |
| `src-ui/core/ipc/generated/ResourceRecord.ts` | 4 | ts-rs 生成的 IPC 类型 |
| `src-ui/core/ipc/generated/TaskSnapshot.ts` | 3 | ts-rs 生成的 IPC 类型 |
| `src-ui/core/ipc/generated/WorkspaceSettings.ts` | 4 | ts-rs 生成的 IPC 类型 |
| `src-ui/core/ipc/generated/WorkspaceSnapshot.ts` | 5 | ts-rs 生成的 IPC 类型 |
| `src-ui/core/ipc/mock.ts` | 186 | 所属模块的配置、领域模型或兼容辅助 |
| `src-ui/core/ipc/transport.ts` | 55 | 所属模块的配置、领域模型或兼容辅助 |
| `src-ui/core/services/app.ts` | 58 | 应用服务及系统浏览器外链适配 |
| `src-ui/env.d.ts` | 1 | 所属模块的配置、领域模型或兼容辅助 |
| `src-ui/hooks/useApp.ts` | 110 | 查询、事件、窗口及操作状态 hooks |
| `src-ui/hooks/useStartup.ts` | 60 | 启动确认、间隔更新检查和偏好恢复 |
| `src-ui/main.tsx` | 18 | React 启动和界面样式加载顺序 |
| `src-ui/compact-workspace.css` | 171 | 紧凑资源工具栏与任务浮层布局 |
| `src-ui/modules/PackageDialog.tsx` | 477 | 包导入导出向导及主题格式选择器 |
| `src-ui/modules/PluginsPage.tsx` | 411 | 工坊及辅助插件界面与主题提取范围选择 |
| `src-ui/modules/ResourceDetail.tsx` | 348 | 原图和 Mod 对比，接入独立图片缩放组件 |
| `src-ui/modules/ResourcePage.tsx` | 368 | 紧凑资源筛选、主题预览尺寸与部位选择 |
| `src-ui/modules/SettingsPage.tsx` | 453 | 设置界面及主题选择器 |
| `src-ui/modules/TaskPopover.tsx` | 205 | 标题栏任务浮层、快照进度、结果与任务控制 |
| `src-ui/modules/WelcomeDialog.tsx` | 76 | 首次迁移检测和确认 |
| `src-ui/shared/Ui.tsx` | 112 | 所属模块的配置、领域模型或兼容辅助 |
| `src-ui/style.css` | 1322 | 所属模块的配置、领域模型或兼容辅助 |
| `tools/acceptance-report.py` | 70 | 构建、生成或隔离验收入口 |
| `tools/archive.py` | 13 | 构建、生成或隔离验收入口 |
| `tools/build-plugin.mjs` | 151 | 构建、生成或隔离验收入口 |
| `tools/build-portable.mjs` | 145 | 构建、生成或隔离验收入口 |
| `tools/cargo.mjs` | 100 | 构建、生成或隔离验收入口 |
| `tools/prepare-local-signing.mjs` | 42 | 构建、生成或隔离验收入口 |
| `tools/source-inventory.py` | 92 | 构建、生成或隔离验收入口 |
| `tools/sync-definitions.py` | 10 | 构建、生成或隔离验收入口 |
| `tools/sync-ipc.mjs` | 49 | 构建、生成或隔离验收入口 |
| `tools/third-party-notices.mjs` | 54 | 构建、生成或隔离验收入口 |
| `tools/verify-assets.py` | 69 | 构建、生成或隔离验收入口 |
| `tools/verify-migration.mjs` | 64 | 构建、生成或隔离验收入口 |
| `tools/verify-native.mjs` | 310 | 构建、生成或隔离验收入口 |
| `tools/verify-release.py` | 62 | 构建、生成或隔离验收入口 |
| `tools/verify-ui.mjs` | 113 | 构建、生成或隔离验收入口 |
| `tools/verify-update-app.mjs` | 64 | 构建、生成或隔离验收入口 |
| `tools/verify-updater.mjs` | 38 | 构建、生成或隔离验收入口 |
| `tsconfig.json` | 23 | 所属模块的配置、领域模型或兼容辅助 |
| `vite.config.ts` | 32 | 所属模块的配置、领域模型或兼容辅助 |

| `src-ui/core/domain/credits.ts` | 32 | 解析完整第三方声明及主页面主要鸣谢分组 |
| `src-ui/core/ipc/generated/PluginDirectories.ts` | 3 | ts-rs 生成的平台目录 DTO |
| `src-ui/interface.css` | 223 | 主题选择器、目录候选、紧凑关于页及图片缩放布局 |
| `src-ui/modules/AboutPage.tsx` | 169 | 紧凑鸣谢、项目引用和完整许可弹窗 |
| `src-ui/modules/ApxDirectories.tsx` | 138 | 按平台独立的目录显示、手动选择和候选确认 |
| `src-ui/modules/ApxPage.tsx` | 293 | APX 平台适配、安装、还原及导出界面 |
| `src-ui/modules/ResourceSearch.tsx` | 141 | 全局搜索和主题搜索模式选择 |
| `src-ui/shared/PanZoomImage.tsx` | 150 | 图片指针缩放、捕获拖动、复位及监听释放 |
| `src-ui/shared/ThemedSelect.tsx` | 66 | 统一的 Radix 主题选择器和键盘导航 |
| `src-tauri/src/apx_directories.rs` | 328 | 平台目录持久化、旧数据迁移、只读匹配和隔离测试 |
| `src-tauri/src/external_links.rs` | 28 | 校验 HTTPS 引用并打开系统浏览器 |
| `pnpm-lock.yaml` | 2930 | 生成的依赖解析锁文件 |
| `THIRD_PARTY_NOTICES.md` | 67615 | 生成的完整依赖许可与版权原文 |
| `tools/verify-about-apx-ui.mjs` | 395 | 关于、主题控件、平台隔离、APX 引导及图片交互验收 |
| `tools/verify-compact-ui.mjs` | 359 | 紧凑布局和任务进程浮层验收，适配 Radix 控件 |

## 超过 500 行的职责评审

以下迁入文件暂时保留兼容边界，不代表拆分已全部结束；对新责任优先提取独立模块。本次已把任务、工作台、资产、下载、迁移恢复、成组事务和便携更新从遗留入口拆出。未通过压缩行数规避评审。

- `src-tauri/src/apx.rs`（1636 行）：APX 平台限制、版本确认、缓存、安装和还原。保留理由：现有协议/测试或页面领域内耦合较高；后续按责任迁移，避免同时改写历史兼容行为。
- `src-tauri/src/legacy_commands.rs`（890 行）：所属模块的配置、领域模型或兼容辅助。保留理由：现有协议/测试或页面领域内耦合较高；后续按责任迁移，避免同时改写历史兼容行为。
- `src-tauri/src/legacy_tests.rs`（1043 行）：旧库、路径与包协议行为夹具。保留理由：现有协议/测试或页面领域内耦合较高；后续按责任迁移，避免同时改写历史兼容行为。
- `src-tauri/src/packages.rs`（2640 行）：历史包编解码、分卷、队列、导入导出事务。保留理由：现有协议/测试或页面领域内耦合较高；后续按责任迁移，避免同时改写历史兼容行为。
- `src-tauri/src/preview_engine.rs`（826 行）：所属模块的配置、领域模型或兼容辅助。保留理由：现有协议/测试或页面领域内耦合较高；后续按责任迁移，避免同时改写历史兼容行为。
- `src-tauri/src/resource_mutation.rs`（2169 行）：遗留资源导入、扫描协调、启禁和导出适配。保留理由：现有协议/测试或页面领域内耦合较高；后续按责任迁移，避免同时改写历史兼容行为。
- `src-tauri/src/storage.rs`（651 行）：Library 数据、设置和原子持久化。保留理由：现有协议/测试或页面领域内耦合较高；后续按责任迁移，避免同时改写历史兼容行为。
- `src-tauri/src/workshop.rs`（795 行）：工坊签名目录、安装及文件归属保护。保留理由：现有协议/测试或页面领域内耦合较高；后续按责任迁移，避免同时改写历史兼容行为。

## 0.4.1 增量职责

完整文件列表、行数与职责评审见 [CHANGESET-041.md](CHANGESET-041.md)。配色由 appearance.rs 和 useAppearance 管理；屏幕取色在 screen_picker.rs；安装身份、解包预检、闸门、交接与 MSI 事务分别位于 installation、update_package、update_gate、update_install、msi_update，沿用 portable_update 的文件恢复边界。
