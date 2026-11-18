# 工坊投稿与本机审核（无需服务器）

默认流程：作者在应用中编辑草稿 → 导出投稿 ZIP → 通过双方约定的渠道交给维护者 → 维护者在本机审核并生成签名目录 → 人工发布到 CNB 静态资源仓库。作者使用应用不需要 Python、账号、管理凭证或投稿服务器。

源码目录名 `workshop-service` 沿用早期实现；下面的 `local-review.py` 不启动 HTTP 服务，不连接投稿 API，不提交或推送 Git，也不发送 Issue/消息。私有服务及 Actions 仅为可选实现，默认关闭，说明见 [SERVER-OPTIONAL.md](SERVER-OPTIONAL.md)。

## 作者操作

1. 在“设置 → 工坊”开启“作者模式”，进入导航中的“投稿”，新建作品。
2. 添加本地 PNG、PNG ZIP、旧版/分卷加密 `.apmod`，或从本地库选择资源。密码仅用于本次本地解析。每个同名候选独立保留。
3. 填写资料并确认完整资源分类。在“主封面”选择或替换 PNG，附图最多六张，也可将附图设为封面。编辑器与投稿快照提供横向卡片和封面网格预览；预览副本缩小，作品资源保留原始 PNG 字节。
4. “预览投稿与导出”核对增加、修改、删除项，导出 ZIP。其状态仅为“已导出”，不代表已交稿、审核或发布。
5. 自行通过约定渠道交稿。大包使用双方可接收的文件渠道，不依赖特定网站的附件限制。不要把包密码、账户令牌、私钥或管理凭证附在投稿中。
6. 以前的投稿 ZIP 可在空白新草稿中重新导入。已发布作品先在“我的作品”同步签名公开版本，再新建更新草稿；内容更新必须包含所有需要保留的资源。人工审核结果和撤回申请通过原收稿渠道沟通。

## 维护者的本机准备

仅维护者工具需要 Python 3.12。建立位于 B 盘的独立虚拟环境，从 `requirements.lock` 安装固定依赖。所有待审包、审核依据、密钥和输出放在源码目录之外，不放进公开资源仓库。

以下命令在 `apmm-desktop` 根目录执行；把示例路径换成自己的独立路径。

```powershell
py -3.12 -m venv B:\APMM-next-work\review-venv
$reviewPython = 'B:\APMM-next-work\review-venv\Scripts\python.exe'
& $reviewPython -m pip install -r workshop-service/requirements.lock
& $reviewPython workshop-service/tools/canonical-index.py --output B:\APMM-next-work\canonical-index.json
& $reviewPython workshop-service/tools/local-review.py inspect B:\APMM-next-work\inbox\submission.zip --index B:\APMM-next-work\canonical-index.json --output B:\APMM-next-work\review-001
```

`inspect` 校验大小、路径、条目一致性、PNG 解码、完整分类及摘要。输出 `review.json`、只读投稿副本、解包资源和空白 `evidence-template.json`。结构检查不等于游戏验收；自行查看资源、测试对应游戏版本并填写实际依据。

将模板另存为 `evidence.json`，填写 `scanResult`、`gameVersion`、`testEvidence`、`reviewNote`、`authorVerification`。保留自动生成的 `snapshotHash`。`authorVerification` 必须记录通过原收稿渠道核实身份/更新授权的依据；作者自填的编号和名称不构成身份认证。审核资料始终私有。

## 准备签名发布目录

先取得当前 CNB 静态资源仓库的本地副本。首次发布可使用空的 `workshop` 目录。签名密钥文件内容为 Ed25519 原始 32 字节私钥的 Base64；其公钥必须与客户端内置工坊公钥一致。不要把测试密钥当作生产密钥；本工具不生成、不替换生产密钥。

```powershell
& $reviewPython workshop-service/tools/local-review.py prepare --review B:\APMM-next-work\review-001 --evidence B:\APMM-next-work\review-001\evidence.json --catalog B:\APMM-next-work\assets\workshop --output B:\APMM-next-work\prepared-001 --key B:\APMM-next-work\signing\workshop.key --index B:\APMM-next-work\canonical-index.json
```

输出中仅 `prepared-001/workshop/` 可放入公开资源仓库；它包含内容地址 PNG/ZIP 和签名 v2、兼容 v1 目录。`private/`、审核资料和 `prepared.json` 留在本机。状态为 `prepared`，此时尚未发布。生成过程不修改现有资产仓库，不锁定服务器状态；失败保留现场，用新的输出目录重新执行即可。

发布前更新本地资产仓库，再运行基线检查，防止期间新增的作品被旧目录覆盖：

```powershell
& $reviewPython workshop-service/tools/local-review.py check-base --prepared B:\APMM-next-work\prepared-001 --catalog B:\APMM-next-work\assets\workshop
```

基线变化则重新准备，不能强行覆盖。检查通过后由维护者审阅文件差异，将准备好的 `workshop` 文件加入同一 Git 提交，再按项目发布权限推送到 CNB。不要复制私有输出，不要删除未审阅的仓库文件。远端发布需另外明确授权，当前任务未执行。

推送后用 `verify-public --prepared <准备目录> --base <客户端固定 CNB 工坊地址>` 逐文件比较公开摘要，全部匹配才生成 `public-verified.json`。失败保留待发布状态，可重试读取，不虚构上传或发布成功。

## 兼容与限额

- 工坊署名区分作者与画师，署名支持换行、最多 120 字。卡片作者最多显示两行，详情展示完整署名。
- 作者、画师各可填写最多 8 个主页链接，并可填写作品来源链接；仅接受无登录凭据的 HTTPS 地址，每项最多 2048 字节。旧投稿缺少新字段时按空资料读取。
- 资料随投稿 ZIP、本机收稿和签名目录传递；普通 `.apmod` 的包格式与导入导出不受本轮变更影响。
- 更新投稿未提供新图片时沿用已发布图片；提供新图片时替换完整展示图片清单，请添加需要保留的附图。封面替换先验证新图和草稿修订，失败保留原清单。

- 投稿 ZIP 最多 80 MiB，内容 ZIP 最多 64 MiB；最多 512 个资源，展开合计 256 MiB。
- 主封面加六张附图，每张最多 2 MiB；预览图不能作为内容替换的压缩步骤。
- v2 同一响应带载荷与签名，支持独立同名候选；v1 仅发布可由旧客户端唯一表达且有封面的作品。
- 历史 `.apmod` 的密码、原来源路径不写入投稿资料；离线包包含公开作品资料、稳定编号、原图与摘要，不含系统凭据。
- 已发布目录是签名基线；本地导出记录不自动推断人工审核结果。暂不提供离线自动撤回，维护者通过原收稿渠道处理并重新签名目录。

## 验证与许可

本地验收证据保存在 `APMM_REPORT_DIR`，不进入公开目录。`tests/test_local.py` 覆盖离线收稿、作者依据、过期基线、同名独立候选及实际 Rust 导出；`tests/test_http.py` 只验证可选服务。依赖原文见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)，它们不被打入桌面便携 ZIP。
