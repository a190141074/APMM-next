import { portableBuildReport } from "./work-paths.mjs";
// 使用发行包的独立副本验证配色 WebView 与真实 IPC；不操作系统取色窗口或实际安装。
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
import { chromium } from "playwright";

const root = path.resolve(import.meta.dirname, "..");
const build = JSON.parse(
  readFileSync(portableBuildReport, "utf8"),
);
const output = path.join(
  process.env.APMM_WORK_DIR || "B:/APMM-next-work",
  "appearance-validation",
  randomUUID(),
);
const app = path.join(output, "app");
cpSync(build.payload, app, { recursive: true });
mkdirSync(path.join(app, "data"), { recursive: true });
writeFileSync(
  path.join(app, "data/library.json"),
  JSON.stringify({
    version: 1,
    heroes: {},
    monsters: {},
    settings: { updateAutoCheck: false, monitorDirectories: false },
  }),
);
const port = await new Promise((resolve) => {
  const server = createServer();
  server.listen(0, "127.0.0.1", () => {
    const port = server.address().port;
    server.close(() => resolve(port));
  });
});
const env = {
  ...process.env,
  APMM_WORK_DIR: "B:/APMM-next-work",
  TEMP: "B:/APMM-next-work/temp",
  TMP: "B:/APMM-next-work/temp",
  WEBVIEW2_USER_DATA_FOLDER: path.join(output, "webview"),
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
};
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
let child, browser, page;
const checks = [];
async function start() {
  child = spawn(path.join(app, "astral-mod-manager.exe"), [], {
    cwd: app,
    env,
    stdio: "ignore",
    windowsHide: true,
  });
  for (let n = 0; n < 100; n++) {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
      break;
    } catch {
      await delay(200);
    }
  }
  if (!browser) throw Error("隔离 WebView 未启动");
  page = browser.contexts().flatMap((c) => c.pages())[0];
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__);
}
async function stop() {
  const process = child;
  await page
    .evaluate(() =>
      window.__TAURI_INTERNALS__.invoke("plugin:window|close", {
        label: "main",
      }),
    )
    .catch(() => {});
  await browser?.close().catch(() => {});
  browser = null;
  for (let n = 0; n < 50 && process.exitCode === null; n++) await delay(100);
  if (process.exitCode === null) process.kill();
}
const settings = () =>
  JSON.parse(readFileSync(path.join(app, "data/library.json"), "utf8"))
    .settings;
