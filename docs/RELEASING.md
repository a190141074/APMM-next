# 发行与预览资源

源码仓库保存可复现构建所需的源码、配置、资源、工具和许可。机器报告使用源码之外的 `APMM_REPORT_DIR`，默认在工作盘的 `reports/<版本>/`。人工任务报告使用 [项目内忽略目录](../CONTRIBUTING.md#任务报告)。

## 目标与操作

| 用途 | 目标和入口 |
| --- | --- |
| CNB 源码 | `cnb` → `astral-weird-egg/APMM-next` 的 `main`，HTTPS Git + CNB 访问令牌 |
| GitHub 源码 | `origin` → `a190141074/APMM-next` 的 `main`，HTTPS Git + 已配置的凭据 |
| CNB 应用与预览 | `astral-weird-egg/astral-party-mod-manager-assets` 的 `main`，客户端读 `config/cnb.json` 指定的 raw 路径 |
| GitHub 应用兜底 | `a190141074/APMM-next` Releases，标签 `v<版本>`，标题 `APMM <版本>` |
| GitHub 原图预览兜底 | `a190141074/AstralPartyModManager-Previews` Releases，标签 `preview-v<N>` |

发行物用途如下：MSI 是普通固定安装包；portable ZIP 是普通便携版；legacy-bridge 仅供已发布的 0.3.36 自动升级；compat-0.3.39 仅供本地 0.3.39 测试版升级。标准下载页只推荐 MSI 和 portable ZIP，两个兼容 ZIP 放在旧版维护区域。

“推送”同步同一源码提交到两个源码仓。先检查工作树与暂存范围，逐项核对改动和公开文件，执行 `node tools/audit-source.mjs`、`git diff --check` 和对应定向检查。向用户列出待推 SHA、两端分支目标与当前差异，按授权推送明确的 refspec，再通过 `git ls-remote` 回读两个 `refs/heads/main`。遇到分叉先查范围。两端以快进同步，已公开历史和版本标签保持原样。普通推送的目标范围仅含 `main`。

“发新版”还包括两端指向同一提交的 `v<版本>` 标签、CNB 资产仓和 GitHub Release。先将本轮源码 SHA 推到 CNB 与 GitHub 的 `main` 并回读确认相同；从该 SHA 创建一个本地注释标签。分别推送标签到 CNB 与 GitHub，再用 `git ls-remote --refs` 回读，确认两端标签对象一致，且本地 `git rev-parse refs/tags/v<版本>^{commit}`、两端标签都解析到该 SHA。只有标签验证完成后才创建或更新 GitHub Release。已有标签时先核对解析到的提交，已有版本路径的内容须与本轮摘要一致。预先整理可审核的发布说明、文件清单与目标。远端推送、上传草稿附件和公开发布按当前对话针对该目标的授权执行。

本地标签命令形式为 `git tag -a v<版本> <已验证的40位SHA> -m "APMM <版本>"`。CNB 源码标签使用项目 skill 的 `Push-Cnb.ps1 -Repository Source -Checkout <应用仓绝对路径> -Tag v<版本>`，先不加 `-Execute` 预检；获授权后再执行。GitHub 使用明确 refspec `git push origin refs/tags/v<版本>:refs/tags/v<版本>`。若任一目标已有不同标签，停止并核对，不强制更新标签。

## 应用发行

`APMM_DIST_DIR` 指向本轮版本的独立产物根目录，默认是工作盘的 `releases/<版本>/`。每次重新构建应使用新目录，脚本拒绝覆盖已存在的包或报告。

```text
releases/<版本>/
├── windows-x86_64/
│   ├── APMM-<版本>-windows-x86_64-portable.zip
│   └── APMM-<版本>-windows-x86_64.msi
└── compatibility/
    ├── APMM-<版本>-windows-x86_64-legacy-bridge.zip
    └── APMM-<版本>-windows-x86_64-compat-0.3.39.zip
```

便携 ZIP 解压后的结构为 `APMM-<版本>/`，其中直接放置主程序、更新助手、运行库和发行清单，许可文件放在 `licenses/`。外层文件夹用于避免解压后文件散落在用户当前目录；不要把程序文件再搬入 `app/` 子目录。更新器会依据 ZIP 内发行清单去除外层前缀，并将文件更新到现有程序目录。MSI 与两个旧版兼容 ZIP 的内部布局不随此调整。

每个包旁有同名 `.sha256`。标准下载页只展示便携 ZIP 与 MSI；旧版桥接与测试版兼容包放在单独目录，并说明适用的旧版本。

1. 以 `VERSION` 定版，按开发指南同步主程序配置和对应 `CHANGELOG.md` 标题。锁定本轮提交与输入摘要。在工作盘给 `APMM_DIST_DIR` 和 `APMM_REPORT_DIR` 分配新目录，执行 `node tools/build-portable.mjs`。
2. `APMM_BUILD_REPORT` 指向本轮便携构建报告，执行 `node tools/build-distributions.mjs`，派生 MSI 与兼容包。它核对输入摘要及 MSI 使用的二进制，拒绝混用旧构建。
3. 执行 `tools/verify-release.py`、`tools/verify-distributions.py`。MSI 可用 `tools/verify-msi.ps1` 在独立目录管理展开，不注册安装。原生界面验收使用独立 Windows desktop。
4. 核对源码审计、包内版本与白名单、安装身份、许可、隔离运行、实际字节数与 SHA-256。签名状态以实际证据为准，资产插件签名独立于主程序和 MSI。

## 准备与上线

在 B 盘隔离区使用已核对来源和基线的独立 CNB 资产 checkout。设置 `APMM_REPORT_DIR`，运行：

```powershell
python -X utf8 -B tools/prepare-release-assets.py --assets-repo <CNB资产仓> --distributions <本轮distributions.json> --github-dir <GitHub待发布目录>
```

预览有内容更新或目标渠道缺少该预览版本时，增加 `--previews <已审核预览构建的public目录>`；若只补发预览，增加 `--previews-only`，不提供 `--distributions`，可选 `--github-dir` 只生成预览附件 staging。预览专用模式不得生成或提交应用包及应用更新清单。预览构建固定为 WebP 有损编码 `lossless=False, quality=50, method=4`，不缩放；Pillow 未显式设置的 `alpha_quality` 使用默认值 100，编码后还会逐像素校验 alpha 通道与原图一致。来源记录和发行准备校验须证明编码参数正确。同一预览版本补齐另一渠道时复用审核过的同一批字节。检查本地 `publication-plan.json` 所列文件、阶段和目标。`remoteApplied: false` 表示只准备了本地文件。

1. CNB 资产仓按阶段显式暂存。先推版本目录及校验文件，必要时一并推预览 ZIP。用公网 URL 回读实际文件，核对字节数和摘要。再提交与核验预览清单，保留历史直链及原字节。
2. GitHub 应用草稿从 `application/v<版本>/` 取四类包、`SHA256SUMS.txt`、`update-manifest-v2.json` 和 `update-manifest.json`。将 `RELEASE-NOTES.md` 内容填入发布说明。新版清单使用同标签的 GitHub 附件地址。旧版清单沿用已验证的 CNB 桥接入口，0.3.39 测试版兼容包独立下载。
3. GitHub 预览草稿从 `previews/preview-v<N>/` 按计划取完整和兼容 ZIP、两份预览清单及摘要。新版和旧版清单分别指向对应集合。需要补齐预览时，先公开并核验该预览 Release，再公开应用 Release。
4. GitHub Release 附件使用 `tools/Upload-GitHubReleaseAssets.ps1`，从 `B:\APMM-next-work\credentials\github-token.txt` 为 `gh` 子进程设置临时 `GH_TOKEN`。推荐细粒度个人访问令牌，仅授予目标仓库的 Contents: write；上传脚本不创建 Release，也不需工作流权限。令牌文件为单行令牌或唯一的 `Token: <令牌>` 记录。先对 staging 文件执行只读摘要检查；得到本次上传授权后才加 `-Execute`。例如：

   `pwsh -File tools/Upload-GitHubReleaseAssets.ps1 -Repository App -PlanPath <publication-plan.json> -Check`

   预览附件将 `-Repository App` 改为 `Previews`。默认模式仅做本地检查；获授权后加 `-Execute` 才访问 GitHub。脚本只允许应用与预览两个固定仓库，要求目标 Release 已存在且为 draft；已存在且远端 digest/size 一致的附件跳过，不同或无法校验的同名附件停止，不覆盖。上传后回读每个附件的服务端 SHA-256 和字节数。Chrome 仅用于核对 Release 页面状态，不作为默认文件上传通道。
5. 等完整预览和 GitHub 应用兜底验证通过，最后提交 CNB 新旧两份应用更新清单和相应下载说明。公网回读两端当前清单、每个实际附件、版本、字节数和 SHA-256，预览另核条目数。记录两端 Git SHA、标签、附件与生效阶段。中途失败按实际远端状态续跑，保留已验证且摘要相同的文件。

根目录 `update-manifest-v2.json` 是新客户端入口，按安装身份选择标准包；`update-manifest.json` 是旧版桥接入口。发布工具使用完整 CNB 版本目录 URL。已被旧客户端或旧清单引用的平铺路径保留为兼容入口，不再用于放置新的常规发行物。

## 原图预览

预览与软件包独立更新，不能用界面截图或本机测试副本替代远端预览包。`tools/build-preview-bundles.py` 接受官方资产工具的 `maps/`、`outimg/`、`state.json`，逐张验证 PNG 来源摘要，沿用旧项目的有损 WebP 质量 `quality=50`，保留完整名称和原尺寸。编码搜索参数使用 `method=4`，平衡构建耗时和下载体积；旧转换器使用 `method=6`。Pillow 的 `alpha_quality` 参数未显式设置，采用默认值 100，构建器解码输出并校验 alpha 通道逐像素一致；这表示颜色有损而透明度无损。输出逐张校验格式、尺寸和透明通道，并在发布前抽样检查画质、统计实际下载体积。新输出必须使用空目录；PNG 原图保留在本地，不进入预览包。

当前游戏已移除但旧资源目录仍保留的项目，只能沿用已核对的历史预览，不能标记为本轮重新提取。其名单和来源摘要写入本地构建报告。

```text
preview-manifest-v4.json       当前客户端完整清单
bundles/preview-v4/            五类资源，角色按部位分为四包，共八包
preview-manifest.json          旧客户端兼容清单
bundles/*-compat.zip           旧客户端所需的平铺兼容入口
```

旧客户端的资源白名单固定在其二进制中，不能直接给它们追加资源。因此兼容清单只刷新旧文件集合，完整新资源通过独立清单交给新客户端。两套包复用同一份压缩 WebP 字节，不二次压缩；完整包按分类和角色部位供应用选择下载。每包仍须小于 256 MiB，不能用拆包替代压缩。已有 `preview-v3` 文件保留原字节，避免中断正在下载的旧清单。

上线顺序是预览包、摘要核验、预览清单。应用更新清单只能在相应的完整预览清单可读取后切换。新旧客户端的 GitHub 兜底也分别读取完整和兼容清单，不能因相同版本号而混用。发布完成必须回读公网清单和实际 ZIP，并重新核对大小、SHA-256 与文件数；本地准备完成不代表远端已生效。
