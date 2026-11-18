// 发行 WebView 与真实 IPC 的隔离验收；Tauri 事件合成单独标记，不冒充资源管理器拖入。
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { createInterface } from "node:readline";
import {
  cpSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  existsSync,
} from "node:fs";
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
const launcher = process.env.APMM_BACKGROUND_LAUNCHER,
  python = process.env.APMM_TEST_PYTHON;
assert.ok(launcher && python, "需要独立桌面启动器与 Python，避免可见测试窗口");
const output = path.join(
  process.env.APMM_DROP_WORK || "B:/APMM-next-work/drop-validation",
  "native",
  randomUUID(),
);
assert.ok(
  path.resolve(output).toLowerCase().startsWith("b:\\apmm-next-work\\"),
);
const app = path.join(output, "app"),
  game = path.join(output, "game/modimg"),
  input = path.join(output, "input");
cpSync(build.payload, app, { recursive: true });
const digest = (file) =>
  createHash("sha256").update(readFileSync(file)).digest("hex");
for (const record of build.files) {
  const file = path.join(app, record.path);
  assert.equal(readFileSync(file).length, record.bytes);
  assert.equal(digest(file), record.sha256);
}
for (const directory of [game, input, path.join(app, "data")])
  mkdirSync(directory, { recursive: true });
