// 发行副本的真实作者 IPC 验收，所有文件和窗口资料均隔离到 B 盘。
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { createInterface } from "node:readline";
import { chromium } from "playwright";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { randomUUID, createHash } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const build = JSON.parse(readFileSync(process.env.APMM_BUILD_REPORT, "utf8"));
const expectedVersion = readFileSync(
  new URL("../VERSION", import.meta.url),
  "utf8",
).trim();
assert.equal(
  build.version,
  expectedVersion,
  "原生验收必须使用当前源码版本的构建",
);
const launcherPath = process.env.APMM_BACKGROUND_LAUNCHER;
const python = process.env.APMM_TEST_PYTHON;
assert.ok(
  launcherPath && python,
  "请指定后台独立桌面启动器与 Python，避免窗口打扰用户",
);
const output = path.join(
  process.env.APMM_WORKSHOP_WORK ||
    `B:/APMM-next-work/workshop-cards-${build.version.replaceAll(".", "")}`,
  "native",
  randomUUID(),
);
assert.ok(
  path.resolve(output).toLowerCase().startsWith("b:\\apmm-next-work\\"),
);
const appDir = path.join(output, "app");
cpSync(build.payload, appDir, { recursive: true });
for (const file of build.files) {
  const bytes = readFileSync(path.join(appDir, file.path));
  assert.equal(bytes.length, file.bytes);
  assert.equal(createHash("sha256").update(bytes).digest("hex"), file.sha256);
}
mkdirSync(path.join(appDir, "data"), { recursive: true });
writeFileSync(
  path.join(appDir, "data/library.json"),
  JSON.stringify({
    version: 1,
    settings: { updateAutoCheck: false },
    heroes: {},
    monsters: {},
  }),
);
const debugPort = await new Promise((resolve) => {
  const server = createServer();
  server.listen(0, "127.0.0.1", () => {
    const port = server.address().port;
    server.close(() => resolve(port));
  });
});
const child = spawn(
  python,
  [
    "-X",
    "utf8",
    launcherPath,
    path.join(appDir, "astral-mod-manager.exe"),
    path.join(output, "heartbeat.jsonl"),
  ],
  {
    cwd: appDir,
    windowsHide: true,
    stdio: ["pipe", "pipe", "pipe"],
    env: {
      ...process.env,
      APMM_WORK_DIR: "B:/APMM-next-work",
      TEMP: "B:/APMM-next-work/temp",
      TMP: "B:/APMM-next-work/temp",
      WEBVIEW2_USER_DATA_FOLDER: path.join(output, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
    },
  },
);
child.stderr.on("data", (bytes) => process.stderr.write(bytes));
const exited = new Promise((resolve) => child.once("exit", resolve));
let browser, page;
let native;
let passed = false;
const checks = [],
  errors = [];
try {
  native = await new Promise((resolve, reject) => {
    const lines = createInterface({ input: child.stdout });
    lines.once("line", (line) => {
      lines.close();
      try {
        resolve(JSON.parse(line));
      } catch (error) {
        reject(error);
      }
    });
    child.once("error", reject);
    child.once("exit", (code) => reject(Error(`独立桌面启动器退出：${code}`)));
  });
  for (let attempt = 0; attempt < 80; attempt++) {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`);
      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  assert.ok(browser, "发行副本没有启动 WebView2");
  // WebView2 调试端口可能先暴露空白页；持续查找已注入 IPC 的应用页。
  for (let attempt = 0; attempt < 120; attempt++) {
    const candidates = browser
      .contexts()
      .flatMap((c) => c.pages())
      .filter((p) => !p.isClosed() && !p.url().startsWith("devtools:"));
    const ready = await Promise.all(
      candidates.map((candidate) =>
        candidate
          .evaluate(() => !!window.__TAURI_INTERNALS__)
          .catch(() => false),
      ),
    );
    page = candidates.find((_, index) => ready[index]);
    if (page) break;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  assert.ok(page, "发行副本没有出现已注入 IPC 的应用页");
  page.on("pageerror", (e) => errors.push(e.message));
  const welcome = page.getByRole("button", {
    name: "先进入工作区",
    exact: true,
  });
  await welcome.waitFor();
  await welcome.click();
  const call = (command, args = {}) =>
    page.evaluate(
      ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
      { command, args },
    );
  const actual = await call("get_data_dir");
  assert.equal(path.resolve(actual), path.resolve(appDir, "data"));
  await page.locator(".nav-item").filter({ hasText: "设置" }).first().click();
  await page.getByRole("tab", { name: "工坊", exact: true }).click();
  await page.getByRole("checkbox", { name: /^作者模式/ }).click();
  await page.locator(".nav-item").filter({ hasText: "投稿" }).first().waitFor();
  assert.equal((await call("workspace_snapshot")).settings.authorMode, true);
  assert.equal((await call("author_service")).available, false);
  const first = await call("author_new", { workId: null });
  const source = process.env.APMM_WORKSHOP_EXPORT_FIXTURE;
  let draft = await call("author_add_source", {
    id: first.id,
    path: source,
    password: "",
    preview: false,
  });
  assert.equal((await call("workspace_snapshot")).version, build.version);
  assert.equal(draft.metadata.artist, "画师测试");
  assert.deepEqual(draft.metadata.authorUrls, ["https://example.com/author"]);
  assert.equal(draft.metadata.sourceUrl, "https://example.com/source");
  const coverSource = path.join(
    appDir,
    "data/author/blobs",
    draft.images[0].sha256 + ".png",
  );
  for (let i = 0; i < 6; i++)
    draft = await call("author_add_source", {
      id: draft.id,
      path: coverSource,
      password: "",
      preview: true,
    });
  const oldImages = structuredClone(draft.images);
  draft = await call("author_set_cover", {
    id: draft.id,
    revision: draft.revision,
    path: coverSource,
  });
  assert.equal(draft.images.length, 7);
  assert.notEqual(draft.images[0].id, oldImages[0].id);
  assert.deepEqual(draft.images.slice(1), oldImages.slice(1));
  await assert.rejects(() =>
    call("author_set_cover", {
      id: draft.id,
      revision: draft.revision - 1,
      path: coverSource,
    }),
  );
  const invalid = path.join(output, "bad.png");
  writeFileSync(invalid, "bad png");
  await assert.rejects(() =>
    call("author_set_cover", {
      id: draft.id,
      revision: draft.revision,
      path: invalid,
    }),
  );
  assert.deepEqual(
    (await call("author_state")).drafts.find((d) => d.id === draft.id).images,
    draft.images,
  );
  checks.push("真实 IPC：满七张时替换主封面、附图保持、旧修订与无效图片拒绝");
  const hash = draft.files[0].sha256;
  const oldRevision = draft.revision;
  draft.metadata.title = "原生离线导出验证";
  draft = await call("author_save", { draft });
  await assert.rejects(() =>
    call("author_save", { draft: { ...draft, revision: oldRevision } }),
  );
  const preview = await call("author_preflight", { id: draft.id });
  const dest = path.join(output, "native-submission.zip");
  await call("author_export", { id: preview.id, destination: dest });
  const receipt = (await call("author_state")).receipts.find(
    (r) => r.id === preview.id,
  );
  assert.equal(receipt.status, "exported");
  assert.equal(
    receipt.snapshotHash,
    createHash("sha256").update(readFileSync(dest)).digest("hex"),
  );
  const next = await call("author_new", { workId: null });
  const restored = await call("author_add_source", {
    id: next.id,
    path: dest,
    password: "",
    preview: false,
  });
  assert.equal(restored.files[0].sha256, hash);
  assert.equal(restored.metadata.title, "原生离线导出验证");
  assert.deepEqual(restored.metadata, draft.metadata);
  assert.deepEqual(
    restored.images.map((f) => f.sha256),
    draft.images.map((f) => f.sha256),
  );
  const state = JSON.parse(
    readFileSync(path.join(appDir, "data/author/state.json"), "utf8"),
  );
  assert.equal(state.receipts[0].status, "exported");
  const library = JSON.parse(
    readFileSync(path.join(appDir, "data/library.json"), "utf8"),
  );
  assert.equal(Object.keys(library.heroes || {}).length, 0);
  assert.equal(Object.keys(library.monsters || {}).length, 0);
  checks.push(
    "真实 IPC：无投稿服务的导入、保存、旧修订拒绝、预检、导出和重新导入",
  );
  checks.push("磁盘回执仅为已导出，原始 PNG 摘要保持，资源库未加入任何候选");
  await page.locator(".nav-item").filter({ hasText: "投稿" }).first().click();
  await page.getByRole("button", { name: "编辑", exact: true }).first().click();
  assert.equal(
    await page.getByLabel("名称 *", { exact: true }).inputValue(),
    "原生离线导出验证",
  );
  assert.equal(
    await page.getByLabel("画师", { exact: true }).inputValue(),
    "画师测试",
  );
  await page.locator(".author-card-preview").scrollIntoViewIfNeeded();
  await page.locator(".author-card-preview img").waitFor();
  await page.screenshot({
    path: path.join(output, "native-author-preview.png"),
  });
  await page
    .getByRole("button", { name: "预览投稿与导出", exact: true })
    .click();
  await page
    .getByRole("dialog", { name: "确认投稿快照", exact: true })
    .locator(".author-card-preview")
    .waitFor();
  await page.screenshot({ path: path.join(output, "native-snapshot.png") });
  checks.push("发行 WebView：新作者资料、真实封面及冻结快照共用工坊卡片展示");
  assert.deepEqual(errors, []);
  passed = true;
} catch (error) {
  errors.push(error.stack || String(error));
  throw error;
} finally {
  if (page && !page.isClosed())
    await page
      .screenshot({ path: path.join(output, "last-state.png") })
      .catch(() => {});
  writeFileSync(
    path.join(output, "report.json"),
    JSON.stringify(
      {
        output,
        passed,
        version: build.version,
        native,
        build: build.sha256,
        checks,
        errors,
        pages: browser
          ?.contexts()
          .flatMap((c) => c.pages().map((p) => p.url())),
      },
      null,
      2,
    ),
  );
  await browser?.close().catch(() => {});
  if (child.exitCode === null) child.stdin.end("stop\n");
  await exited;
}
console.log(JSON.stringify({ output, checks, errors }, null, 2));
