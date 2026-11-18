# 项目 skill 与工作流文档

本目录是 APMM Next 工作流 Markdown 和 `apmm-next-modding` 的集中维护入口。Codex 用户级 skill 由这里同步安装；工程已有详细文档保留原址，下面按任务引用唯一正文。不要从聊天记录重建一套平行规范。

| 要做的工作 | 先读哪里 |
| --- | --- |
| 新会话接手、继续任务 | [SKILL.md](apmm-next-modding/SKILL.md)、[开发指南](../CONTRIBUTING.md)、本地 `reports/` 中对应任务 |
| 技术框架、职责与文件定位 | [架构](../docs/ARCHITECTURE.md)、[源码导航](../docs/FILE-MAP.md) |
| 资源库、APX、原图与界面约定 | [界面与数据补充](apmm-next-modding/references/ui-and-data.md)、[迁移兼容](../docs/MIGRATION.md) |
| 开发、Codex 预览、定向验收 | [开发指南](../CONTRIBUTING.md)、[验证补充](apmm-next-modding/references/workflows.md) |
| 推送、CNB/GitHub 令牌、新版本结构与发布 | [发行规范](../docs/RELEASING.md)、[发行补充](apmm-next-modding/references/releases-and-preview.md) |
| 覆盖更新、助手、启动和增量扫描 | [更新协议](../docs/UPDATING.md)、[更新与启动补充](apmm-next-modding/references/updates-and-startup.md) |
| 参考了哪些项目 | [参考项目表](../docs/REFERENCE-PROJECTS.md)：GitHub/上游链接、本地位置、参考范围、许可与状态 |
| 每次完成后的归档 | [任务报告规则](../CONTRIBUTING.md#任务报告)、[报告模板](apmm-next-modding/references/workflows.md#任务报告与接续) |

## 维护与同步

1. 新规范先修改本仓的唯一正文。工程指南更新 `CONTRIBUTING.md` 或 `docs/`；Agent 操作补充更新 `skill/apmm-next-modding/`；入口改变时同步上面的索引、`AGENTS.md` 和 skill 路由。
2. 从应用根执行 `pwsh -File tools/sync-apmm-skill.ps1`，然后执行同命令加 `-Check`。默认只安装本项目的 `apmm-next-modding`，目标为 `$CODEX_HOME/skills`，未设置时使用用户 `.codex/skills`。
3. 脚本逐文件按 SHA-256 比较，只新增或更新来源已有文件。安装目标出现额外文件时保留并报告；先核对用途，不用目录镜像删除。安装副本只占少量文本和脚本空间，大产物与缓存仍在 B 盘。
4. 在任务报告记录同步结果。新会话读取项目维护版并执行 `-Check`；缺失或落后时用当前源码安装，不能根据旧安装副本回退文档。新机器先确认应用根目录和工具链路径。

同步关系为：项目 `skill/apmm-next-modding/` → Codex 用户级 `skills/apmm-next-modding/`。详细工程文档通过项目路径读取，不复制整个项目到 Codex 安装目录。
