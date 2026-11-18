// 隔离发行包的启动、文件事件和完整校验验收；原生窗口由独立桌面启动器托管。
import { observeRenderer } from "./incremental-native-frontend.mjs";
import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import {
  readFileSync,
  writeFileSync,
  existsSync,
  renameSync,
  statSync,
  utimesSync,
  unlinkSync,
  mkdirSync,
} from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import {
  prepareRound,
  probeUnavailableGame,
} from "./incremental-native-fixture.mjs";
const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const root = path.resolve(
  process.env.APMM_INCREMENTAL_WORK || "B:/APMM-next-work/incremental-044",
);
const app = path.join(root, "app");
const launcherPath = process.env.APMM_BACKGROUND_LAUNCHER;
const python = process.env.APMM_TEST_PYTHON;
const expectedVersion =
  process.env.APMM_EXPECTED_VERSION ||
  readFileSync(new URL("../VERSION", import.meta.url), "utf8").trim();
assert.ok(root.toLowerCase().startsWith("b:\\apmm-next-work\\"));
assert.ok(launcherPath && python, "需要显式指定独立桌面启动器与 Python");
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const report = [];
const note = (name, value = {}) => {
  const row = { name, ...value };
  report.push(row);
  console.log(JSON.stringify(row));
};
const logPath = path.join(app, "data/logs/astral-mod-manager.jsonl");
const logs = () =>
  existsSync(logPath)
    ? readFileSync(logPath, "utf8")
        .trim()
        .split("\n")
        .filter(Boolean)
        .map(JSON.parse)
    : [];
const rounds = process.env.APMM_NATIVE_ROUNDS?.split(",") ?? [
  "cold",
  "warm-1",
  "warm-2",
  "warm-3",
  "changes",
];

async function until(callback, timeout = 180000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const value = await callback();
    if (value) return value;
    await sleep(150);
  }
  throw Error("等待状态超时");
}

