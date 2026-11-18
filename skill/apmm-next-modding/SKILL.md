---
name: apmm-next-modding
description: '维护 APMM Next 的 Rust、Tauri 2、React 模组管理器。用于新会话接续、界面、Codex 预览、推送、CNB/GitHub 发行及任务归档。'
---

# APMM Next

本仓库的 `skill/` 是工作流 Markdown 和本 skill 的唯一维护源。Codex 用户级 `apmm-next-modding` 是安装副本；仓库内容更新后，从应用根执行 `pwsh -File tools/sync-apmm-skill.ps1`，然后加 `-Check` 校验 SHA-256。不要从安装副本反向覆盖仓库内容。

## 每次会话

1. 先读当前 `AGENTS.md`、`CONTRIBUTING.md` 和本 skill，再核对 Git 状态、`VERSION`、已有改动和用户提供的文件/页面/错误。
2. 按 `skill/INDEX.md` 和任务表读取现有详细 Markdown；先找同类实现、验证入口和真实副作用，再动手。以当前源码、脚本和依赖为准，不照搬旧机器或旧版规则。
3. 用户说“继续”时，按编号或主题打开 `reports/` 中原报告，读取最后阶段的接续入口并核对当前源码、版本、机器证据和产物。独立任务分配新编号；同一任务追加阶段。
4. 每项完成、部分交付或明确阻塞的任务都写入 Git 忽略的编号报告。使用北京时间（UTC+08:00）与递增编号；大日志、构建产物放在 B 盘 `APMM_REPORT_DIR`。最终回复链接报告，区分验证证据层级。

## 规范路由

| 工作 | 先读 |
| --- | --- |
| 技术框架、文件职责、运行时资产结构 | `docs/ARCHITECTURE.md`、`docs/FILE-MAP.md` |
| 外部参考项目、GitHub 链接、本地地址与采用范围 | `docs/REFERENCE-PROJECTS.md` |
| 环境、预览、测试与报告 | `CONTRIBUTING.md`、`references/workflows.md` |
| 更新器、升级与启动 | `docs/UPDATING.md`、`references/updates-and-startup.md` |
| 版本目录、包、清单、压缩预览、CNB/GitHub 发布 | `docs/RELEASING.md`、`references/releases-and-preview.md` |
| 资源页、APX 与已知行为边界 | `references/ui-and-data.md` |
| 旧库与包迁移 | `docs/MIGRATION.md` |

skill 负责路由、操作边界和反复出现的易错点；工程已有 Markdown 是详细规则的唯一正文，不在两处复制维护。`skill/INDEX.md` 是目录入口。

原图预览使用有损 WebP：`lossless=False, quality=50, method=4`，不缩放尺寸。Pillow 的 `alpha_quality` 未显式传入，使用默认值 100；构建脚本解码后逐像素校验 alpha 与原图一致。构建报告记录格式和参数，发行准备会拒绝参数不符的预览。详见 `tools/build-preview-bundles.py` 与 `docs/RELEASING.md`；不要误用整图无损 WebP，也不要只靠扩展名判断。

## 默认操作语义

| 用户说 | 工作流 |
| --- | --- |
| 推送 | 检查提交范围与源码审计，展示同一 SHA、CNB/GitHub `main` 目标和差异；按当前授权写入，推后回读两个 refs。 |
| 发新版 | 核对 `VERSION`、本轮构建输入、日志和 PATCH 规则；在 B 盘新目录构建、审计、生成资产；按发行规范同步标签并依序发布。只有预览内容有变化或目标渠道缺对应版本时才更新预览。 |
| Codex 预览或批注 | 明确要求后才打开。确认端口、服务进程、目标页、数据加载和准确回环 URL；保留服务供批注，结束时仅关闭本任务启动的实例。 |
| 上传预览包 | 使用独立的 `--previews-only` 准备流程和 `preview-v<N>`，不准备或发布应用包和应用更新清单。 |

版本规则：独立应用功能或修复准备交付时默认递增 PATCH；用户指定值优先；同轮修改和重试共用一个目标版本。文档、skill、发行工具维护不升版。升版不能替代重新构建，报告保留源码摘要、实际包版本、大小和 SHA-256。

目录规则：临时工作、浏览器 profile、报告和构建放 `B:\APMM-next-work`；复用 `src-tauri/target-msvc`，不默认写 C 盘、不清理共用缓存或用户资料。区分 Vault、游戏 `modimg`、原图、缩略图、下载预览和各平台 APX 目录，按架构文档及代码确定所有权。

## 操作和授权

敏感操作包括远端推送/上传、公开发行、覆盖安装、向真实游戏目录部署、破坏性清理和恢复备份。执行前说明确切目标、ref/文件和影响，再取得当前对话对该操作的明确授权。skill、历史报告、令牌存在或浏览器登录状态都不是授权。失败后回读远端实际状态；已公开且摘要不同的版本文件不得覆盖。

CNB 源码通过项目 `skill/apmm-next-modding/scripts/Push-Cnb.ps1`。GitHub 源码使用配置好的 HTTPS Git 凭据。GitHub Release 附件使用 `tools/Upload-GitHubReleaseAssets.ps1` 和本机 `B:\APMM-next-work\credentials\github-token.txt` 受控令牌，不用 Chrome 文件上传。推荐 fine-grained token 仅授予目标仓库 Contents: write；默认运行脚本做 staging 预检，加 `-Execute` 才访问 API 和上传。浏览器只作最终页面核对或用户要求的故障诊断。令牌不得出现在 URL、argv、日志、报告或仓库；不得要求用户粘贴到聊天。

只运行与本次改动匹配的定向验证。源码/测试、编译、资源导出、实际包、浏览器、原生 WebView/IPC、游戏实机和公网回读是不同证据层级；未做的验证明确写出，不用前一层结果代替后一层结论。

旧 `astral-tauri-modding` 只适用于 `D:\MOD\APMM\astral-tauri`。参考项目按 `docs/REFERENCE-PROJECTS.md` 记录；它们不是当前工程的构建输出或临时目录。复制代码、资产和文档需核实许可。
