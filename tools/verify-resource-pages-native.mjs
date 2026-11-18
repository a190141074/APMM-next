// 使用当前发行包和真实 IPC 复核资源页；窗口、库、Mod 目录均隔离到 B 盘。
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { createInterface } from "node:readline";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createHash, randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { png } from "./management-fixtures.mjs";

const build = JSON.parse(readFileSync(process.env.APMM_BUILD_REPORT, "utf8"));
assert.equal(
  build.version,
  readFileSync(new URL("../VERSION", import.meta.url), "utf8").trim(),
);
const launcher = process.env.APMM_BACKGROUND_LAUNCHER;
const python = process.env.APMM_TEST_PYTHON;
assert.ok(launcher && python, "需要独立桌面启动器及 Python，避免可见测试窗口");
const output = path.join(
  process.env.APMM_RESOURCE_UI_WORK ||
    "B:/APMM-next-work/resource-ui-validation",
  "native",
  randomUUID(),
);
assert.ok(
  path.resolve(output).toLowerCase().startsWith("b:\\apmm-next-work\\"),
);
const app = path.join(output, "app");
const game = path.join(output, "game/modimg");
const outimg = path.join(output, "outimg fixture");
cpSync(build.payload, app, { recursive: true });
for (const record of build.files) {
  const bytes = readFileSync(path.join(app, record.path));
  assert.equal(bytes.length, record.bytes);
  assert.equal(createHash("sha256").update(bytes).digest("hex"), record.sha256);
}
for (const directory of [
  game,
  outimg,
  path.join(app, "data"),
  path.join(output, "input"),
])
  mkdirSync(directory, { recursive: true });
