// 工坊双视图、固定操作区和作者封面的定向浏览器验收，所有数据使用隔离夹具。
import { chromium } from "playwright";
import { mkdirSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
import { installWorkshopFixture } from "./workshop-ui-fixture.mjs";

const output = path.join(
  process.env.APMM_WORKSHOP_WORK ||
    `B:/APMM-next-work/workshop-cards-${readFileSync(
      new URL("../VERSION", import.meta.url),
      "utf8",
    )
      .trim()
      .replaceAll(".", "")}`,
  "ui",
  randomUUID(),
);
mkdirSync(output, { recursive: true });
const executablePath = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
const context = await chromium.launchPersistentContext(
  path.join(output, "profile"),
  { executablePath, headless: true, viewport: { width: 1380, height: 960 } },
);
const page = await context.newPage();
const errors = [],
  checks = [];
let passed = false;
page.on("pageerror", (error) => errors.push(error.message));
await context.addInitScript(() => {
  localStorage.setItem("apmm.welcomed", "true");
  localStorage.setItem("apmm.reduce-motion", "true");
});
await page.route("**/__workspace", (route) =>
  route.fulfill({ json: { resources: [], gameDir: null } }),
);
const nav = (name) =>
  page.locator(".nav-item").filter({ hasText: name }).first();
const base = process.env.APMM_UI_URL || "http://127.0.0.1:1435/";
async function load() {
  await page.goto(base);
  await nav("设置").waitFor();
  await installWorkshopFixture(page);
  await page.evaluate(() => {
    const items = window.__workshopFixture.listing.works;
    const original = items[0];
    original.work.author = "星空工作室与创作伙伴\n联合作者第二行应当截断";
    original.work.description = "为角色添加星空主题卡面与透明效果。\n".repeat(
      50,
    );
    original.work.updatedAt = 1794697200;
    original.work.tags = Array.from({ length: 10 }, (_, i) => ({
      name: `主题标签${i + 1}`,
      highlight: i === 0,
    }));
    items.push({
      ...structuredClone(original),
      work: {
        ...structuredClone(original.work),
        id: "update",
        title: "有更新的星空主题作品",
        version: "2.0",
      },
      installedVersion: "1.0",
      updateAvailable: true,
    });
    items[1].work.cover = "";
    items[2].work.cover = "broken.png";
  });
  await nav("创意工坊").click();
  await page.locator(".workshop-card").first().waitFor();
}
const cards = () => page.locator(".workshop-card");
const acquire = (root) =>
  root.getByRole("button", { name: "获取作品", exact: true });
async function visibleInside(locator) {
  const rect = await locator.boundingBox();
  assert.ok(
    rect &&
      rect.y >= 0 &&
      rect.x >= 0 &&
      rect.y + rect.height <= page.viewportSize().height,
    "主操作须完整位于视口内",
  );
  return rect;
}
async function select(label, name) {
  await page.getByRole("combobox", { name: label, exact: true }).click();
  await page.getByRole("option", { name, exact: true }).click();
}
try {
  await load();
  const widths = await page
    .locator(".workshop-grid")
    .evaluate(
      (el) => getComputedStyle(el).gridTemplateColumns.split(" ").length,
    );
  assert.equal(widths, 2);
  assert.equal(
    await page
      .getByRole("button", { name: "横向卡片", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  const authorLines = await cards()
    .first()
    .locator(".work-card-author")
    .evaluate((el) => ({
      height: el.clientHeight,
      line: parseFloat(getComputedStyle(el).lineHeight),
    }));
  assert.ok(authorLines.height <= authorLines.line + 1);
  await page.locator(".workshop-image img").first().waitFor();
  await page.getByText("封面暂不可用", { exact: true }).waitFor();
  await page.screenshot({ path: path.join(output, "horizontal.png") });
  await page.getByRole("button", { name: "封面网格", exact: true }).click();
  await page.screenshot({ path: path.join(output, "grid.png") });
  await load();
  assert.equal(
    await page
      .getByRole("button", { name: "封面网格", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  await page.getByRole("textbox", { name: "搜索工坊" }).fill("星空画师");
  await page.getByRole("button", { name: "横向卡片", exact: true }).click();
  assert.equal(
    await page.getByRole("textbox", { name: "搜索工坊" }).inputValue(),
    "星空画师",
  );
  assert.equal(await cards().count(), 4);
  checks.push("双视图、两列断点、署名单行截断、缺图及坏图、偏好恢复和画师搜索");

  await cards()
    .first()
    .getByRole("button", { name: "星空卡面", exact: true })
    .click();
  const detail = page.locator(".workshop-detail");
  const start = await visibleInside(acquire(detail));
  await detail.locator(".work-detail-scroll").evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  const end = await visibleInside(acquire(detail));
  assert.equal(start.y, end.y);
  await page.screenshot({ path: path.join(output, "detail-scrolled.png") });
  await detail.locator(".work-detail-scroll").evaluate((el) => {
    el.scrollTop = 0;
  });
  await detail
    .getByRole("button", { name: "作者主页 ↗", exact: true })
    .click();
  assert.deepEqual(await page.evaluate(() => window.__workshopFixture.links), [
    "https://example.com/author",
  ]);
  await detail
    .getByRole("button", { name: "放大星空卡面", exact: true })
    .click();
  await page.locator(".pan-zoom-scale").waitFor();
  await page.keyboard.press("Escape");
  assert.equal(
    await page.evaluate(() => window.__workshopFixture.requests.length),
    0,
  );
  await page.screenshot({ path: path.join(output, "detail.png") });
  await page.setViewportSize({ width: 800, height: 680 });
  await visibleInside(acquire(detail));
  await detail.locator(".work-detail-scroll").evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  await visibleInside(acquire(detail));
  await page.screenshot({ path: path.join(output, "detail-narrow.png") });
  await page.keyboard.press("Escape");
  assert.equal(
    await page
      .locator(".workshop-grid")
      .evaluate(
        (el) => getComputedStyle(el).gridTemplateColumns.split(" ").length,
      ),
    1,
  );
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.screenshot({ path: path.join(output, "horizontal-narrow.png") });
  checks.push("长详情滚动及窄窗口主操作始终可见，图片放大独立、作者链接可点击");

  await acquire(cards().first()).click();
  assert.equal(await page.locator(".workshop-detail").count(), 0);
  const apply = page.getByRole("checkbox", { name: "加入并应用到游戏" });
  assert.equal(await apply.isChecked(), false);
  await page.getByRole("button", { name: "加入本地库", exact: true }).click();
  assert.deepEqual(
    await page.evaluate(
      () => window.__workshopFixture.requests.at(-1).args.applyFiles,
    ),
    [],
  );
  await acquire(cards().first()).click();
  await apply.check();
  await select("UT_HandCard_10001 应用候选", "同名.png · bbbbbbbb");
  await page
    .getByRole("button", { name: "确认加入并应用", exact: true })
    .click();
  assert.deepEqual(
    await page.evaluate(
      () => window.__workshopFixture.requests.at(-1).args.applyFiles,
    ),
    ["two.png"],
  );
  await cards()
    .filter({ hasText: "同版本作品" })
    .getByRole("button", { name: "同版本作品", exact: true })
    .click();
  assert.equal(
    await page
      .locator(".workshop-detail")
      .getByRole("button", { name: "已是当前版本" })
      .isDisabled(),
    true,
  );
  await page.getByRole("button", { name: "移除作品", exact: true }).click();
  await page.getByRole("alert").waitFor();
  const count = await page.evaluate(
    () => window.__workshopFixture.requests.length,
  );
  await page.getByRole("button", { name: "取消移除", exact: true }).click();
  assert.equal(
    await page.evaluate(() => window.__workshopFixture.requests.length),
    count,
  );
  // 模拟任务完成后目录重新查询，详情应读取最新条目而非打开时的副本。
  await page.evaluate(() => {
    window.__workshopFixture.listing.works[1].updateAvailable = true;
  });
  await page
    .locator(".workshop-page .page-heading button")
    .evaluate((el) => el.click());
  await page
    .locator(".workshop-detail")
    .getByRole("button", { name: "检查更新内容" })
    .waitFor();
  await page.keyboard.press("Escape");
  checks.push(
    "卡片直达预检、默认仅入库、显式候选应用、移除取消及打开详情后的状态刷新",
  );

  await page.setViewportSize({ width: 1380, height: 960 });
  await nav("设置").click();
  await page.getByRole("tab", { name: "工坊", exact: true }).click();
  await page.getByRole("checkbox", { name: /^作者模式/ }).setChecked(true);
  await nav("投稿").click();
  await page.getByRole("button", { name: "新建作品", exact: true }).click();
  await page.getByLabel("名称 *", { exact: true }).fill("投稿封面验证");
  await page
    .getByLabel("作者显示名", { exact: true })
    .fill("作者甲\n作者乙\n作者丙");
  await page.getByLabel("画师", { exact: true }).fill("画师甲");
  await page
    .getByLabel("简介 *", { exact: true })
    .fill("作者模式使用同一套工坊卡片预览。");
  await page.getByLabel("作品版本 *", { exact: true }).fill("1.0");
  await page
    .getByLabel("作者主页链接", { exact: true })
    .fill("https://example.com/author\n");
  await page
    .getByLabel("作品来源链接", { exact: true })
    .fill("https://example.com/source");
  await page.getByLabel("标签", { exact: true }).fill("主题");
  await page.getByRole("button", { name: "添加标签", exact: true }).click();
  await page.getByLabel("标签", { exact: true }).fill("主题");
  await page.getByRole("button", { name: "添加标签", exact: true }).click();
  await page.getByRole("alert").filter({ hasText: "不能重复" }).waitFor();
  await page.getByLabel("标签", { exact: true }).fill("");
  await page.evaluate(() =>
    window.__workshopFixture.pickPaths.push(
      "B:/APMM-next-work/fixture/cover.png",
    ),
  );
  await page.getByRole("button", { name: "选择封面", exact: true }).click();
  await page.getByRole("button", { name: "替换封面", exact: true }).waitFor();
  const firstCover = await page.evaluate(
    () => window.__workshopFixture.author.drafts[0].images[0].id,
  );
  await page.evaluate(() =>
    window.__workshopFixture.pickPaths.push(
      "B:/APMM-next-work/fixture/bad.png",
    ),
  );
  await page.getByRole("button", { name: "替换封面", exact: true }).click();
  await page.getByRole("status").filter({ hasText: "图片解码失败" }).waitFor();
  assert.equal(
    await page.evaluate(
      () => window.__workshopFixture.author.drafts[0].images[0].id,
    ),
    firstCover,
  );
  await page.evaluate(() =>
    window.__workshopFixture.pickPaths.push(
      "B:/APMM-next-work/fixture/new.png",
    ),
  );
  await page.getByRole("button", { name: "替换封面", exact: true }).click();
  await page.waitForFunction(
    (id) => window.__workshopFixture.author.drafts[0].images[0].id !== id,
    firstCover,
  );
  await page.locator(".author-card-preview").scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(output, "author-preview.png") });
  await page
    .getByRole("button", { name: "预览投稿与导出", exact: true })
    .click();
  const snapshot = page.getByRole("dialog", {
    name: "确认投稿快照",
    exact: true,
  });
  await snapshot.locator(".author-card-preview").waitFor();
  await snapshot.getByRole("button", { name: "封面网格", exact: true }).click();
  assert.equal(
    await snapshot.locator(".work-card-author").textContent(),
    "作者：作者甲\n作者乙\n作者丙",
  );
  const saved = await page.evaluate(
    () => window.__workshopFixture.author.drafts[0].metadata,
  );
  assert.equal(saved.artist, "画师甲");
  assert.deepEqual(saved.authorUrls, ["https://example.com/author"]);
  assert.equal(saved.sourceUrl, "https://example.com/source");
  await page.screenshot({ path: path.join(output, "snapshot.png") });
  await page.keyboard.press("Escape");
  checks.push(
    "作者画师与链接保存、标签去重、封面替换失败保留、草稿与快照共用双视图预览",
  );
  await page.getByRole("button", { name: "返回草稿", exact: true }).click();
  await page.getByRole("button", { name: "编辑", exact: true }).first().click();
  assert.equal(
    await page.getByLabel("画师", { exact: true }).inputValue(),
    "画师甲",
  );
  assert.equal(
    await page.getByLabel("作者主页链接", { exact: true }).inputValue(),
    "https://example.com/author",
  );
  await nav("创意工坊").click();
  await page.getByTitle("切换主题", { exact: true }).click();
  await page.waitForFunction(
    () => document.documentElement.dataset.theme === "dark",
  );
  await page.screenshot({ path: path.join(output, "dark.png") });
  assert.deepEqual(errors, []);
  passed = true;
} catch (error) {
  errors.push(error.stack || String(error));
  throw error;
} finally {
  await page
    .screenshot({ path: path.join(output, "last-state.png") })
    .catch(() => {});
  writeFileSync(
    path.join(output, "report.json"),
    JSON.stringify({ output, passed, checks, errors }, null, 2),
  );
  await context.close();
  console.log(JSON.stringify({ output, passed, checks, errors }, null, 2));
}
