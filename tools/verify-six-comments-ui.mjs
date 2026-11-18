// 六项批注的隔离浏览器验收；更新命令仅记录到夹具，不下载或安装真实程序。
import { chromium } from "playwright";
import { mkdirSync, existsSync, writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
import { measureCandidates } from "./resource-size-ui-checks.mjs";
import {
  verifyMigrationDialog,
  verifyMinuteSchedule,
  verifyInheritanceDetection,
} from "./settings-ui-checks.mjs";
const output = path.join(
  process.env.APMM_SIX_UI_WORK || "B:/APMM-next-work/six-comments-ui",
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
  { executablePath, headless: true, viewport: { width: 1014, height: 905 } },
);
const page = await context.newPage();
const checks = [],
  errors = [],
  candidateMetrics = [];
page.on("pageerror", (error) => errors.push(error.message));
const picture =
  "data:image/svg+xml," +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" width="120" height="160"><rect width="120" height="160" fill="#7089b0"/><circle cx="60" cy="50" r="25" fill="#dce5f0"/></svg>',
  );
const resource = (category, id, section = "card") => {
  const canonical =
    category === "heroes" && section === "item_icons"
      ? `UT_Item_Hero_${id}`
      : `${category}_${section}_${id}`;
  return {
    key: canonical,
    category,
    entityId: String(id),
    entityName: category === "heroes" ? `测试角色${id}` : `测试资源${id}`,
    nameKnown: true,
    aliases: [],
    section,
    sectionLabel: section,
    resourceLabel: section,
    variantKey: "default",
    variantLabel: "默认",
    variantOrder: 0,
    canonical,
    groupKey: String(id),
    fullName: canonical,
    applicationSlot: canonical,
    originalUrl: picture,
    originalStatus: "ready",
    files: [{ hash: "a".repeat(64), path: picture, sourceName: "候选.png" }],
    activeHash: null,
    disabled: false,
  };
};
const resources = [
  ...Array.from({ length: 35 }, (_, i) => [
    resource("heroes", 101 + i),
    resource("heroes", 101 + i, "item_icons"),
  ]).flat(),
  ...["cards", "monsters", "chips", "lands", "cursors", "temporary"].flatMap(
    (category) =>
      Array.from({ length: 30 }, (_, i) => resource(category, 10001 + i)),
  ),
];
await context.addInitScript(() => {
  localStorage.setItem("apmm.welcomed", "true");
  localStorage.setItem("apmm.reduce-motion", "true");
  window.__six = {
    calls: [],
    receipt: null,
    mode: "success",
    inherited: localStorage.getItem("six.inherited") === "true",
    detected: localStorage.getItem("six.detected") === "true",
    fresh: localStorage.getItem("six.fresh") === "true",
  };
});
await page.route("**/__workspace", (route) =>
  route.fulfill({ json: { resources, gameDir: "隔离游戏夹具" } }),
);
await page.route("**/src-ui/core/services/app.ts*", async (route) => {
  const response = await route.fetch();
  await route.fulfill({
    response,
    body:
      (await response.text()) +
      `
const sixCommand = appService.command;
appService.native = true;
appService.command = async (name, args) => {
  if (name === 'migration_status') return {mode:window.__six.fresh ? 'fresh' : window.__six.inherited ? 'inherited' : 'migrated',destination:'B:/APMM-next-work/new-fixture',mods:window.__six.fresh ? 0 : 806,multipleSlots:1,sources:window.__six.detected ? [{source:'B:/APMM-next-work/legacy-fixture',version:'0.3.36',mods:1028,newMods:226}] : []};
  if (name === 'migration_inspect' && window.__six.self) return {source:'B:/APMM-next-work/new-fixture',destination:'B:/APMM-next-work/new-fixture',mode:'inherited',message:'所选目录就是当前正在使用的 data，已就地接续，无需复制到自身。'};
  if (name === 'migration_inspect') return {source:'B:/APMM-next-work/legacy-fixture',destination:'B:/APMM-next-work/new-fixture',mods:1028,newMods:226,files:2949,bytes:1146528228,mode:window.__six.migrationMode || 'merge'};
  if (name.startsWith('update_')) window.__six.calls.push({name, args,at:Date.now()});
  if (name === 'update_check') { if (window.__six.mode === 'check-error') throw Error('检查失败夹具'); return {available:true, latestVersion:'99.0.0', notes:'界面验收版本'}; }
  if (name === 'update_startup_status') return null;
  if (name === 'update_download_status') return window.__six.receipt;
  if (name === 'update_apply') { if (window.__six.mode === 'install-error') throw Error('安装失败夹具'); return '隔离安装交接'; }
  return sixCommand(name, args);
};
const sixTask = appService.task;
appService.chooseDirectory = async () => 'B:/APMM-next-work/legacy-fixture';
appService.task = async (kind, title, args) => { if (kind === 'migration') {window.__six.calls.push({name:kind,title,args});return 'migration-1';} if (kind !== 'update-download') return sixTask(kind,title,args); window.__six.calls.push({name:kind,args}); return 'update-1'; };
const sixWait = appService.waitTask;
appService.waitTask = async (id, signal) => {
  if (id !== 'update-1') return sixWait(id,signal);
  await new Promise(resolve => setTimeout(resolve, 350));
  if (window.__six.mode === 'cancelled') return {status:'cancelled',message:'取消下载夹具'};
  if (window.__six.mode === 'failed') return {status:'failed',message:'下载失败夹具'};
  window.__six.receipt = {version:window.__six.mode === 'wrong-receipt' ? '98.0.0':'99.0.0', path:'B:/APMM-next-work/fixture.zip',bytes:1234,kind:'portable'};
  return {status:'completed'};
};
`,
  });
});
const nav = (name) =>
  page.locator(".sidebar").getByRole("button", { name, exact: true });
