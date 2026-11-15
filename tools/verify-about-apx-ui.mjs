// 关于、主题选择器和平台目录的隔离交互验收；路径夹具不写入真实游戏或工作区。
import { chromium } from "playwright";
import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const output = path.join(
  "B:/APMM-next-work/about-apx-validation",
  randomUUID(),
);
mkdirSync(output, { recursive: true });
process.env.TEMP = "B:/APMM-next-work/temp";
process.env.TMP = process.env.TEMP;
const executablePath = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
const context = await chromium.launchPersistentContext(
  path.join(output, "profile"),
  { executablePath, headless: true, viewport: { width: 1405, height: 852 } },
);
const page = await context.newPage();
const checks = [],
  errors = [];
page.on("pageerror", (e) => errors.push(e.message));
const nav = (name) =>
  page.locator(".nav-item").filter({ hasText: name }).first();
async function select(label, option) {
  await page.getByRole("combobox", { name: label, exact: true }).click();
  await page.getByRole("option", { name: option, exact: true }).click();
}
try {
  await context.addInitScript(() => {
    localStorage.setItem("apmm.theme", "light");
    localStorage.setItem("apmm.welcomed", "true");
  });
  await page.goto("http://127.0.0.1:1420/");
  await page
    .getByRole("button", { name: "安装 APX 插件", exact: true })
    .click();
  await page
    .getByRole("heading", { name: "APX 客户端补丁", exact: true })
    .waitFor();
  await nav("总览").click();
  assert.match(
    await page.locator(".quick-card").first().innerText(),
    /1. 安装 APX/,
  );
  checks.push("总览首要入口与第一步为安装 APX，进入渠道与适配检查页");
  await nav("关于").click();
  await page
    .getByRole("heading", { name: "鸣谢 · 参考项目", exact: true })
    .waitFor();
  const navTexts = await page
    .locator("nav section:last-child .nav-item")
    .allTextContents();
  assert.deepEqual(navTexts, ["设置", "关于"]);
  assert.equal(
    await page
      .locator('a[href="https://github.com/NapNeko/NapCatQQ-Desktop"]')
      .count(),
    1,
  );
  assert.equal(
    await page.getByRole("textbox", { name: "查找第三方依赖" }).count(),
    0,
  );
  assert.equal(await page.locator(".credit-row").count(), 9);
  await page.getByRole("button", { name: "第三方声明", exact: true }).click();
  await page
    .getByRole("textbox", { name: "查找第三方依赖" })
    .fill("react 18.3.1");
  const credit = page
    .getByRole("dialog")
    .locator(".credit-row")
    .filter({ has: page.getByText("react 18.3.1", { exact: true }) });
  await credit.getByRole("button", { name: "许可证原文" }).click();
  const dialog = page.getByRole("dialog", {
    name: "react 18.3.1",
    exact: true,
  });
  assert.match(
    await dialog.locator("pre").innerText(),
    /Permission is hereby granted, free of charge/,
  );
  assert.match(await dialog.locator("pre").innerText(), /Copyright/);
  await page.keyboard.press("Escape");
  await page.screenshot({ path: path.join(output, "about-light.png") });
  checks.push(
    "设置下方独立关于页、项目引用、筛选依赖、MIT 与版权完整原文可阅读",
  );
  await nav("角色").click();
  await page.getByRole("combobox", { name: "搜索模式", exact: true }).click();
  assert.equal(await page.locator(".select-content").count(), 1);
  await page.screenshot({ path: path.join(output, "search-select-light.png") });
  await page.getByRole("option", { name: "ID", exact: true }).click();
  assert.match(
    await page
      .getByRole("combobox", { name: "搜索资源" })
      .getAttribute("placeholder"),
    /完整资源键/,
  );
  await page.getByRole("combobox", { name: "搜索模式", exact: true }).focus();
  await page.keyboard.press("Enter");
  await page.getByRole("listbox").waitFor();
  await page.waitForFunction(
    () => document.activeElement?.getAttribute("role") === "option",
  );
  await page.keyboard.press("Home");
  await page.waitForFunction(
    () => document.activeElement?.textContent === "智能",
  );
  await page.keyboard.press("Enter");
  await page.waitForFunction(() =>
    document
      .querySelector('[aria-label="搜索资源"]')
      ?.getAttribute("placeholder")
      ?.includes("昵称"),
  );
  assert.match(
    await page
      .getByRole("combobox", { name: "搜索资源" })
      .getAttribute("placeholder"),
    /昵称/,
  );
  await select("资源部位", "全部部位");
  await select("预览尺寸", "小图");
  await nav("设置").click();
  await select("主题", "深色");
  await nav("角色").click();
  await page.getByRole("combobox", { name: "搜索模式", exact: true }).click();
  await page.screenshot({ path: path.join(output, "search-select-dark.png") });
  await page.keyboard.press("Escape");
  await nav("辅助插件").click();
  await select("提取范围", "全部卡面");
  checks.push(
    "搜索、资源筛选、预览、主题与提取选择器使用 Radix 主题浮层，键盘选择有效",
  );
  await nav("APX 插件").click();
  assert.equal(
    await page.getByRole("heading", { name: "补丁缓存", exact: true }).count(),
    0,
  );
  assert.equal(
    await page
      .getByRole("button", { name: "下载最新缓存", exact: true })
      .count(),
    0,
  );
  assert.equal(
    await page.getByRole("button", { name: "安装补丁", exact: true }).count(),
    1,
  );
  assert.equal(
    await page.getByRole("button", { name: "导出补丁", exact: true }).count(),
    1,
  );
  assert.equal(
    await page.getByRole("button", { name: "智能匹配", exact: true }).count(),
    3,
  );
  await page.screenshot({ path: path.join(output, "apx-dark.png") });
  checks.push(
    "独立补丁缓存卡片已移除，三个目录都有智能匹配，安装/还原/导出入口保留",
  );
  await page.route("**/src-ui/core/ipc/mock.ts*", async (route) => {
    const response = await route.fetch();
    const body = await response.text();
    const marker = "if (command === IPC.workspace_snapshot)";
    assert.ok(body.includes(marker));
    await route.fulfill({
      response,
      body: body.replace(
        marker,
        `
      if (command === IPC.plugin_status) {
        const dirs = window.__platformPaths[args.platform] || {};
        return {platform:args.platform, locallowDir:dirs.locallow || "", gameDataDir:dirs.data || "", gameRootDir:dirs.root || "", remoteVersion:"test", files:[]};
      }
      if (command === IPC.plugin_match_directory) {
        return structuredClone(window.__directoryCandidates[args.platform]?.[args.which] || []);
      }
      if (command === IPC.plugin_set_directory) {
        window.__directoryWrites.push(structuredClone(args));
        const dirs = window.__platformPaths[args.platform] ||= {};
        dirs[args.which] = args.path;
        return structuredClone(dirs);
      }
      ${marker}`,
      ),
    });
  });
  await context.addInitScript(() => {
    window.__platformPaths = {
      CN_STEAM: {
        root: "B:/steam-original",
        data: "B:/steam-original/Data",
        locallow: "B:/steam-cache",
      },
      CN_TAPTAP: {
        root: "B:/tap-original",
        data: "B:/tap-original/Data",
        locallow: "B:/tap-cache",
      },
    };
    window.__directoryCandidates = {
      CN_STEAM: { root: ["B:/steam-copy-1", "B:/steam-copy-2"] },
      CN_TAPTAP: { locallow: ["B:/tap-new-cache"] },
    };
    window.__directoryWrites = [];
    window.__pickedDirectory = "B:/manual-root";
  });
  await page.route("**/src-ui/core/services/app.ts*", async (route) => {
    const response = await route.fetch();
    const body = await response.text();
    assert.ok(body.includes("chooseDirectory: transport.pickDirectory"));
    await route.fulfill({
      response,
      body: body.replace(
        "chooseDirectory: transport.pickDirectory",
        "chooseDirectory: async () => window.__pickedDirectory",
      ),
    });
  });
  await page.reload();
  await nav("APX 插件").click();
  const rootRow = page
    .locator(".directory-row")
    .filter({ hasText: "游戏根目录" });
  await rootRow.getByText("B:/steam-original", { exact: true }).waitFor();
  await rootRow.getByRole("button", { name: "智能匹配", exact: true }).click();
  const match = page.getByRole("dialog", {
    name: "智能匹配 · 游戏根目录",
    exact: true,
  });
  await match.waitFor();
  assert.equal(await match.locator(".directory-candidate").count(), 2);
  assert.equal(await page.evaluate(() => window.__directoryWrites.length), 0);
  await match
    .locator(".directory-candidate")
    .filter({ hasText: "B:/steam-copy-2" })
    .getByRole("button", { name: "使用此目录", exact: true })
    .click();
  await match.waitFor({ state: "hidden" });
  await rootRow.getByText("B:/steam-copy-2", { exact: true }).waitFor();
  await page.getByRole("button", { name: "TapTap", exact: true }).click();
  await rootRow.getByText("B:/tap-original", { exact: true }).waitFor();
  const localRow = page
    .locator(".directory-row")
    .filter({ hasText: "LocalLow 目录" });
  await localRow.getByRole("button", { name: "智能匹配", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "使用此目录", exact: true })
    .click();
  await localRow.getByText("B:/tap-new-cache", { exact: true }).waitFor();
  await rootRow.getByRole("button", { name: "更改", exact: true }).click();
  await rootRow.getByText("B:/manual-root", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Steam 国服", exact: true }).click();
  await rootRow.getByText("B:/steam-copy-2", { exact: true }).waitFor();
  const paths = await page.evaluate(() => window.__platformPaths);
  assert.deepEqual(paths.CN_STEAM, {
    root: "B:/steam-copy-2",
    data: "B:/steam-original/Data",
    locallow: "B:/steam-cache",
  });
  assert.deepEqual(paths.CN_TAPTAP, {
    root: "B:/manual-root",
    data: "B:/tap-original/Data",
    locallow: "B:/tap-new-cache",
  });
  assert.deepEqual(await page.evaluate(() => window.__directoryWrites), [
    { platform: "CN_STEAM", which: "root", path: "B:/steam-copy-2" },
    { platform: "CN_TAPTAP", which: "locallow", path: "B:/tap-new-cache" },
    { platform: "CN_TAPTAP", which: "root", path: "B:/manual-root" },
  ]);
  checks.push(
    "多安装先选候选，手动更改与智能匹配携带正确平台，互不覆盖其他平台和其他目录",
  );
  for (const channel of ["哔哩哔哩", "WeGame", "Steam 国际服"]) {
    await page.getByRole("button", { name: channel, exact: true }).click();
    await page.waitForFunction(
      () =>
        document.querySelectorAll(".directory-row code")[2]?.textContent ===
        "尚未定位",
    );
  }
  await rootRow.getByRole("button", { name: "智能匹配", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByText(/未找到可确认的目录/)
    .waitFor();
  assert.equal(
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "使用此目录", exact: true })
      .count(),
    0,
  );
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Android", exact: true }).click();
  assert.equal(
    await page
      .getByRole("button", { name: "智能匹配", exact: true })
      .first()
      .isDisabled(),
    true,
  );
  checks.push(
    "其他平台没有借用已配置渠道的路径，匹配失败不写入，Android 不设置本机目录",
  );
  await page.setViewportSize({ width: 1089, height: 743 });
  await page.getByRole("button", { name: "Steam 国服", exact: true }).click();
  await rootRow.getByText("B:/steam-copy-2", { exact: true }).waitFor();
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.screenshot({
    path: path.join(output, "apx-independent-directories.png"),
  });
  for (const label of ["地块", "鼠标", "临时"])
    assert.equal(
      await page.locator(".nav-item").getByText(label, { exact: true }).count(),
      1,
    );
  await nav("角色").click();
  await page.locator(".resource-open").first().click();
  const part = page
    .locator(".part-row")
    .filter({ has: page.locator(".candidate:nth-child(2)") })
    .first();
  await part.locator('.candidate:nth-child(2) button[title^="比较"]').click();
  await page.locator(".pan-zoom-image").nth(1).waitFor();
  const original = page.locator(".pan-zoom-image").first(),
    preview = page.locator(".pan-zoom-image").nth(1);
  const zoomState = () =>
    preview.locator(".pan-zoom-content").evaluate((el) => {
      const m = new DOMMatrix(getComputedStyle(el).transform);
      return { scale: m.a, x: m.e, y: m.f };
    });
  const area = await preview.boundingBox();
  const center = { x: area.x + area.width / 2, y: area.y + area.height / 2 };
  await page.mouse.move(center.x, center.y);
  await page.mouse.wheel(0, -300);
  await page.waitForFunction(
    () =>
      Number(
        document
          .querySelectorAll(".pan-zoom-scale")[1]
          ?.textContent?.replace("%", ""),
      ) > 100,
  );
  assert.ok((await zoomState()).scale > 1);
  assert.equal(await original.locator(".pan-zoom-scale").innerText(), "100%");
  await page.mouse.down();
  await page.mouse.move(center.x + 75, center.y + 40, { steps: 5 });
  await page.mouse.up();
  const shifted = await zoomState();
  assert.ok(
    Math.abs(shifted.x - 75) < 2 && Math.abs(shifted.y - 40) < 2,
    JSON.stringify(shifted),
  );
  await preview.dblclick();
  assert.deepEqual(await zoomState(), { scale: 1, x: 0, y: 0 });
  await page.mouse.move(center.x, center.y);
  await page.mouse.wheel(0, 400);
  await page.waitForFunction(
    () =>
      Number(
        document
          .querySelectorAll(".pan-zoom-scale")[1]
          ?.textContent?.replace("%", ""),
      ) < 100,
  );
  await preview.press("Home");
  assert.deepEqual(await zoomState(), { scale: 1, x: 0, y: 0 });
  await page.screenshot({ path: path.join(output, "comparison-pan-zoom.png") });
  await page.keyboard.press("Escape");
  await part.locator('.candidate:nth-child(2) button[title^="比较"]').click();
  assert.equal(await preview.locator(".pan-zoom-scale").innerText(), "100%");
  await page.keyboard.press("Escape");
  checks.push(
    "地块/鼠标/临时两字导航，真实对比图片滚轮缩放、拖动、双击与 Home 复位、两侧独立、重新打开复位",
  );
  assert.deepEqual(errors, []);
} finally {
  writeFileSync(
    "verification-about-apx-ui.json",
    JSON.stringify({ output, checks, errors }, null, 2),
  );
  await context.close();
}
console.log(JSON.stringify({ output, checks, errors }, null, 2));
