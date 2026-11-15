# APMM 0.3.37 本地验收报告

日期：2026-11-14。交付范围为新工程、Windows 便携 ZIP 和独立资产工具包。旧安装未覆盖，远端未发布。

## 实际产物

| 包 | 字节数 | SHA-256 |
|---|---:|---|
| `APMM-0.3.37-windows-x86_64-portable.zip` | 4,489,766 | `ae02b5b029aae404b49ae19f58f4fb6d032fe88d8129afeb6f2f0ad401e3819d` |
| `APMM-AssetTool-0.1.0-windows-x86_64.zip` | 21,749,578 | `feb0ddf809f9d674035ee6ede592eaa50451a2ed0de4b61edcc0156218d36002` |

主包清单为 2 个 EXE、外部程序 manifest、2 个运行 DLL、发行清单和许可文本。实际 ZIP 清单及每个文件摘要已重新核对。主包不含用户 data、游戏原图、前端源码、Python 或资产插件运行时。插件包不含现成游戏数据，250 个签名载荷文件逐一核对。

`verification-portable-build.json`、`verification-plugin-build.json` 记录构建来源；`verification-release-audit.json` 记录最终 ZIP 清单。Cargo `.d` 确认嵌入构建后的 index.html，并排除旧 src/previews。

## 分层证据

| 层级 | 实际结果 | 证据文件 |
|---|---|---|
| 静态与生成 | TypeScript 通过；命令从 Rust 注册面生成，DTO 从 ts-rs 导出；旧版 22 个源码基线哈希未变化 | verification-source-inventory.json、构建日志 |
| Rust 自动化 | 主套件 79 通过、2 个产物用例另跑；更新助手 2 通过；产物专项 2 通过；随后新增的缩略图专项 1 通过 | verification-rust-tests.log、verification-artifact-tests.log、verification-thumbnail-tests.log |
| 前端自动化 | 4 个纯规则用例通过 | verification-frontend-tests.log |
| 资产工具自动化 | 49 个用例通过 | verification-plugin-tests.log |
| 浏览器交互 | 8 项通过，无 pageerror | verification-ui.json |
| 原生 WebView/IPC | 15 项通过，无 pageerror；使用实际发行 EXE、独立签名插件和隔离 modimg | verification-native.json |
| 真实旧库复制 | 2,740 个文件，714,957,376 字节，985 个 Mod 载荷核验；旧索引和游戏文件哈希未变 | verification-migration-native.json |
| 更新助手进程 | 成功确认、坏 EXE 回滚、applying 中断恢复三条路径通过，测试用户文件保留 | verification-updater.json |
| 真实 APMM 更新 | 在 B 盘副本中以同版本包验证替换、重启及工作区就绪确认；data 与无关文件保留 | verification-update-app.json |
| 编译与打包 | Windows MSVC release 和两个 ZIP 均成功；有遗留 dead_code/unused 警告，未宣称零警告 | verification-portable-build.log、verification-plugin-build.log |
| 游戏内行为 | 未验证替换后的实际画面、渠道插件效果或游戏运行兼容性 | 需按下方场景手工验收 |

## 资源与兼容覆盖

真实资源来自本机 Astral Party 3.3.0 / Unity 2022.3.62f3，只读提取。插件 EXE 提取、复用及取消均有实际运行记录。

| 分类 | 图片数 |
|---|---:|
| `account_background` | 80 |
| `busts` | 322 |
| `card` | 204 |
| `card2` | 205 |
| `card_in_game` | 5 |
| `cursor` | 2 |
| `event_card` | 31 |
| `hand_card` | 78 |
| `item_icons` | 36 |
| `land` | 36 |
| `level_up` | 138 |
| `map_event` | 73 |
| `monster_bust` | 71 |
| `monster_card` | 73 |
| `monster_profile` | 69 |
| `platform` | 36 |
| `profile_photos` | 138 |
| `relic` | 71 |
| `relic_icon` | 8 |
| `role_photos` | 201 |
| `standing_paintings` | 103 |
| `thin_card` | 199 |

