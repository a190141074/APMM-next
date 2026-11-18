# 发行组织与下载预览

适用于打包交付、公开源码整理、软件发布和预览包更新。先读当前工作区应用根下的 `docs/RELEASING.md`、`docs/UPDATING.md` 与对应脚本，不执行旧 astral-tauri 的发布入口。

## 把交付对象核对到文件

| 用户指定对象 | 必须对应的证据 |
|---|---|
| 界面截图 | 对应版本、页面与截图文件 |
| 应用下载的预览图 | 预览清单、实际 ZIP、安装后可读取的 WebP |
| 新构建 | 当前源码输入摘要、EXE、发行清单、ZIP/MSI 和本轮报告 |
| 远端已更新 | 授权范围内完成发布后回读的公网清单、ZIP 大小与摘要 |

本地截图、测试副本和远端预览曾被混为一谈。用户要求更新预览时，从应用实际读取的清单 URL 开始追到包内容；只修改文件名、日期或本地缓存不表示公网生效。

## 公开源码与产物目录

- 公开源码保存可复现构建必需的源码、config、锁文件、资源和许可。稳定文档如 ARCHITECTURE、MIGRATION、UPDATING、RELEASING、FILE-MAP 与展示 images 可以保留；不能把所有 Markdown 一律视为 AI 日志删除。
- 机器验收结果、环境指纹和临时截图写入源码之外的 `APMM_REPORT_DIR`。人工任务报告写入被 Git 忽略的 `apmm-desktop/reports/`。FILE-MAP 保持稳定导航；分类与元数据统一读取当前 config。
- 新增 `.gitignore` 不会自动取消已经跟踪的文件；发布整理需核对索引与引用，只移除已确认不公开的索引记录并保留本地资料。用户数据、凭据、私钥和缓存不进入源码或包。
- 当前共享路径入口为 `tools/work-paths.mjs`、`tools/work_paths.py`。默认报告为工作盘 `reports/<版本>/`，产物为 `releases/<版本>/`；每轮重建显式使用新的独立目录，不能覆盖旧包或报告。

```text
releases/<版本>/
├── windows-x86_64/    portable.zip、MSI 及各自 .sha256
└── compatibility/    legacy-bridge、compat-0.3.39 及各自 .sha256
bundles/preview-v4/   当前完整压缩预览，按分类下载
bundles/*-compat.zip  旧客户端固定集合的兼容入口
```

标准下载入口展示便携 ZIP 与 MSI；兼容包说明适用版本。已有平铺旧链接及原字节继续保留，避免中断旧清单下载；不把整理目录等同于删除历史文件。

普通下载中 MSI 用于固定安装，portable ZIP 用于独立目录运行；legacy-bridge 只服务已发布的 0.3.36，compat-0.3.39 只服务该本地测试版。便携 ZIP 解压后包含 `APMM-<版本>/` 单一顶层目录，程序仍直接位于此目录；更新器会按发行清单剥除压缩包前缀并原位更新。兼容包继续平铺，手动覆盖旧便携目录时应复制顶层目录内文件，不能再嵌一层文件夹。

## 构建必须与最终源码一致

- 版本递增按 SKILL.md：独立应用功能或修复默认 PATCH，同轮重试沿用目标版本；skill、文档和本地发行工具整理不升版。
- 用 `APMM_DIST_DIR`、`APMM_REPORT_DIR`、`APMM_BUILD_REPORT` 指定本轮路径，执行 `tools/build-portable.mjs`。保留前端扩展 Windows 路径及 `.d` 检查，不能裸 Cargo 编译代替正式嵌入前端。
- 构建前后记录并比对应用输入摘要。共享工作区或 Cargo target 可能被另一聊天改写；派生 MSI 前核对 target 中主程序和助手与本轮便携 payload 的摘要，不因版本号相同而混用。
- `APMM_BUILD_REPORT` 指向本轮便携报告，`APMM_DISTRIBUTIONS_REPORT` 指向本轮派生报告；`tools/build-distributions.mjs` 生成 MSI 和兼容包。用户要求重建就从当前源码重建，不能用旧报告替代。
- Tauri MSI 打包器曾只把 Cargo.toml 的 CRLF 改成 LF。当前工具保留原字节，只恢复已证明的换行变化；发现实质内容变化停止交付并复核，不能无条件恢复备份覆盖并发编辑。
- 主 ZIP 仅含主程序、助手、必要 DLL/manifest、发行清单与许可；排除 data/Vault、原图、前端源码、Python 和资产插件运行时。资产插件独立发行。
- 分开执行源码审计、`verify-release.py`、`verify-distributions.py` 与本轮原生验收。`verify-msi.ps1` 的管理展开只验证包，不注册安装，不能据此宣称覆盖升级、UAC 取消或卸载实机通过。