async function until(check) {
  for (let n = 0; n < 100; n++) {
    if (check()) return;
    await delay(100);
  }
  assert.ok(check());
}
async function nav(name) {
  await page
    .locator(".sidebar")
    .getByRole("button", { name, exact: true })
    .click();
}
try {
  await start();
  await page.getByRole("button", { name: "先进入工作区", exact: true }).click();
  await nav("设置");
  await page.getByRole("tab", { name: "配色", exact: true }).click();
  await page.getByRole("button", { name: "新建方案", exact: true }).click();
  await page
    .getByRole("textbox", { name: "方案名称", exact: true })
    .fill("原生验收方案");
  await page.getByRole("tab", { name: "亮色方案", exact: true }).click();
  await page.getByRole("textbox", { name: "HEX", exact: true }).fill("#123456");
  await page.getByRole("tab", { name: "暗色方案", exact: true }).click();
  await page.getByRole("textbox", { name: "HEX", exact: true }).fill("#abcdef");
  await page.getByRole("button", { name: "保存配色", exact: true }).click();
  await until(() => settings().appearance?.schemes.length === 1);
  let saved = settings().appearance;
  assert.equal(saved.schemes[0].light.accent, "#123456");
  assert.equal(saved.schemes[0].dark.accent, "#abcdef");
  assert.equal(saved.mode, "system");
  checks.push("亮暗独立保存且保留跟随系统");
  await page
    .getByRole("button", { name: "编辑 / 重命名", exact: true })
    .click();
  await page.getByRole("textbox", { name: "HEX", exact: true }).fill("#ff0000");
  await page.getByRole("button", { name: "重置整套配色", exact: true }).click();
  assert.deepEqual(settings().appearance, saved);
  await page.getByRole("button", { name: "取消", exact: true }).click();
  await page.getByRole("button", { name: "丢弃修改", exact: true }).click();
  assert.deepEqual(settings().appearance, saved);
  checks.push("取消草稿不写入持久设置");
  const exported = path.join(output, "theme.json");
  await page.evaluate(
    ({ destination, scheme }) =>
      window.__TAURI_INTERNALS__.invoke("theme_export", {
        destination,
        scheme,
      }),
    { destination: exported, scheme: saved.schemes[0] },
  );
  const themeFile = JSON.parse(readFileSync(exported, "utf8"));
  assert.equal(themeFile.schema, 1);
  assert.equal(themeFile.type, "apmm-theme");
  await page
    .getByLabel("导入配色文件", { exact: true })
    .setInputFiles(exported);
  await page.getByRole("button", { name: "保存配色", exact: true }).click();
  await until(() => settings().appearance.schemes.length === 2);
  assert.notEqual(
    settings().appearance.schemes[0].id,
    settings().appearance.schemes[1].id,
  );
  checks.push("方案导出与导入生成独立身份");
  const invalid = structuredClone(settings().appearance);
  invalid.schemes[0].light.accent = "url(bad)";
  const rejected = await page.evaluate(async (appearance) => {
    try {
      await window.__TAURI_INTERNALS__.invoke("settings_set", {
        patch: { appearance },
      });
      return "";
    } catch (e) {
      return String(e);
    }
  }, invalid);
  assert.match(rejected, /颜色/);
  checks.push("真实 IPC 拒绝非法色值");
  await page.getByRole("tab", { name: "外观", exact: true }).click();
  await page.getByRole("checkbox", { name: /^减少动画/ }).click();
  await page.getByRole("checkbox", { name: /加载时显示吸猫动画/ }).click();
  await until(
    () =>
      settings().appearance.reduceMotion && settings().appearance.catAnimation,
  );
  checks.push("连续设置保留两个字段");
  await stop();
  await start();
  await nav("设置");
  await page.getByRole("tab", { name: "配色", exact: true }).click();
  await page
    .getByRole("button", { name: "编辑 / 重命名", exact: true })
    .first()
    .waitFor();
  assert.equal(settings().appearance.schemes[0].light.accent, "#123456");
  checks.push("退出并重启后配色仍保留");
  await page.getByRole("tab", { name: "外观", exact: true }).click();
  await page.getByRole("checkbox", { name: /^减少动画/ }).click();
  await until(() => !settings().appearance.reduceMotion);
  for (const name of ["总览", "设置", "调试", "总览", "调试"]) await nav(name);
  await page.getByRole("heading", { name: "调试", exact: true }).waitFor();
  assert.equal(await page.getByText("敬请期待", { exact: true }).count(), 1);
  checks.push("快速切页最终目标正确");
  await nav("APX 插件");
  const tabs = page.getByRole("tablist", { name: "选择客户端", exact: true });
  await tabs.waitFor();
  assert.equal(await tabs.getByRole("tab").count(), 6);
  assert.doesNotMatch(await tabs.innerText(), /WINDOWS|ANDROID/);
  await tabs.getByRole("tab").first().focus();
  await page.keyboard.press("ArrowRight");
  assert.equal(
    await tabs
      .getByRole("tab")
      .nth(1)
      .evaluate((e) => e === document.activeElement),
    true,
  );
  assert.equal(
    await tabs.getByRole("tab").first().getAttribute("aria-selected"),
    "true",
  );
  checks.push("六个客户端单行标签及手动键盘激活");
  await nav("设置");
  const layout = [];
  for (const width of [800, 945, 1360]) {
    await page.setViewportSize({ width, height: 905 });
    await page.waitForFunction(() =>
      document.getAnimations().every((a) => a.playState !== "running"),
    );
    const measured = await page
      .locator(".settings-nav")
      .evaluate((e) => ({
        width: e.clientWidth,
        scroll: e.scrollWidth,
        rows: new Set(
          [...e.children].map((x) => Math.round(x.getBoundingClientRect().top)),
        ).size,
        overflow: document.documentElement.scrollWidth > innerWidth,
        documentWidth: document.documentElement.scrollWidth,
        viewport: innerWidth,
      }));
    assert.equal(measured.rows, 1);
    assert.equal(
      measured.overflow,
      false,
      JSON.stringify({ width, ...measured }),
    );
    layout.push({ requestedWidth: width, ...measured });
  }
  checks.push("800/945/1360 宽度横向设置且页面无横向溢出");
  const screenshots = path.join(output, "screenshots");
  mkdirSync(screenshots);
  await page.screenshot({ path: path.join(screenshots, "settings.png") });
  await page.getByRole("tab", { name: "配色", exact: true }).click();
  await page
    .getByRole("button", { name: "编辑 / 重命名", exact: true })
    .first()
    .click();
  await page.waitForFunction(() => document.getAnimations().every(a => a.playState !== "running"));
  await page.screenshot({ path: path.join(screenshots, "theme-editor.png") });
  await page.getByRole("button", { name: "取消", exact: true }).click();
  await nav("APX 插件");
  await page.waitForFunction(() => document.getAnimations().every(a => a.playState !== "running"));
  await page.screenshot({ path: path.join(screenshots, "apx-tabs.png") });
  const report = {
    version: build.version,
    output,
    checks,
    layout,
    screenPickerTested: false,
    systemFileDialogTested: false,
  };
  writeFileSync(
    path.join(root, "verification-appearance-native.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report));
} catch (error) {
  if (page) {
    await page
      .screenshot({ path: path.join(output, "failure.png") })
      .catch(() => {});
    writeFileSync(
      path.join(output, "failure.txt"),
      await page
        .locator("body")
        .innerText()
        .catch(() => ""),
    );
  }
  console.error(JSON.stringify({ output, checks }));
  throw error;
} finally {
  if (browser) await stop();
  else if (child?.exitCode === null) child.kill();
}
