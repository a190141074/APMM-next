# 安装与更新

MSI 适合固定安装与系统卸载，便携 ZIP 适合单独目录。安装身份由发行文件 `installation.json` 声明，不扫描另一份安装来改变当前副本。缺失标记的旧版按便携版处理。

| 分发          | 程序位置                              | 用户数据          | 更新方式                                 |
| ------------- | ------------------------------------- | ----------------- | ---------------------------------------- |
| MSI           | Program Files/APMM 或安装时选择的目录 | LocalAppData/APMM | Windows Installer 事务                   |
| portable      | 解压目录                              | 程序旁 data       | 白名单文件备份、替换、就绪确认、失败回滚 |
| legacy-bridge | 0.3.36 原目录                         | 原 data           | 旧助手平铺安装，新程序核验并交接新版助手 |
| compat-0.3.39 | 本地 0.3.39 原目录                    | 原 data           | 旧 runner 安装平铺白名单文件并替换助手   |

新版只接受 schema 2 更新清单中与平台和安装类型精确匹配的唯一资产，不回退到第一个包。旧版继续读取 schema 1 清单，仅向它提供桥接 ZIP。发布时先上传文件并回读摘要，最后更新清单。

安装前校验下载收据、SHA-256、文件白名单、版本和数据格式；关闭新写入入口，等已登记任务清空，再启动独立助手。助手确认准备完成，主窗口才正常退出。便携助手等待旧 PID 真正退出；启动确认来自工作区初始化，而非只检查进程是否存在。启动失败恢复原文件；未知用户文件、data 和 Vault 不在替换范围内。

MSI 助手使用一个普通权限观察进程和一个提权安装进程。取消 UAC 时保留原窗口。提权进程只运行 Installer，并清空 MSI 的自动启动属性；安装后的启动由普通权限进程完成。Installer 成功后仍须核对安装身份、版本和发行文件。MSI 的安装事务与修复归 Windows Installer 管理，不调用便携文件回滚。卸载不包含 LocalAppData/APMM 的清理动作。

日志和就绪记录保存在 data/updates 与本次独立 staging 下。一次性结果提示被领取后保留已读记录。MSI 安装日志为 installer.log；失败后可重新运行同版本安装包修复。不要手动删除用户 data 或共用暂存目录来解决更新问题。

## 打包与许可

`node tools/build-portable.mjs` 构建主程序及 helper，`node tools/build-distributions.mjs` 派生 MSI、0.3.36 桥接 ZIP 与 0.3.39 兼容 ZIP，生成大小与摘要报告。MSI 使用 WiX Toolset 3.14.1、固定 UpgradeCode，拒绝较旧版本。WiX 工具与包均保留各自许可；项目源码采用 GPL-3.0-only。

已打包的本地 0.3.39 不认识 next 助手、安装标记与独立 NOTICE；它的兼容包只携带旧白名单文件，并把归属说明合入 THIRD_PARTY_NOTICES.md。此前公开的是 0.3.36，因此旧在线清单仍提供 legacy-bridge；0.3.39 本地测试版使用独立 compat 包，或退出后将标准便携包解压到原目录。两者均保留原 data。

制作和侧载 MSI 本身不需要申请 Microsoft Store 账号。MSI 包可以不签名；可信证书签名、SmartScreen 信誉及 Microsoft Store 上架是另行处理的分发事项。当前产物未做 Authenticode 签名。采用 MSIX 或商店分发须另外评估证书、应用身份和发布流程，本次不切换到 MSIX。

资料：Microsoft 的 Windows Installer [签名说明](https://learn.microsoft.com/en-us/windows/win32/msi/digital-signatures-and-windows-installer)、Tauri [Windows Installer](https://v2.tauri.app/distribute/windows-installer/)、GNU [GPL 常见问题](https://www.gnu.org/licenses/gpl-faq.html)。GPL 分发随版本提供对应源码与构建说明，不能只改许可名称而遗漏第三方归属。
