# 0.3.39 受影响文件与职责

对比只读源码快照 `B:\APMM-next-work\snapshots\management-0.3.38-20261115-021320`。没有 Git 提交，以下为全部手写代码、生成契约和配置文档差异；构建缓存及验收 JSON 不作为源码。

| 文件 | 变更 | 行数 | 职责 |
|---|---|---:|---|
| `CHANGELOG.md` | modified | 172 | 本轮开发、迁移、变更或验收文档 |
| `CONTRIBUTING.md` | modified | 75 | 本轮开发、迁移、变更或验收文档 |
| `VERSION` | modified | 1 | 版本与依赖锁同步 |
| `docs/ACCEPTANCE-MANAGEMENT-0.3.39.md` | added | 117 | 本轮开发、迁移、变更或验收文档 |
| `docs/CHANGED-MANAGEMENT-0.3.39.md` | added | 94 | 本轮开发、迁移、变更或验收文档 |
| `docs/FILE-MAP.md` | modified | 235 | 本轮开发、迁移、变更或验收文档 |
| `docs/MIGRATION.md` | modified | 29 | 本轮开发、迁移、变更或验收文档 |
| `package.json` | modified | 46 | 版本与依赖锁同步 |
| `src-tauri/Cargo.lock` | modified | 5238 | 版本与依赖锁同步 |
| `src-tauri/Cargo.toml` | modified | 60 | 版本与依赖锁同步 |
| `src-tauri/src/app.rs` | modified | 124 | 应用监听生命周期、图片修订协议与目录读取边界 |
| `src-tauri/src/candidate_management.rs` | added | 452 | 候选信息编辑、归属合并和跨盘文件回滚 |
| `src-tauri/src/directory_files.rs` | added | 71 | 递归 PNG 枚举、禁用后缀与持久文件标识 |
| `src-tauri/src/directory_monitor.rs` | added | 286 | 目录通知、稳定等待、任务互斥和退出取消 |
| `src-tauri/src/directory_sync.rs` | added | 210 | 被动游戏读取、实际应用计算与外部源绑定 |
| `src-tauri/src/dto.rs` | modified | 133 | 工作区候选信息与监听设置 DTO |
| `src-tauri/src/image_import.rs` | modified | 187 | 完整 PNG 校验、逐项归属预览和入库事务 |
| `src-tauri/src/legacy_commands.rs` | modified | 836 | 旧 PNG 导入入口委托新流程 |
| `src-tauri/src/legacy_tests.rs` | modified | 1072 | 历史包构造兼容和真实 PNG 迁移夹具 |
| `src-tauri/src/lib.rs` | modified | 237 | 新领域模块、IPC 命令及测试注册 |
| `src-tauri/src/management_tests.rs` | added | 326 | 预设兼容、局部原版、问题聚合及事务回滚测试 |
| `src-tauri/src/management_types.rs` | added | 178 | 候选信息、来源、归属确认和预设持久契约 |
| `src-tauri/src/originals.rs` | modified | 133 | 已连接游戏目录只读预览和禁用 PNG 读取 |
| `src-tauri/src/packages.rs` | modified | 2732 | 可选来源和手动身份分享、重复补空及 PNG 载荷校验 |
| `src-tauri/src/preset_domain.rs` | added | 303 | 固定内容与范围捕获、问题预览和整批事务 |
| `src-tauri/src/presets.rs` | modified | 122 | 预设管理 IPC 和旧预设命令兼容入口 |
| `src-tauri/src/resource_mutation.rs` | modified | 1880 | 扫描和临时迁移委托新领域，保留维护入口 |
| `src-tauri/src/settings_commands.rs` | modified | 92 | 监听开关和既有设置兼容入口 |
| `src-tauri/src/storage.rs` | modified | 649 | Library 新字段、默认监听和旧数据加载 |
| `src-tauri/src/sync_management_tests.rs` | added | 252 | 外部扫描、独立信息、重命名和启禁同步测试 |
| `src-tauri/src/task_dispatch.rs` | modified | 166 | 后台 PNG 导入和被动扫描协调 |
| `src-tauri/src/vault_sync.rs` | added | 178 | 库候选编辑和重命名关联、规则归类及冲突提示 |
| `src-tauri/src/workshop.rs` | modified | 797 | 既有候选构造适配兼容默认字段 |
| `src-tauri/src/workspace.rs` | modified | 150 | 资源快照、独立候选信息和实际状态展示 |
| `src-tauri/tauri.conf.json` | modified | 37 | 版本同步和原生窗口最小宽度 |
| `src-ui/app/App.tsx` | modified | 497 | 独立预设导航和路由 |
| `src-ui/core/domain/candidateInfo.ts` | added | 28 | 兼容信息默认字段、隐藏图片后缀与安全导出命名 |
| `src-ui/core/domain/importClassification.ts` | added | 32 | 导入目标预览与目标槽位重复判断 |
| `src-ui/core/domain/management.test.ts` | added | 111 | 名称搜索、归属预览和实际分类映射测试 |
| `src-ui/core/domain/resources.ts` | modified | 240 | 实际分类槽位映射、名称容错和稳定搜索排序 |
| `src-ui/core/ipc/commands.ts` | modified | 106 | sync-ipc 生成的新管理命令与事件常量 |
| `src-ui/core/ipc/generated/CandidateInfo.ts` | added | 4 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/ClassificationRecord.ts` | added | 8 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/DirectorySyncStatus.ts` | added | 3 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/GlobalPreset.ts` | added | 4 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/ModFile.ts` | modified | 5 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/PresetChange.ts` | added | 4 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/PresetItem.ts` | added | 7 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/PresetPreview.ts` | added | 5 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/Provenance.ts` | added | 3 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/ResourceSlot.ts` | added | 3 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/generated/WorkspaceSettings.ts` | modified | 4 | ts-rs 生成的管理 DTO |
| `src-ui/core/ipc/mock.ts` | modified | 90 | 开发浏览器管理入口与版本适配 |
| `src-ui/core/ipc/mockManagement.ts` | added | 139 | 浏览器内存预设夹具，生产仍使用 Rust |
| `src-ui/core/ipc/transport.ts` | modified | 64 | 图片 URL 的哈希修订参数 |
| `src-ui/core/services/app.ts` | modified | 62 | 目录变化订阅和现有服务整合 |
| `src-ui/core/services/management.ts` | added | 19 | 预设管理和候选管理 IPC 服务 |
| `src-ui/hooks/useApp.ts` | modified | 122 | 目录事件刷新工作区和预设详情快照 |
| `src-ui/hooks/useManagement.ts` | added | 14 | 预设详情查询与命令服务 hooks |
| `src-ui/main.tsx` | modified | 19 | 加载模组信息的独立样式职责 |
| `src-ui/management-ui.css` | added | 184 | 模组信息左右分栏、紧凑字段和窄窗堆叠 |
| `src-ui/modules/AboutPage.tsx` | modified | 188 | 半宽参考项目与工具鸣谢、JASM/APX/Codex 链接 |
| `src-ui/modules/CreatePresetDialog.tsx` | added | 149 | 全局、角色全变体和固定部位创建范围 |
| `src-ui/modules/PackageDialog.tsx` | modified | 488 | 逐项归属传输与导入任务反馈 |
| `src-ui/modules/PackageSelection.tsx` | modified | 162 | PNG 逐项目标选择和错误项禁用 |
| `src-ui/modules/PresetsPage.tsx` | added | 408 | 预设详情、修复、锁定、范围编辑和应用清单 |
| `src-ui/modules/ResourceDetail.tsx` | modified | 357 | 候选和放大图片的内容修订刷新 |
| `src-ui/modules/ResourceFileActions.tsx` | modified | 305 | 模组信息、来源外链和归属冲突确认 |
| `src-ui/modules/ResourcePage.tsx` | modified | 371 | 候选缩略图的内容修订刷新 |
| `src-ui/modules/ResourceSearch.tsx` | modified | 141 | 相似标记、稳定结果与键盘定位 |
| `src-ui/modules/SettingsPage.tsx` | modified | 427 | 目录监听开关与预设页导航 |
| `src-ui/shared/CandidateComparison.tsx` | added | 100 | 模组信息左侧对比和独立缩放、平移、复位及放大 |
| `src-ui/shared/CandidateCreditsFields.tsx` | added | 177 | 作者、可选画师与各自多链接的紧凑编辑 |
| `src-ui/shared/PanZoomImage.tsx` | modified | 157 | 修订换图时重置缩放并释放监听 |
| `src-ui/shared/ResourceSlotSelect.tsx` | added | 108 | 分类、对象、变体、部位四级选择 |
| `src-ui/shared/Ui.tsx` | modified | 129 | 图片修订请求和失败状态刷新 |
| `src-ui/style.css` | modified | 1427 | 预设、信息、归属布局及 800 宽窗口支持 |
| `tools/management-about-ui.mjs` | added | 65 | 关于页同宽布局、链接和窄深主题验收 |
| `tools/management-fixtures.mjs` | added | 32 | 大小固定、完整 CRC 的真实 PNG 夹具 |
| `tools/management-image-refresh.mjs` | added | 82 | 同路径时间戳不变编辑的 WebView 像素验收 |
| `tools/management-mod-info-ui.mjs` | added | 125 | 对比缩放、平移、复位、Enter 改名和作者画师多链接界面验收 |
| `tools/management-native-temporary.mjs` | added | 68 | 正式外部临时启禁、只读预览和本机选择保持验收 |
| `tools/management-native-ui.mjs` | added | 151 | 正式管理界面与真实 PNG IPC 的导入向导验收 |
| `tools/management-source-inventory.py` | added | 114 | 本轮快照对比、变更职责和行数审计 |
| `tools/sync-ipc.mjs` | modified | 50 | 生成目录变化事件常量 |
| `tools/verify-management-native.mjs` | added | 659 | B 盘发行 EXE、目录和事务整体验收编排 |
| `tools/verify-management-release.py` | added | 45 | 本轮 ZIP 白名单、摘要、版本与原生副本一致性审计 |
| `tools/verify-management-ui.mjs` | added | 184 | B 盘浏览器管理、搜索和关于页交互验收 |
