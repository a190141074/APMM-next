// 在隔离浏览器中验证紧凑布局与任务浮层；任务夹具只注入测试页面，不写入工作区。
import { chromium } from "playwright";
import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const work = process.env.APMM_WORK_DIR || "B:/APMM-next-work";
process.env.TEMP = path.join(work, "temp");
process.env.TMP = process.env.TEMP;
const output = path.join(work, "compact-ui-validation", randomUUID());
mkdirSync(output, { recursive: true });
const executablePath = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
const context = await chromium.launchPersistentContext(
  path.join(output, "profile"),
  {
    executablePath,
    headless: true,
    viewport: { width: 1089, height: 743 },
  },
);
const page = await context.newPage();
const checks = [];
const measurements = [];
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
const panel = page.getByRole("dialog", { name: "任务进程", exact: true });
const trigger = page.getByRole("button", { name: "任务进程", exact: true });
async function selectSize(value) {
  await page.getByRole("combobox", { name: "预览尺寸", exact: true }).click();
  await page
    .getByRole("option", {
      name: { small: "小图", medium: "中图", large: "大图" }[value],
      exact: true,
    })
    .click();
}
const fixture = (id, status, extra = {}) => ({
  id,
  title: `测试任务 ${id}`,
  kind: "preview",
  status,
  phase: "下载",
  message: "处理任务中",
  done: 25,
  total: 100,
  speed: 2048,
  revision: 1,
  canCancel: ["running", "queued"].includes(status),
  canRetry: ["failed", "paused"].includes(status),
  result: null,
  ...extra,
});
async function setTasks(tasks) {
  await page.evaluate((value) => {
    window.__taskFixtures = value;
  }, tasks);
  await trigger.click();
  await panel.waitFor();
  await page.waitForFunction(
    (count) => document.querySelectorAll(".task-card").length === count,
    tasks.length,
  );
}
try {
  await context.addInitScript(() => {
    localStorage.setItem("apmm.welcomed", "true");
    localStorage.setItem("apmm.theme", "light");
    window.__taskFixtures = [];
    window.__taskCommands = [];
  });
  await page.goto("http://127.0.0.1:1420/");
  await page.locator(".nav-item").filter({ hasText: "角色" }).first().click();
  await page.locator(".resource-card").first().waitFor();
  assert.equal(
    await page.locator(".nav-label").filter({ hasText: "关于" }).count(),
    1,
  );
  assert.equal(
    await page.locator(".nav-item").filter({ hasText: "任务中心" }).count(),
    0,
  );
  assert.equal(
    await page
      .locator(".resource-page")
      .getByText(/收藏每一种风格|等待你的创作|RESOURCE LIBRARY/)
      .count(),
    0,
  );
  const themeBounds = await page
    .getByRole("button", { name: "切换主题", exact: true })
    .boundingBox();
  const triggerBounds = await trigger.boundingBox();
  assert.ok(Math.abs(triggerBounds.y - themeBounds.y) < 4);
  assert.ok(themeBounds.x - (triggerBounds.x + triggerBounds.width) < 8);
  checks.push("关于分组、任务入口位于主题按钮旁、资源宣传文案移除");
  for (const width of [1089, 900, 1360]) {
    await page.setViewportSize({ width, height: 743 });
    await page.waitForTimeout(250);
    await selectSize("large");
    const boxes = await page.evaluate(() => {
      const box = (selector) => {
        const r = document.querySelector(selector).getBoundingClientRect();
        return {
          x: r.x,
          y: r.y,
          width: r.width,
          height: r.height,
          bottom: r.bottom,
        };
      };
      return {
        toolbar: box(".resource-toolbar"),
        filter: box(".filterbar"),
        title: box(".resource-heading"),
        grid: box(".resource-scroll"),
        overflow: document.documentElement.scrollWidth > innerWidth,
      };
    });
    assert.equal(boxes.overflow, false);
    assert.ok(
      boxes.grid.y <= (width <= 1000 ? 200 : 160),
      JSON.stringify(boxes),
    );
    if (width > 1000) assert.ok(Math.abs(boxes.title.y - boxes.filter.y) < 12);
    const scan = await page
      .getByRole("button", { name: "扫描本地", exact: true })
      .boundingBox();
    assert.ok(scan.x + scan.width <= width);
    measurements.push({ width, ...boxes });
    await page.screenshot({
      path: path.join(output, `resources-${width}-light.png`),
    });
  }
  checks.push("1089、900、1360 宽度下标题和筛选紧凑且无横向溢出");
  for (const size of ["small", "medium", "large"]) {
    await selectSize(size);
    await page.waitForTimeout(150);
    const spacing = await page
      .locator(".resource-row")
      .evaluateAll((rows) =>
        rows
          .slice(0, -1)
          .map(
            (row, i) =>
              rows[i + 1].getBoundingClientRect().top -
              row.getBoundingClientRect().bottom,
          ),
      );
    assert.ok(
      spacing.length > 0 && spacing.every((gap) => Math.abs(gap) < 1),
      `${size}: ${spacing}`,
    );
  }
  checks.push(
    "小/中/大预览切换后虚拟列表重新计算行高，卡片不重叠也无多余行间空隙",
  );
  await page.setViewportSize({ width: 1089, height: 743 });
  await trigger.click();
  await panel.waitFor();
  await panel.getByText("暂无任务", { exact: true }).waitFor();
  const bounds = await panel.boundingBox();
  const anchor = await trigger.boundingBox();
  assert.ok(bounds.y >= anchor.y + anchor.height);
  assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= 1089);
  await page.screenshot({ path: path.join(output, "tasks-empty-light.png") });
  await page.keyboard.press("Escape");
  await panel.waitFor({ state: "hidden" });
  assert.equal(
    await trigger.evaluate((el) => el === document.activeElement),
    true,
  );
  await trigger.press("Enter");
  await panel.waitFor();
  await page.locator(".resource-heading").click();
  await panel.waitFor({ state: "hidden" });
  assert.match(await page.locator(".breadcrumb strong").innerText(), /角色/);
  await page.locator(".statusbar button").click();
  await panel.waitFor();
  await panel
    .getByRole("button", { name: "关闭任务进程", exact: true })
    .click();
  checks.push(
    "真实空态、向下展开、Esc 返回焦点、键盘打开、点外关闭、状态栏打开且不跳页",
  );
  // 只替换预览传输边界，让现有 hook 的轮询和 revision 合并继续实际执行。
  await page.route("**/src-ui/core/ipc/mock.ts*", async (route) => {
    const response = await route.fetch();
    const source = await response.text();
    assert.ok(source.includes("if (command === IPC.tasks_list) return [];"));
    await route.fulfill({
      response,
      body: source.replace(
        "if (command === IPC.tasks_list) return [];",
        `if (command === IPC.tasks_list) {
        if (window.__taskReadError) throw Error("测试读取失败");
        return structuredClone(window.__taskFixtures || []);
      }
      if ([IPC.task_cancel, IPC.task_pause, IPC.task_retry].includes(command)) {
        window.__taskCommands.push({ command, id: args.id });
        return;
      }`,
      ),
    });
  });
  await page.reload();
  await page.locator(".nav-item").filter({ hasText: "角色" }).first().click();
  await page.locator(".resource-card").first().waitFor();
  const snapshots = [
    fixture("01", "completed", { result: { files: 3 }, canCancel: false }),
    fixture("02", "cancelled"),
    fixture("03", "queued"),
    fixture("04", "failed"),
    fixture("05", "paused"),
    fixture("06", "running", { title: "下载原图" }),
  ];
  await setTasks(snapshots);
  const card = (id) =>
    panel
      .locator(".task-card")
      .filter({ has: page.getByRole("heading", { name: id, exact: true }) });
  assert.match(await trigger.innerText(), /2/);
  assert.equal(
    await card("下载原图").getByRole("progressbar").getAttribute("value"),
    "25",
  );
  await card("下载原图")
    .getByRole("button", { name: "暂停", exact: true })
    .click();
  await card("测试任务 05")
    .getByRole("button", { name: "继续", exact: true })
    .click();
  await card("测试任务 04")
    .getByRole("button", { name: "重试", exact: true })
    .click();
  await card("测试任务 03")
    .getByRole("button", { name: "取消", exact: true })
    .click();
  assert.deepEqual(await page.evaluate(() => window.__taskCommands), [
    { command: "task_pause", id: "06" },
    { command: "task_retry", id: "05" },
    { command: "task_retry", id: "04" },
    { command: "task_cancel", id: "03" },
  ]);
  await card("测试任务 01").getByText("查看处理结果", { exact: true }).click();
  assert.match(
    await card("测试任务 01").locator("pre").innerText(),
    /"files": 3/,
  );
  checks.push(
    "六种后端快照状态、进度和活动计数、暂停/继续/重试/取消发出正确任务 ID、结果可展开",
  );
  await page.evaluate(
    (values) => {
      window.__taskFixtures = values;
    },
    snapshots.map((t) => (t.id === "06" ? { ...t, done: 80, revision: 3 } : t)),
  );
  await page.waitForFunction(
    () => document.querySelector("progress")?.value === 80,
  );
  await page.evaluate(
    (values) => {
      window.__taskFixtures = values;
    },
    snapshots.map((t) => (t.id === "06" ? { ...t, done: 10, revision: 2 } : t)),
  );
  await page.waitForTimeout(2300);
  assert.equal(
    await card("下载原图").getByRole("progressbar").getAttribute("value"),
    "80",
  );
  await page.evaluate(
    (values) => {
      window.__taskFixtures = values;
    },
    snapshots.map((t) =>
      t.id === "06"
        ? {
            ...t,
            status: "completed",
            done: 100,
            canCancel: false,
            result: { ok: true },
            revision: 4,
          }
        : t,
    ),
  );
  await card("下载原图").getByText("已完成", { exact: true }).waitFor();
  assert.equal(await card("下载原图").getByRole("button").count(), 0);
  checks.push("轮询更新和终态控件刷新，旧 revision 不回退已显示进度");
  await page.getByRole("button", { name: "切换主题", exact: true }).click();
  await trigger.click();
  await panel.waitFor();
  await page.screenshot({ path: path.join(output, "tasks-states-dark.png") });
  await page.keyboard.press("Escape");
  await page.screenshot({ path: path.join(output, "resources-1089-dark.png") });
  await page.evaluate(
    (values) => {
      window.__taskFixtures = values;
    },
    [
      ...snapshots.map((t) =>
        t.id === "06"
          ? { ...t, revision: 5, done: 90, message: "进度信息".repeat(80) }
          : t,
      ),
      ...Array.from({ length: 20 }, (_, i) =>
        fixture(`extra-${i}`, "completed"),
      ),
    ],
  );
  await trigger.click();
  await panel.locator(".task-card").nth(25).waitFor();
  const scroll = await panel.locator(".task-popover-scroll").evaluate((el) => ({
    inner: el.scrollHeight,
    visible: el.clientHeight,
    width: el.clientWidth,
    content: el.scrollWidth,
  }));
  assert.ok(scroll.inner > scroll.visible);
  assert.ok(scroll.content <= scroll.width);
  await page.setViewportSize({ width: 900, height: 600 });
  const narrow = await panel.boundingBox();
  assert.ok(
    narrow.x >= 0 &&
      narrow.x + narrow.width <= 900 &&
      narrow.y + narrow.height <= 600,
  );
  await page.screenshot({ path: path.join(output, "tasks-narrow-dark.png") });
  checks.push("浅色/深色、多任务内部滚动、长文本换行、900×600 小窗边界");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "关闭", exact: true }).click();
  await panel.waitFor();
  assert.match(await page.locator(".toast").innerText(), /右上角任务进程/);
  checks.push("有任务时关闭窗口会展开进程小窗并提示");
  await page.evaluate(() => {
    window.__taskReadError = true;
  });
  await panel.getByRole("alert").waitFor();
  assert.ok((await panel.locator(".task-card").count()) > 0);
  await page.evaluate(() => {
    window.__taskReadError = false;
  });
  await panel.getByRole("button", { name: "重新读取", exact: true }).click();
  await panel.getByRole("alert").waitFor({ state: "hidden" });
  checks.push("读取错误保留已有任务，重新读取可恢复");
  assert.deepEqual(errors, []);
} finally {
  writeFileSync(
    "verification-compact-ui.json",
    JSON.stringify({ output, checks, measurements, errors }, null, 2),
  );
  await context.close();
}
console.log(JSON.stringify({ output, checks, measurements, errors }, null, 2));
