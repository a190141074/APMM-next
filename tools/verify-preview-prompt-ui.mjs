// 在隔离浏览器资料中验证缺图提醒、设置持久化和原图定位布局。
import { chromium } from "playwright";
import { mkdirSync, existsSync, writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const output = path.join(
  process.env.APMM_PREVIEW_UI_WORK || "B:/APMM-next-work/preview-prompt-ui",
  randomUUID(),
);
mkdirSync(output, { recursive: true });
const executablePath = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
assert.ok(executablePath, "未找到已安装浏览器");
const context = await chromium.launchPersistentContext(
  path.join(output, "profile"),
  { executablePath, headless: true, viewport: { width: 1014, height: 796 } },
);
const page = await context.newPage();
const picture =
  "data:image/svg+xml," +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" width="120" height="160"><rect width="120" height="160" fill="#7089b0"/></svg>',
  );
const resource = (id, outimgPath) => ({
  key: "UT_Card_" + id,
  category: "cards",
  entityId: String(id),
  entityName: "测试卡面",
  nameKnown: true,
  aliases: [],
  section: "card",
  sectionLabel: "卡面",
  resourceLabel: "卡面",
  variantKey: "default",
  variantLabel: "默认",
  variantOrder: 0,
  canonical: "UT_Card_" + id,
  groupKey: String(id),
  fullName: "UT_Card_" + id,
  applicationSlot: "UT_Card_" + id,
  originalUrl: picture,
  outimgPath,
  originalStatus: "ready",
  files: [
    {
      hash: "a".repeat(64),
      path: picture,
      sourceName: "候选.png",
    },
  ],
  activeHash: null,
  disabled: false,
});
const resources = [
  resource(10001, "B:/APMM-next-work/preview-prompt-ui/outimg/UT_Card_10001.png"),
  resource(10002, undefined),
];
await context.addInitScript(() => {
  if (localStorage.getItem("apmm.welcomed") === null)
    localStorage.setItem("apmm.welcomed", "true");
  if (localStorage.getItem("apmm.reduce-motion") === null)
    localStorage.setItem("apmm.reduce-motion", "true");
  if (localStorage.getItem("apmm.demo-preview-download-prompt") === null)
    localStorage.setItem("apmm.demo-preview-download-prompt", "true");
  if (localStorage.getItem("apmm.preview-missing") === null)
    localStorage.setItem("apmm.preview-missing", "true");
  if (localStorage.getItem("apmm.preview-download-count") === null)
    localStorage.setItem("apmm.preview-download-count", "0");
  window.__previewUi ??= { revealCalls: 0 };
});
await page.route("**/__workspace", (route) =>
  route.fulfill({ json: { resources, gameDir: "隔离预览夹具" } }),
);
await page.route("**/src-ui/core/services/app.ts*", async (route) => {
  const response = await route.fetch();
  const body = await response.text();
  const shim = [
    "const previewUiCommand = appService.command;",
    "appService.native = true;",
    "appService.command = async (name, args = {}) => {",
    "  if (name === 'preview_status') { const missing = localStorage.getItem('apmm.preview-missing') === 'true'; return {expectedFiles: 4, missingFiles: missing ? 4 : 0, bundles: [{localVersion: missing ? null : 'preview-v1'}]}; }",
    "  if (name === 'migration_status') return {mode: 'inherited', destination: 'B:/APMM-next-work/preview-fixture', mods: 2, multipleSlots: 0, sources: []};",
    "  if (name === 'update_startup_status') return null;",
    "  if (name === 'update_check') return {available: false, currentVersion: '0.4.11', latestVersion: '0.4.11'};",
    "  if (name === 'outimg_reveal') { window.__previewUi.revealCalls++; return null; }",
    "  return previewUiCommand(name, args);",
    "};",
    "const previewUiTask = appService.task;",
    "appService.task = async (kind, title, args) => {",
    "  if (kind === 'preview') { localStorage.setItem('apmm.preview-download-count', String(Number(localStorage.getItem('apmm.preview-download-count') || 0) + 1)); return 'preview-ui-task'; }",
    "  return previewUiTask(kind, title, args);",
    "};",
  ].join("\n");
  await route.fulfill({ response, body: body + "\n" + shim });
});
const checks = [];
try {
  await page.goto(process.env.APMM_UI_URL || "http://127.0.0.1:1421/");
  const dialog = page.getByRole("dialog", { name: "尚未下载预览图" });
  await dialog.waitFor();
  assert.match(await dialog.innerText(), /4 张/);
  assert.match(await dialog.innerText(), /设置 → 目录与资源/);
  assert.equal(await dialog.getByRole("button").count(), 4);
  await page.screenshot({ path: path.join(output, "preview-prompt.png") });
  await dialog.getByRole("button", { name: "是", exact: true }).click();
  assert.equal(
    await page.evaluate(() => localStorage.getItem("apmm.preview-download-count")),
    "1",
  );
  checks.push("启动提示的“是”会启动一次预览下载任务");

  await page.reload();
  await page.getByRole("dialog", { name: "尚未下载预览图" }).waitFor();
  await page.getByRole("button", { name: "否", exact: true }).click();
  assert.equal(await page.getByRole("dialog").count(), 0);
  assert.equal(
    await page.evaluate(() => localStorage.getItem("apmm.preview-download-count")),
    "1",
  );
  checks.push("“否”仅关闭本次提示，不发起下载");

  await page.reload();
  await page.getByRole("dialog", { name: "尚未下载预览图" }).waitFor();
  await page
    .getByRole("button", { name: "不再提醒", exact: true })
    .click();
  await page.waitForFunction(
    () => localStorage.getItem("apmm.demo-preview-download-prompt") === "false",
  );
  await page.reload();
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page.locator('main[data-route="settings"]').waitFor();
  const promptToggle = page.getByRole("checkbox", {
    name: /缺少预览图时提醒/,
  });
  assert.equal(await promptToggle.isChecked(), false);
  await promptToggle.setChecked(true);
  await page.getByRole("dialog", { name: "尚未下载预览图" }).waitFor();
  await page.getByRole("button", { name: "否", exact: true }).click();
  await page
    .getByRole("button", { name: "下载预览图", exact: true })
    .click();
  await page.waitForFunction(
    () => localStorage.getItem("apmm.preview-download-count") === "2",
  );
  checks.push("不再提醒设置持久化；设置中可重新开启，并可手动下载");

  await page.getByRole("button", { name: "卡面", exact: true }).click();
  await page.locator('main[data-route="cards"]').waitFor();
  await page.getByRole("button", { name: "列表模式", exact: true }).click();
  const reveal = page.getByRole("button", {
    name: "定位原图文件 UT_Card_10001",
  });
  await reveal.waitFor();
  assert.equal(await page.locator(".candidate-reveal").count(), 1);
  const layout = async () =>
    reveal.evaluate((button) => {
      const rect = button.getBoundingClientRect();
      const strip = button.parentElement.querySelector(".candidate-strip").getBoundingClientRect();
      return {
        width: rect.width,
        height: rect.height,
        rightOfCandidates: rect.left >= strip.right - 1,
        color: getComputedStyle(button).backgroundColor,
      };
    });
  let buttonBox = await layout();
  assert.ok(buttonBox.height > buttonBox.width, "原图定位按钮为竖向长方形");
  assert.ok(buttonBox.rightOfCandidates, "定位按钮位于候选条右侧空位");
  assert.notEqual(buttonBox.color, "rgba(0, 0, 0, 0)");
  await reveal.click();
  assert.equal(await page.evaluate(() => window.__previewUi.revealCalls), 1);
  await page.screenshot({ path: path.join(output, "outimg-reveal.png") });
  await page.setViewportSize({ width: 700, height: 796 });
  buttonBox = await layout();
  assert.ok(buttonBox.height > buttonBox.width);
  assert.ok(buttonBox.rightOfCandidates, "窄窗定位按钮仍在候选条右侧");
  checks.push("outimg 按钮仅按存在的路径显示，醒目竖排并保持在宽/窄窗候选区域右侧");

  await page.evaluate(() =>
    localStorage.setItem("apmm.preview-missing", "false"),
  );
  await page.reload();
  await page.locator(".sidebar").waitFor();
  await page.waitForTimeout(150);
  assert.equal(await page.getByRole("dialog").count(), 0);
  checks.push("本地预览缓存完整时启动不显示下载提醒");

  writeFileSync(
    path.join(output, "result.json"),
    JSON.stringify({ status: "passed", checks }, null, 2),
  );
  console.log(JSON.stringify({ output, checks }, null, 2));
} catch (error) {
  await page.screenshot({ path: path.join(output, "failure.png") }).catch(() => {});
  writeFileSync(
    path.join(output, "result.json"),
    JSON.stringify({ status: "failed", checks, error: String(error), stack: error.stack }, null, 2),
  );
  console.error(JSON.stringify({ output, checks, error: String(error) }, null, 2));
  process.exitCode = 1;
} finally {
  await context.close();
}
