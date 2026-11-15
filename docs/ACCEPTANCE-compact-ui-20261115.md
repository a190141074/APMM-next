# 紧凑资源布局与任务进程小窗

资源库原先把英文装饰标题、宣传说明和筛选分成多层，挤占了图片浏览空间。本次将资源标题、数量、筛选、尺寸、全选和扫描收为工具栏，移除宣传说明，将空资源卡片的“等待你的创作”改为事实状态“无 Mod”。侧栏“工具”改为“关于”，任务入口移至主题切换旁，通过 Radix Popover 向下展开。

窗口宽度 1089×743 时工具栏高 38px，资源列表从 y=155 开始；900px 宽时标题和筛选自然分两行，列表从 y=198 开始。切换小/中/大预览会清除虚拟列表的旧行高，避免卡片重叠或出现额外空隙。

任务仍采用 App 现有的后端查询快照和 revision 合并规则。小窗只维护展开状态，不增加任务副本；保留进度、速度、取消、暂停、继续、重试和结果查看。读取失败时保留缓存任务并提供重新读取。Esc 关闭后焦点返回入口，也可点外部关闭；状态栏和关闭窗口保护均打开同一小窗。

## 受影响文件

```text
apmm-desktop/
├─ package.json                         Radix Popover 固定依赖
├─ pnpm-lock.yaml                       依赖锁定
├─ src-ui/
│  ├─ app/App.tsx                       导航、标题栏入口及关闭保护
│  ├─ compact-workspace.css             紧凑工具栏、任务浮层和内部滚动
│  ├─ main.tsx                          加载布局样式
│  ├─ core/domain/resources.ts          移除任务页面路由
│  ├─ core/services/app.ts              修正后端队列职责说明
│  └─ modules/
│     ├─ ResourcePage.tsx               工具栏、中立文案、虚拟行高刷新
│     └─ TaskPopover.tsx                从 TasksPage.tsx 迁移任务展示与控制
├─ tools/
│  ├─ verify-compact-ui.mjs              隔离浏览器的针对性验收
│  └─ verify-native.mjs                  原有验收入口改用标题栏任务按钮
├─ verification-compact-ui.json          实测几何、检查结果和截图目录
└─ docs/
   ├─ FILE-MAP.md                       更新文件职责和行数
   └─ ACCEPTANCE-compact-ui-20261115.md  本次验收记录
```

新增和本次明显扩展的手写文件均低于 500 行。App 475 行保持导航编排职责；任务展示迁至独立组件。

## 已执行验证

- `pnpm typecheck`：通过。
- `node node_modules/vitest/vitest.mjs run src-ui/core/domain/domain.test.ts --testNamePattern '迟到的轮询|搜索完整官方后缀|智能搜索'`：3 项通过，5 项无关测试跳过。
- `node tools/verify-compact-ui.mjs`：9 项通过，无页面运行异常。覆盖真实空态、浅/深色、1089/900/1360 宽度、900×600 浮层边界、键盘及外部关闭、状态栏入口、关闭保护、任务六种状态、控制命令与任务 ID、结果展开、轮询进度更新、旧 revision 不回退、错误重读和内部滚动。
- `pnpm exec vite build --outDir B:/APMM-next-work/ui-refinement-20261115-000915/frontend`：通过，实际前端产物在源码以外的 B 盘隔离目录。

任务按钮和状态转换使用只注入隔离测试页面的传输夹具，正常预览仍返回真实的空任务列表。没有把虚假任务写入业务代码。

## 证据与边界

源码备份：`B:\APMM-next-work\ui-refinement-20261115-000915`。

截图目录：`B:\APMM-next-work\compact-ui-validation\faf59a58-9585-4a26-9384-38f2c40fd45d`，包括资源页浅/深色、任务空态、多状态和窄窗口截图。

本次仅更改前端界面，没有改变 Rust、任务协议、资源身份或工作区数据。未运行新的真实桌面 IPC 验收，未构建、部署新的 EXE 或 ZIP；现有 0.3.38 便携包保持之前的产物。未验证 Linux 桌面和游戏实机行为。