async function run(label) {
  const game = path.join(root, "game/modimg");
  const offline = prepareRound(root, label);
  const heartbeat = path.join(root, `${label}-heartbeat.jsonl`);
  const beforeLogs = logs().length;
  const launcher = spawn(
    python,
    [launcherPath, path.join(app, "astral-mod-manager.exe"), heartbeat],
    {
      cwd: app,
      windowsHide: true,
      stdio: ["pipe", "pipe", "pipe"],
      env: {
        ...process.env,
        APMM_WORK_DIR: "B:/APMM-next-work",
        TEMP: "B:/APMM-next-work/temp",
        TMP: "B:/APMM-next-work/temp",
        WEBVIEW2_USER_DATA_FOLDER: path.join(root, "webview"),
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9437",
      },
    },
  );
  launcher.stderr.on("data", (b) => process.stderr.write(b));
  const exit = new Promise((resolve) => launcher.once("exit", resolve));
  let browser;
  try {
    const native = await new Promise((resolve, reject) => {
      const lines = createInterface({ input: launcher.stdout });
      lines.once("line", (line) => {
        resolve(JSON.parse(line));
        lines.close();
      });
      launcher.once("exit", (code) =>
        reject(Error(`独立桌面启动失败 ${code}`)),
      );
    });
    await until(async () => {
      try {
        browser = await chromium.connectOverCDP("http://127.0.0.1:9437");
        return true;
      } catch {
        return false;
      }
    }, 45000);
    const page = await until(async () => {
      for (const candidate of browser
        .contexts()
        .flatMap((context) => context.pages())) {
        if (
          await candidate
            .evaluate(() => !!window.__TAURI_INTERNALS__)
            .catch(() => false)
        )
          return candidate;
      }
      return false;
    }, 45000);
    page.setDefaultTimeout(180000);
    assert.ok(
      Number.isFinite(native.createdEpochMs),
      "启动器必须提供 Windows 进程创建时间",
    );
    let previewRequests = 0;
    page.on("request", (request) => {
      if (/\b(?:thumb|vault)(?:\.localhost|:)/.test(request.url()))
        previewRequests++;
    });
    await page.locator(".overview-version-card").first().waitFor();
    const sinceProcessStart = () =>
      page.evaluate(
        (started) =>
          Math.round(performance.timeOrigin + performance.now() - started),
        native.createdEpochMs,
      );
    const readyMs = await sinceProcessStart();
    const welcome = page.getByRole("button", {
      name: "先进入工作区",
      exact: true,
    });
    if (await welcome.isVisible()) await welcome.click();
    await page.locator(".nav-item").filter({ hasText: "卡面" }).first().click();
    await page.locator(".part-row, .resource-card").first().waitFor();
    const resourceReadyMs = await sinceProcessStart();
    await observeRenderer(page);
    const invoke = (command, args = {}) =>
      page.evaluate(
        ({ command, args }) => window.__apmmProbe.invoke(command, args),
        { command, args },
      );
    const snap = () => invoke("workspace_snapshot");
    const initial = await snap();
    assert.equal(initial.version, expectedVersion);
    assert.equal(
      path.resolve(initial.dataDir).toLowerCase(),
      path.join(app, "data").toLowerCase(),
    );
    note("ready", {
      label,
      version: initial.version,
      readyMs,
      resourceReadyMs,
      createdEpochMs: native.createdEpochMs,
      webviewTimeOrigin: await page.evaluate(() => performance.timeOrigin),
      resources: initial.resources.length,
      pid: native.pid,
    });
    if (label.startsWith("warm-"))
      assert.ok(
        resourceReadyMs <= 3000,
        `暖启动资源列表超出目标：${resourceReadyMs} ms`,
      );
    const navAt = Date.now();
    await page.locator(".nav-item").filter({ hasText: "预设" }).first().click();
    await page.locator('main[data-route="presets"]').waitFor();
    await page.locator(".nav-item").filter({ hasText: "总览" }).first().click();
    await page.locator('main[data-route="overview"]').waitFor();
    note("navigation", { label, elapsedMs: Date.now() - navAt });
    await until(async () => {
      const state = await invoke("directory_sync_status");
      return (
        !state.running &&
        !state.lastError &&
        logs()
          .slice(beforeLogs)
          .some((l) => l.operation === "directory.incremental")
      );
    });
    const settled = await snap();
    if (label === "cold" || label.startsWith("warm-")) {
      assert.equal(
        settled.resources.reduce((n, r) => n + r.files.length, 0),
        806,
      );
      assert.equal(
        settled.resources.filter((r) => r.activeHash && !r.disabled).length,
        773,
      );
      writeFileSync(
        path.join(root, `${label}-snapshot.json`),
        JSON.stringify(settled),
      );
    }
    if (label.startsWith("warm-")) {
      const scans = logs()
        .slice(beforeLogs)
        .filter((l) => l.operation === "directory.incremental")
        .map((l) => JSON.parse(l.detail));
      assert.ok(scans.length);
      assert.ok(
        scans.every((l) => l.stats.contentReads === 0 && l.stats.decoded === 0),
        "暖启动不应读取或解码未变 PNG",
      );
      note("warm-scan", { label, scans });
    }
    if (offline) {
      const a = hash(readFileSync(path.join(root, "probe-a.png")));
      assert.equal(
        settled.resources.find((r) => r.key === offline[0].row.key)?.activeHash,
        null,
      );
      assert.equal(
        settled.resources.find((r) => r.key === offline[1].row.key)?.activeHash,
        a,
      );
      const library = JSON.parse(
        readFileSync(path.join(app, "data/library.json"), "utf8"),
      );
      assert.equal(
        library.externalBindings["offline-added.png"]?.hash,
        hash(readFileSync(path.join(root, "probe-b.png"))),
      );
      for (const entry of offline) writeFileSync(entry.file, entry.bytes);
      unlinkSync(path.join(game, "offline-added.png"));
      await until(async () =>
        offline.every(
          (entry) =>
            JSON.parse(
              readFileSync(path.join(app, "data/library.json"), "utf8"),
            ).externalBindings[`${entry.row.canonical}.png`]?.hash ===
            entry.row.activeHash,
        ),
      );
      note("offline-add-edit-delete");
    }
    if (label === "cache-corrupt") {
      const scans = logs()
        .slice(beforeLogs)
        .filter((l) => l.operation === "directory.incremental")
        .map((l) => JSON.parse(l.detail));
      assert.ok(scans.some((l) => l.stats.decoded >= 1600));
      assert.ok(
        settled.resources.reduce((n, r) => n + r.files.length, 0) >= 806,
      );
      note("corrupt-cache-rebuilt", { scans });
    }
    if (label === "unavailable") {
      await probeUnavailableGame(root, invoke, snap, until);
      note("unavailable-directory-preserved-state");
    }
    if (label === "changes") {
      const probe = path.join(game, "apmm-incremental-probe.png");
      const pngA = readFileSync(path.join(root, "probe-a.png"));
      const pngB = readFileSync(path.join(root, "probe-b.png"));
      const a = hash(pngA),
        b = hash(pngB);
      const contains = async (digest, name) =>
        (await snap()).resources.some((r) =>
          r.files.some(
            (f) => f.hash === digest && (!name || f.sourceName === name),
          ),
        );
      await sleep(1200);
      const refreshes = await page.evaluate(() => window.__apmmProbe.snapshots);
      writeFileSync(probe, pngA);
      await until(() => contains(a));
      note("external-create");
      const deltaState = await page.evaluate(() => ({
        snapshots: window.__apmmProbe.snapshots,
        deltas: window.__apmmProbe.deltas,
      }));
      assert.ok(
        deltaState.deltas.some((d) =>
          d.resources.some((r) => r.files.some((f) => f.hash === a)),
        ),
      );
      assert.equal(
        deltaState.snapshots,
        refreshes,
        "普通目录事件不应触发前端整库重取",
      );
      note("delta-without-snapshot-refresh", {
        events: deltaState.deltas.length,
      });
      const time = statSync(probe);
      writeFileSync(probe, pngB);
      utimesSync(probe, time.atime, time.mtime);
      await until(() => contains(b));
      note("same-size-time-edit");
      const nested = path.join(game, "incremental-subtree");
      mkdirSync(nested, { recursive: true });
      const renamed = path.join(nested, "apmm-incremental-renamed.png");
      renameSync(probe, renamed);
      await until(
        async () =>
          (await invoke("directory_sync_status")).running === false &&
          existsSync(renamed),
      );
      await sleep(1600);
      assert.ok(await contains(b));
      note("external-rename");
      const disabled = `${renamed}.disabled`;
      renameSync(renamed, disabled);
      await until(async () =>
        (await snap()).resources.some(
          (r) => r.disabled && r.files.some((f) => f.hash === b),
        ),
      );
      note("external-disable");
      writeFileSync(disabled, "invalid PNG");
      await until(
        async () => !!(await invoke("directory_sync_status")).lastError,
      );
      assert.ok(await contains(b));
      note("invalid-preserves-candidate");
      writeFileSync(disabled, pngB);
      await until(async () => {
        const state = await invoke("directory_sync_status");
        return !state.running && !state.lastError;
      });
      const cancelId = await invoke("task_execute", {
        kind: "scan",
        title: "验收取消完整扫描",
        args: {},
      });
      await sleep(250);
      await invoke("task_cancel", { id: cancelId });
      const cancelled = await until(async () =>
        (await invoke("tasks_list")).find(
          (t) =>
            t.id === cancelId &&
            ["cancelled", "failed", "completed"].includes(t.status),
        ),
      );
      assert.equal(cancelled.status, "cancelled", JSON.stringify(cancelled));
      note("full-scan-cancelled");
      const id = await invoke("task_execute", {
        kind: "scan",
        title: "验收完整扫描",
        args: {},
      });
      await until(async () => {
        const s = await invoke("directory_sync_status");
        return s.phase === "full" && s.checked > 32 && s.checked < s.total;
      });
      const applied = settled.resources.find(
        (r) => r.activeHash && !r.disabled,
      );
      const action = {
        entityId: applied.entityId,
        section: applied.section,
        canonical: applied.canonical,
        hash: null,
      };
      const applyAt = Date.now();
      await invoke("resources_apply", { items: [action] });
      const appliedFile = applied.files.find(
        (f) => f.hash === applied.activeHash,
      );
      writeFileSync(
        path.join(game, "created-during-scan.png"),
        readFileSync(appliedFile.path),
      );
      writeFileSync(disabled, pngA);
      note("apply-and-edit-during-scan", { elapsedMs: Date.now() - applyAt });
      const done = await until(
        async () =>
          (await invoke("tasks_list")).find(
            (t) =>
              t.id === id &&
              ["completed", "failed", "cancelled"].includes(t.status),
          ),
        240000,
      );
      assert.equal(done.status, "completed", JSON.stringify(done));
      note("full-scan", { status: done.status });
      assert.ok(await contains(b));
      const after = await snap();
      assert.equal(
        after.resources.find((r) => r.key === applied.key)?.activeHash,
        null,
      );
      assert.ok(await contains(applied.activeHash, "created-during-scan.png"));
      await invoke("resources_apply", {
        items: [{ ...action, hash: applied.activeHash }],
      });
      note("concurrent-scan-preserved-application");
    }
    let exitTask;
    if (label === "exit-scan") {
      exitTask = await invoke("task_execute", {
        kind: "scan",
        title: "验收扫描中退出",
        args: {},
      });
      await until(async () => {
        const s = await invoke("directory_sync_status");
        return s.phase === "full" && s.checked > 32 && s.checked < s.total;
      });
    }
    await page.screenshot({ path: path.join(root, `${label}.png`) });
    const jsMemory = await page.evaluate(
      () => performance.memory?.usedJSHeapSize ?? null,
    );
    note("settled", {
      label,
      previewRequests,
      revision: settled.revision,
      jsHeapBytes: jsMemory,
      checks: logs()
        .slice(beforeLogs)
        .filter((l) => l.operation === "directory.incremental")
        .map((l) => JSON.parse(l.detail)),
    });
    const close = page.getByRole("button", { name: "关闭", exact: true });
    if (exitTask) {
      await close.click();
      await page
        .getByText("仍有任务进行中，请在右上角任务进程中等待完成或取消任务。", {
          exact: true,
        })
        .waitFor();
      await invoke("task_cancel", { id: exitTask });
      await until(async () =>
        (await invoke("tasks_list")).some(
          (t) => t.id === exitTask && t.status === "cancelled",
        ),
      );
      note("exit-guard-and-cancel");
    }
    await close.click().catch(() => {});
    const exited = await Promise.race([
      exit.then(() => true),
      sleep(8000).then(() => false),
    ]);
    assert.ok(exited, "关闭后原生进程应自行退出");
    note("native-exit", { label });
  } finally {
    await browser?.close().catch(() => {});
    launcher.stdin.end("stop\n");
    await exit;
    const samples = readFileSync(heartbeat, "utf8")
      .trim()
      .split("\n")
      .filter(Boolean)
      .map(JSON.parse);
    note("heartbeat", {
      label,
      samples: samples.length,
      blocked: samples.filter((s) => !s.responding).length,
      maxLatencyMs: Math.max(0, ...samples.map((s) => s.latencyMs)),
    });
    assert.ok(samples.length > 0);
    assert.ok(
      samples.every((s) => s.responding && s.latencyMs <= 200),
      "窗口消息出现超过 200 ms 的阻塞",
    );
  }
  await sleep(500);
}
try {
  for (const round of rounds) await run(round);
} catch (error) {
  note("error", { message: error.stack });
  process.exitCode = 1;
} finally {
  writeFileSync(
    path.join(root, `native-${rounds.join("-")}.json`),
    JSON.stringify(report, null, 2),
  );
}