## 预览编码和兼容集合

- 官方 PNG 原图、本地缩略图和下载预览分别维护。下载预览使用既有有损 WebP：`lossless=False, quality=50, method=4`，保留原尺寸和透明度；旧转换器为 method 6。WebP 扩展名不能证明使用了压缩策略。
- 曾误用全尺寸无损 WebP，最大包约 180 MiB。拆包只是下载边界，不能替代正确编码；每包仍核对当前 256 MiB 下载与 512 MiB 解包上限，不为错误大包扩大内存限制。
- `tools/build-preview-bundles.py` 读取资产工具 `maps/`、`outimg/`、`state.json` 和当前 `config/manifest.json`、`preview-groups.json`，逐张核对 PNG 来源摘要、输出 WebP 格式、尺寸与透明通道。新输出用空目录；发布前抽样对照画质并统计包实际字节数。
- 当前游戏已移除的历史资源只能保留已核对的旧预览，报告标明来源，不写成重新提取。统计唯一文件数与各包条目数，跨包重复不能当作新增资源。
- 旧客户端白名单写在旧二进制中；兼容包只刷新它能识别的固定集合。新客户端通过 `preview-manifest-v4.json` 获取完整集合；`preview-manifest.json` 保留旧入口。新旧包复用同一份压缩 WebP 字节，不二次压缩。
- CNB 与 GitHub 兜底分别读取完整/兼容清单；相同版本号不能掩盖不同集合。保留版本目录完整 URL，不能截为 basename；旧来源不能覆盖较新缓存。
- 定向验证：Rust `preview_release_artifacts` 和同文件相关测试；`tools/verify-preview-native.mjs` 使用本轮 `APMM_BUILD_REPORT` 与独立 desktop，在新空缓存目录验证实际包安装、覆盖数、缺图和视口解码。先读脚本核对夹具与环境变量，不向正式后端塞假安装状态。

## 源码推送与渠道认证

“推送”只同步同一 SHA 到 CNB 与 GitHub 源码 `main`。从远端读取分支和标签，因为本地 tracking ref 可能过期。先核对待提交文件、源码审计和提交范围，向用户列出 SHA、目标与影响。敏感操作只在当前对话明确授权对应目标后执行；skill、历史报告不构成授权。使用快进和明确 refspec；遇到分叉、冲突、既有标签指向不同提交时保留已完成结果并停在冲突目标。