合计 22 类、2,179 张。所有 PNG 摘要和尺寸与 map 相符；UT_Relic_50075_1 独立保留；36 组数字地块配对；鼠标两态均为 60×60 RGBA，alpha 范围 0–255。全量重复同步提取 0 张，文件哈希未变。标准输入取消返回 130，并保留已有产物。

缩略图缓存与原图分离；专项测试验证透明度、原图哈希和内容变化后的缓存身份。内置 cursor 的最小类型描述仅对已验证的 Unity 系列承诺可用，其他版本拒绝猜测字节偏移。

APMMOD01/02/03 的兼容实现和夹具保留；同槽位三文件使用真实 PNG 完成加密往返，旧包缺少 storageName 时验证路径分配。错误密码、损坏内容、缺失分卷、成组失败回滚、迁移源变化与迁移中断恢复由对应自动化用例覆盖。真实旧库已单独迁移验证，未要求用户重新导出旧 Mod。

下载回归覆盖错误范围、正确 206 续传、200 从头、校验失败、取消和换源。断网或服务器完全不响应通过超时退出，但尚未覆盖所有代理/运营商环境。

## 已实现功能与剩余验收

| 功能 | 实现与证据 | 剩余边界 |
|---|---|---|
| 主题、导航、浏览、详情、比较、批量与任务 | 浏览器和实际 WebView 检查 | Windows 125%/150%/200% 系统缩放、真实文件对话框和鼠标拖入仍需手工验证 |
| 导入、导出、预设和成组资源 | 原生 IPC 往返、实际隔离文件和事务回归 | 用户所有历史超大包/分卷组合未逐一测试 |
| APX 与工坊 | 旧实现迁入，平台限制、版本确认、缓存/还原及归属保护保留；归属/路径测试通过 | 未向真实游戏目录执行 APX 或工坊安装，需确认后按渠道验收 |
| 辅助插件 | 签名安装、执行、提取、取消、卸载保留成果及增量同步通过 | 线上 release.json 与插件 ZIP 尚未发布；旧版本目录保留供诊断 |
| 软件更新 | 下载校验、发行清单限制、等待任务、备份、替换、重启确认、失败恢复通过 | 实际 APMM 测试使用同版本本地包；未在线发布升级 |
| Linux | 文档列出前端与工具开发准备 | 未编译原生 Linux 程序、未交付 Linux 包 |
| 外部拖出 | 继承的 Windows OLE 入口保留 | 未证明 Photoshop/QQ/资源管理器接收；不能把 IPC 通过当外拖完成 |

最小手工验收：在新的便携目录用文件选择器和资源管理器拖入同槽位三张 PNG/.apmod，核对只选中不自动应用；依次验证两种主题和三档 Windows 缩放；单独备份游戏 modimg 后检查卡面、地块半组/整组、cursor 抬起/按下效果。APX/工坊安装和覆盖旧程序另行确认后执行。

## 存储与责任清单

完整文件职责及超过 500 行的遗留边界见 FILE-MAP.md；复制旧库流程见 MIGRATION.md；开发命令和平台限制见 CONTRIBUTING.md。

| 保留目录 | 实测大小 | 原因 |
|---|---:|---|
| `D:\MOD\APMM-next\apmm-desktop\src-tauri\target-msvc` | 10.11 GiB | 复用的 MSVC 编译/测试缓存 |
| `D:\MOD\APMM-next\apmm-desktop\node_modules` | 0.15 GiB | 锁定的前端开发依赖 |
| `D:\MOD\APMM-next\apmm-desktop\plugins\asset-tool\.venv` | 0.07 GiB | 独立资产插件的构建环境 |
| `B:\APMM-next-work` | 1.85 GiB | 源码快照、真实资产结果、隔离 IPC/更新副本、可复现构建暂存和本地签名密钥 |

MSVC 缓存已超过 8 GiB，已单独告知；本次未删除共用缓存。B 盘隔离结果和暂存保留用于复现，私钥只留在 B 盘签名目录，不进入包。以上目录均非旧工程的清理目标。

旧版、参考工程和 demo 作为对照保留。当前只交付本地验收包；未把尚未完成的原生输入、系统缩放或游戏画面检查计为通过。
