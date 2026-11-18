# 源码导航

本页维护稳定的职责入口。人工任务报告归档在已忽略的 `reports/`。机器生成的构建报告、验收日志和源码摘要保留在工作盘，由 `APMM_REPORT_DIR` 等现有配置指定。

| 目录                  | 职责                                            |
| --------------------- | ----------------------------------------------- |
| `src-ui/`             | React 界面、主题、交互和查询快照                |
| `src-tauri/`          | Rust 后端、Tauri 配置和独立更新助手             |
| `config/`             | 资源目录、完整名称、分类、下载来源和公钥        |
| `public/`             | 随界面构建的静态资源，例如 `logo.svg`          |
| `src-ui/assets/`、`src-tauri/icons/` | 界面所用资源与原生程序图标 |
| `plugins/asset-tool/` | 可选的本地游戏资产提取工具                      |
| `workshop-service/`   | 本地投稿审核和静态目录生成，HTTP 服务为可选能力 |
| `tools/`              | 类型生成、构建、打包和隔离验收入口              |
| `vendor/`             | 固定的第三方源码、运行库及许可                  |
| `docs/`               | 使用、迁移、架构、发行说明和界面截图            |
| `skill/`              | 工作流 Markdown 索引、项目 skill 维护源与操作补充；同步安装到 Codex |
| `reports/`            | 被 Git 忽略的编号任务报告，不上传源码仓库       |

技术栈、状态所有权和运行时资源位置见[架构与不变量](ARCHITECTURE.md)。上游链接、本地参考副本及采用范围见[参考项目](REFERENCE-PROJECTS.md)。

## 界面

`src-ui/app/App.tsx` 编排导航和页面生命周期；`modules/` 放置页面与直接子组件；`hooks/` 处理查询和订阅；`core/services/` 封装 IPC；`core/domain/` 保存纯业务规则；`core/ipc/generated/` 由 Rust 生成。

总览入口为 `OverviewPage.tsx`；资源页面由 `ResourcePage.tsx` 编排，`ResourceList.tsx` 展开候选，`ResourceCandidateRow.tsx` 展示原版、候选及 outimg 定位，`ResourceGroupCard.tsx` 展示完整实体的大卡片与角色横向紧凑卡，`core/domain/resourcePresentation.ts` 统一六档尺寸与偏好读取，`ResourceSizeControls.tsx` 共用尺寸面板和统一开关；工坊位于 `WorkshopPage.tsx` 与 `modules/workshop/`；作者模式位于 `AuthorPage.tsx` 与 `modules/author/`。关于与作者开关在设置中。

软件更新由 `UpdateSettings.tsx` 展示检查、安装入口和进度，`hooks/useStartup.ts` 管理默认开启的定时检查与持续提醒，`core/services/softwareUpdate.ts` 串联后台下载终态、收据版本复核与既有安装命令。`modules/PreviewDownloadPrompt.tsx` 读取本地缓存状态并提供缺少预览图时的启动提示，开关由工作区设置持久化。`tools/verify-preview-prompt-ui.mjs` 使用隔离浏览器资料验收下载提醒与 outimg 定位，不执行真实下载。`tools/verify-six-comments-ui.mjs` 验收六项界面批注，不执行真实安装。

## 后端

旧库完整复制由 `migration.rs` 编排；非空库候选补齐由 `migration_merge.rs` 处理，逐槽位保留同名多候选和当前应用；两者共用 `migration_journal.rs` 的提交与恢复。`migration_candidate_tests.rs` 验证候选保留、取消、失败恢复，以及真实旧库在隔离目录中的迁移与重新扫描。

更新分钟间隔的读取兼容位于 `storage.rs`，保存入口为 `settings_commands.rs`，展示契约为 `dto.rs`；`update_interval_tests.rs` 验证默认值、旧小时转换及范围。前端 `core/domain/updateSchedule.ts` 计算剩余等待时间，`hooks/useStartup.ts` 管理定时器取消及失败节流，`modules/UpdateSettings.tsx` 提供分钟输入。已有 `storage.rs` 超过 500 行，本轮仅扩展其设置兼容职责，测试放在独立文件。

拖入由 `hooks/useResourceDrop.ts` 管理事件、命中与异步反馈，`core/domain/dropImport.ts` 处理纯分流规则，未知 PNG 由 `modules/DropImportDialog.tsx` 选择归属。后端 `image_drop.rs` 提供只读名称匹配和首个候选应用，`image_import.rs` 保留校验与入库提交；普通文件选择器不启用拖入策略。

| 模块                                                                    | 职责                                  |
| ----------------------------------------------------------------------- | ------------------------------------- |
| `app.rs`、`lib.rs`、`dto.rs`                                            | 应用生命周期、命令注册和生成契约      |
| `storage.rs`、`workspace.rs`、`workspace_cache.rs`                      | 权威资源库、展示快照和可重建缓存      |
| `directory_observation.rs`、`directory_scan.rs`、`file_verification.rs` | 锁外文件观察、提交复核和 PNG 校验缓存 |
| `directory_watch.rs`、`directory_monitor.rs`                            | 路径通知、事件合并、监听取消与回退    |
| `vault_sync.rs`、`directory_sync.rs`                                    | Vault 与游戏目录各自的同步语义        |
| `originals.rs`、`original_cache.rs`、`thumbnails.rs`                    | 原图来源、后台校验和独立缩略图        |
| `resource_transaction.rs`、`migration*.rs`                              | 资源事务、备份回滚与旧库迁移          |
| `packages.rs`、`presets.rs`、`apx.rs`                                   | 包、预设和 APX 的独立兼容行为         |
| `author/`、`workshop/`                                                  | 离线草稿投稿、签名目录、预检与安装    |
| `preview_engine.rs`、`preview_install.rs`                               | 预览清单、来源选择、安全解包与安装    |
| `updates.rs`、`update_channel.rs`、`portable_update.rs`                 | 更新选择、收据、白名单和恢复          |

已有较大的 `packages.rs`、`resource_mutation.rs`、`preview_engine.rs` 保留历史协议边界；修改时只接入所需适配入口。新的文件校验、目录观察和工坊流程各自独立，避免继续扩充兼容模块。逐文件行数可运行 `tools/source-inventory.py` 输出到本地报告目录。