async function go(name, route) {
  await nav(name).click();
  await page.locator(`main[data-route="${route}"]`).waitFor();
}
function sizeTrigger() {
  return page
    .locator(".showing-detail")
    .count()
    .then((count) =>
      (count
        ? page.locator(".detail-top")
        : page.locator(".resource-toolbar")
      ).getByRole("combobox", { name: "预览尺寸", exact: true }),
    );
}
async function chooseSize(value) {
  await (await sizeTrigger()).click();
  await page.getByRole("option", { name: value, exact: true }).click();
}
async function toggleUnified(checked) {
  await (await sizeTrigger()).click();
  await page
    .getByRole("checkbox", { name: "统一大小", exact: true })
    .setChecked(checked);
  await page.keyboard.press("Escape");
}
async function noOverflow() {
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
    false,
  );
}
try {
  await page.goto(process.env.APMM_UI_URL || "http://127.0.0.1:1421/");
  await page.getByRole("button", { name: "查看更新", exact: true }).waitFor();
  await page.waitForTimeout(6800);
  assert.equal(
    await page.getByRole("button", { name: "查看更新", exact: true }).count(),
    1,
  );
  await page.getByRole("button", { name: "查看更新", exact: true }).click();
  assert.equal(
    await page
      .getByRole("tab", { name: "更新", exact: true })
      .getAttribute("aria-selected"),
    "true",
  );
  assert.equal(
    await page.getByRole("checkbox", { name: /自动检查更新/ }).isChecked(),
    true,
  );
  await go("总览", "overview");
  await page.locator(".overview-version-card").first().click();
  await page.getByRole("heading", { name: "软件更新", exact: true }).waitFor();
  checks.push("批注1：版本卡和持续更新提醒均直达更新分区，自动检查默认开启");
  await page
    .getByRole("button", { name: "安装更新并重启", exact: true })
    .click();
  await page.waitForFunction(() =>
    window.__six.calls.some((call) => call.name === "update_apply"),
  );
  assert.equal(await page.getByRole("dialog").count(), 0);
  assert.deepEqual(
    await page.evaluate(() =>
      window.__six.calls
        .filter((call) =>
          ["update-download", "update_apply"].includes(call.name),
        )
        .map((call) => call.name),
    ),
    ["update-download", "update_apply"],
  );
  await page.screenshot({ path: path.join(output, "01-update.png") });
  for (const mode of ["cancelled", "failed", "wrong-receipt"]) {
    await page.evaluate((mode) => {
      window.__six.mode = mode;
      window.__six.receipt = null;
      window.__six.calls = [];
    }, mode);
    await page
      .getByRole("button", { name: "安装更新并重启", exact: true })
      .click();
    await page
      .getByText(
        mode === "cancelled"
          ? "取消下载夹具"
          : mode === "failed"
            ? "下载失败夹具"
            : "更新包与目标版本不一致，请重新检查更新",
        { exact: true },
      )
      .waitFor();
    assert.equal(
      await page.evaluate(() =>
        window.__six.calls.some((call) => call.name === "update_apply"),
      ),
      false,
    );
  }
  checks.push(
    "批注1：一次点击串联下载与安装，取消、下载失败、收据错误均未安装",
  );
  await page.evaluate(() => {
    window.__six.mode = "success";
  });
  await go("角色", "heroes");
  const search = page.getByRole("combobox", { name: "搜索模式", exact: true });
  const triggerBox = await search.boundingBox();
  await search.click();
  const dropdownBox = await page.locator(".select-content").boundingBox();
  assert.ok(Math.abs(triggerBox.x - dropdownBox.x) <= 2, "搜索下拉左边缘对齐");
  assert.ok(
    dropdownBox.y >= triggerBox.y + triggerBox.height,
    "浮层不覆盖触发器",
  );
  await page.screenshot({ path: path.join(output, "02-search-select.png") });
  await page.keyboard.press("Escape");
  checks.push("批注2：1014宽度搜索下拉与触发器左边缘对齐");
  await (await sizeTrigger()).click();
  assert.deepEqual(await page.getByRole("option").allTextContents(), [
    "特小",
    "小",
    "中",
    "标准",
    "大",
    "特大",
  ]);
  assert.equal(
    await page.getByRole("checkbox", { name: "统一大小" }).isChecked(),
    true,
  );
  const listBox = await page
    .getByRole("listbox", { name: "预览尺寸档位" })
    .boundingBox();
  const toggleBox = await page.locator(".size-unified-toggle").boundingBox();
  assert.ok(
    toggleBox.x >= listBox.x + listBox.width - 1,
    "统一大小在展开面板右侧",
  );
  await page.screenshot({ path: path.join(output, "03-size-panel.png") });
  await page.keyboard.press("Escape");
  await chooseSize("标准");
  for (const [name, route] of [
    ["卡面", "cards"],
    ["怪物", "monsters"],
    ["筹码", "chips"],
    ["地块", "lands"],
    ["鼠标", "cursors"],
    ["临时", "temporary"],
    ["角色", "heroes"],
  ]) {
    await go(name, route);
    assert.equal(await (await sizeTrigger()).innerText(), "标准");
  }
  await toggleUnified(false);
  await chooseSize("特小");
  await go("卡面", "cards");
  await chooseSize("特大");
  await go("角色", "heroes");
  assert.equal(await (await sizeTrigger()).innerText(), "特小");
  await toggleUnified(true);
  await go("卡面", "cards");
  assert.equal(await (await sizeTrigger()).innerText(), "特小");
  await toggleUnified(false);
  assert.equal(await (await sizeTrigger()).innerText(), "特大");
  checks.push(
    "批注3：六档顺序、右侧默认勾选、七板块同步、独立设置与重新统一后恢复均通过",
  );
  await toggleUnified(true);
  for (const width of [1014, 800, 700]) {
    await page.setViewportSize({ width, height: 905 });
    for (const [size, pixels] of width === 1014
      ? [
          ["特小", 52],
          ["小", 64],
          ["中", 76],
          ["标准", 88],
          ["大", 112],
          ["特大", 144],
        ]
      : [["标准", 88]]) {
      await go("卡面", "cards");
      await chooseSize(size);
      const list = await measureCandidates(
        page,
        ".resource-list .part-row",
        pixels,
      );
      if (size === "标准" && width === 1014)
        await page.screenshot({
          path: path.join(output, "07-standard-list.png"),
        });
      await page.locator(".candidate-more").first().click();
      const cardDetail = await measureCandidates(
        page,
        ".detail-page .part-row",
        pixels,
      );
      assert.deepEqual(
        cardDetail.layout,
        list.layout,
        "卡面列表和详情内部布局一致",
      );
      const details = { cards: cardDetail };
      for (const [name, route] of [
        ["怪物", "monsters"],
        ["角色", "heroes"],
      ]) {
        await go(name, route);
        await page.locator(".resource-open").first().click();
        const detail = await measureCandidates(
          page,
          ".detail-page .part-row",
          pixels,
        );
        assert.deepEqual(
          detail.layout,
          list.layout,
          `${name}详情和列表内部布局一致`,
        );
        details[route] = detail;
        if (size === "标准" && width === 1014)
          await page.screenshot({
            path: path.join(output, `07-standard-${route}.png`),
          });
      }
      await noOverflow();
      candidateMetrics.push({ width, size, pixels, list, details });
    }
  }
  checks.push(
    "追加批注1/2：六档候选、原版、添加和更多实际边长一致；卡面列表/详情、怪物详情与角色详情共用布局，800/700窄窗通过",
  );
  await page.setViewportSize({ width: 1014, height: 905 });
  await go("角色", "heroes");
  for (const size of ["特小", "小", "中", "标准", "大", "特大"]) {
    await chooseSize(size);
    const rows = await page.locator(".resource-row").evaluateAll((rows) =>
      rows.map((row) => {
        const r = row.getBoundingClientRect();
        return { top: r.top, bottom: r.bottom };
      }),
    );
    assert.ok(
      rows.every((row, i) => !i || row.top >= rows[i - 1].bottom - 1),
      `${size}虚拟行不重叠`,
    );
  }
  await page.locator(".resource-open").first().click();
  await page.locator(".detail-top").waitFor();
  await chooseSize("小");
  assert.equal(
    await page
      .locator(".detail-top")
      .getByRole("combobox", { name: "预览尺寸", exact: true })
      .innerText(),
    "小",
  );
  await page.screenshot({ path: path.join(output, "04-detail-size.png") });
  await nav("角色").click();
  await page.locator(".showing-detail").waitFor({ state: "detached" });
  await go("设置", "settings");
  await page.getByRole("tab", { name: "外观", exact: true }).click();
  await nav("设置").click();
  assert.equal(
    await page
      .getByRole("tab", { name: "目录与资源", exact: true })
      .getAttribute("aria-selected"),
    "true",
  );
  checks.push("批注4/5：详情可调尺寸；重复点击角色和设置侧栏均返回一级界面");
  await verifyMigrationDialog(page, output);
  checks.push(
    "追加迁移批注：确认框区分空库完整复制与非空库补齐，显示全部候选和待补数量，确认后才提交任务",
  );
  await go("APX 插件", "apx");
  const directories = page.locator(".apx-directories");
  const metrics = await directories.evaluate((element) => ({
    height: element.getBoundingClientRect().height,
    summaryFont: getComputedStyle(element.querySelector("summary")).fontSize,
    rowHeights: [...element.querySelectorAll(".directory-row")].map(
      (row) => row.getBoundingClientRect().height,
    ),
  }));
  assert.ok(metrics.height < 280, `目录卡片高度${metrics.height}`);
  assert.equal(metrics.summaryFont, "13px");
  const apxCardStyles = await page
    .locator(".apx-page details.settings-card")
    .evaluateAll((cards) =>
      cards.map((card) => {
        const summary = getComputedStyle(card.querySelector("summary"));
        return {
          padding: getComputedStyle(card).padding,
          fontSize: summary.fontSize,
          fontWeight: summary.fontWeight,
        };
      }),
    );
  assert.equal(apxCardStyles.length, 2);
  assert.deepEqual(
    apxCardStyles[0],
    apxCardStyles[1],
    "目录和文件校验卡的标题字号、字重、留白统一",
  );
  await noOverflow();
  await page.screenshot({ path: path.join(output, "06-apx-1014.png") });
  await page.setViewportSize({ width: 800, height: 720 });
  await noOverflow();
  await page.screenshot({ path: path.join(output, "06-apx-800.png") });
  await page.setViewportSize({ width: 716, height: 796 });
  await noOverflow();
  await page
    .locator(".apx-page details.settings-card")
    .last()
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(output, "06-apx-716.png") });
  checks.push(
    `批注6：APX目录高度${metrics.height}px，标题13px，1014和800宽度无横向溢出`,
  );
  await verifyInheritanceDetection(page, output, go);
  checks.push(
    "追加继承批注：同目录接续不弹自复制确认；已有迁移标记仍提醒缺失备选，自动发现后直达检查入口",
  );
  await verifyMinuteSchedule(page, output, go);
  checks.push(
    "追加更新批注：分钟输入保存并重新进入保留，真实计时器在一分钟到期触发，关闭自动检查后取消调度",
  );
  assert.deepEqual(errors, []);
  writeFileSync(
    path.join(output, "result.json"),
    JSON.stringify(
      { status: "passed", checks, metrics, candidateMetrics, errors },
      null,
      2,
    ),
  );
  console.log(JSON.stringify({ output, checks, metrics, errors }, null, 2));
} catch (error) {
  await page
    .screenshot({ path: path.join(output, "failure.png") })
    .catch(() => {});
  writeFileSync(
    path.join(output, "result.json"),
    JSON.stringify(
      {
        status: "failed",
        checks,
        errors,
        error: String(error),
        stack: error.stack,
      },
      null,
      2,
    ),
  );
  console.error(
    JSON.stringify({ output, checks, errors, error: String(error) }, null, 2),
  );
  process.exitCode = 1;
} finally {
  await context.close();
}