writeFileSync(
  path.join(app, "data/library.json"),
  JSON.stringify({
    version: 1,
    heroes: {},
    monsters: {},
    settings: {
      modimgPath: game,
      monitorDirectories: false,
      updateAutoCheck: false,
    },
  }),
);
const make = (directory, name, color) => {
  mkdirSync(directory, { recursive: true });
  const file = path.join(directory, name);
  writeFileSync(file, png(color));
  return file;
};
const known = make(input, "UT_Hero_Card_101.png", 11);
const later = make(path.join(input, "second"), "UT_Hero_Card_101.png", 12);
const unknown = make(input, "我的图片.png", 13);
const temporary = make(input, "临时图片.png", 14);
const jpg = path.join(input, "UT_Hero_Card_101.jpg");
writeFileSync(jpg, "unsupported");
const packagePath = path.join(input, "sample.apmod");
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
const checks = [],
  errors = [];
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
    for (const candidate of browser.contexts().flatMap((c) => c.pages()))
      if (
        await candidate
          .evaluate(() => !!window.__TAURI_INTERNALS__)
          .catch(() => false)
      ) {
        page = candidate;
        return true;
      }
    return false;
  }, "IPC 未就绪");
  page.on("pageerror", (error) => errors.push(error.message));
  const call = (command, args = {}) =>
    page.evaluate(
      ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
      { command, args },
    );
  const nav = (name) =>
    page.locator(".nav-item").filter({ hasText: name }).first();
  const emit = (type, paths, position) =>
    call("plugin:event|emit_to", {
      target: { kind: "AnyLabel", label: "main" },
      event: `tauri://drag-${type}`,
      payload: { paths, position },
    });
  const runTask = async (kind, args) => {
    const id = await call("task_execute", {
      kind,
      title: "拖入隔离验收",
      args,
    });
    let task;
    await until(async () => {
      task = (await call("tasks_list")).find((t) => t.id === id);
      return task && !["queued", "running"].includes(task.status);
    }, `${kind} 任务未结束`);
    assert.equal(task.status, "completed", JSON.stringify(task));
    return task.result;
  };
  const positionOf = async (element) => {
    const box = await element.boundingBox();
    const dpr = await page.evaluate(() => window.devicePixelRatio);
    return {
      x: Math.round((box.x + box.width / 2) * dpr),
      y: Math.round((box.y + box.height / 2) * dpr),
    };
  };
  await page.getByRole("button", { name: "先进入工作区", exact: true }).click();
  const initial = await call("workspace_snapshot");
  assert.equal(initial.version, build.version);
  assert.equal(
    path.normalize(initial.dataDir),
    path.normalize(path.join(app, "data")),
  );
  const matches = await call("images_match_drop", {
    paths: [known, unknown, jpg],
  });
  assert.equal(matches[0].target.canonical, "UT_Hero_Card_101");
  assert.equal(matches[1].target, null);
  assert.equal(matches[2].target, null);
  const originalHash = digest(known);
  await nav("角色").click();
  await page.locator(".resource-open").first().waitFor();
  assert.equal(
    await page.locator(".resource-card[data-drop-resource-key]").count(),
    0,
  );
  await page.locator('[data-resource-view-key="102"] .resource-open').click();
  const wrong = page
    .locator(".part-row[data-drop-resource-key]")
    .filter({
      has: page.locator("code").filter({ hasText: /^UT_Hero_Card_102$/ }),
    })
    .first();
  await wrong.scrollIntoViewIfNeeded();
  const position = await positionOf(wrong);
  await emit("enter", [known], position);
  await page.locator(".resource-drop-hint").waitFor();
  await until(
    async () =>
      /UT_Hero_Card_101/.test(
        await page.locator(".resource-drop-hint").textContent(),
      ),
    "名称规则目标未显示",
  );
  assert.equal(
    await wrong.evaluate((e) => e.classList.contains("resource-drop-target")),
    false,
  );
  await emit("leave", undefined, undefined);
  await page.locator(".resource-drop-hint").waitFor({ state: "detached" });
  await page.evaluate(() => {
    window.__dropInvoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, args, options) => {
      const result = await window.__dropInvoke(command, args, options);
      if (command === "images_match_drop")
        await new Promise((resolve) => setTimeout(resolve, 250));
      return result;
    };
  });
  await emit("enter", [known], position);
  await emit("leave", undefined, undefined);
  await delay(400);
  assert.equal(await page.locator(".resource-drop-hint").count(), 0);
  assert.equal(await page.locator(".resource-drop-target").count(), 0);
  await page.evaluate(() => {
    window.__TAURI_INTERNALS__.invoke = window.__dropInvoke;
    delete window.__dropInvoke;
  });
  checks.push("延迟真实名称匹配响应，离开后旧响应不会恢复提示或高亮");
  await emit("enter", [known, later, known], position);
  await emit("drop", [known, later, known], position);
  await until(
    async () => existsSync(path.join(game, "UT_Hero_Card_101.png")),
    "PNG 没有直接应用",
  );
  let snapshot;
  await until(async () => {
    snapshot = await call("workspace_snapshot");
    return (
      snapshot.resources.find((r) => r.canonical === "UT_Hero_Card_101")?.files
        .length === 2
    );
  }, "同槽位候选未完整入库");
  const firstRow = snapshot.resources.find(
    (r) => r.canonical === "UT_Hero_Card_101",
  );
  assert.equal(digest(path.join(game, "UT_Hero_Card_101.png")), originalHash);
  assert.equal(firstRow.activeHash, originalHash);
  assert.equal(await page.getByRole("dialog").count(), 0);
  checks.push(
    "真实名称匹配和导入 IPC；错误落点不覆盖规则，隐藏目标仍匹配，首个应用且同槽位候选独立保存",
  );
  await until(
    async () =>
      /新入库 2/.test(
        await page
          .locator(".toast")
          .textContent()
          .catch(() => ""),
      ),
    "直接导入统计未展示",
  );
  await emit("enter", [unknown], position);
  await until(
    async () =>
      /UT_Hero_Card_102/.test(
        await page.locator(".resource-drop-hint").textContent(),
      ),
    "明确落点未提示",
  );
  assert.equal(
    await wrong.evaluate((e) => e.classList.contains("resource-drop-target")),
    true,
  );
  await emit("drop", [unknown], position);
  await until(
    async () => existsSync(path.join(game, "UT_Hero_Card_102.png")),
    "明确落点未直接导入",
  );
  assert.equal(
    digest(unknown),
    digest(path.join(game, "UT_Hero_Card_102.png")),
  );
  const lib = JSON.parse(
    readFileSync(path.join(app, "data/library.json"), "utf8"),
  );
  const entry =
    lib.heroes["102"].sections["card"].resources["UT_Hero_Card_102"].files[0];
  assert.equal(entry.sourceName, "UT_Hero_Card_102.png");
  assert.equal(entry.classification.originalName, "我的图片.png");
  checks.push(
    "无规则 PNG 定向归属，库内名称 canonical.png、原名留存，外部文件未改动",
  );
  await until(
    async () =>
      /新入库 1/.test(
        await page
          .locator(".toast")
          .textContent()
          .catch(() => ""),
      ),
    "定向导入未完成",
  );
  await wrong.locator(".candidate-add").click();
  await page.getByRole("dialog").waitFor();
  const guarded = make(input, "UT_Hero_Card_104.png", 17);
  await emit("drop", [guarded], position);
  await page.getByText("请先完成当前对话框或导入", { exact: true }).waitFor();
  assert.equal(existsSync(path.join(game, "UT_Hero_Card_104.png")), false);
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "关闭", exact: true })
    .click();
  checks.push(
    "部位的添加候选对话框打开期间拒绝另一次拖入，避免重复提交和弹窗冲突",
  );
  const blank = { x: 20, y: 20 };
  await emit("drop", [jpg], blank);
  await page
    .getByText(/不支持：.*仅支持 PNG／APMod/)
    .first()
    .waitFor();
  await emit("drop", [temporary], blank);
  await page.getByRole("dialog", { name: "选择 PNG 归属" }).waitFor();
  await page.getByRole("button", { name: "确认导入", exact: true }).click();
  await page.getByText("拖入处理完成", { exact: true }).waitFor();
  assert.match(await page.getByRole("dialog").textContent(), /已应用 0/);
  await page.getByRole("button", { name: "返回工作区", exact: true }).click();
  snapshot = await call("workspace_snapshot");
  assert.ok(
    snapshot.resources.some(
      (r) =>
        r.category === "temporary" &&
        r.files.some((f) => f.sourceName === "临时图片.png"),
    ),
  );
  checks.push("JPG 明确拒绝，只有未知图片进入归属步骤，临时图片不应用");
  await call("settings_set", { patch: { modimgPath: "" } });
  const disconnected = await runTask("images-import", {
    paths: [make(input, "UT_Hero_Card_103.png", 16)],
    dropMode: true,
  });
  assert.ok(
    disconnected[0].ok &&
      !disconnected[0].applied &&
      disconnected[0].applyError,
  );
  assert.equal(existsSync(path.join(game, "UT_Hero_Card_103.png")), false);
  await call("settings_set", { patch: { modimgPath: game } });
  checks.push("真实后台任务在未配置游戏目录时保留入库并报告应用失败");
  const exported = await runTask("package-export", {
    destination: packagePath,
    password: "drop-fixture",
    mode: "zip",
    includeApplicationState: false,
    maxVolumeBytes: 512 * 1024 * 1024,
    items: [
      {
        category: firstRow.category,
        entityId: firstRow.entityId,
        sectionKey: firstRow.section,
        canonical: firstRow.canonical,
        hash: originalHash,
      },
    ],
  });
  const packageInput = exported.volumes?.[0] || exported.path;
  assert.ok(
    packageInput && existsSync(packageInput),
    "必须读取后台返回的实际分卷路径",
  );
  await emit("drop", [make(input, "另一张未知.png", 15), packageInput], blank);
  await page.getByRole("dialog", { name: "选择 PNG 归属" }).waitFor();
  await page
    .getByRole("button", { name: "跳过图片，继续 APMod", exact: true })
    .click();
  await page.getByRole("dialog", { name: "导入加密包", exact: true }).waitFor();
  assert.match(
    await page.getByRole("dialog").textContent(),
    /sample\.part001\.apmod/,
  );
  await page
    .getByRole("dialog")
    .locator('input[type="password"]')
    .fill("drop-fixture");
  await page.getByRole("button", { name: "下一步", exact: true }).click();
  await page.getByText("相同文件", { exact: true }).waitFor();
  await until(
    () => page.getByRole("button", { name: "上一步", exact: true }).isEnabled(),
    "包识别尚未结束",
  );
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "关闭", exact: true })
    .click();
  checks.push(
    "混合拖入分流，取消未知 PNG 后真实 APMod 进入密码及预览向导，保持重复判断",
  );
  await page.getByRole("dialog").waitFor({ state: "detached" });
  await nav("设置").click();
  await page.getByRole("tab", { name: "关于", exact: true }).click();
  await page.getByText("AstralParty.Toys", { exact: true }).waitFor();
  await page.getByText("WorkshopUploaderMod", { exact: true }).waitFor();
  await page.screenshot({ path: path.join(output, "about.png") });
  assert.deepEqual(errors, []);
  const report = {
    version: build.version,
    buildSha256: build.sha256,
    output,
    native,
    checks,
    errors,
    game,
    dataDir: initial.dataDir,
    evidence:
      "真实发行 WebView/IPC，合成 Tauri 拖入事件；资源管理器实际拖放未验证",
  };
  writeFileSync(
    path.join(output, "report.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report));
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