CNB 源码 `cnb` 指向 `astral-weird-egg/APMM-next`；独立的资产仓 checkout 以 `origin` 指向 `astral-weird-egg/astral-party-mod-manager-assets`。CNB 通过 HTTPS Git 使用用户名 `cnb`，密码从受控凭据文件读取。当前本机入口为 `B:\APMM-next-work\credentials\cnb-token.txt`。参照 [CNB Git 访问文档](https://docs.cnb.cool/zh/guide/git-access.html)。

按 [Push-Cnb.ps1](../scripts/Push-Cnb.ps1) 的参数使用 CNB 推送辅助脚本。`-Repository Source|Assets` 和 `-Checkout <仓库绝对路径>` 必填。`-Commit <40位 SHA>` 用于分支，`-Tag v<版本>` 用于版本标签，两者互斥。先不加 `-Execute` 进行只读预检；得到本次授权后才加 `-Execute`。脚本必须核对远端 URL、快进关系，且只将 Basic 认证头放进 Git 子进程环境，关闭终端/GCM 交互，不在参数、URL、日志和报告中暴露令牌。失败时只报告仓库、ref 与认证/网络状态，不回显异常环境或响应头。

GitHub 源码用 `origin` 的 HTTPS Git，预期仓库 `a190141074/APMM-next`，依赖已配置的本机 Git Credential Manager。通过 `git ls-remote origin refs/heads/main` 读取真实远端，确认快进后使用明确 SHA/refspec。不要把 CNB PAT 传给 GitHub。

GitHub Release 附件通过应用根下的 `tools/Upload-GitHubReleaseAssets.ps1` 使用 GitHub CLI 完成，不依赖 Chrome 文件选择器。推荐 fine-grained token 只开目标仓库的 Contents: write，保存于 `B:\APMM-next-work\credentials\github-token.txt`；文件为单行 token 或唯一的 `Token: <token>` 记录。先以 `-Repository App|Previews -PlanPath <publication-plan.json> -Check` 只读校验 staging，再在当前对话得到明确上传授权后加 `-Execute`。令牌仅以子进程 `GH_TOKEN` 环境传给 `gh`，不放入命令行、远端 URL 或日志。脚本固定目标仓库，要求 Release 是 draft；已存在附件只有在 uploaded 状态、SHA-256 digest 和 size 均匹配时才跳过，不同或无法验证时停止，禁止 `--clobber`。上传后回读服务端 digest 与大小。Chrome 只用于查看仓库、标签、草稿与附件列表状态。上传中断后重新运行脚本，先核对远端，跳过已验证附件后续传。

目标由工程 `docs/RELEASING.md` 的当前契约确定：应用源码 GitHub、应用 Release `a190141074/APMM-next`、预览 Release `a190141074/AstralPartyModManager-Previews`、CNB 资产 Git 仓彼此独立。

## 发行准备与切换

本地 `tools/prepare-release-assets.py` 生成审查目录，不发布远端文件。预览内容有变化或目标渠道缺少对应标签时才准备预览。应用发布包含预览更新时使用 `--previews`；仅补发预览使用 `--previews-only --previews <已审核public目录>`，不读取 distributions，也不生成应用文件或 app 清单；可指定 `--github-dir` 只生成预览附件 staging。版本必须匹配，现有不同内容的路径拒绝覆盖。`publication-plan.json` 中 `stages[].files` 是需审查及择项暂存的完整候选集，`writtenFiles` 才表示本次工具写入的文件。待推集合始终根据 Git diff 判断。

预览构建必须使用 `tools/build-preview-bundles.py` 当前配置的 `WEBP, lossless=False, quality=50, method=4, resized=False, alpha=lossless`。发行准备会校验预览来源记录；只看 `.webp` 后缀不够。沿用原图尺寸和透明像素，按发行规范检查每包大小、摘要、清单与兼容集合。

源码 `main` 两端核对一致后，从目标 SHA 创建本地 `v<版本>` 注释标签；分别推送到 CNB 和 GitHub，回读标签并验证解析到同一 SHA 后才创建 Release。再按计划先上传 CNB 版本文件及摘要，并回读相同的大小和 SHA-256。预览需要更新时先提交并验证实际包及两份完整/兼容清单。GitHub 新版 app manifest 使用该 Release 标签下的 GitHub 附件 URL，旧版 bridge manifest 保留已经验证的 CNB 桥接 URL。必要的 GitHub 预览 Release 先于应用 Release；等完整预览入口可读取后再发布应用兜底，最后更新 CNB 应用清单。任何步骤中断，记录已完成仓库、提交、标签、文件摘要和剩余步骤，下一轮先回读远端并复用相同字节，不覆盖已公开的版本文件。

## 任务报告与接续

具体位置和正文字段见当前工作区应用根的 `CONTRIBUTING.md#任务报告`。新会话遇到“继续”，先按用户提供的编号、任务名称或变更路径，从应用根 `reports/` 找到报告，读取最后一个阶段的未完成项，再对照当前 Git、VERSION、机器报告和产物。新任务使用新编号；同一任务在原文件增加北京时间阶段小节。报告目录应通过 `git check-ignore` 确认为忽略状态，并由 `git ls-files -- reports/` 确认无已跟踪报告。

机器测试、构建和资产报告继续放在工作盘的 `APMM_REPORT_DIR` 下，不把高体积日志和发行产物放进源码目录。报告保存结论、改动职责、实测命令、确切证据路径、分渠道远端状态、剩余验证与下一接续入口。

## 这次经验的证据入口

2026-11-15 的 0.4.7 本地交付位于 `B:\APMM-next-work\repository-cleanup-20261115\final\DELIVERY.md`。当时 2187 张有损 WebP 合计约 100.1 MiB、最大包约 39.7 MiB，其中 2179 张本轮提取、8 张历史保留；完成包审计、隔离 MSI 展开与后台 WebView 验收，尚未上传。method 4 的耗时收益只测了两张编码样本，不能扩展成应用启动或全量解码性能结论。后续查当前报告与公网状态，不继承这份历史结论。
