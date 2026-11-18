# 可选自建服务参考

当前交付默认采用离线人工收稿，不需要部署本文服务。以下保留此前可选服务实现的运维说明。

# APMM 工坊收件与发布

Python 3.12、FastAPI、SQLite 单实例服务。桌面只持有各作品的管理凭证；收件服务只保存凭证摘要与私有待审包；签名私钥和 CNB 写凭据只进入受保护的发布环境。

## 本地启动

在独立虚拟环境安装 `requirements.lock`。本机资料使用 `B:\APMM-next-work\workshop-20261115`。执行 `tools/canonical-index.py --output <索引文件>` 生成与应用资源目录一致的受信分类清单。

设置以下环境变量后，从本目录启动 `python -m uvicorn workshop_service.api:from_env --factory --host 127.0.0.1 --port 18421 --workers 1 --no-access-log`：

- `APMM_SUBMISSION_DATA`：绝对路径，独立私有数据目录。
- `APMM_REVIEW_TOKEN`：`secrets.token_urlsafe(32)` 生成的独立维护者凭证。
- `APMM_CANONICAL_INDEX`：生成的完整资源名称清单路径。
- `APMM_SUBMISSION_QUOTA`：可选，总存储预算，默认 10 GiB。接收时为两个并发请求预留接收、展开和归档空间。

桌面调试构建用 `APMM_SUBMISSION_TEST_URL=http://127.0.0.1:18421`；工坊目录另用 `ASTRAL_WORKSHOP_BASE_URL` 与 `APMM_WORKSHOP_TEST_PUBLIC_KEY`。正式构建忽略这些测试入口，生产来源在应用 `config/cnb.json` 中固定，当前默认关闭。

## 接口与状态

作品凭证为客户端生成的 32 字节随机数，编码为 43 字符 URL-safe Base64，所有私有接口使用 `Authorization: Bearer ...`。公开编号不能认领旧作品；旧目录无管理凭证的作品需由维护者另行迁移。

| 接口 | 行为 |
|---|---|
| `GET /v1/health` | 限额和协议版本 |
| `PUT /v1/works/{work}` | 凭证绑定新 UUID；相同凭证可幂等重试 |
| `GET /v1/works/{work}` | 私有发布基线与投稿状态 |
| `PUT /v1/works/{work}/submissions/{id}` | 流式 ZIP；必需 `X-Content-SHA256`，相同编号只接受相同内容 |
| `DELETE /v1/works/{work}/submissions/{id}` | 只取消待审稿 |
| `GET /v1/admin/submissions/{id}` | 维护者读取审核资料 |
| `GET /v1/admin/submissions/{id}/archive` | 鉴权取得私有待审包 |
| `POST /v1/admin/submissions/{id}/{action}` | approve、reject、begin、finish |

上传 ZIP 只包含 `submission.json` 及其声明的平铺 PNG。内容上限 512 文件、256 MiB 展开、64 MiB 内容 ZIP；封面与六张附图每张 2 MiB，传输包最多 80 MiB。每件作品最多 10 个同时待审或发布稿、1000 个历史稿；全服务最多 10000 件作品。资源展示名、存储名和应用槽位分别记录，同名资源不能覆盖。预览缩放只修改独立副本。分类索引未收录的新资源需维护者更新受信索引，不能上传自定义服务规则。

状态依次为 pending → approved → publishing → published；另有 rejected、cancelled、withdrawn。仅实际发布成功后推进作品基线。发布开始获得全局持久租约；断线、进程重启后用同一投稿编号和操作重试，其他发布等待该事务完成，不通过超时抢锁覆盖可能已完成的远端写入。

## 审核与发布

维护工具通过 `APMM_REVIEW_URL` 与 `APMM_REVIEW_TOKEN` 连接服务。`tools/review.py issue <id> --output <path>` 仅生成公开 Issue 草稿，包含资料、摘要、数量，不含私有附件地址。`fetch` 需要明确的私有输出位置；请勿把待审包上传到公开 Issue。

批准/拒绝需 `--evidence` JSON，包含 `snapshotHash`、`scanResult`、`gameVersion`、`testEvidence`、`reviewNote`。批准必须对应实际待审包摘要，结构检查不替代扫描或实际游戏验证。

`tools/publish.py <id> --repo <独立资产仓库> --stage <独立暂存> --index <分类清单>` 在本地仓库验证发布；`--push --public-base <固定 CNB 工坊地址>` 才执行远端发布。密钥从 `APMM_PUBLISH_PRIVATE_KEY` 读取，值为 Ed25519 原始 32 字节私钥的 Base64。该密钥不得配置到收件服务。资源先成为不可变哈希文件，签名目录最后写入，并与兼容目录在同一 Git 提交发布。公开内容复核成功后才回写服务；失败用原投稿编号重试。

v2 签名和 Base64 载荷在同一响应内。v1 保留能够以唯一原名称表达且有封面的作品，并生成适配的归档；同名多候选及不能由旧格式表达的作品仅进入 v2。撤回移除两份目录条目，保留用户本地作品与服务器审核记录。

`.github/workflows/workshop-review.yml` 只允许默认分支的受信代码，检查操作者为 maintain/admin，并使用 `workshop-publish` 受保护环境。部署前必须配置该环境的必需维护者审批、默认分支限制和独立最小权限密钥。公开 Issue 由维护者手动 dispatch 生成，审核不是由 Issue 正文里的任意指令触发。

## 生产部署边界

使用 HTTPS 反向代理、专用系统用户、私有数据目录及单个 Uvicorn worker。样例在 `deploy/`。反向代理另配置域名、TLS、IP 限速、管理员访问来源限制、磁盘配额、监控和备份；默认应用限速为每来源每分钟 120 次。不要启用授权头或请求体日志。数据库与私有附件必须在同一备份恢复点。

本次只提供源码、本地隔离验证和部署样例；正式域名、TLS、服务部署、受保护环境设置、实际远端发布与真实游戏测试需在对应环境单独授权和验收。初次生产密钥必须匹配桌面内置的工坊发布公钥，不能使用本地测试密钥。
