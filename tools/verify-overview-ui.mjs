// 总览与逐项动效的浏览器验收，服务夹具和浏览器资料只用于隔离验证。
import { chromium } from "playwright";
import { mkdirSync, existsSync, writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const output = path.join(
  "B:/APMM-next-work/workshop-20261115/overview",
  randomUUID(),
);
mkdirSync(output, { recursive: true });
const executablePath = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
const context = await chromium.launchPersistentContext(
  path.join(output, "profile"),
  { executablePath, headless: true, viewport: { width: 1543, height: 905 } },
);
const page = await context.newPage();
const errors = [],
  checks = [];
page.on("pageerror", (e) => errors.push(e.message));
await context.addInitScript(() => {
  localStorage.setItem("apmm.welcomed", "true");
  localStorage.setItem("apmm.reduce-motion", "false");
  window.__reveals = [];
  const animate = Element.prototype.animate;
  Element.prototype.animate = function (frames, options) {
    if (options?.duration === 230)
      window.__reveals.push({
        route: this.closest("main")?.dataset.route,
        target: this.className,
        delay: options.delay,
      });
    return animate.call(this, frames, options);
  };
});
await page.route("**/__workspace", (route) =>
  route.fulfill({ json: { resources: [], gameDir: null } }),
);
const nav = (name) =>
  page.locator(".nav-item").filter({ hasText: name }).first();
async function go(name, route) {
  await nav(name).click();
  await page.locator(`main[data-route="${route}"]`).waitFor();
  await page.waitForTimeout(450);
}
async function selected(selector) {
  const state = await page.locator(selector).evaluate((el) => {
    const actual = getComputedStyle(el);
    const probe = document.createElement("span");
    probe.style.color = "var(--accent)";
    el.append(probe);
    const accent = getComputedStyle(probe).color;
    probe.remove();
    return {
      border: actual.borderTopColor,
      accent,
      width: actual.borderTopWidth,
    };
  });
  assert.equal(state.border, state.accent);
  assert.equal(state.width, "1px");
}
try {
  await page.goto(process.env.APMM_UI_URL || "http://127.0.0.1:1432/");
  await page.locator(".overview-version-card").first().waitFor();
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .map((e) => e.name)
      .find((url) => new URL(url).pathname === "/src-ui/core/services/app.ts");
    const { appService } = await import(url);
    const original = appService.command;
    window.__apx = {
      platformLabel: "Steam 国服",
      localPluginVersion: "0.3.0",
      remoteVersion: "0.4.0",
      installedFiles: 3,
      totalFiles: 3,
      directoryError: null,
      error: null,
      files: [],
    };
    appService.command = async (command, args) => {
      if (command === "plugin_status") return structuredClone(window.__apx);
      if (command === "plugin_check_update") return null;
      return original(command, args);
    };
  });
  await go("APX", "apx");
  await page.getByRole("button", { name: "检查更新", exact: true }).click();
  await selected('.segmented-tabs button[aria-selected="true"]');
  await page.screenshot({
    path: path.join(output, "apx-selected.png"),
    fullPage: true,
  });
  await go("总览", "overview");
  const cards = page.locator(".overview-version-card");
  assert.equal(await cards.count(), 2);
  assert.match(await cards.nth(0).innerText(), /APMM\s+v0\.4\.1/);
  assert.match(await cards.nth(1).innerText(), /v0\.3\.0/);
  assert.match(await cards.nth(1).innerText(), /待更新/);
  const bounds = await cards.evaluateAll((els) =>
    els.map((el) => ({
      top: el.getBoundingClientRect().top,
      left: el.getBoundingClientRect().left,
    })),
  );
  assert.equal(bounds[0].top, bounds[1].top);
  assert.ok(bounds[1].left > bounds[0].left);
  await page.screenshot({
    path: path.join(output, "overview-wide.png"),
    fullPage: true,
  });
  const reveals = await page.evaluate(() =>
    window.__reveals.filter((r) => r.route === "overview"),
  );
  assert.ok(reveals.length >= 5);
  assert.ok(reveals.some((r) => r.delay > 0));
  const before = await page.evaluate(() => window.__reveals.length);
  await page.waitForTimeout(2300);
  assert.equal(await page.evaluate(() => window.__reveals.length), before);
  checks.push(
    "总览并排版本卡、真实状态投影与共享刷新、页面逐项延迟、任务轮询不重播",
  );
  await cards.nth(0).click();
  await page.locator('main[data-route="settings"]').waitFor();
  await page.waitForTimeout(450);
  await selected(".settings-nav button.active");
  await page.getByRole("tab", { name: "配色", exact: true }).click();
  await page.waitForTimeout(450);
  await selected(".settings-nav button.active");
  await page.screenshot({
    path: path.join(output, "settings-selected.png"),
    fullPage: true,
  });
  await page.getByRole("tab", { name: "外观", exact: true }).click();
  await page.getByRole("checkbox", { name: /^减少动画/ }).check();
  await page.waitForFunction(
    () => document.documentElement.dataset.reduceMotion === "true",
  );
  const reducedBefore = await page.evaluate(() => window.__reveals.length);
  await go("总览", "overview");
  assert.equal(
    await page.evaluate(() => window.__reveals.length),
    reducedBefore,
  );
  assert.equal(
    await cards.nth(0).evaluate((el) => getComputedStyle(el).opacity),
    "1",
  );
  await page.setViewportSize({ width: 945, height: 905 });
  await page.screenshot({
    path: path.join(output, "overview-narrow.png"),
    fullPage: true,
  });
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.getByRole("button", { name: "切换主题" }).click();
  await page.waitForFunction(
    () => document.documentElement.dataset.theme === "dark",
  );
  await page.screenshot({
    path: path.join(output, "overview-dark.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 1543, height: 905 });
  await go("设置", "settings");
  await page.getByRole("checkbox", { name: /^减少动画/ }).uncheck();
  await page.waitForFunction(
    () => document.documentElement.dataset.reduceMotion === "false",
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.waitForTimeout(50);
  const systemBefore = await page.evaluate(() => window.__reveals.length);
  await go("总览", "overview");
  assert.equal(
    await page.evaluate(() => window.__reveals.length),
    systemBefore,
  );
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.evaluate(() => {
    const click = (text) =>
      [...document.querySelectorAll(".nav-item")]
        .find((e) => e.textContent.includes(text))
        .click();
    click("APX");
    setTimeout(() => click("设置"), 20);
    setTimeout(() => click("总览"), 45);
  });
  await page.waitForTimeout(550);
  await page.locator('main[data-route="overview"]').waitFor();
  assert.deepEqual(
    await page.locator("main").evaluate((el) => ({
      opacity: getComputedStyle(el).opacity,
      pointerEvents: getComputedStyle(el).pointerEvents,
    })),
    { opacity: "1", pointerEvents: "auto" },
  );
  checks.push(
    "APX 与设置鲜明选中边框、宽窄与暗色布局、应用及系统减少动画、快速反向切页可见",
  );
  await cards.nth(1).click();
  await page.locator('main[data-route="apx"]').waitFor();
  await page.evaluate(() => {
    window.__apx.directoryError = "未选择目录";
    window.__apx.localPluginVersion = null;
  });
  await page.getByRole("button", { name: "检查更新", exact: true }).click();
  await go("总览", "overview");
  assert.match(await cards.nth(1).innerText(), /待配置/);
  assert.doesNotMatch(await cards.nth(1).innerText(), /未安装/);
  checks.push("APX 卡片点击导航，目录未配置不会误报未安装");
  assert.deepEqual(errors, []);
  const report = { output, checks, errors, revealSamples: reveals };
  writeFileSync(
    path.join(output, "report.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  await page.screenshot({
    path: path.join(output, "failed.png"),
    fullPage: true,
  });
  console.error(output);
  throw error;
} finally {
  await context.close();
}
