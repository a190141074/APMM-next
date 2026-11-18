# 发行与预览资源

源码仓库只保存应用源码、共享配置、可复现构建工具和长期维护的文档。测试结果、差异清单、环境指纹和工作记录保存到源码之外的 `APMM_REPORT_DIR`，默认是工作盘的 `reports/<版本>/`。`docs/FILE-MAP.md` 是人工维护的导航，不再由一次验收的文件列表覆盖。

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

每个包旁有同名 `.sha256`。标准下载页只展示便携 ZIP 与 MSI；旧版桥接与测试版兼容包放在单独目录，并说明适用的旧版本。

1. 同步 `VERSION`、主程序配置和更新日志，执行 `node tools/build-portable.mjs`。
2. `APMM_BUILD_REPORT` 指向本轮便携构建报告，执行 `node tools/build-distributions.mjs`，派生 MSI 与兼容包。它核对输入摘要及 MSI 使用的二进制，拒绝混用旧构建。
3. 执行 `tools/verify-release.py`、`tools/verify-distributions.py`。MSI 可用 `tools/verify-msi.ps1` 在独立目录管理展开，不注册安装。原生界面验收使用独立 Windows desktop。
4. 发布前核对源码审计、包摘要、安装身份、版本、许可和隔离运行结果。确认远端目标后，先上传版本目录，再回读核验，最后更新当前清单。

根目录 `update-manifest-v2.json` 是新客户端入口，按安装身份选择标准包；`update-manifest.json` 是旧版桥接入口。发布工具使用完整 CNB 版本目录 URL。已被旧客户端或旧清单引用的平铺路径保留为兼容入口，不再用于放置新的常规发行物。

## 原图预览

预览与软件包独立更新，不能用界面截图或本机测试副本替代远端预览包。`tools/build-preview-bundles.py` 接受官方资产工具的 `maps/`、`outimg/`、`state.json`，逐张验证 PNG 来源摘要，沿用旧项目的有损 WebP 质量 `quality=50`，保留完整名称、原尺寸与透明度。编码搜索参数使用 `method=4`，平衡构建耗时和下载体积；旧转换器使用 `method=6`。输出逐张校验格式、尺寸和透明通道，并在发布前抽样检查画质、统计实际下载体积。新输出必须使用空目录；PNG 原图保留在本地，不进入预览包。

当前游戏已移除但旧资源目录仍保留的项目，只能沿用已核对的历史预览，不能标记为本轮重新提取。其名单和来源摘要写入本地构建报告。

```text
preview-manifest-v4.json       当前客户端完整清单
bundles/preview-v4/            五类资源，角色按部位分为四包，共八包
preview-manifest.json          旧客户端兼容清单
bundles/*-compat.zip           旧客户端所需的平铺兼容入口
```

旧客户端的资源白名单固定在其二进制中，不能直接给它们追加资源。因此兼容清单只刷新旧文件集合，完整新资源通过独立清单交给新客户端。两套包复用同一份压缩 WebP 字节，不二次压缩；完整包按分类和角色部位供应用选择下载。每包仍须小于 256 MiB，不能用拆包替代压缩。已有 `preview-v3` 文件保留原字节，避免中断正在下载的旧清单。

上线顺序是预览包、摘要核验、预览清单。应用更新清单只能在相应的完整预览清单可读取后切换。新旧客户端的 GitHub 兜底也分别读取完整和兼容清单，不能因相同版本号而混用。发布完成必须回读公网清单和实际 ZIP，并重新核对大小、SHA-256 与文件数；本地准备完成不代表远端已生效。
