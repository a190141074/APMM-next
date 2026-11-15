# 参考工程复核

本次读取工作区中的 NapCatQQ-Desktop-main 和 JASM-main，保持参考工程只读。

| 来源 | 借鉴内容 | 0.4.1 的处理 |
|---|---|---|
| NapCat `modules/settings/shared/sections.tsx`、`SettingsPage.next.tsx` | 横向标签、标题层级、左侧细引导线、平铺行与右侧控件 | 设置使用横向分区，外观拆为界面与动效，细分隔线和紧凑开关 |
| NapCat `shared/ui/motion/PageTransition.tsx` | WAAPI 退出后换页、反向时承接当前帧、取消旧回调 | 75ms 退出、165ms 进入、6px 横移；快速点击以最后目标为准，遵循系统与应用减少动画 |
| NapCat 主机选择 | 横向分段客户端选择 | APX 保持既有渠道与平台隔离，删除重复 WINDOWS/ANDROID 副标题 |
| JASM `JASM.AutoUpdater/MainPageVM.cs` | 独立更新流程、阶段日志、取消反馈 | 保留独立助手和阶段消息，以校验与真实进程握手推进；不加入参考流程中的固定一秒停顿 |
| JASM `SkinManagerService.cs` 与冷启动 benchmark | 按领域管理资源、针对实际启动场景测量 | 保留 APMM 原有资源身份、事务和缩略图边界；未测量前不扩大并发，也不声称性能提升 |

适合后续单独测量的方向：大型库首次快照耗时、图片解码/缩略图队列、快速切换到重页面时的主线程占用。没有引入新的动画库、发光背景或高幅度缩放。当前语义配色统一背景、文字、边界、强调和四类状态，使后续模块复用同一组颜色角色。

项目按用户确认改用 GPL-3.0-only，保留 NapCat、JASM、旧 APMM 与独立依赖的归属说明。
