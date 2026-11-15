# 0.3.39 管理体验验收

验收日期：2026-11-15。四阶段功能与后续界面意见均已实现；本报告分别记录源码测试、浏览器、编译、发行 WebView/IPC 和实际产物证据。

## 交付内容

- 预设页：全局搭配、指定角色全变体、固定局部部位，创建、详情、编辑与修复、复制、锁定、删除、应用清单和整批事务。
- 模组信息：左侧官方原图与具体候选对比，独立滚轮缩放、拖动、双击或按钮复位及放大；右侧紧凑编辑，哈希与路径折叠，名称隐藏后缀并支持 Enter 自由保存，操作按钮放在名称旁。
- 候选信息与分享：作者、作者多链接、可选画师、画师多链接、HTTPS 来源、版本和本机备注。可分享字段随 apmod 传递，备注留在本机，重复导入优先保留本机非空信息。
- 导入与归类：完整校验 PNG，未知有效图片进入临时，导入逐项选槽位；库候选事务归类和合并，保留名称与信息，规则更新支持自动归类与手动冲突确认。
- 目录监听与搜索：默认监听游戏 Mod 目录及 Vault，等待写入稳定后被动同步，切换路径和退出释放监听；名称与昵称容错、直接匹配优先、稳定排序和键盘定位。
- 关于页：参考项目和工具同宽并排，加入 JASM；APX 与 Codex 放入工具鸣谢，窄窗口堆叠。

预设保存内容不随外部候选编辑、删除或归类改写。局部范围是保存的固定槽位集合；原版通过清除替换文件恢复。候选显示名称与物理 Vault 路径分离，信息编辑不改 PNG 或哈希。Rust 管理持久状态，React Query 只保存展示快照。

## 产物与摘要

交付包：[APMM-0.3.39-windows-x86_64-portable.zip](B:/APMM-next-work/portable-build/063b9e6a-a13a-458d-adbb-3b8a82d27484/APMM-0.3.39-windows-x86_64-portable.zip)。源码工程的 `dist` 中也保留同摘要副本。

| 项目 | 已核对的值 |
|---|---|
| 版本 | `0.3.39` |
| ZIP 字节数 | `5021039` |
| ZIP SHA-256 | `51f6e39c613e0bfcb194add1cb128936d022c711fdead90184fed349ae086993` |
| 主 EXE SHA-256 | `4484aaa6512a10209952a9fd35816670d61b699d017744b2559aa04fbcbd3ed4` |
| 构建与前端暂存 | `B:\APMM-next-work\portable-build\063b9e6a-a13a-458d-adbb-3b8a82d27484` |
| Cargo | 复用 `src-tauri/target-msvc` |
| 正式原生夹具 | `B:\APMM-next-work\management-native\759193e4-1679-4717-8fe7-7c63286b5eb3` |
| 浏览器夹具 | `B:\APMM-next-work\management-ui\096eb863-904b-40c4-b886-1e32bd9938b8` |

`VERSION`、package.json、Cargo.toml、Cargo.lock 和 tauri.conf.json 已同步。发行审计核对 ZIP 的 8 项白名单、manifest 每项字节数和摘要、B 盘交付副本及原生验收副本，确认主 EXE 一致。正式前端嵌入 EXE，依赖记录包含构建后的 index.html。包内不携带测试库、游戏图片或浏览器 profile。

机器报告：[构建](../verification-portable-build.json)、[发行审计](../verification-management-release.json)、[原生](../verification-management-native.json)、[浏览器](../verification-management-ui.json)、[源码职责和摘要](../verification-management-source.json)。

## 定向测试与编译

