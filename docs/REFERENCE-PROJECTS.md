# 参考项目

此表记录 APMM Next 实际借鉴、迁入或用于对照的项目。本地路径对应维护工作区的现有副本，其他机器按实际检出位置查找。

| 项目 | GitHub 与其他上游 | 本地参考地址 | 参考范围 | 依据与许可 |
| --- | --- | --- | --- | --- |
| NapCatQQ-Desktop | [GitHub](https://github.com/NapNeko/NapCatQQ-Desktop) | 源码 `D:\MOD\APMM-next\NapCatQQ-Desktop-main` | 设置分组、紧凑字段布局、APX 状态卡、检查后直接安装的更新流程及页面切换体验。WAAPI 过渡、打断续接和减少动画支持在 APMM 路由上实现。 | GPL-3.0。见 [NOTICE](../NOTICE.md)、[关于页](../src-ui/modules/AboutPage.tsx)、上游 `modules/settings/tabs/AboutTab.tsx` 及[过渡逻辑](../src-ui/hooks/usePageNavigation.ts)。 |
| JASM | [GitHub](https://github.com/Jorixon/JASM) | 源码 `D:\MOD\APMM-next\JASM-main` | 模组管理体验参考。 | GPL-3.0。现有参考记录见 [NOTICE](../NOTICE.md)和[关于页](../src-ui/modules/AboutPage.tsx)。 |
| AstralParty.Toys | [GitHub](https://github.com/higashitaniyume/AstralParty.Toys) | 源码 `D:\MOD\APMM-next\AstralParty.Toys-master`<br>发行副本 `D:\MOD\APMM-next\AstralParty.Toys` | 参考 `ReplayLibraryService.ReadMeta` 的内存与磁盘缓存、逐文件复用及失效重解析，用于启动扫描优化。 | AGPL-3.0。见[关于页](../src-ui/modules/AboutPage.tsx)和上游 [ReplayLibraryService.cs](https://github.com/higashitaniyume/AstralParty.Toys/blob/master/src/AstralParty.Toys/Services/Replay/ReplayLibraryService.cs)。 |
| WorkshopUploaderMod | [GitHub](https://github.com/BAKAOLC/STS2-WorkshopUploaderMod) | 源码 `D:\MOD\APMM-next\STS2-WorkshopUploaderMod-main` | 工坊投稿资料、预览图和提交状态参考。APMM 的投稿流程为本地导出、人工收稿审核和静态签名目录发布。 | AGPL-3.0-only。见[关于页](../src-ui/modules/AboutPage.tsx)和[工坊维护说明](../workshop-service/README.md)。 |
| 旧版 Astral Party Mod Manager | [历史 GitHub 发行入口](https://github.com/a190141074/AstralPartyModManager) · [CNB 发行与资源仓](https://cnb.cool/astral-weird-egg/astral-party-mod-manager-assets) | 源码 `D:\MOD\APMM\astral-tauri`<br>历史软件包及更新清单 `D:\MOD\APMM\astral-tauri\release-cnb` | 参考角色紧凑卡、原图定位、缺失预览缓存时的启动提醒及模组输出。迁入历史兼容代码、应用图标与吸猫动画，并保留旧库迁移能力。 | 历史 GitHub 地址来自旧源码 `config/cnb.json`。旧包目录用于对照历史发布结构。迁入内容见 [NOTICE](../NOTICE.md)、[关于页](../src-ui/modules/AboutPage.tsx)和[迁移指南](MIGRATION.md)。第三方内容沿用各自许可。 |
| apmm-util | 暂未确认公开 GitHub 仓库 | 独立工具 `D:\MOD\APMM-next\apmm-util` | 对照原图索引、完整资源名定位和按分类增量提取。当前应用的提取插件入口见 `plugins/asset-tool/`。 | 本地工具 README 记录命令及来源。工具依赖的许可与当前插件声明分别查阅。 |
| JiXingModHelper | [GitHub](https://github.com/kldsjfas/JiXingModHelper) | 源码 `D:\MOD\APMM-next\JiXingModHelper-main` | 候选对照。实际采用范围待确认。 | MIT。上游及许可见本地 README、LICENSE。 |

实际采用新项目的设计、行为或实现时，同轮在这里补上主页、本地路径、具体参考点和证据，并更新设置中的[关于页](../src-ui/modules/AboutPage.tsx)。同一项目的源码、发行副本等位置合并记录。浏览过但尚未采用的项目可以标为候选，等到确认采用后再加入关于页。迁入代码或资产前单独核对所用文件的来源和许可，随分发保留要求的版权及许可文本。