writeFileSync(
  path.join(app, "data/library.json"),
  JSON.stringify({
    version: 1,
    heroes: {},
    monsters: {},
    settings: {
      modimgPath: game,
      outimgPath: outimg,
      monitorDirectories: false,
      updateAutoCheck: false,
    },
  }),
);
const avatarFile = path.join(outimg, "UT_Item_Hero_101.png");
writeFileSync(avatarFile, png(75));
writeFileSync(path.join(outimg, "UT_Hero_Card_101.png"), png(76));
const inputs = ["101", "102"].map((id, i) => {
  const file = path.join(output, "input", `UT_Hero_Card_${id}.png`);
  writeFileSync(file, png(70 + i));
  return file;
});
const port = await new Promise((resolve) => {
  const server = createServer();
  server.listen(0, "127.0.0.1", () => {
    const port = server.address().port;
    server.close(() => resolve(port));
  });
});
const child = spawn(
  python,
  [
    "-X",
    "utf8",
    "-B",
    launcher,
    path.join(app, "astral-mod-manager.exe"),
    path.join(output, "heartbeat.jsonl"),
  ],
  {
    cwd: app,
    windowsHide: true,
    stdio: ["pipe", "pipe", "pipe"],
    env: {
      ...process.env,
      TEMP: "B:/APMM-next-work/temp",
      TMP: "B:/APMM-next-work/temp",
      WEBVIEW2_USER_DATA_FOLDER: path.join(output, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
  },
);
child.stderr.on("data", (bytes) => process.stderr.write(bytes));
const exited = new Promise((resolve) => child.once("exit", resolve));
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let browser, page, native;
const errors = [],
  checks = [];
async function until(check, message) {
  for (let i = 0; i < 100; i++) {
    if (await check()) return;
    await delay(200);
  }
  throw Error(message);
}
try {
  native = await new Promise((resolve, reject) => {
    const lines = createInterface({ input: child.stdout });
    lines.once("line", (line) => {
      lines.close();
      try {
        resolve(JSON.parse(line));
      } catch (error) {
        reject(error);
      }
    });
    child.once("error", reject);
    child.once("exit", (code) => reject(Error(`启动器提前退出 ${code}`)));
  });
  await until(async () => {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
      return true;
    } catch {
      return false;
    }
  }, "发行 WebView 未启动");
  await until(async () => {
    for (const candidate of browser.contexts().flatMap((c) => c.pages())) {
      if (
        await candidate
          .evaluate(() => !!window.__TAURI_INTERNALS__)
          .catch(() => false)
      ) {
        page = candidate;
        return true;
      }
    }
    return false;
  }, "IPC 未就绪");
  page.on("pageerror", (error) => errors.push(error.message));
  const call = (command, args = {}) =>
    page.evaluate(
      ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
      { command, args },
    );
  await page.getByRole("button", { name: "先进入工作区", exact: true }).click();
  const initial = await call("workspace_snapshot");
  assert.equal(initial.version, build.version);
  assert.equal(
    path.normalize(initial.dataDir),
    path.normalize(path.join(app, "data")),
  );
  const taskId = await call("task_execute", {
    kind: "images-import",
    title: "资源页隔离验收",
    args: {
      paths: inputs,
      expectedHashes: Object.fromEntries(
        inputs.map((file) => [
          file,
          createHash("sha256").update(readFileSync(file)).digest("hex"),
        ]),
      ),
    },
  });
  let task;
  await until(async () => {
    task = (await call("tasks_list")).find((t) => t.id === taskId);
    return task && !["queued", "running"].includes(task.status);
  }, "导入任务未完成");
  assert.equal(task.status, "completed", JSON.stringify(task));
  let snapshot = await call("workspace_snapshot");
  const a = snapshot.resources.find((r) => r.canonical === "UT_Hero_Card_101");
  const b = snapshot.resources.find((r) => r.canonical === "UT_Hero_Card_102");
  assert.ok(a.files.length && b.files.length);
  assert.ok(a.outimgPath);
  assert.equal(
    path.toNamespacedPath(a.outimgPath).toLowerCase(),
    path.toNamespacedPath(path.join(outimg, "UT_Hero_Card_101.png")).toLowerCase(),
  );
  assert.equal(b.outimgPath, undefined);
  await assert.rejects(
    () => call("outimg_reveal", { path: inputs[0] }),
    /目标文件不在 outimg 目录内/,
  );
  await call("resources_apply", {
    items: [
      {
        entityId: a.entityId,
        section: a.section,
        canonical: a.canonical,
        hash: a.files[0].hash,
      },
    ],
  });
  await call("resources_apply", {
    items: [
      {
        entityId: b.entityId,
        section: b.section,
        canonical: b.canonical,
        hash: b.files[0].hash,
      },
    ],
  });
  await call("mod_set_disabled", {
    args: { heroId: b.entityId, sectionKey: b.section, canonical: b.canonical },
    disabled: true,
  });
  await call("workspace_snapshot");
  await page.reload();
  await page.locator(".nav-item").filter({ hasText: "角色" }).first().click();
  await page.locator(".resource-open").first().waitFor();
  snapshot = await call("workspace_snapshot");
  const heroes = snapshot.resources.filter((r) => r.category === "heroes");
  const total = new Set(heroes.map((r) => r.entityId)).size;
  const count = () => page.locator(".heading-count").first().textContent();
  const filter = async (name) => {
    await page
      .locator(".resource-toolbar")
      .getByRole("button", { name, exact: true })
      .click();
    await delay(120);
  };
  assert.equal(Number(await count()), total);
  await filter("无 Mod");
  assert.equal(Number(await count()), total - 2);
  await filter("有 Mod");
  assert.equal(Number(await count()), 2);
  await filter("已应用");
  assert.equal(Number(await count()), 1);
  const hero = page.locator('[data-resource-view-key="101"]');
  const parts = heroes.filter((r) => r.entityId === "101").length;
  assert.match(await hero.textContent(), new RegExp(`${parts} 个部位`));
  assert.equal(await page.locator('[data-resource-view-key="102"]').count(), 0);
  checks.push(
    "真实导入、B 盘 Mod 应用和禁用后，整名角色筛选与完整部位统计正确",
  );
  const select = async (label, name) => {
    await page.getByRole("combobox", { name: label, exact: true }).click();
    await page.getByRole("option", { name, exact: true }).click();
  };
  const emptyPart = heroes.find(
    (r) => r.entityId === "101" && r.section !== a.section,
  );
  await select("资源部位", emptyPart.sectionLabel);
  await hero.locator(".resource-open").click();
  await page.getByText("当前筛选没有匹配资源", { exact: true }).waitFor();
  await page.getByRole("button", { name: "清除筛选", exact: true }).click();
  assert.equal(await page.locator(".detail-page .part-row").count(), parts);
  assert.equal(
    await page
      .getByRole("button", {
        name: "定位原图文件 UT_Hero_Card_101",
        exact: true,
      })
      .count(),
    1,
  );
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  await page.getByRole("button", { name: "紧凑模式", exact: true }).click();
  await page.locator(".hero-compact-card").first().waitFor();
  assert.equal(await page.locator(".compact-part").count(), 0);
  const heights = [];
  for (const name of ["小", "中", "大", "特大"]) {
    await select("预览尺寸", name);
    await delay(120);
    heights.push(
      await page
        .locator(".hero-compact-card .asset-image")
        .first()
        .evaluate((el) => el.getBoundingClientRect().height),
    );
  }
  assert.deepEqual(heights, [44, 56, 72, 84]);
  const avatar = hero.locator(".hero-avatar img");
  await avatar.evaluate((img) => img.decode());
  assert.match(
    decodeURIComponent(await avatar.getAttribute("src")),
    /UT_Item_Hero_101\.png/,
  );
  await hero.locator(".resource-open").click();
  assert.equal(
    await page
      .locator(".detail-page .candidate-apply .asset-image")
      .first()
      .evaluate((el) => el.getBoundingClientRect().height),
    208,
  );
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  await page.screenshot({ path: path.join(output, "heroes-compact.png") });
  checks.push("发行 WebView 角色头像、四档预览、原图定位按钮及目录边界");
  await page.evaluate(() =>
    localStorage.setItem("apmm.resource-layout.heroes", "list"),
  );
  await page.reload();
  await page.locator(".nav-item").filter({ hasText: "角色" }).first().click();
  await page.locator('.resource-scroll[data-layout="compact"]').waitFor();
  await page.locator(".nav-item").filter({ hasText: "卡面" }).first().click();
  await page.locator('.resource-scroll[data-layout="list"]').waitFor();
  await select("预览尺寸", "特大");
  await delay(120);
  assert.equal(
    await page
      .locator(".compact-part .candidate-apply .asset-image")
      .first()
      .evaluate((el) => el.getBoundingClientRect().height),
    144,
  );
  await page.locator(".nav-item").filter({ hasText: "设置" }).first().click();
  await page.getByRole("tab", { name: "目录与资源", exact: true }).waitFor();
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
  await page.screenshot({
    path: path.join(output, "settings-directories.png"),
  });
  checks.push(
    "旧角色 list 偏好迁为 compact，卡面独立默认列表且支持特大；设置默认页和顺序正确",
  );
  assert.deepEqual(errors, []);
  const report = {
    version: build.version,
    buildSha256: build.sha256,
    output,
    native,
    checks,
    errors,
    heroes: total,
    resources: heroes.length,
    game,
    dataDir: initial.dataDir,
  };
  writeFileSync(
    path.join(output, "report.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  await page
    ?.screenshot({ path: path.join(output, "failed.png") })
    .catch(() => {});
  console.error(output);
  throw error;
} finally {
  await page
    ?.evaluate(() =>
      window.__TAURI_INTERNALS__.invoke("plugin:window|close", {
        label: "main",
      }),
    )
    .catch(() => {});
  await browser?.close().catch(() => {});
  child.stdin.write("\n");
  await Promise.race([exited, delay(5000)]);
  if (child.exitCode === null) child.kill();
}