| 检查 | 实际结果 | B 盘日志 |
|---|---|---|
| `pnpm typecheck` | 通过 | `management-typecheck.log` |
| management.test.ts 与 domain.test.ts | 18 通过，2 个文件 | `management-front-tests.log` |
| `test --lib management_` | 20 通过，90 过滤；包含新 DTO 导出 | `management-tests.log` |
| `test --lib package_` | 6 通过，104 过滤 | `management-package-tests.log` |
| `test --lib v3_tests` | 5 通过，1 忽略，104 过滤 | `management-v3-tests.log` |
| `node tools/sync-ipc.mjs` | 通过，命令常量由脚本生成 | 无错误输出 |
| `node tools/cargo.mjs check` | 通过，有遗留未使用项警告 | `management-check.log` |
| `pnpm build:portable` | release 成功，3 分 08 秒 | `management-build.log` |
| `python tools/verify-management-release.py` | 全部通过，验收 EXE 与 ZIP 一致 | 发行审计 JSON |

以上日志均位于 `B:\APMM-next-work`。未启动无关的全仓测试。Rust 管理测试覆盖旧预设分类容器兼容、固定局部原版与禁用捕获、问题聚合、锁定与复制、候选元信息及自由名称、归属合并、跨目录同内容独立重命名、规则确认、被动扫描和事务失败回滚。包测试覆盖旧格式、分卷、损坏和密码校验。

忽略项为 `real_10007_pngs_survive_encrypted_roundtrip_and_legacy_path_allocation`，需要另行提供 `ASTRAL_TEST_10007_DIR` 的历史三图夹具。正式原生验收另用完整 CRC 的真实 PNG 完成加密包往返和来源信息校验，但不将其描述为该历史夹具测试已执行。

## 浏览器证据

浏览器报告 `allPassed=true`，6 项检查通过：预设创建及固定范围、锁定和未锁定副本、最新模组信息对比操作与多链接编辑、四级归属确认、窄窗口深色搜索与键盘定位、关于页同宽布局和链接。

该层使用受限内存工作区夹具，不作为 Rust 文件事务证据。独立 Vite 使用 1439 端口，profile 存 B 盘；用户原有 1420 页面未被关闭或替换。

## 正式 WebView/IPC 证据

使用 ZIP 的正式 EXE 副本，断言 dataDir 位于本轮 B 盘 app；游戏目录、输入、Vault、失败占位和 WebView profile 均隔离。最终报告 `allPassed=true`，17 项检查通过，页面错误为空，程序正常退出码为 0。

1. 正式 EXE 与真实 IPC 启动。
2. 未知有效 PNG、伪装或损坏 PNG 拒绝、逐项归属、同槽位及跨槽位信息隔离，信息编辑不改哈希。
3. 外部临时 PNG 的 disabled 后缀同步、禁用图片只读预览、重复扫描保留本机启禁选择。
4. apmod 作者、画师和多链接往返，备注不分享，重复导入保留本机非空信息并补空。
5. 全局与固定局部捕获、禁用保存原版、范围外不变、锁定校验及未锁定副本。
6. 正式全局预设整套应用，未保存替换槽位恢复原版。
7. 同大小且恢复旧时间戳的修改重算哈希，已知库候选编辑继承信息，预设旧哈希保留，原子保存不凭槽位继承旧作者，修复后整批应用。
8. 库文件重命名保留信息、禁用临时保留、事务归类、重复扫描无临时副本、手动规则冲突保留或采用及同规则不重复提示。
9. 路径切换释放旧监听，新目录新增及删除、设置关闭和重新启用，自身事件最终安静。
10. 用本轮 `library.json.part` 目录占位模拟提交失败，核对游戏文件实际字节回滚。
11. 左侧独立缩放、平移、双击或按钮复位、放大；右侧紧凑编辑、文件详情折叠、Enter 保存自由名称和作者／画师多链接。
12. 正式界面信息编辑后核对库记录和 PNG 哈希，浅深主题及 800 宽窗口可操作。
13. 导入向导的真实 PNG 与错误 IPC、逐项归属选择和窄窗口深色界面。
14. 预设页浅深主题、窄窗口、范围详情和编辑操作。
15. 同路径、同大小、时间戳不变的外部修改后，实际索引、图片请求修订和画布像素均更新；保存的预设图片不变，thumb/vault 均可读取。
16. 关于页参考项目与工具同宽并排，JASM/APX/Codex 链接正确，窄窗口深色堆叠。
17. 主窗口正常关闭，取消待稳定事件，退出后未继续提交索引。

