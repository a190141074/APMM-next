// 浏览器展示与键盘验收使用受限内存夹具，截图和 profile 不进入源码或用户目录。
import { spawn } from "node:child_process";
import { chromium } from "playwright";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
import { verifyManagementAbout } from "./management-about-ui.mjs";
import { verifyModInfoUI } from "./management-mod-info-ui.mjs";
const root = path.resolve(import.meta.dirname, "..");
const output = path.join("B:/APMM-next-work/management-ui", randomUUID());
mkdirSync(output, { recursive: true });
const port = 1439;
const server = spawn(
  process.execPath,
  [
    "node_modules/vite/bin/vite.js",
    "--host",
    "127.0.0.1",
    "--port",
    String(port),
    "--strictPort",
  ],
  {
    cwd: root,
    windowsHide: true,
    stdio: "ignore",
    env: {
      ...process.env,
      TEMP: "B:/APMM-next-work/temp",
      TMP: "B:/APMM-next-work/temp",
    },
  },
);
const image =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/lX8AAAAASUVORK5CYII=";
const catalog = JSON.parse(
  readFileSync(path.join(root, "config/resources.json"), "utf8"),
);
const resources = ["UT_Hero_Card_101", "UT_Hero_Card2_101"].map((cn, i) => ({
  ...catalog.find((r) => r.canonical === cn),
  originalUrl: image,
  activeHash: null,
  disabled: false,
  files: [
    {
      hash: String(i + 1).repeat(64),
      sourceName: `示例候选 ${i + 1}`,
      path: image,
      info: {
        provenance: { author: "", sourceUrl: "", version: "" },
        note: "",
      },
    },
  ],
}));
const exe = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
let context;
const checks = [],
  errors = [];
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
try {
  for (let i = 0; i < 80; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}`);
      if (r.ok) break;
    } catch {}
    await delay(250);
  }
  context = await chromium.launchPersistentContext(
    path.join(output, "profile"),
    {
      executablePath: exe,
      headless: true,
      viewport: { width: 1360, height: 900 },
    },
  );
  await context.addInitScript(() => {
    localStorage.setItem("apmm.welcomed", "true");
    localStorage.setItem("apmm.theme", "light");
  });
  const page = await context.newPage();
  page.on("pageerror", (e) => errors.push(e.message));
  await page.route("**/__workspace", (route) =>
    route.fulfill({ json: { resources, gameDir: "隔离浏览器夹具" } }),
  );
  await page.goto(`http://127.0.0.1:${port}`);
  await page.getByRole("button", { name: "预设", exact: true }).click();
  await page.getByRole("button", { name: "创建预设", exact: true }).click();
  await page.getByLabel("预设名称", { exact: true }).fill("角色局部");
  await page.getByRole("combobox", { name: "创建范围" }).click();
  await page.getByRole("option", { name: "指定角色", exact: true }).click();
  await page.getByText("已选择 2 个槽位", { exact: true }).waitFor();
  await page.locator(".scope-options input").first().uncheck();
  await page.getByText("已选择 1 个槽位", { exact: true }).waitFor();
  await page.getByRole("button", { name: "保存当前搭配", exact: true }).click();
  await page.getByRole("button", { name: "角色局部", exact: true }).waitFor();
  await page.getByRole("checkbox", { name: /显示全部保存范围/ }).check();
  await page.screenshot({ path: path.join(output, "presets-local-light.png") });
  checks.push("独立预设页、角色默认全范围、取消部位及原版详情");
  await page.getByRole("button", { name: "锁定", exact: true }).click();
  await page.getByRole("button", { name: "解锁", exact: true }).waitFor();
  assert.equal(
    await page
      .getByRole("button", { name: "编辑 / 修复", exact: true })
      .isDisabled(),
    true,
  );
  await page.getByRole("button", { name: "复制", exact: true }).click();
  await page
    .getByRole("button", { name: "角色局部 副本", exact: true })
    .waitFor();
  await page.getByRole("button", { name: "编辑 / 修复", exact: true }).click();
  await page.getByRole("button", { name: "保存编辑", exact: true }).click();
  checks.push("锁定限制、未锁定副本和编辑保存");
  await page.getByRole("button", { name: "角色", exact: true }).click();
  await page.locator(".resource-open").first().click();
  await page.getByTitle("管理 示例候选 1", { exact: true }).click();
  await page.getByRole("dialog", { name: "模组信息", exact: true }).waitFor();
  await page.getByLabel("作者", { exact: true }).fill("浏览器作者");
  await page
    .getByLabel("来源链接（HTTPS）", { exact: true })
    .fill("https://example.com/mod");
  await page.getByLabel("版本", { exact: true }).fill("2.0");
  await page.getByLabel("备注（仅本机）", { exact: true }).fill("本机备注");
  await page.getByRole("button", { name: "保存信息", exact: true }).click();
  await verifyModInfoUI({ page, output, checks });
  await page.getByRole("combobox", { name: "归属部位" }).click();
  await page
    .getByRole("option")
    .filter({ hasText: "UT_Hero_Card2_101" })
    .click();
  await page.getByRole("button", { name: "更改归属", exact: true }).click();
  await page.getByRole("button", { name: "确认归类", exact: true }).waitFor();
  await page.screenshot({ path: path.join(output, "mod-info-light.png") });
  checks.push("模组信息四字段、归属四级选择与具体归类确认");
  await page.setViewportSize({ width: 800, height: 880 });
  await page.evaluate(() => (document.documentElement.dataset.theme = "dark"));
  await page.waitForTimeout(220);
  await page.screenshot({
    path: path.join(output, "mod-info-narrow-dark.png"),
  });
  assert.equal(
    await page
      .getByRole("button", { name: "确认归类", exact: true })
      .isEnabled(),
    true,
  );
  await page.getByRole("button", { name: "关闭", exact: true }).click();
  const search = page.getByRole("combobox", { name: "搜索资源" });
  await search.fill("Parunam");
  await page.getByText("相似结果", { exact: true }).first().waitFor();
  await search.press("ArrowDown");
  await search.press("Enter");
  await page.locator(".detail-page").waitFor();
  await page.screenshot({
    path: path.join(output, "search-keyboard-dark.png"),
  });
  checks.push("窄窗口深色主题、名称相似标记和键盘定位");
  await verifyManagementAbout({ page, output, checks });
  assert.deepEqual(errors, []);
} catch (e) {
  errors.push(e.stack || String(e));
  console.error(e);
} finally {
  await context?.close();
  server.kill();
  const report = {
    version: "0.3.39",
    output,
    checks,
    errors,
    allPassed: !errors.length,
  };
  writeFileSync(
    path.join(root, "verification-management-ui.json"),
    JSON.stringify(report, null, 2) + "\n",
  );
  console.log(JSON.stringify(report, null, 2));
}
if (errors.length) process.exitCode = 1;
