// 直接列表和原详情的交互回归，所有应用请求只记录到浏览器夹具。
import { chromium } from "playwright";
import { mkdirSync, existsSync, writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const output = path.join(
  process.env.APMM_RESOURCE_UI_WORK ||
    "B:/APMM-next-work/resource-ui-validation",
  randomUUID(),
);
mkdirSync(output, { recursive: true });
const executablePath = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
const context = await chromium.launchPersistentContext(
  path.join(output, "profile"),
  { executablePath, headless: true, viewport: { width: 1232, height: 905 } },
);
const page = await context.newPage();
const errors = [],
  checks = [];
page.on("pageerror", (e) => errors.push(e.message));
await context.addInitScript(() => {
  localStorage.setItem("apmm.welcomed", "true");
  localStorage.setItem("apmm.reduce-motion", "false");
  window.__layoutAnimations = [];
  const animate = Element.prototype.animate;
  Element.prototype.animate = function (frames, options) {
    if (options?.duration === 280)
      window.__layoutAnimations.push({
        key: this.dataset.resourceViewKey,
        frames,
      });
    return animate.call(this, frames, options);
  };
});
const picture = (name, color) =>
  `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120"><rect width="120" height="120" rx="8" fill="${color}"/><circle cx="60" cy="40" r="24" fill="white" opacity=".4"/><path d="M10 110L40 55L85 80L120 60V120H0Z" fill="white" opacity=".2"/><text x="60" y="108" text-anchor="middle" fill="white" font-size="13">${name}</text></svg>`)}`;
const outimgPath =
  "B:/APMM-next-work/game fixture/outimg/UT_HandCard_10001.png";
const resource = (index, category = "cards", section = "hand") => {
  const canonical =
    category === "cards"
      ? `UT_HandCard_${10001 + index}`
      : `${category}_${section}_${index}`;
  return {
    key: canonical,
    category,
    entityId: category === "cards" ? String(10001 + index) : "10001",
    entityName:
      category === "cards"
        ? ["攻击(中)", "攻击(大)", "名刀：嘎鸣切", "暗影突袭"][index % 4] +
          ` ${index + 1}`
        : "测试角色",
    nameKnown: true,
    aliases: [],
    section,
    sectionLabel:
      section === "event" ? "事件牌" : section === "hand" ? "手牌" : section,
    resourceLabel: section,
    variantKey: "default",
    variantLabel: "默认",
    variantOrder: 0,
    canonical,
    groupKey: "10001",
    fullName: canonical,
    applicationSlot: canonical,
    originalUrl: picture("原版", "#747eae"),
    originalStatus: "ready",
    ...(category === "cards" && index === 0 ? { outimgPath } : {}),
    files: Array.from({ length: index === 0 ? 5 : 2 }, (_, f) => ({
      hash: String(f + 1).repeat(64),
      sourceName: `候选${f + 1}.png`,
      path: picture(`候选${f + 1}`, ["#ac7ab4", "#499796", "#b48162"][f % 3]),
    })),
    activeHash: index % 3 === 0 ? "1".repeat(64) : null,
    disabled: false,
  };
};
const heroAvatar = picture("角色物品图标", "#406884");
const resources = [
  ...Array.from({ length: 90 }, (_, i) =>
    resource(i, "cards", i < 70 ? "hand" : "event"),
  ),
  resource(0, "heroes", "card"),
  resource(1, "heroes", "body"),
  {
    ...resource(2, "heroes", "item_icons"),
    canonical: "UT_Item_Hero_10001",
    originalUrl: heroAvatar,
    files: [],
    activeHash: null,
    variantOrder: 1000,
  },
  ...Array.from({ length: 43 }, (_, i) =>
    ["card", "body", "portrait"].map((section, j) => ({
      ...resource(200 + i * 3 + j, "heroes", section),
      entityId: String(11000 + i),
      entityName: `测试角色 ${i + 2}`,
      files: i === 0 || j !== 1 ? [] : resource(200 + i, "heroes").files,
      activeHash: i === 1 && j === 1 ? "1".repeat(64) : null,
      disabled: i === 1 && j === 1,
    })),
  ).flat(),
  resource(0, "cursors", "up"),
  resource(1, "cursors", "down"),
];
await page.route("**/__workspace", (route) =>
  route.fulfill({ json: { resources, gameDir: null } }),
);
const nav = (name) =>
  page.locator(".nav-item").filter({ hasText: name }).first();
async function go(name, route) {
  await nav(name).click();
  await page.locator(`main[data-route="${route}"]`).waitFor();
  await page.waitForTimeout(450);
}
async function select(label, value) {
  await page.getByRole("combobox", { name: label, exact: true }).click();
  await page.getByRole("option", { name: value, exact: true }).click();
}
try {
  await page.goto(process.env.APMM_UI_URL || "http://127.0.0.1:1433/");
  await nav("卡面").waitFor();
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .map((e) => e.name)
      .find((url) => new URL(url).pathname === "/src-ui/core/services/app.ts");
    const { appService } = await import(url);
    const apply = appService.apply;
    const command = appService.command;
    window.__applyRequests = [];
    window.__revealRequests = [];
    appService.command = async (name, args) => {
      if (name === "outimg_reveal") {
        window.__revealRequests.push(args);
        return;
      }
      return command(name, args);
    };
    appService.apply = async (items) => {
      window.__applyRequests.push(structuredClone(items));
      return apply(items);
    };
  });
  await go("卡面", "cards");
  await page.locator('.resource-scroll[data-layout="list"]').waitFor();
  assert.ok((await page.locator(".compact-part").count()) > 0);
  assert.ok(
    (await page.locator(".compact-part").count()) < 25,
    "列表只挂载可视区域",
  );
  const first = page.locator(
    '.compact-part[data-resource-view-key="UT_HandCard_10001"]',
  );
  assert.equal(await first.locator(".candidate").count(), 4);
  await first
    .getByRole("button", { name: "定位原图文件 UT_HandCard_10001" })
    .click();
  assert.deepEqual(await page.evaluate(() => window.__revealRequests), [
    { path: outimgPath },
  ]);
  assert.equal(await page.evaluate(() => window.__applyRequests.length), 0);
  assert.equal(
    await page
      .getByRole("button", { name: "定位原图文件 UT_HandCard_10002" })
      .count(),
    0,
  );
  assert.equal(
    await first
      .getByRole("button", { name: "应用 候选2.png 22222222", exact: true })
      .count(),
    1,
  );
  await first
    .getByRole("button", { name: "比较 候选2.png", exact: true })
    .click();
  await page.getByRole("dialog").waitFor();
  assert.equal(await page.evaluate(() => window.__applyRequests.length), 0);
  await page.keyboard.press("Escape");
  await first
    .getByRole("button", { name: "管理 候选2.png", exact: true })
    .click();
  await page.getByRole("dialog").waitFor();
  assert.equal(await page.evaluate(() => window.__applyRequests.length), 0);
  await page.keyboard.press("Escape");
  await first
    .getByRole("button", { name: "应用 候选2.png 22222222", exact: true })
    .click();
  await page.waitForFunction(() => window.__applyRequests.length === 1);
  assert.deepEqual(await page.evaluate(() => window.__applyRequests[0]), [
    {
      entityId: "10001",
      section: "hand",
      canonical: "UT_HandCard_10001",
      hash: "2".repeat(64),
    },
  ]);
  await page.waitForFunction(() =>
    document.querySelector(
      '.compact-part[data-resource-view-key="UT_HandCard_10001"] .candidate.applied button[aria-label^="应用 候选2"]',
    ),
  );
  await first
    .getByRole("button", { name: "还原 UT_HandCard_10001", exact: true })
    .click();
  await page.waitForFunction(() => window.__applyRequests.length === 2);
  assert.equal(
    await page.evaluate(() => window.__applyRequests[1][0].hash),
    null,
  );
  await first
    .getByRole("button", { name: "更多 UT_HandCard_10001", exact: true })
    .click();
  await page.locator(".detail-page").waitFor();
  assert.equal(await page.locator(".detail-page .candidate").count(), 6);
  assert.equal(await page.locator(".detail-page .candidate-reveal").count(), 1);
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  await page.getByRole("status").filter({ hasText: "应用状态已刷新" }).click();
  await page.screenshot({ path: path.join(output, "cards-list-wide.png") });
  checks.push(
    "卡面默认直接展开、候选限制与更多完整详情、原图定位／比较／管理不应用、精确候选应用及还原",
  );
  await select("资源部位", "事件牌");
  await page.waitForFunction(() =>
    [...document.querySelectorAll(".resource-list-heading")].some((el) =>
      el.textContent.includes("事件牌"),
    ),
  );
  await page.getByRole("button", { name: /事件牌.*20 个资源/ }).click();
  assert.equal(await page.locator(".compact-part").count(), 0);
  await page.getByRole("button", { name: /事件牌.*20 个资源/ }).click();
  await select("资源部位", "全部部位");
  await page.getByRole("button", { name: "卡片模式", exact: true }).click();
  await page
    .locator('.resource-scroll[data-layout="grid"] .resource-card')
    .first()
    .waitFor();
  await page.waitForTimeout(350);
  await page.getByRole("button", { name: "列表模式", exact: true }).click();
  await page.waitForTimeout(400);
  assert.ok((await page.evaluate(() => window.__layoutAnimations.length)) > 0);
  assert.ok(
    await page
      .locator(".compact-part")
      .first()
      .evaluate((el) => getComputedStyle(el).opacity === "1"),
  );
  await page.evaluate(() => {
    document.querySelector('button[title="卡片模式"]').click();
    setTimeout(
      () => document.querySelector('button[title="列表模式"]').click(),
      20,
    );
    setTimeout(
      () => document.querySelector('button[title="卡片模式"]').click(),
      45,
    );
    setTimeout(
      () => document.querySelector('button[title="列表模式"]').click(),
      65,
    );
  });
  await page.waitForTimeout(500);
  assert.equal(
    await page.locator(".resource-scroll").getAttribute("data-layout"),
    "list",
  );
  assert.equal(
    await page
      .locator(".resource-scroll")
      .getAttribute("data-layout-switching"),
    null,
  );
  assert.ok(
    await page
      .locator(".compact-part")
      .first()
      .evaluate((el) => getComputedStyle(el).opacity === "1"),
  );
  await select("预览尺寸", "大");
  await page.waitForTimeout(120);
  const rects = await page.locator(".compact-part").evaluateAll((els) =>
    els.map((el) => ({
      top: el.getBoundingClientRect().top,
      bottom: el.getBoundingClientRect().bottom,
    })),
  );
  assert.ok(rects.slice(1).every((r, i) => r.top >= rects[i].bottom - 1));
  await select("预览尺寸", "中");
  await page.setViewportSize({ width: 850, height: 905 });
  await page.screenshot({ path: path.join(output, "cards-list-narrow.png") });
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.setViewportSize({ width: 1232, height: 905 });
  await page.locator(".resource-scroll").evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  await page.waitForTimeout(150);
  assert.ok((await page.locator(".compact-part").count()) < 25);
  await page.screenshot({ path: path.join(output, "cards-list-end.png") });
  await go("总览", "overview");
  await go("卡面", "cards");
  assert.equal(
    await page.locator(".resource-scroll").getAttribute("data-layout"),
    "list",
  );
  await go("角色", "heroes");
  assert.equal(
    await page.locator(".resource-scroll").getAttribute("data-layout"),
    "grid",
  );
  await page.locator(".resource-open").first().click();
  assert.equal(await page.locator(".detail-page .part-row").count(), 3);
  await page.getByRole("button", { name: "按部位", exact: true }).click();
  assert.equal(await page.locator(".detail-page .skin-group").count(), 3);
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  const toolbar = page.locator(".resource-toolbar");
  const stateFilter = async (name) => {
    await toolbar.getByRole("button", { name, exact: true }).click();
    await page.waitForTimeout(100);
  };
  await stateFilter("无 Mod");
  assert.equal(await page.locator(".heading-count").first().textContent(), "1");
  assert.equal(await page.locator(".resource-card").count(), 1);
  assert.match(
    await page.locator(".resource-card").textContent(),
    /测试角色 2/,
  );
  await stateFilter("有 Mod");
  assert.equal(
    await page.locator(".heading-count").first().textContent(),
    "43",
  );
  const disabled = page.locator('[data-resource-view-key="11001"]');
  assert.match(await disabled.textContent(), /2 个 Mod/);
  assert.equal(await disabled.getByText("已应用", { exact: true }).count(), 0);
  await stateFilter("已应用");
  assert.equal(await page.locator(".heading-count").first().textContent(), "1");
  const hero = page.locator('[data-resource-view-key="10001"]');
  assert.match(await hero.textContent(), /3 个部位/);
  assert.match(await hero.textContent(), /7 个 Mod/);
  await select("资源部位", "body");
  await hero.locator(".resource-open").click();
  await page.getByText("当前筛选没有匹配资源", { exact: true }).waitFor();
  await page.getByRole("button", { name: "清除筛选", exact: true }).click();
  assert.equal(await page.locator(".detail-page .part-row").count(), 3);
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  await page.getByRole("button", { name: "紧凑模式", exact: true }).click();
  await page
    .locator('.resource-scroll[data-layout="compact"] .hero-compact-card')
    .first()
    .waitFor();
  assert.equal(await page.locator(".compact-part").count(), 0);
  assert.equal(
    await page.locator(".heading-count").first().textContent(),
    "44",
  );
  assert.match(
    await hero.textContent(),
    /3 个部位.*7 个 Mod.*已应用 1 个部位/s,
  );
  const avatar = hero.getByRole("img", { name: "测试角色头像", exact: true });
  assert.equal(await avatar.getAttribute("src"), heroAvatar);
  assert.equal(
    await page
      .locator('[data-resource-view-key="11000"] .hero-avatar img')
      .count(),
    0,
  );
  await select("资源部位", "body");
  assert.equal(await avatar.getAttribute("src"), heroAvatar);
  await select("资源部位", "全部部位");
  for (const size of ["小", "中", "大", "特大"]) {
    await select("预览尺寸", size);
    await page.waitForTimeout(100);
    const boxes = await page.locator(".resource-row").evaluateAll((els) =>
      els
        .map((el) => ({
          top: el.getBoundingClientRect().top,
          bottom: el.getBoundingClientRect().bottom,
        }))
        .sort((a, b) => a.top - b.top),
    );
    assert.ok(boxes.slice(1).every((r, i) => r.top >= boxes[i].bottom - 1));
    if (size === "中") {
      const firstRow = page.locator(".resource-row").first();
      assert.ok((await firstRow.locator(".hero-compact-card").count()) >= 4);
      const card = await hero.boundingBox();
      assert.ok(card && card.height >= 68 && card.height <= 82);
      await page.screenshot({
        path: path.join(output, "heroes-compact-medium.png"),
      });
    }
  }
  await page.setViewportSize({ width: 850, height: 905 });
  await page.waitForTimeout(120);
  assert.equal(
    await page
      .locator(".resource-row")
      .first()
      .locator(".resource-card")
      .count(),
    1,
  );
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.screenshot({
    path: path.join(output, "heroes-compact-xlarge.png"),
  });
  await page.setViewportSize({ width: 1232, height: 905 });
  const scroller = page.locator(".resource-scroll");
  await scroller.evaluate((el) => {
    el.scrollTop = 900;
  });
  await page.waitForTimeout(100);
  const compactScroll = await scroller.evaluate((el) => el.scrollTop);
  await page.getByRole("button", { name: "卡片模式", exact: true }).click();
  await page.waitForTimeout(350);
  await scroller.evaluate((el) => {
    el.scrollTop = 1600;
  });
  await page.waitForTimeout(100);
  const gridScroll = await scroller.evaluate((el) => el.scrollTop);
  await page.getByRole("button", { name: "紧凑模式", exact: true }).click();
  await page.waitForTimeout(350);
  assert.ok(
    Math.abs((await scroller.evaluate((el) => el.scrollTop)) - compactScroll) <
      2,
    "紧凑滚动位置恢复",
  );
  await page.getByRole("button", { name: "卡片模式", exact: true }).click();
  await page.waitForTimeout(350);
  assert.ok(
    Math.abs((await scroller.evaluate((el) => el.scrollTop)) - gridScroll) < 2,
    "卡片滚动位置恢复",
  );
  await page.getByRole("button", { name: "紧凑模式", exact: true }).click();
  await go("总览", "overview");
  await go("角色", "heroes");
  assert.equal(await scroller.getAttribute("data-layout"), "compact");
  await scroller.evaluate((el) => {
    el.scrollTop = 0;
  });
  await page.waitForTimeout(100);
  await hero.locator(".resource-open").click();
  assert.equal(
    await page
      .locator(".detail-page .candidate-apply .asset-image")
      .first()
      .evaluate((el) => el.getBoundingClientRect().height),
    142,
  );
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  checks.push(
    "角色整体状态筛选、完整统计、紧凑卡物品头像与缺图占位、四档尺寸、窄窗单列、两种滚动记忆与详情尺寸",
  );
  await go("鼠标", "cursors");
  await page.locator(".resource-open").first().click();
  await page.getByRole("button", { name: "还原整组", exact: true }).click();
  await page.waitForFunction(() => window.__applyRequests.length === 3);
  assert.equal(await page.evaluate(() => window.__applyRequests[2].length), 2);
  checks.push(
    "部位过滤与折叠、列表/卡片动画、尺寸和窄窗、虚拟滚动、分类记忆、角色分组与鼠标整组回归",
  );
  await go("设置", "settings");
  assert.deepEqual(await page.getByRole("tab").allTextContents(), [
    "目录与资源",
    "外观",
    "配色",
    "工坊",
    "更新",
    "安全与维护",
    "关于",
  ]);
  assert.equal(
    await page
      .getByRole("tab", { name: "目录与资源", exact: true })
      .getAttribute("aria-selected"),
    "true",
  );
  await page.getByRole("tab", { name: "外观", exact: true }).click();
  await page.getByRole("checkbox", { name: /^减少动画/ }).check();
  await page.waitForFunction(
    () => document.documentElement.dataset.reduceMotion === "true",
  );
  await go("卡面", "cards");
  const before = await page.evaluate(() => window.__layoutAnimations.length);
  await page.getByRole("button", { name: "卡片模式", exact: true }).click();
  await page.waitForTimeout(350);
  assert.equal(
    await page.evaluate(() => window.__layoutAnimations.length),
    before,
  );
  await page.getByRole("button", { name: "列表模式", exact: true }).click();
  await page.getByRole("button", { name: "切换主题" }).click();
  await page.waitForTimeout(200);
  await page.screenshot({ path: path.join(output, "cards-list-dark.png") });
  const scrollbar = await page.locator(".resource-scroll").evaluate((el) => ({
    width: getComputedStyle(el, "::-webkit-scrollbar").width,
    track: getComputedStyle(el, "::-webkit-scrollbar-track").backgroundColor,
  }));
  assert.equal(scrollbar.width, "5px");
  assert.equal(scrollbar.track, "rgba(0, 0, 0, 0)");
  checks.push("减少动画停用布局动画、暗色主题及 5px 透明轨道滚动条");
  assert.deepEqual(errors, []);
  const report = { output, checks, errors };
  writeFileSync(
    path.join(output, "report.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  await page.screenshot({ path: path.join(output, "failed.png") });
  console.error(output);
  throw error;
} finally {
  await context.close();
}