导入向导通过公开 `plugin:event|emit_to` 入口合成路径事件，随后执行真实识别 IPC；没有替换 Tauri 的只读 invoke 桥接。这证明向导、协议和识别行为，不代表真实 Windows 文件选择器或真实外部拖放已验收。

图片刷新检查直接加载当前 img 地址，在画布中确认新像素 `[73,30,40,255]`，同时确认保存预设图的地址保留旧哈希。验收 PNG 只有 2 像素宽，截图中的候选呈小点属于夹具尺寸；官方原图未提取的提示也属于隔离夹具。

已人工检查正式截图：[模组信息浅色](B:/APMM-next-work/management-native/759193e4-1679-4717-8fe7-7c63286b5eb3/mod-info-comparison-light.png)、[模组信息窄窗深色](B:/APMM-next-work/management-native/759193e4-1679-4717-8fe7-7c63286b5eb3/mod-info-comparison-narrow-dark.png)、[导入归属窄窗](B:/APMM-next-work/management-native/759193e4-1679-4717-8fe7-7c63286b5eb3/import-classification-narrow-dark.png)、[预设窄窗](B:/APMM-next-work/management-native/759193e4-1679-4717-8fe7-7c63286b5eb3/presets-narrow-dark.png)。同目录保留关于页与像素刷新截图。

## 受影响职责

完整文件及职责列表见 [CHANGED-MANAGEMENT-0.3.39.md](CHANGED-MANAGEMENT-0.3.39.md)。以只读 `0.3.38` 快照差异为准，没有 Git 提交或远端发布记录；共用缓存不作为源码变更。

```text
src-tauri/src/
  management_types.rs                   候选、来源、归属和预设契约
  presets.rs / preset_domain.rs          管理入口、问题预览和整批事务
  candidate_management.rs                候选信息、归属与合并回滚
  directory_files.rs / vault_sync.rs     文件身份、库编辑关联和规则确认
  directory_sync.rs / directory_monitor.rs 被动同步与监听生命周期
  management_tests.rs / sync_management_tests.rs 定向兼容与失败夹具
src-ui/
  modules/PresetsPage.tsx / CreatePresetDialog.tsx
  modules/ResourceFileActions.tsx / PackageDialog.tsx / PackageSelection.tsx
  shared/CandidateComparison.tsx / CandidateCreditsFields.tsx / ResourceSlotSelect.tsx
  core/domain/                           名称、归属和搜索规则
  core/services/ / hooks/ / core/ipc/     命令适配与展示快照
  management-ui.css                      信息两栏和窄窗布局
tools/
  verify-management-*.mjs / .py          浏览器、正式 EXE 和产物审计
  management-*.mjs / .py                 PNG、界面、像素及源码清单职责
```

新增业务模块均在 500 行内；遗留大文件保留兼容边界，新的预设、归属和监听流程已拆出。原生验收编排超过 500 行，PNG、界面和像素已分开，仍统一拥有一个隔离副本及有序的失败旅程。逐项职责复核记录在 FILE-MAP。

## 验证边界与后续复核

未覆盖安装、未向实际游戏部署、未远端发布，未清理共用缓存或用户数据。实际游戏加载、Windows 系统文件选择器、真实外拖、Linux 原生运行和上述历史三图夹具尚未验证；自定义目录体系及远端 Mod 更新检查按计划留作后续。

最小人工复核：将 ZIP 解压到新的 B 盘目录，用系统选择器选两张真实尺寸 PNG；分别编辑作者及画师多链接、Enter 改名并确认原 PNG 摘要不变；对比两图并缩放、拖动、复位。连接一个隔离 modimg，创建局部预设后外部修改同路径图片，确认当前图刷新而保存图保留；修复预设后应用，范围外保持原状。实际游戏部署另行说明目标并取得授权。
