# 0.4.1 验收

验收日期：2026-11-15。最终发行程序来自本版本 build-portable 与 build-distributions 的构建报告；所有执行测试使用 B 盘隔离副本，没有覆盖旧安装或写入实际游戏。

## 已通过

| 层级 | 范围与结果 |
|---|---|
| TypeScript | `pnpm typecheck` 通过。 |
| 前端纯逻辑 | appearance.test.ts 4 项通过：RGB/CMYK/HSV 转换、配色独立身份、非法输入、对比度。 |
| Rust | update 过滤的 8 项通过，111 项过滤；appearance 过滤的 4 项通过，115 项过滤，其中 3 项是 DTO 导出。保留既有未使用代码告警。 |
| 编译与发行 | Release 构建通过；标准便携 ZIP、MSI、0.3.36 桥接 ZIP、0.3.39 兼容 ZIP 均生成。逐文件、包体和 SHA-256 审计见 verify-distributions.py。 |
| 原生 WebView 与真实 IPC | 亮暗独立保存、保留跟随系统、取消草稿、整套重置不立即写入、导出/导入新身份、拒绝非法色值、连续设置、退出重启持久化、快速切页、六客户端键盘导航均通过。 |
| 窗口布局 | 800、945、1360 宽度设置分类保持单行横向排列，页面没有横向溢出。截图等待动画完成后捕获。 |
| 便携更新助手 | 成功更新、无法启动后回滚、中断恢复三条真实进程测试通过；嵌套许可、用户 data 与额外文件保留。 |
| 完整更新链 | 当前版本同版本隔离更新通过；真实 0.3.39 程序经兼容包升级到 0.4.1 后工作区确认就绪，数据和额外文件保留。 |
| 0.3.36 兼容 | 真实旧助手在隔离目录升级、启动新版并交接助手通过；不等同于旧版在线下载 UI 全程实测。 |
| MSI 内容 | Windows Installer 管理展开通过，产品名、版本、固定 UpgradeCode、安装身份和 9 个发行文件摘要匹配。 |
| 远端资源 | CNB 现有 preview-v3 的三个包回读摘要、ZIP 完整性与当前旧版预览清单逐文件匹配：角色 1371、卡面 157、怪物 191，共 1719 张。保留已有版本，不把相同图片伪称新资源。 |

## 兼容测试发现与修正

已经发行到本地的 0.3.39 二进制没有 installation.json、NOTICE.md 和 astral-updater-next.exe 白名单。直接传新版标准包或 0.3.36 桥接包会明确拒绝，当前应用不会退出。它需要单独的 compat-0.3.39.zip，由旧版自身的 runner 替换助手。0.3.36 的旧助手会跳过自身，因此仍需包含 next 助手的 legacy-bridge.zip。

两个旧版包采用平铺许可；额外归属说明合并到旧版允许的 THIRD_PARTY_NOTICES.md。缺少安装身份标记仍明确按便携版使用 data。新版用户只接收标准便携包或 MSI。公开 schema 1 清单面向此前已发布的 0.3.36，schema 2 按安装身份分发；0.3.39 兼容包是本地测试版的独立下载。

## 尚未验证

- 系统屏幕取色：自动化工具能读到原生窗口文字，但截图返回 `FrameArrived timed out`，输入返回 `coordinate input geometry is unavailable`。没有确认实际点击、Esc/右键取消、多屏或不同 DPI；不把 WebView 配色测试视为取色实测。
- 原生系统文件选择框：配色导出 IPC 与导入控件已测，系统对话框未实测。
- MSI 实际注册安装、升级、UAC 取消、安装失败回滚、卸载保留数据与重启后的普通权限未实测。本次仅管理展开，不把解包当成实际安装。产物未作 Authenticode 签名。
- 本次未新增实际游戏应用、原生外拖或 Linux 编译运行证据，也未进行性能测量。

最小人工复核：运行隔离便携版，打开设置配色，连续取色及取消，在不同缩放屏幕取样；在独立 Windows 测试机安装 MSI、取消一次更新提权、完成升级并重开，最后卸载确认 LocalAppData/APMM 保留。

## 复现入口

`tools/verify-appearance-native.mjs`、`tools/verify-update-app.mjs`、`tools/verify-updater.mjs`、`tools/verify-legacy-bridge.mjs`、`tools/verify-msi.ps1`、`tools/verify-distributions.py`。构建与测试记录在本地 verification-*.json；临时数据和日志位于 B:/APMM-next-work，按各报告的 output/stage 定位。

全部受影响文件和职责见 [CHANGESET-041.md](CHANGESET-041.md)，参考取舍见 [REFERENCE-REVIEW-041.md](REFERENCE-REVIEW-041.md)，安装与恢复见 [UPDATE-041.md](UPDATE-041.md)。
