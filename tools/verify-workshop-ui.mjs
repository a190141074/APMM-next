// 工坊与作者界面定向验收；IPC 夹具仅存在本测试，不写游戏或生产服务。
import { installWorkshopFixture } from "./workshop-ui-fixture.mjs";
import { chromium } from "playwright";
import { mkdirSync, existsSync, writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const output = path.join(
  "B:/APMM-next-work/workshop-20261115/ui",
  randomUUID(),
);
mkdirSync(output, { recursive: true });
const executablePath = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
const context = await chromium.launchPersistentContext(
  path.join(output, "profile"),
  { executablePath, headless: true, viewport: { width: 1500, height: 960 } },
);
const page = await context.newPage();
const errors = [];
const checks = [];
const base = process.env.APMM_UI_URL || "http://127.0.0.1:1431/";
page.on("pageerror", (error) => errors.push(error.message));
await context.addInitScript(() => {
  localStorage.setItem("apmm.reduce-motion", "true");
  localStorage.setItem("apmm.welcomed", "true");
});
await page.route("**/__workspace", (route) =>
  route.fulfill({ json: { resources: [], gameDir: null } }),
);
const nav = (name) =>
  page.locator(".nav-item").filter({ hasText: name }).first();
const fixture = (serial = false) => installWorkshopFixture(page, serial);
async function load(serial = false) {
  await page.goto(base);
  await nav("设置").waitFor();
  await fixture(serial);
}
async function select(label, value) {
  await page.getByRole("combobox", { name: label, exact: true }).click();
  await page.getByRole("option", { name: value, exact: true }).click();
}
async function setAuthorMode(enabled) {
  await nav("设置").click();
  await page.locator('main[data-route="settings"]').waitFor();
  await page.getByRole("tab", { name: "工坊", exact: true }).click();
  await page.getByRole("checkbox", { name: /^作者模式/ }).setChecked(enabled);
}
try {
  await load();
  assert.equal(await nav("关于").count(), 0);
  assert.equal(await page.locator(".titlebar .author-mode").count(), 0);
  await nav("设置").click();
  await page.getByRole("tab", { name: "关于", exact: true }).click();
  await page.locator(".about-page").waitFor();
  assert.equal(await page.locator(".social-card").count(), 0);
  await nav("总览").click();
  await page.getByRole("heading", { name: /社交媒体/ }).waitFor();
  const ratio = await page
    .locator(".social-card")
    .evaluate(
      (el) =>
        el.getBoundingClientRect().width /
        el.closest(".overview-social-row").getBoundingClientRect().width,
    );
  assert.ok(ratio > 0.25 && ratio < 0.34, `social ratio ${ratio}`);
  const linksLayout = await page
    .locator(".social-links .social-link")
    .evaluateAll((links) =>
      links.map((link) => ({
        top: link.getBoundingClientRect().top,
        left: link.getBoundingClientRect().left,
      })),
    );
  assert.equal(linksLayout[0].top, linksLayout[1].top);
  assert.ok(
    linksLayout[1].left > linksLayout[0].left &&
      linksLayout[2].top > linksLayout[0].top,
  );
  assert.equal(
    await page.getByRole("button", { name: "展开", exact: true }).count(),
    0,
  );
  assert.equal(
    await page.locator(".social-links").getByRole("link").count(),
    4,
  );
  assert.equal(linksLayout[2].top, linksLayout[3].top);
  await page.getByRole("link", { name: "TAPTAP", exact: true }).click();
  await page.waitForFunction(() => window.__workshopFixture.links.length > 0);
  assert.equal(
    await page.evaluate(() => window.__workshopFixture.links.at(-1)),
    "https://www.taptap.cn/app/524022",
  );
  await page.screenshot({
    path: path.join(output, "overview-wide.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 945, height: 960 });
  await page.screenshot({
    path: path.join(output, "overview-narrow.png"),
    fullPage: true,
  });
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
    true,
  );
  checks.push(
    "总览全部四项双列社交链接、关于进入设置、标题栏无作者开关、约三成宽度及窄窗口",
  );
  await page.setViewportSize({ width: 1500, height: 960 });
  const start = Date.now();
  await nav("创意工坊").click();
  await page.locator(".workshop-card").first().waitFor();
  const current = Date.now() - start;
  assert.equal(await page.locator(".workshop-card").count(), 3);
  await page.getByRole("textbox", { name: "搜索工坊" }).fill("作者 透明");
  assert.equal(await page.locator(".workshop-card").count(), 3);
  await select("获取状态", "已加入本地库");
  assert.equal(await page.locator(".workshop-card").count(), 1);
  await page.locator(".workshop-card").click();
  assert.equal(
    await page.getByRole("button", { name: "已是当前版本" }).isDisabled(),
    true,
  );
  await page
    .getByRole("button", { name: "放大同版本作品", exact: true })
    .click();
  await page.locator(".pan-zoom-scale").waitFor();
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "移除作品", exact: true }).click();
  await page.getByRole("alert").waitFor();
  await page.getByRole("button", { name: "取消移除", exact: true }).click();
  assert.equal(
    await page.evaluate(() => window.__workshopFixture.requests.length),
    0,
  );
  await page.keyboard.press("Escape");
  await select("获取状态", "全部状态");
  await page.getByRole("textbox", { name: "搜索工坊" }).fill("找不到");
  await page.getByText("没有符合条件的作品").waitFor();
  await page.getByRole("textbox", { name: "搜索工坊" }).fill("");
  await page.locator(".workshop-card").filter({ hasText: "星空卡面" }).click();
  await page.getByRole("button", { name: "获取作品", exact: true }).click();
  await page.getByRole("button", { name: "加入本地库", exact: true }).waitFor();
  assert.equal(
    await page.getByRole("checkbox", { name: "加入并应用到游戏" }).isChecked(),
    false,
  );
  await page.getByRole("button", { name: "加入本地库", exact: true }).click();
  assert.deepEqual(
    await page.evaluate(
      () => window.__workshopFixture.requests[0].args.applyFiles,
    ),
    [],
  );
  await page.locator(".workshop-card").filter({ hasText: "星空卡面" }).click();
  await page.getByRole("button", { name: "获取作品", exact: true }).click();
  await page.getByRole("checkbox", { name: "加入并应用到游戏" }).check();
  await select("UT_HandCard_10001 应用候选", "同名.png · bbbbbbbb");
  await page.getByRole("button", { name: "确认加入并应用" }).click();
  assert.deepEqual(
    await page.evaluate(
      () => window.__workshopFixture.requests[1].args.applyFiles,
    ),
    ["two.png"],
  );
  await page.screenshot({
    path: path.join(output, "workshop-wide.png"),
    fullPage: true,
  });
  checks.push("搜索、状态筛选、同版本禁用、默认只入库、同槽位候选明确选择");
  await setAuthorMode(true);
  await nav("投稿").click();
  await page.getByRole("button", { name: "新建作品", exact: true }).click();
  await page.getByLabel("名称 *", { exact: true }).fill("草稿初稿");
  await page.waitForTimeout(650);
  await page.getByLabel("名称 *", { exact: true }).fill("草稿最终稿");
  await page.getByLabel("简介 *", { exact: true }).fill("编辑中快速离开");
  await page.getByLabel("作品版本 *", { exact: true }).fill("1.0");
  await nav("创意工坊").click();
  await page.waitForTimeout(700);
  await nav("投稿").click();
  await page.getByRole("button", { name: "编辑", exact: true }).first().click();
  assert.equal(
    await page.getByLabel("名称 *", { exact: true }).inputValue(),
    "草稿最终稿",
  );
  await page
    .getByRole("button", { name: "预览投稿与导出", exact: true })
    .click();
  assert.equal(
    await page.getByRole("button", { name: "确认上传", exact: true }).count(),
    0,
  );
  await page.getByRole("button", { name: "导出投稿包", exact: true }).click();
  await page.waitForFunction(
    () => window.__workshopFixture.exports.length === 1,
  );
  await page.screenshot({
    path: path.join(output, "author-offline-export.png"),
    fullPage: true,
  });
  await page.keyboard.press("Escape");
  await setAuthorMode(false);
  await nav("创意工坊").click();
  await page.getByRole("heading", { name: "创意工坊", exact: true }).waitFor();
  assert.equal(await nav("投稿").count(), 0);
  assert.equal(
    await page.evaluate(
      () =>
        window.__workshopFixture.requests.filter(
          (r) => r.kind === "author-upload",
        ).length,
    ),
    0,
  );
  assert.equal(
    await page.evaluate(
      () => window.__workshopFixture.author.drafts[0].metadata.title,
    ),
    "草稿最终稿",
  );
  checks.push(
    "作者模式、串行自动保存、快速离页恢复、无服务器导出、无上传请求、关闭模式保留草稿与任务",
  );
  await page.evaluate(
    () =>
      (window.__workshopFixture.listing = {
        available: false,
        status: "unavailable",
        message: "连接失败",
        works: [],
      }),
  );
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  await page.getByText("暂时无法连接工坊", { exact: true }).waitFor();
  assert.equal(
    await page.getByRole("button", { name: "导入本地包" }).count(),
    1,
  );
  await page.evaluate(
    () =>
      (window.__workshopFixture.listing = {
        available: true,
        status: "ready",
        message: "",
        works: [],
      }),
  );
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  await page.getByText("工坊暂时没有作品", { exact: true }).waitFor();
  await page.getByRole("button", { name: "切换主题" }).click();
  await page.screenshot({
    path: path.join(output, "workshop-dark-empty.png"),
    fullPage: true,
  });
  checks.push("连接失败与真实空目录分离、重试与本地包入口、暗色主题");
  // 对比夹具完全一致；旧模型仅复现原 handler 串行等待三张封面的返回时机。
  await load(true);
  const oldStart = Date.now();
  await nav("创意工坊").click();
  await page.locator(".workshop-card").first().waitFor();
  const priorModel = Date.now() - oldStart;
  assert.ok(current < priorModel);
  assert.deepEqual(errors, []);
  const report = {
    output,
    checks,
    errors,
    timing: {
      currentUiReadyMs: current,
      priorSerialHandlerModelMs: priorModel,
      coverDelayMs: 600,
      covers: 3,
      note: "旧值是原后端串行封面等待模型的夹具，不是旧正式二进制实测",
    },
  };
  writeFileSync(
    path.join(output, "report.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report, null, 2));
} finally {
  await context.close();
}
