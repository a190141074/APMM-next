// 使用本轮预览缓存验证发行 WebView 的资源来源与原图解码；窗口位于独立桌面。
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
const sourceRoot = path.resolve(import.meta.dirname, "..");
assert.ok(build.sourceInputs?.length, "需要记录当前程序输入的正式构建报告");
for (const input of build.sourceInputs) {
  assert.equal(
    createHash("sha256")
      .update(readFileSync(path.join(sourceRoot, input.path)))
      .digest("hex"),
    input.sha256,
    input.path,
  );
}
const launcherPath = process.env.APMM_BACKGROUND_LAUNCHER;
const python = process.env.APMM_TEST_PYTHON;
assert.ok(
  launcherPath && python,
  "请指定后台独立桌面启动器与 Python，避免窗口打扰用户",
);
const output = path.join(
  process.env.APMM_PREVIEW_WORK ||
    `B:/APMM-next-work/preview-${build.version.replaceAll(".", "")}`,
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
assert.ok(
  process.env.APMM_PREVIEW_CACHE,
  "需要已通过真实安装协议验收的预览缓存",
);
cpSync(process.env.APMM_PREVIEW_CACHE, path.join(appDir, "data/previews"), {
  recursive: true,
});
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
  assert.equal(
    path.resolve(await call("get_data_dir")),
    path.resolve(appDir, "data"),
  );
  let snapshot;
  for (let attempt = 0; attempt < 40; attempt++) {
    snapshot = await call("workspace_snapshot");
    if (snapshot.resources.every((row) => row.originalStatus === "cache"))
      break;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  assert.equal(snapshot.version, build.version);
  assert.equal(snapshot.gameDir, null);
  const status = await call("preview_status");
  assert.equal(status.expectedFiles, 2187);
  assert.equal(status.cachedFiles, 2187);
  assert.equal(status.missingFiles, 0);
  assert.equal(status.bundles.length, 8);
  for (const category of [
    "heroes",
    "cards",
    "monsters",
    "chips",
    "lands",
    "cursors",
  ]) {
    const rows = snapshot.resources.filter((r) => r.category === category);
    assert.ok(rows.length, category);
    assert.ok(
      rows.every((r) => r.originalUrl && r.originalStatus === "cache"),
      category,
    );
  }
  checks.push(
    "发行程序版本、数据目录与八个预览分包均来自本轮构建；2187 个资源原图无缺失",
  );
  await page.locator(".nav-item").filter({ hasText: "卡面" }).first().click();
  await page.locator(".resource-page img").first().waitFor();
  await page.waitForFunction(() => {
    const visible = [...document.querySelectorAll(".resource-page img")].filter(
      (image) => {
        const rect = image.getBoundingClientRect();
        return (
          rect.width > 0 &&
          rect.height > 0 &&
          rect.bottom > 0 &&
          rect.top < innerHeight
        );
      },
    );
    return (
      visible.length > 0 &&
      visible.every((image) => image.complete && image.naturalWidth > 0)
    );
  });
  await page.screenshot({ path: path.join(output, "preview-cards.png") });
  checks.push(
    "正式 WebView 原图协议成功解码卡面预览，未依赖本机 outimg 或游戏目录",
  );
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
