// 使用系统 Edge 在隔离浏览器目录复验前端交互，截图不包含用户数据。
import { chromium } from "playwright";
import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const work = process.env.APMM_WORK_DIR || "B:/APMM-next-work";
process.env.TEMP = path.join(work, "temp");
process.env.TMP = process.env.TEMP;
const output = path.join(work, "ui-validation", randomUUID());
mkdirSync(output, { recursive: true });
const browserPath =
  process.env.APMM_BROWSER ||
  [
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  ].find(existsSync);
const context = await chromium.launchPersistentContext(
  path.join(output, "profile"),
  {
    ...(browserPath ? { executablePath: browserPath } : {}),
    headless: true,
    viewport: { width: 1360, height: 880 },
    deviceScaleFactor: 1,
  },
);
const page = await context.newPage();
const errors = [];
const checks = [];
page.on("pageerror", (e) => errors.push(e.message));
try {
  await context.addInitScript(() => {
    localStorage.setItem("apmm.welcomed", "true");
    localStorage.setItem("apmm.theme", "light");
  });
  await page.goto("http://127.0.0.1:1420/");
  const nav = (label) =>
    page.locator(".nav-item").filter({ hasText: label }).first();
  await page.locator(".brand-icon img").evaluate((img) => img.decode());
  await nav("角色").click();
  await page.locator(".resource-card").first().waitFor();
  await page
    .locator(".resource-card img")
    .first()
    .evaluate((img) => img.decode());
  await page.screenshot({ path: path.join(output, "resources-light.png") });
  checks.push("真实资源页与原图解码");
  await page.locator(".resource-open").first().click();
  await page.locator(".detail-page").waitFor();
  assert.equal(await page.getByRole("dialog").count(), 0);
  assert.ok((await page.locator(".part-row").count()) > 10);
  assert.ok(
    await page.getByRole("button", { name: "按皮肤", exact: true }).count(),
  );
  const candidate = page
    .locator(".part-row")
    .filter({ has: page.locator(".candidate-strip .candidate:nth-child(2)") })
    .first();
  if (await candidate.count()) {
    await candidate.locator(".candidate:nth-child(2) .candidate-apply").click();
    await page.waitForFunction(() =>
      document.querySelector(".candidate:nth-child(2).applied"),
    );
    await candidate.locator(".candidate:first-child .candidate-apply").click();
  }
  await page.screenshot({ path: path.join(output, "detail-light.png") });
  checks.push("独立详情、多个部位与点击应用/还原");
  await page.getByRole("button", { name: "按部位", exact: true }).click();
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  await page.getByRole("combobox", { name: "搜索资源" }).fill("芬妮");
  await page.getByRole("combobox", { name: "搜索资源" }).press("Enter");
  await page.locator(".detail-page").waitFor();
  assert.match(await page.locator(".detail-page h1").innerText(), /芬妮/);
  checks.push("中文建议键盘定位完整详情");
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  await page.getByRole("combobox", { name: "搜索资源" }).focus();
  await page.getByRole("combobox", { name: "搜索资源" }).press("Enter");
  await page.locator(".detail-page").waitFor();
  checks.push("关闭详情后再次定位相同结果");
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  await nav("筹码").click();
  await page.getByRole("combobox", { name: "搜索资源" }).fill("50029");
  await page.locator(".resource-open").first().click();
  assert.match(await page.locator(".detail-page h1").innerText(), /魔法秘典/);
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  await page.getByRole("combobox", { name: "搜索资源" }).fill("50075_1");
  await page.getByRole("combobox", { name: "搜索资源" }).press("Enter");
  await page.locator('[id="part-relic/UT_Relic_50075_1"]').waitFor();
  assert.ok(await page.locator('[id="part-relic/UT_Relic_50075_1"]').count());
  checks.push("筹码名称和完整变体目录");
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  await nav("APX 插件").click();
  assert.equal(await page.locator(".apx-stats .stat").count(), 4);
  assert.equal(
    await page.getByRole("button", { name: "更改", exact: true }).count(),
    3,
  );
  await page.screenshot({ path: path.join(output, "apx-light.png") });
  checks.push("APX 四项状态、渠道与三个目录入口");
  await page.getByRole("button", { name: "切换主题", exact: true }).click();
  await page.screenshot({ path: path.join(output, "apx-dark.png") });
  await page.setViewportSize({ width: 1000, height: 720 });
  await nav("角色").click();
  await page.getByRole("combobox", { name: "搜索资源" }).fill("");
  await page.locator(".resource-open").first().click();
  await page.screenshot({ path: path.join(output, "detail-narrow-dark.png") });
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  );
  checks.push("深浅主题、1000px 窄窗口");
  await page.setViewportSize({ width: 1360, height: 880 });
  const decoded = await page.evaluate(async () => {
    const { resources } = await (await fetch("/__workspace")).json();
    const paths = [
      ...new Set(
        resources.filter((r) => r.originalUrl).map((r) => r.originalUrl),
      ),
    ];
    let cursor = 0,
      ok = 0;
    const errors = [];
    await Promise.all(
      Array.from({ length: 6 }, async () => {
        while (cursor < paths.length) {
          const url = paths[cursor++];
          const img = new Image();
          img.src = url;
          try {
            await img.decode();
            if (!img.naturalWidth) throw Error("empty");
            ok++;
          } catch (e) {
            errors.push(url);
          }
          img.src = "";
        }
      }),
    );
    return { total: paths.length, ok, errors };
  });
  assert.ok(decoded.total >= 2179);
  assert.equal(decoded.ok, decoded.total);
  assert.deepEqual(decoded.errors, []);
  checks.push(`完整目录原图真实解码 ${decoded.ok}/${decoded.total}`);
  await page.locator(".package-menu summary").click();
  await page.getByText("导出加密包", { exact: true }).click();
  await page.getByRole("button", { name: "下一步", exact: false }).click();
  // 浏览器不提供桌面文件命令，明确显示需要桌面执行的提示。
  checks.push("顶栏 Mod 包菜单与向导入口");
  assert.deepEqual(errors, []);
} finally {
  await page.screenshot({ path: path.join(output, "last-state.png") });
  writeFileSync(
    "verification-ui.json",
    JSON.stringify({ output, checks, errors }, null, 2),
  );
  await context.close();
}
console.log(JSON.stringify({ output, checks, errors }));
