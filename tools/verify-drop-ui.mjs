// 浏览器只验收悬停、清理与关于页，不把合成 DOM 拖入当作本地导入成功。
import { chromium } from "playwright";
import { mkdirSync, existsSync, writeFileSync } from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
const output = path.join(
  process.env.APMM_DROP_WORK || "B:/APMM-next-work/drop-validation",
  "browser",
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
  { executablePath, headless: true, viewport: { width: 1145, height: 796 } },
);
const page = await context.newPage();
const checks = [],
  errors = [];
page.on("pageerror", (e) => errors.push(e.message));
const nav = (name) =>
  page.locator(".nav-item").filter({ hasText: name }).first();
async function drag(type, position, name = "unknown.png") {
  await page.evaluate(
    ({ type, position, name }) => {
      const transfer = new DataTransfer();
      transfer.items.add(new File(["synthetic"], name, { type: "image/png" }));
      const element =
        document.elementFromPoint(position.x, position.y) || document.body;
      element.dispatchEvent(
        new DragEvent(type, {
          bubbles: true,
          cancelable: true,
          dataTransfer: transfer,
          clientX: position.x,
          clientY: position.y,
        }),
      );
    },
    { type, position, name },
  );
}
try {
  await context.addInitScript(() =>
    localStorage.setItem("apmm.welcomed", "true"),
  );
  await page.goto(process.env.APMM_UI_URL || "http://127.0.0.1:1420/");
  await nav("角色").click();
  await page.locator(".resource-open").first().waitFor();
  assert.equal(
    await page.locator(".resource-card[data-drop-resource-key]").count(),
    0,
  );
  await page.locator(".resource-open").first().click();
  const row = page.locator(".part-row[data-drop-resource-key]").first();
  await row.waitFor();
  const box = await row.boundingBox();
  const position = {
    x: box.x + Math.min(180, box.width / 2),
    y: box.y + box.height / 2,
  };
  await drag("dragenter", position);
  await drag("dragover", position);
  await page.locator(".resource-drop-target").waitFor();
  assert.match(
    await page.locator(".resource-drop-hint").textContent(),
    /松开导入到.*浏览器预览/,
  );
  assert.equal(await page.locator(".app.resource-drop-active").count(), 1);
  await drag("dragleave", position);
  await page.locator(".resource-drop-hint").waitFor({ state: "detached" });
  assert.equal(await page.locator(".resource-drop-target").count(), 0);
  checks.push("浏览器明确部位悬停、窗口边框、提示及离开清理");
  await drag("dragenter", position);
  await drag("drop", position);
  await page
    .getByText("请在桌面版拖入本地 PNG／APMod；浏览器预览只展示拖入反馈。", {
      exact: true,
    })
    .waitFor();
  assert.equal(await page.getByRole("dialog").count(), 0);
  assert.equal(await page.locator(".resource-drop-hint").count(), 0);
  await drag("dragenter", position);
  await nav("设置").click();
  await page.locator(".resource-drop-hint").waitFor({ state: "detached" });
  checks.push("提交清除且不伪造入库，切页清除；整名角色卡不猜部位");
  await page.getByRole("tab", { name: "关于", exact: true }).click();
  await page.getByText("AstralParty.Toys", { exact: true }).waitFor();
  const references = page.locator(".settings-card").filter({
    has: page.getByRole("heading", { name: "鸣谢 · 参考项目", exact: true }),
  });
  assert.match(
    await references.textContent(),
    /逐文件元数据缓存与启动扫描优化参考/,
  );
  assert.match(
    await references.textContent(),
    /工坊投稿资料、预览图与提交流程参考/,
  );
  for (const [name, url] of [
    ["AstralParty.Toys", "https://github.com/higashitaniyume/AstralParty.Toys"],
    [
      "WorkshopUploaderMod",
      "https://github.com/BAKAOLC/STS2-WorkshopUploaderMod",
    ],
  ]) {
    const credit = references
      .locator(".credit-row")
      .filter({ has: page.getByText(name, { exact: true }) });
    assert.equal(
      await credit
        .getByRole("link", { name: "主页", exact: true })
        .getAttribute("href"),
      url,
    );
    assert.equal(
      await credit.getByRole("link", { name: "许可", exact: true }).count(),
      1,
    );
  }
  await page.setViewportSize({ width: 860, height: 796 });
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth,
    ),
    false,
  );
  await page.screenshot({ path: path.join(output, "about.png") });
  checks.push("两个参考项目名称、说明、主页和许可入口，窄窗无水平溢出");
  assert.deepEqual(errors, []);
  const report = {
    output,
    checks,
    errors,
    evidence: "浏览器合成 DOM 事件；未执行本地入库或资源管理器拖放",
  };
  writeFileSync(
    path.join(output, "report.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report));
} catch (error) {
  await page
    .screenshot({ path: path.join(output, "failed.png") })
    .catch(() => {});
  console.error(output);
  throw error;
} finally {
  await context.close();
}
