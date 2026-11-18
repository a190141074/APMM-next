# 源码导航

本页维护稳定的职责入口。逐次修改清单、测试日志和源码摘要保存到本地工作盘，不纳入公开文档。

| 目录                  | 职责                                            |
| --------------------- | ----------------------------------------------- |
| `src-ui/`             | React 界面、主题、交互和查询快照                |
| `src-tauri/`          | Rust 后端、Tauri 配置和独立更新助手             |
| `config/`             | 资源目录、完整名称、分类、下载来源和公钥        |
| `public/`             | 随界面构建的静态资源                            |
| `plugins/asset-tool/` | 可选的本地游戏资产提取工具                      |
| `workshop-service/`   | 本地投稿审核和静态目录生成，HTTP 服务为可选能力 |
| `tools/`              | 类型生成、构建、打包和隔离验收入口              |
| `vendor/`             | 固定的第三方源码、运行库及许可                  |
| `docs/`               | 使用、迁移、架构、发行说明和界面截图            |

## 界面

`src-ui/app/App.tsx` 编排导航和页面生命周期；`modules/` 放置页面与直接子组件；`hooks/` 处理查询和订阅；`core/services/` 封装 IPC；`core/domain/` 保存纯业务规则；`core/ipc/generated/` 由 Rust 生成。

总览入口为 `OverviewPage.tsx`；资源页面由 `ResourcePage.tsx` 编排，`ResourceList.tsx` 展开候选，`ResourceGroupCard.tsx` 展示完整实体的大卡片与角色横向紧凑卡，`core/domain/resourcePresentation.ts` 统一四档尺寸；工坊位于 `WorkshopPage.tsx` 与 `modules/workshop/`；作者模式位于 `AuthorPage.tsx` 与 `modules/author/`。关于与作者开关在设置中。

## 后端

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
